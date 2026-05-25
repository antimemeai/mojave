use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::canonical::{self, CanonicalEncodingError};
use crate::entry::{AuditEntry, AuditEntryBuilder, Principal};
use crate::model_identity::ModelIdentity;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum SealedAuditEntry {
    Genesis {
        base: AuditEntry,
        model_identity: ModelIdentity,
        entry_hash: [u8; 32],
    },
    Chained {
        base: AuditEntry,
        parent_hash: [u8; 32],
        entry_hash: [u8; 32],
    },
}

impl SealedAuditEntry {
    pub fn base(&self) -> &AuditEntry {
        match self {
            Self::Genesis { base, .. } | Self::Chained { base, .. } => base,
        }
    }

    pub fn entry_hash(&self) -> [u8; 32] {
        match self {
            Self::Genesis { entry_hash, .. } | Self::Chained { entry_hash, .. } => *entry_hash,
        }
    }

    pub fn seq(&self) -> u64 {
        self.base().seq
    }

    pub fn is_genesis(&self) -> bool {
        matches!(self, Self::Genesis { .. })
    }

    pub fn parent_hash(&self) -> Option<[u8; 32]> {
        match self {
            Self::Genesis { .. } => None,
            Self::Chained { parent_hash, .. } => Some(*parent_hash),
        }
    }

