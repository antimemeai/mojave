use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::canonical::{self, CanonicalEncodingError};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct AuditEntry {
    pub seq: u64,
    pub envelope_version: u32,
    pub at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monotonic_ns: Option<u64>,
    pub actor: Principal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<[u8; 16]>,
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<ResourceRef>,
    pub authorization: String,
    pub outcome: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tags: BTreeMap<String, String>,
    #[serde(default = "default_detail")]
    pub detail: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_ref: Option<BlobRef>,
}

fn default_detail() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

impl AuditEntry {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CanonicalEncodingError> {
        canonical::encode(self)
    }

    pub fn canonical_digest(&self) -> Result<[u8; 32], CanonicalEncodingError> {
        let bytes = self.canonical_bytes()?;
        Ok(Sha256::digest(&bytes).into())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Principal {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceRef {
    pub kind: String,
    pub id: String,
}

impl ResourceRef {
    pub fn new(kind: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlobRef {
    pub hash: [u8; 32],
    pub location: BlobLocation,
    pub size_bytes: u64,
    pub content_type: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BlobLocation {
    File { path: PathBuf },
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BuildError {
    #[error("missing required field: {0}")]
    MissingField(&'static str),
}

#[derive(Debug, Default)]
pub struct AuditEntryBuilder {
    seq: Option<u64>,
    envelope_version: u32,
    at: Option<DateTime<Utc>>,
    monotonic_ns: Option<u64>,
    actor: Option<Principal>,
    trace_id: Option<[u8; 16]>,
    event: Option<String>,
    resource: Option<ResourceRef>,
    authorization: Option<String>,
    outcome: Option<String>,
    tags: BTreeMap<String, String>,
    detail: serde_json::Value,
    blob_ref: Option<BlobRef>,
}

impl AuditEntryBuilder {
    pub fn new() -> Self {
        Self {
            envelope_version: 1,
            detail: serde_json::Value::Object(serde_json::Map::new()),
            ..Self::default()
        }
    }

    pub fn seq(mut self, seq: u64) -> Self {
        self.seq = Some(seq);
        self
    }

    pub fn at(mut self, at: DateTime<Utc>) -> Self {
        self.at = Some(at);
        self
    }

    pub fn monotonic_ns(mut self, ns: u64) -> Self {
        self.monotonic_ns = Some(ns);
        self
    }

    pub fn actor(mut self, actor: Principal) -> Self {
        self.actor = Some(actor);
        self
    }

    pub fn trace_id(mut self, id: [u8; 16]) -> Self {
        self.trace_id = Some(id);
        self
    }

    pub fn event(mut self, event: impl Into<String>) -> Self {
        self.event = Some(event.into());
        self
    }

    pub fn resource(mut self, resource: ResourceRef) -> Self {
        self.resource = Some(resource);
        self
    }

    pub fn authorization(mut self, auth: impl Into<String>) -> Self {
        self.authorization = Some(auth.into());
        self
    }

    pub fn outcome(mut self, outcome: impl Into<String>) -> Self {
        self.outcome = Some(outcome.into());
        self
    }

    pub fn tags(mut self, tags: BTreeMap<String, String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn detail(mut self, detail: serde_json::Value) -> Self {
        self.detail = detail;
        self
    }

    pub fn blob_ref(mut self, blob_ref: BlobRef) -> Self {
        self.blob_ref = Some(blob_ref);
        self
    }

    pub fn build(self) -> Result<AuditEntry, BuildError> {
        Ok(AuditEntry {
            seq: self.seq.ok_or(BuildError::MissingField("seq"))?,
            envelope_version: self.envelope_version,
            at: self.at.ok_or(BuildError::MissingField("at"))?,
            monotonic_ns: self.monotonic_ns,
            actor: self.actor.ok_or(BuildError::MissingField("actor"))?,
            trace_id: self.trace_id,
            event: self.event.ok_or(BuildError::MissingField("event"))?,
            resource: self.resource,
            authorization: self
                .authorization
                .ok_or(BuildError::MissingField("authorization"))?,
            outcome: self.outcome.ok_or(BuildError::MissingField("outcome"))?,
            tags: self.tags,
            detail: self.detail,
            blob_ref: self.blob_ref,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample_entry(seq: u64) -> AuditEntry {
        AuditEntryBuilder::new()
            .seq(seq)
            .actor(Principal {
                kind: "System".into(),
                id: "test".into(),
            })
            .event("eval.started")
            .authorization("Allowed")
            .outcome("Succeeded")
            .at(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap())
            .detail(serde_json::json!({"trial": 1}))
            .build()
            .unwrap()
    }

    #[test]
    fn builder_requires_mandatory_fields() {
        let result = AuditEntryBuilder::new().build();
        assert!(matches!(result, Err(BuildError::MissingField("seq"))));
    }

    #[test]
    fn builder_succeeds_with_all_fields() {
        let entry = sample_entry(0);
        assert_eq!(entry.seq, 0);
    }

    #[test]
    fn builder_defaults_context_to_empty_object() {
        let entry = AuditEntryBuilder::new()
            .seq(0)
            .actor(Principal {
                kind: "System".into(),
                id: "test".into(),
            })
            .event("eval.started")
            .authorization("Allowed")
            .outcome("Succeeded")
            .at(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap())
            .build()
            .unwrap();
        assert!(entry.detail.is_object());
    }

    #[test]
    fn canonical_bytes_deterministic() {
        let e1 = sample_entry(0);
        let e2 = sample_entry(0);
        assert_eq!(e1.canonical_bytes().unwrap(), e2.canonical_bytes().unwrap());
    }

    #[test]
    fn canonical_digest_is_32_bytes() {
        let entry = sample_entry(0);
        let digest = entry.canonical_digest().unwrap();
        assert_eq!(digest.len(), 32);
    }

    #[test]
    fn different_seq_produces_different_digest() {
        let d0 = sample_entry(0).canonical_digest().unwrap();
        let d1 = sample_entry(1).canonical_digest().unwrap();
        assert_ne!(d0, d1);
    }

    #[test]
    fn serde_round_trip() {
        let entry = sample_entry(42);
        let json = serde_json::to_string(&entry).unwrap();
        let back: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.seq, 42);
    }

    #[test]
    fn resource_ref_optional() {
        let entry = sample_entry(0);
        assert!(entry.resource.is_none());

        let with_resource = AuditEntryBuilder::new()
            .seq(0)
            .actor(Principal {
                kind: "evaluator".into(),
                id: "agent-1".into(),
            })
            .event("eval.completed")
            .resource(ResourceRef::new("task", "task-42"))
            .authorization("Allowed")
            .outcome("Succeeded")
            .at(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap())
            .build()
            .unwrap();
        assert!(with_resource.resource.is_some());
    }

    #[test]
    fn principal_serializes() {
        let p = Principal {
            kind: "System".into(),
            id: "test".into(),
        };
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["kind"], "System");
        assert_eq!(json["id"], "test");
    }
}
