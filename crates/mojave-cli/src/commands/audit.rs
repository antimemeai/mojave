use std::io::Read;
use std::path::{Path, PathBuf};

use audit_chain::entry::{AuditEntryBuilder, Principal, ResourceRef};
use audit_chain::model_identity::{ModelHashMethod, ModelIdentity};
use audit_chain::seal::{ChainHead, SealedAuditEntry};
use audit_sign::signing::{AuditSigner, KeyRef, LocalEd25519Signer, SignerKeyId};
use audit_sign::snapshot::ChainHeadSnapshot;
use sha2::{Digest, Sha256};

use crate::error::CliError;

#[derive(Debug, serde::Deserialize)]
pub struct SealInput {
    pub run_id: String,
    pub eval_name: String,
    pub date_issued: String,
    pub data_file: PathBuf,
    pub data_sha256: String,
    pub actor: ActorInput,
    pub model: ModelInput,
}

#[derive(Debug, serde::Deserialize)]
pub struct ActorInput {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ModelInput {
    pub name: String,
    pub provider: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub quantization: Option<String>,
    pub hash_method: String,
    pub hash: String,
}

#[derive(Debug, serde::Serialize)]
pub struct SealOutput {
    pub chain_tip_hash: String,
    pub chain_tip_seq: u64,
    pub entry_hash: String,
    pub data_file_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestation_cbor_b64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verifying_key_spki_b64: Option<String>,
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            use std::fmt::Write;
            let _ = write!(s, "{b:02x}");
            s
        })
}

fn hash_file(path: &Path) -> Result<String, CliError> {
    let data = std::fs::read(path)
        .map_err(|e| CliError::Audit(format!("cannot read data file {}: {e}", path.display())))?;
    let digest = Sha256::digest(&data);
    Ok(hex_encode(&digest))
}

fn parse_model_input(input: &ModelInput) -> Result<ModelIdentity, CliError> {
    let hash = hex_decode_32(&input.hash)?;
    let hash_method = match input.hash_method.as_str() {
        "WeightFile" => ModelHashMethod::WeightFile,
        "StructuredDescriptor" => ModelHashMethod::StructuredDescriptor,
        other => {
            return Err(CliError::Audit(format!(
                "unknown hash_method: {other} (expected WeightFile or StructuredDescriptor)"
            )))
        }
    };
    Ok(ModelIdentity {
        name: input.name.clone(),
        provider: input.provider.clone(),
        version: input.version.clone(),
        quantization: input.quantization.clone(),
        hash_method,
        hash,
    })
}

fn load_chain_head(audit_dir: &Path) -> Result<(ChainHead, Option<String>), CliError> {
    let head_path = audit_dir.join("chain-head.json");
    if !head_path.exists() {
        return Err(CliError::Audit(
            "no chain-head.json found; use 'seal' to create a new chain".into(),
        ));
    }
    let data = std::fs::read_to_string(&head_path)
        .map_err(|e| CliError::Audit(format!("cannot read chain head: {e}")))?;
    let state: ChainHeadState = serde_json::from_str(&data)
        .map_err(|e| CliError::Audit(format!("invalid chain head JSON: {e}")))?;
    match state.tip_hash {
        Some(hex) => {
            let bytes = hex_decode_32(&hex)?;
            Ok((ChainHead::resume(bytes, state.next_seq), state.model_hash))
        }
        None => Err(CliError::Audit("chain-head.json has no tip_hash".into())),
    }
}

fn save_chain_head(
    audit_dir: &Path,
    head: &ChainHead,
    model_hash: Option<&str>,
) -> Result<(), CliError> {
    let state = ChainHeadState {
        tip_hash: head.last_entry_hash().map(|h| hex_encode(&h)),
        next_seq: head.next_seq(),
        model_hash: model_hash.map(String::from),
    };
    let json = serde_json::to_string_pretty(&state)
        .map_err(|e| CliError::Audit(format!("cannot serialize chain head: {e}")))?;
    std::fs::write(audit_dir.join("chain-head.json"), json)
        .map_err(|e| CliError::Audit(format!("cannot write chain head: {e}")))?;
    Ok(())
}

fn append_chain_entry(audit_dir: &Path, sealed: &SealedAuditEntry) -> Result<(), CliError> {
    use std::io::Write;
    let line = serde_json::to_string(sealed)
        .map_err(|e| CliError::Audit(format!("cannot serialize chain entry: {e}")))?;
    let chain_path = audit_dir.join("chain.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&chain_path)
        .map_err(|e| CliError::Audit(format!("cannot open chain file: {e}")))?;
    writeln!(file, "{line}")
        .map_err(|e| CliError::Audit(format!("cannot write chain entry: {e}")))?;
    Ok(())
}

fn hex_decode_32(hex: &str) -> Result<[u8; 32], CliError> {
    if hex.len() != 64 {
        return Err(CliError::Audit(format!(
            "expected 64-char hex string, got {} chars",
            hex.len()
        )));
    }
    let mut out = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk).map_err(|_| CliError::Audit("invalid hex".into()))?;
        out[i] = u8::from_str_radix(s, 16)
            .map_err(|_| CliError::Audit(format!("invalid hex byte: {s}")))?;
    }
    Ok(out)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ChainHeadState {
    #[serde(skip_serializing_if = "Option::is_none")]
    tip_hash: Option<String>,
    next_seq: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_hash: Option<String>,
}

