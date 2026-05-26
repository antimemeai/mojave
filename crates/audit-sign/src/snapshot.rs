use audit_chain::seal::ChainHead;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChainHeadSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tip_hash: Option<String>,
    pub seq_through: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_hash: Option<String>,
}

impl ChainHeadSnapshot {
    pub fn from_chain_head(head: &ChainHead) -> Self {
        Self {
            tip_hash: head.last_entry_hash().map(hex_encode),
            seq_through: if head.next_seq() == 0 {
                0
            } else {
                head.next_seq() - 1
            },
            model_hash: None,
        }
    }

    pub fn with_model_hash(mut self, model_hash: [u8; 32]) -> Self {
        self.model_hash = Some(hex_encode(model_hash));
        self
    }

    pub fn canonical_bytes(
        &self,
    ) -> Result<Vec<u8>, audit_chain::canonical::CanonicalEncodingError> {
        audit_chain::canonical::encode(self)
    }
}

fn hex_encode(bytes: [u8; 32]) -> String {
    bytes.iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use audit_chain::entry::{AuditEntryBuilder, Principal};
    use audit_chain::model_identity::{ModelHashMethod, ModelIdentity};
    use audit_chain::seal::ChainHead;
    use chrono::{TimeZone, Utc};

    fn sample_model() -> ModelIdentity {
        ModelIdentity {
            name: "test-model".into(),
            provider: "test-provider".into(),
            version: None,
            quantization: None,
            hash_method: ModelHashMethod::StructuredDescriptor,
            hash: [42u8; 32],
        }
    }

    fn fixed_time() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
    }

    fn sample_entry() -> audit_chain::entry::AuditEntry {
        AuditEntryBuilder::new()
            .seq(0)
            .actor(Principal {
                kind: "System".into(),
                id: "test".into(),
            })
            .event("eval.started")
            .authorization("Allowed")
            .outcome("Succeeded")
            .at(fixed_time())
            .detail(serde_json::json!({"trial": 1}))
            .build()
            .unwrap()
    }

    #[test]
    fn snapshot_after_genesis() {
        let (head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let snap = ChainHeadSnapshot::from_chain_head(&head);
        assert!(snap.tip_hash.is_some());
        assert_eq!(snap.tip_hash.as_ref().unwrap().len(), 64);
        assert_eq!(snap.seq_through, 0);
        assert!(snap.model_hash.is_none());
    }

    #[test]
    fn snapshot_after_entries() {
        let (mut head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        head.link(sample_entry()).unwrap();
        let snap = ChainHeadSnapshot::from_chain_head(&head);
        assert!(snap.tip_hash.is_some());
        assert_eq!(snap.seq_through, 1);
    }

    #[test]
    fn snapshot_with_model_hash() {
        let (head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let snap = ChainHeadSnapshot::from_chain_head(&head).with_model_hash(sample_model().hash);
        assert!(snap.model_hash.is_some());
        assert_eq!(snap.model_hash.as_ref().unwrap().len(), 64);
    }

    #[test]
    fn snapshot_serde_round_trip() {
        let (mut head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        head.link(sample_entry()).unwrap();
        let snap = ChainHeadSnapshot::from_chain_head(&head).with_model_hash(sample_model().hash);
        let json = serde_json::to_string(&snap).unwrap();
        let back: ChainHeadSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tip_hash, snap.tip_hash);
        assert_eq!(back.seq_through, snap.seq_through);
        assert_eq!(back.model_hash, snap.model_hash);
    }

    #[test]
    fn snapshot_omits_model_hash_when_none() {
        let (head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let snap = ChainHeadSnapshot::from_chain_head(&head);
        let json = serde_json::to_string(&snap).unwrap();
        assert!(!json.contains("model_hash"));
    }

    #[test]
    fn canonical_bytes_deterministic() {
        let (mut head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        head.link(sample_entry()).unwrap();
        let snap = ChainHeadSnapshot::from_chain_head(&head);
        let b1 = snap.canonical_bytes().unwrap();
        let b2 = snap.canonical_bytes().unwrap();
        assert_eq!(b1, b2);
    }
}
