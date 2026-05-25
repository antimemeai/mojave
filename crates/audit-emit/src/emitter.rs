use std::io::Write;
use std::path::{Path, PathBuf};

use audit_chain::entry::{
    AuditEntryBuilder, BlobLocation as ChainBlobLocation, BlobRef as ChainBlobRef,
    Principal as ChainPrincipal, ResourceRef as ChainResourceRef,
};
use audit_chain::model_identity::ModelIdentity;
use audit_chain::seal::{ChainHead, SealedAuditEntry};
use audit_events::{validate_tags, AuditEvent, BlobLocation};
use audit_sign::signing::AuditSigner;
use fs2::FileExt;

use crate::blob_store::BlobStore;
use crate::config::EmitterConfig;
use crate::error::AuditError;

pub struct Emitter {
    chain: ChainHead,
    chain_path: PathBuf,
    blob_store: BlobStore,
    signer: Option<Box<dyn AuditSigner>>,
    config: EmitterConfig,
    lock_file: std::fs::File,
    audit_dir: PathBuf,
    genesis_pending: Option<SealedAuditEntry>,
}

impl Emitter {
    pub fn open(audit_dir: &Path, model: ModelIdentity) -> Result<Self, AuditError> {
        std::fs::create_dir_all(audit_dir)?;

        let lock_path = audit_dir.join(".lock");
        let lock_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)?;
        lock_file.lock_exclusive()?;

        let chain_path = audit_dir.join("chain.jsonl");
        let (chain, genesis_pending) = if chain_path.exists() {
            let result = audit_recover::replay::replay_chain_file(&chain_path)?;
            match result.chain_head {
                Some(head) => (head, None),
                None => {
                    let (head, genesis) = ChainHead::new(model, chrono::Utc::now())?;
                    (head, Some(genesis))
                }
            }
        } else {
            let (head, genesis) = ChainHead::new(model, chrono::Utc::now())?;
            (head, Some(genesis))
        };

        let blob_store = BlobStore::new(audit_dir.join("blobs"));

        let mut emitter = Self {
            chain,
            chain_path: chain_path.clone(),
            blob_store,
            signer: None,
            config: EmitterConfig::default(),
            lock_file,
            audit_dir: audit_dir.to_path_buf(),
            genesis_pending,
        };

        if let Some(genesis) = emitter.genesis_pending.take() {
            let line = serde_json::to_string(&genesis)?;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&chain_path)?;
            writeln!(file, "{line}")?;
            file.sync_all()?;
        }

        Ok(emitter)
    }

    pub fn with_signer(mut self, signer: Box<dyn AuditSigner>) -> Self {
        self.signer = Some(signer);
        self
    }

    pub fn with_config(mut self, config: EmitterConfig) -> Self {
        self.config = config;
        self
    }

    pub fn emit(&mut self, event: AuditEvent) -> Result<SealedAuditEntry, AuditError> {
        self.emit_inner(event, None)
    }

    pub fn emit_with_blob(
        &mut self,
        event: AuditEvent,
        blob: &[u8],
        content_type: &str,
    ) -> Result<SealedAuditEntry, AuditError> {
        self.emit_inner(event, Some((blob, content_type)))
    }

    fn emit_inner(
        &mut self,
        mut event: AuditEvent,
        blob: Option<(&[u8], &str)>,
    ) -> Result<SealedAuditEntry, AuditError> {
        validate_tags(
            &event.tags,
            self.config.tags_max_pairs,
            self.config.tag_value_max_bytes,
        )?;

        if let Some((data, ct)) = blob {
            let blob_ref = self.blob_store.store(data, ct)?;
            event.blob_ref = Some(audit_events::BlobRef {
                hash: blob_ref.hash,
                location: blob_ref.location,
                size_bytes: blob_ref.size_bytes,
                content_type: blob_ref.content_type,
            });
        }

        let detail_json = serde_json::to_string(&event.detail)?;
        if detail_json.len() > self.config.detail_max_bytes && event.blob_ref.is_none() {
            let blob_ref = self
                .blob_store
                .store(detail_json.as_bytes(), "application/json")?;
            event.detail = serde_json::json!({
                "__promoted_to_blob": true
            });
            event.blob_ref = Some(audit_events::BlobRef {
                hash: blob_ref.hash,
                location: blob_ref.location,
                size_bytes: blob_ref.size_bytes,
                content_type: blob_ref.content_type,
            });
        }

        let authorization_str = serde_json::to_string(&event.authorization)?;
        let outcome_str = serde_json::to_string(&event.outcome)?;

        let mut builder = AuditEntryBuilder::new()
            .seq(0)
            .at(event.at)
            .actor(ChainPrincipal {
                kind: event.actor.kind.clone(),
                id: event.actor.id.clone(),
            })
            .event(event.event.as_str())
            .authorization(authorization_str.trim_matches('"'))
            .outcome(outcome_str.trim_matches('"'))
            .tags(event.tags)
            .detail(event.detail);

        if let Some(ns) = event.monotonic_ns {
            builder = builder.monotonic_ns(ns);
        }
        if let Some(trace_id) = event.trace_id {
            builder = builder.trace_id(trace_id.0);
        }
        builder = builder.resource(ChainResourceRef::new(
            &event.resource.kind,
            &event.resource.id,
        ));
        if let Some(blob_ref) = event.blob_ref {
            builder = builder.blob_ref(ChainBlobRef {
                hash: blob_ref.hash,
                location: ChainBlobLocation::File {
                    path: match blob_ref.location {
                        BlobLocation::File { path } => path,
                    },
                },
                size_bytes: blob_ref.size_bytes,
                content_type: blob_ref.content_type,
            });
        }

        let entry = builder
            .build()
            .map_err(|e| AuditError::BlobStore(format!("entry build failed: {e}")))?;

        let sealed = self.chain.link(entry)?;

        let line = serde_json::to_string(&sealed)?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.chain_path)?;
        writeln!(file, "{line}")?;
        file.sync_all()?;

        if let Some(signer) = &self.signer {
            let snapshot = audit_sign::snapshot::ChainHeadSnapshot::from_chain_head(&self.chain);
            let cbor = audit_sign::attestation::build_tip_attestation(signer.as_ref(), &snapshot)
                .map_err(|e| AuditError::BlobStore(format!("attestation failed: {e}")))?;

            let att_dir = self.audit_dir.join("attestations");
            std::fs::create_dir_all(&att_dir)?;
            std::fs::write(att_dir.join(format!("{}.cbor", sealed.seq())), &cbor)?;
        }

        Ok(sealed)
    }

    pub fn chain_head(&self) -> &ChainHead {
        &self.chain
    }
}

impl Drop for Emitter {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.lock_file);
    }
}
