use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::canonical::{self, CanonicalEncodingError};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct AuditEntry {
    pub seq: u64,
    pub actor: Principal,
    pub action: Action,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<ResourceRef>,
    pub decision: Decision,
    pub at: DateTime<Utc>,
    pub context: serde_json::Value,
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
#[serde(tag = "kind")]
#[non_exhaustive]
pub enum Principal {
    Actor { id: String, role: String },
    System { id: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "name")]
#[non_exhaustive]
pub enum Action {
    Started,
    Completed,
    Failed,
    Observed,
    Custom(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
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
#[non_exhaustive]
pub enum Decision {
    Allowed,
    Denied,
    Observed,
    Completed,
    Failed,
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
    actor: Option<Principal>,
    action: Option<Action>,
    resource: Option<ResourceRef>,
    decision: Option<Decision>,
    at: Option<DateTime<Utc>>,
    context: Option<serde_json::Value>,
}

impl AuditEntryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seq(mut self, seq: u64) -> Self {
        self.seq = Some(seq);
        self
    }

    pub fn actor(mut self, actor: Principal) -> Self {
        self.actor = Some(actor);
        self
    }

    pub fn action(mut self, action: Action) -> Self {
        self.action = Some(action);
        self
    }

    pub fn resource(mut self, resource: ResourceRef) -> Self {
        self.resource = Some(resource);
        self
    }

    pub fn decision(mut self, decision: Decision) -> Self {
        self.decision = Some(decision);
        self
    }

    pub fn at(mut self, at: DateTime<Utc>) -> Self {
        self.at = Some(at);
        self
    }

    pub fn context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }

    pub fn build(self) -> Result<AuditEntry, BuildError> {
        Ok(AuditEntry {
            seq: self.seq.ok_or(BuildError::MissingField("seq"))?,
            actor: self.actor.ok_or(BuildError::MissingField("actor"))?,
            action: self.action.ok_or(BuildError::MissingField("action"))?,
            resource: self.resource,
            decision: self.decision.ok_or(BuildError::MissingField("decision"))?,
            at: self.at.ok_or(BuildError::MissingField("at"))?,
            context: self
                .context
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
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
            .actor(Principal::System { id: "test".into() })
            .action(Action::Observed)
            .decision(Decision::Observed)
            .at(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap())
            .context(serde_json::json!({"trial": 1}))
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
            .actor(Principal::System { id: "test".into() })
            .action(Action::Observed)
            .decision(Decision::Observed)
            .at(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap())
            .build()
            .unwrap();
        assert!(entry.context.is_object());
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
            .actor(Principal::Actor {
                id: "agent-1".into(),
                role: "evaluator".into(),
            })
            .action(Action::Completed)
            .resource(ResourceRef::new("task", "task-42"))
            .decision(Decision::Completed)
            .at(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap())
            .build()
            .unwrap();
        assert!(with_resource.resource.is_some());
    }

    #[test]
    fn principal_variants_serialize() {
        let actor = Principal::Actor {
            id: "a".into(),
            role: "r".into(),
        };
        let json = serde_json::to_value(&actor).unwrap();
        assert_eq!(json["kind"], "Actor");

        let system = Principal::System { id: "s".into() };
        let json = serde_json::to_value(&system).unwrap();
        assert_eq!(json["kind"], "System");
    }

    #[test]
    fn action_variants_serialize() {
        let custom = Action::Custom("ingest".into());
        let json = serde_json::to_value(&custom).unwrap();
        assert_eq!(json["kind"], "Custom");
        assert_eq!(json["name"], "ingest");
    }
}