pub fn run_seal(key_file: Option<&Path>) -> Result<(), CliError> {
    let mut stdin_buf = String::new();
    std::io::stdin()
        .read_to_string(&mut stdin_buf)
        .map_err(|e| CliError::Audit(format!("cannot read stdin: {e}")))?;

    let input: SealInput = serde_json::from_str(&stdin_buf)
        .map_err(|e| CliError::Audit(format!("invalid seal input JSON: {e}")))?;

    let actual_hash = hash_file(&input.data_file)?;
    if actual_hash != input.data_sha256 {
        return Err(CliError::Audit(format!(
            "data file hash mismatch: expected {}, got {actual_hash}",
            input.data_sha256
        )));
    }

    let model = parse_model_input(&input.model)?;
    let model_hash_hex = hex_encode(&model.hash);

    let audit_dir = PathBuf::from("data/audit/chains").join(&model_hash_hex[..16]);
    std::fs::create_dir_all(&audit_dir)
        .map_err(|e| CliError::Audit(format!("cannot create audit dir: {e}")))?;

    let head_path = audit_dir.join("chain-head.json");
    let mut head = if head_path.exists() {
        let (h, stored_model_hash) = load_chain_head(&audit_dir)?;
        if let Some(stored) = &stored_model_hash {
            if *stored != model_hash_hex {
                return Err(CliError::Audit(format!(
                    "model hash mismatch: chain has {stored}, input has {model_hash_hex}"
                )));
            }
        }
        h
    } else {
        let (h, genesis) = ChainHead::new(model.clone(), chrono::Utc::now())
            .map_err(|e| CliError::Audit(format!("cannot create chain: {e}")))?;
        append_chain_entry(&audit_dir, &genesis)?;
        h
    };

    let actor = Principal {
        kind: input.actor.kind.clone(),
        id: input.actor.id.clone(),
    };

    let entry = AuditEntryBuilder::new()
        .seq(0)
        .actor(actor)
        .event("run_card.generated")
        .resource(ResourceRef::new("eval", &input.eval_name))
        .authorization("Allowed")
        .outcome("Succeeded")
        .at(chrono::Utc::now())
        .detail(serde_json::json!({
            "run_id": input.run_id,
            "eval_name": input.eval_name,
            "date_issued": input.date_issued,
            "data_file": input.data_file.to_string_lossy(),
            "data_sha256": input.data_sha256,
        }))
        .build()
        .map_err(|e| CliError::Audit(format!("cannot build audit entry: {e}")))?;

    let sealed = head
        .link(entry)
        .map_err(|e| CliError::Audit(format!("cannot seal audit entry: {e}")))?;

    append_chain_entry(&audit_dir, &sealed)?;
    save_chain_head(&audit_dir, &head, Some(&model_hash_hex))?;

    let entry_hash = hex_encode(&sealed.entry_hash());
    let chain_tip_hash = hex_encode(&sealed.entry_hash());
    let chain_tip_seq = sealed.seq();

    let (attestation_cbor_b64, verifying_key_spki_b64) = match resolve_signer(key_file)? {
        Some(signer) => {
            let snapshot = ChainHeadSnapshot::from_chain_head(&head).with_model_hash(model.hash);
            let cbor = audit_sign::attestation::build_tip_attestation(&signer, &snapshot)
                .map_err(|e| CliError::Audit(format!("attestation failed: {e}")))?;

            let att_dir = audit_dir.join("attestations");
            std::fs::create_dir_all(&att_dir)
                .map_err(|e| CliError::Audit(format!("cannot create attestations dir: {e}")))?;
            std::fs::write(att_dir.join(format!("{chain_tip_seq}.cbor")), &cbor)
                .map_err(|e| CliError::Audit(format!("cannot write attestation: {e}")))?;

            let spki = signer
                .verifying_key_spki_der()
                .map_err(|e| CliError::Audit(format!("cannot export public key: {e}")))?;
            let pubkey_path = PathBuf::from("data/audit/pubkey.spki.der");
            if let Some(parent) = pubkey_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| CliError::Audit(format!("cannot create dir: {e}")))?;
            }
            std::fs::write(&pubkey_path, &spki)
                .map_err(|e| CliError::Audit(format!("cannot write public key: {e}")))?;

            use base64::Engine;
            let b64_cbor = base64::engine::general_purpose::STANDARD.encode(&cbor);
            let b64_spki = base64::engine::general_purpose::STANDARD.encode(&spki);
            (Some(b64_cbor), Some(b64_spki))
        }
        None => (None, None),
    };

    let output = SealOutput {
        chain_tip_hash,
        chain_tip_seq,
        entry_hash,
        data_file_hash: actual_hash,
        attestation_cbor_b64,
        verifying_key_spki_b64,
    };

    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| CliError::Audit(format!("cannot serialize output: {e}")))?;
    println!("{json}");
    Ok(())
}