    pub fn model_identity(&self) -> Option<&ModelIdentity> {
        match self {
            Self::Genesis { model_identity, .. } => Some(model_identity),
            Self::Chained { .. } => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SealError {
    #[error(transparent)]
    CanonicalEncoding(#[from] CanonicalEncodingError),
    #[error("sequence number exhausted (u64::MAX reached)")]
    SeqExhausted,
    #[error("model identity hash is all zeros")]
    ZeroModelHash,
    #[error("genesis entry construction failed")]
    GenesisConstruction,
}

#[derive(Debug)]
pub struct ChainHead {
    last_entry_hash: Option<[u8; 32]>,
    next_seq: u64,
}

impl ChainHead {
    pub fn new(
        model: ModelIdentity,
        at: DateTime<Utc>,
    ) -> Result<(Self, SealedAuditEntry), SealError> {
        if model.is_zero_hash() {
            return Err(SealError::ZeroModelHash);
        }

        let model_hash = model.hash;

        let detail = serde_json::to_value(&model).map_err(|_| SealError::GenesisConstruction)?;

        let genesis_base = AuditEntryBuilder::new()
            .seq(0)
            .actor(Principal {
                kind: "System".into(),
                id: "chain.init".into(),
            })
            .event("chain.genesis")
            .authorization("Allowed")
            .outcome("Succeeded")
            .at(at)
            .detail(detail)
            .build()
            .map_err(|_| SealError::GenesisConstruction)?;

        let entry_hash = compute_genesis_hash(&genesis_base, model_hash)?;

        let genesis = SealedAuditEntry::Genesis {
            base: genesis_base,
            model_identity: model,
            entry_hash,
        };

        let head = Self {
            last_entry_hash: Some(entry_hash),
            next_seq: 1,
        };

        Ok((head, genesis))
    }

    pub fn resume(last_entry_hash: [u8; 32], next_seq: u64) -> Self {
        Self {
            last_entry_hash: Some(last_entry_hash),
            next_seq,
        }
    }

    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }

    pub fn last_entry_hash(&self) -> Option<[u8; 32]> {
        self.last_entry_hash
    }

    pub fn link(&mut self, mut base: AuditEntry) -> Result<SealedAuditEntry, SealError> {
        if self.next_seq == u64::MAX {
            return Err(SealError::SeqExhausted);
        }

        base.seq = self.next_seq;
        let parent_hash = self.last_entry_hash.ok_or(SealError::GenesisConstruction)?;
        let entry_hash = compute_chained_hash(&base, parent_hash)?;

        self.last_entry_hash = Some(entry_hash);
        self.next_seq += 1;

        Ok(SealedAuditEntry::Chained {
            base,
            parent_hash,
            entry_hash,
        })
    }
}

const DOMAIN_TAG: &[u8] = b"mojave-audit-v1\x00";

pub(crate) fn compute_genesis_hash(
    base: &AuditEntry,
    model_hash: [u8; 32],
) -> Result<[u8; 32], CanonicalEncodingError> {
    let canonical = canonical::encode(base)?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_TAG);
    hasher.update(&canonical);
    hasher.update(model_hash);
    Ok(hasher.finalize().into())
}

pub(crate) fn compute_chained_hash(
    base: &AuditEntry,
    parent_hash: [u8; 32],
) -> Result<[u8; 32], CanonicalEncodingError> {
    let canonical = canonical::encode(base)?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_TAG);
    hasher.update(&canonical);
    hasher.update(parent_hash);
    Ok(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_identity::ModelHashMethod;
    use chrono::TimeZone;

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

    fn fixed_time() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
    }

    fn sample_entry() -> AuditEntry {
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
    fn genesis_entry_created_by_new() {
        let (head, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        assert!(genesis.is_genesis());
        assert_eq!(genesis.seq(), 0);
        assert_eq!(head.next_seq(), 1);
        assert!(head.last_entry_hash().is_some());
    }

    #[test]
    fn genesis_has_model_identity() {
        let model = sample_model();
        let (_, genesis) = ChainHead::new(model.clone(), fixed_time()).unwrap();
        let mi = genesis.model_identity().unwrap();
        assert_eq!(mi.name, "test-model");
        assert_eq!(mi.hash, model.hash);
    }

    #[test]
    fn genesis_has_no_parent_hash() {
        let (_, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        assert!(genesis.parent_hash().is_none());
    }

    #[test]
    fn genesis_event_is_chain_genesis() {
        let (_, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        assert_eq!(genesis.base().event, "chain.genesis");
    }

    #[test]
    fn genesis_hash_uses_model_hash() {
        let (_, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let recomputed = compute_genesis_hash(genesis.base(), sample_model().hash).unwrap();
        assert_eq!(recomputed, genesis.entry_hash());
    }

    #[test]
    fn different_model_hash_changes_genesis_hash() {
        let model_a = sample_model();
        let mut model_b = sample_model();
        model_b.hash = [43u8; 32];
        let (_, genesis_a) = ChainHead::new(model_a, fixed_time()).unwrap();
        let (_, genesis_b) = ChainHead::new(model_b, fixed_time()).unwrap();
        assert_ne!(genesis_a.entry_hash(), genesis_b.entry_hash());
    }

    #[test]
    fn zero_model_hash_rejected() {
        let model = ModelIdentity {
            hash: [0u8; 32],
            ..sample_model()
        };
        let result = ChainHead::new(model, fixed_time());
        assert!(matches!(result, Err(SealError::ZeroModelHash)));
    }

    #[test]
    fn second_entry_chains_to_genesis() {
        let (mut head, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let second = head.link(sample_entry()).unwrap();
        assert!(!second.is_genesis());
        assert_eq!(second.seq(), 1);
        assert_eq!(second.parent_hash(), Some(genesis.entry_hash()));
    }

    #[test]
    fn chained_entry_has_parent_hash() {
        let (mut head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let chained = head.link(sample_entry()).unwrap();
        assert!(chained.parent_hash().is_some());
    }

    #[test]
    fn seq_advances_monotonically() {
        let (mut head, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        assert_eq!(genesis.seq(), 0);
        for i in 1..5u64 {
            let sealed = head.link(sample_entry()).unwrap();
            assert_eq!(sealed.seq(), i);
        }
    }

    #[test]
    fn resumed_chain_continues() {
        let known_hash = [99u8; 32];
        let mut head = ChainHead::resume(known_hash, 10);
        let sealed = head.link(sample_entry()).unwrap();
        assert_eq!(sealed.seq(), 10);
        assert_eq!(sealed.parent_hash(), Some(known_hash));
    }

    #[test]
    fn entry_hash_is_deterministic() {
        let (_, g1) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let (_, g2) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        assert_eq!(g1.entry_hash(), g2.entry_hash());
    }

    #[test]
    fn different_parent_hash_changes_entry_hash() {
        let entry = sample_entry();
        let h1 = compute_chained_hash(&entry, [1u8; 32]).unwrap();
        let h2 = compute_chained_hash(&entry, [2u8; 32]).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn single_bit_detail_change_changes_hash() {
        let mut e1 = sample_entry();
        let mut e2 = sample_entry();
        e1.detail = serde_json::json!({"trial": 1});
        e2.detail = serde_json::json!({"trial": 2});
        let h1 = compute_chained_hash(&e1, [1u8; 32]).unwrap();
        let h2 = compute_chained_hash(&e2, [1u8; 32]).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn sealed_entry_serde_round_trip_genesis() {
        let (_, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let json = serde_json::to_string(&genesis).unwrap();
        assert!(json.contains(r#""type":"Genesis"#));
        let back: SealedAuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.entry_hash(), genesis.entry_hash());
        assert!(back.is_genesis());
    }

    #[test]
    fn sealed_entry_serde_round_trip_chained() {
        let (mut head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let chained = head.link(sample_entry()).unwrap();
        let json = serde_json::to_string(&chained).unwrap();
        assert!(json.contains(r#""type":"Chained"#));
        let back: SealedAuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.entry_hash(), chained.entry_hash());
        assert_eq!(back.parent_hash(), chained.parent_hash());
    }

    #[test]
    fn domain_tag_is_included_in_genesis_hash() {
        let (_, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let canonical = crate::canonical::encode(genesis.base()).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(b"mojave-audit-v1\x00");
        hasher.update(&canonical);
        hasher.update(sample_model().hash);
        let expected: [u8; 32] = hasher.finalize().into();
        assert_eq!(genesis.entry_hash(), expected);
    }

    #[test]
    fn next_seq_accessor() {
        let (head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        assert_eq!(head.next_seq(), 1);
    }

    #[test]
    fn last_entry_hash_accessor() {
        let (head, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        assert_eq!(head.last_entry_hash(), Some(genesis.entry_hash()));
    }

    #[test]
    fn model_identity_returns_none_for_chained() {
        let (mut head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let chained = head.link(sample_entry()).unwrap();
        assert!(chained.model_identity().is_none());
    }
}