fn resolve_signer(key_file: Option<&Path>) -> Result<Option<LocalEd25519Signer>, CliError> {
    if let Some(path) = key_file {
        let signer = KeyRef::FilePath {
            key_id: SignerKeyId::new("mojave-audit"),
            path: path.to_path_buf(),
        }
        .load()
        .map_err(|e| CliError::Audit(format!("cannot load signing key: {e}")))?;
        return Ok(Some(signer));
    }

    if std::env::var("MOJAVE_AUDIT_KEY").is_ok() {
        let signer = KeyRef::Env {
            key_id: SignerKeyId::new("mojave-audit"),
            var: "MOJAVE_AUDIT_KEY".into(),
        }
        .load()
        .map_err(|e| CliError::Audit(format!("cannot load signing key from env: {e}")))?;
        return Ok(Some(signer));
    }

    Ok(None)
}

pub fn run_emit(blob_file: Option<&Path>, audit_dir: Option<&Path>) -> Result<(), CliError> {
    let audit_path = audit_dir.unwrap_or(Path::new("data/audit"));

    let mut stdin_buf = String::new();
    std::io::stdin()
        .read_to_string(&mut stdin_buf)
        .map_err(|e| CliError::Audit(format!("cannot read stdin: {e}")))?;

    let event: audit_events::AuditEvent = serde_json::from_str(&stdin_buf)
        .map_err(|e| CliError::Audit(format!("invalid event JSON: {e}")))?;

    let model = ModelIdentity {
        name: "unknown".into(),
        provider: "unknown".into(),
        version: None,
        quantization: None,
        hash_method: ModelHashMethod::StructuredDescriptor,
        hash: [0u8; 32],
    };

    let mut emitter = audit_emit::emitter::Emitter::open(audit_path, model)
        .map_err(|e| CliError::Audit(format!("cannot open emitter: {e}")))?;

    let sealed = if let Some(blob_path) = blob_file {
        let blob_data = std::fs::read(blob_path)
            .map_err(|e| CliError::Audit(format!("cannot read blob file: {e}")))?;
        emitter
            .emit_with_blob(event, &blob_data, "application/octet-stream")
            .map_err(|e| CliError::Audit(format!("emit failed: {e}")))?
    } else {
        emitter
            .emit(event)
            .map_err(|e| CliError::Audit(format!("emit failed: {e}")))?
    };

    let output = serde_json::json!({
        "seq": sealed.seq(),
        "entry_hash": hex_encode(&sealed.entry_hash()),
        "event": sealed.base().event,
    });

    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| CliError::Audit(format!("cannot serialize output: {e}")))?;
    println!("{json}");
    Ok(())
}

pub fn run_gc(audit_dir: Option<&Path>) -> Result<(), CliError> {
    let audit_path = audit_dir.unwrap_or(Path::new("data/audit"));

    let result = audit_recover::gc::gc_blobs(audit_path)
        .map_err(|e| CliError::Audit(format!("GC failed: {e}")))?;

    let output = serde_json::json!({
        "blobs_scanned": result.blobs_scanned,
        "blobs_referenced": result.blobs_referenced,
        "blobs_deleted": result.blobs_deleted,
    });

    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| CliError::Audit(format!("cannot serialize output: {e}")))?;
    println!("{json}");
    Ok(())
}

pub fn run_verify(chain_path: Option<&Path>) -> Result<(), CliError> {
    let chain_file = chain_path.unwrap_or(Path::new("data/audit/chain.jsonl"));
    if !chain_file.exists() {
        return Err(CliError::Audit(format!(
            "chain file not found: {}",
            chain_file.display()
        )));
    }

    let contents = std::fs::read_to_string(chain_file)
        .map_err(|e| CliError::Audit(format!("cannot read chain file: {e}")))?;

    let entries: Vec<SealedAuditEntry> = contents
        .lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str(line).map_err(|e| CliError::Audit(format!("line {}: {e}", i + 1)))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let findings = audit_chain::verify::ChainVerifier::verify(&entries);

    let model_info = audit_chain::verify::ChainVerifier::model_identity(&entries);

    let output = serde_json::json!({
        "entries_verified": entries.len(),
        "is_clean": findings.is_clean(),
        "findings": findings.findings().iter().map(|f| format!("{f:?}")).collect::<Vec<_>>(),
        "model": model_info.map(|mi| serde_json::json!({
            "name": mi.name,
            "provider": mi.provider,
            "hash": hex_encode(&mi.hash),
        })),
    });

    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| CliError::Audit(format!("cannot serialize output: {e}")))?;
    println!("{json}");

    if !findings.is_clean() {
        return Err(CliError::Audit("chain verification found issues".into()));
    }
    Ok(())
}
