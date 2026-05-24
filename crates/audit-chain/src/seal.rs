use sha2::{Digest, Sha256};

use crate::canonical::{self, CanonicalEncodingError};
use crate::entry::AuditEntry;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct SealedAuditEntry {
    pub base: AuditEntry,
    pub parent_hash: Option<[u8; 32]>,
    pub entry_hash: [u8; 32],
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SealError {
    #[error(transparent)]
    CanonicalEncoding(#[from] CanonicalEncodingError),
    #[error("sequence number exhausted (u64::MAX reached)")]
    SeqExhausted,
}

#[derive(Debug)]
pub struct ChainHead {
    last_entry_hash: Option<[u8; 32]>,
    next_seq: u64,
}

impl ChainHead {
    pub fn new() -> Self {
        Self {
            last_entry_hash: None,
            next_seq: 0,
        }
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
        let parent_hash = self.last_entry_hash;
        let entry_hash = compute_entry_hash(&base, parent_hash)?;

        self.last_entry_hash = Some(entry_hash);
        self.next_seq += 1;

        Ok(SealedAuditEntry {
            base,
            parent_hash,
            entry_hash,
        })
    }
}

impl Default for ChainHead {
    fn default() -> Self {
        Self::new()
    }
}

const DOMAIN_TAG: &[u8] = b"mojave-audit-v1\x00";
const GENESIS_SENTINEL: [u8; 32] = [0u8; 32];

pub(crate) fn compute_entry_hash(
    base: &AuditEntry,
    parent_hash: Option<[u8; 32]>,
) -> Result<[u8; 32], CanonicalEncodingError> {
    let canonical = canonical::encode(base)?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_TAG);
    hasher.update(&canonical);
    hasher.update(parent_hash.unwrap_or(GENESIS_SENTINEL));
    Ok(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{Action, AuditEntryBuilder, Decision, Principal};
    use chrono::{TimeZone, Utc};

    fn sample_entry() -> AuditEntry {
        AuditEntryBuilder::new()
            .seq(0)
            .actor(Principal::System { id: "test".into() })
            .action(Action::Observed)
            .decision(Decision::Observed)
            .at(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap())
            .context(serde_json::json!({"trial": 1}))
            .build()
            .unwrap()
    }

    #[test]
    fn genesis_entry_has_no_parent() {
        let mut head = ChainHead::new();
        let sealed = head.link(sample_entry()).unwrap();
        assert_eq!(sealed.base.seq, 0);
        assert!(sealed.parent_hash.is_none());
        assert_eq!(sealed.entry_hash.len(), 32);
    }

    #[test]
    fn second_entry_chains_to_first() {
        let mut head = ChainHead::new();
        let first = head.link(sample_entry()).unwrap();
        let second = head.link(sample_entry()).unwrap();
        assert_eq!(second.base.seq, 1);
        assert_eq!(second.parent_hash, Some(first.entry_hash));
    }

    #[test]
    fn seq_advances_monotonically() {
        let mut head = ChainHead::new();
        for i in 0..5 {
            let sealed = head.link(sample_entry()).unwrap();
            assert_eq!(sealed.base.seq, i);
        }
    }

    #[test]
    fn resumed_chain_continues() {
        let known_hash = [42u8; 32];
        let mut head = ChainHead::resume(known_hash, 10);
        let sealed = head.link(sample_entry()).unwrap();
        assert_eq!(sealed.base.seq, 10);
        assert_eq!(sealed.parent_hash, Some(known_hash));
    }

    #[test]
    fn entry_hash_is_deterministic() {
        let e1 = sample_entry();
        let e2 = sample_entry();
        let h1 = compute_entry_hash(&e1, None).unwrap();
        let h2 = compute_entry_hash(&e2, None).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_parent_hash_changes_entry_hash() {
        let entry = sample_entry();
        let h_genesis = compute_entry_hash(&entry, None).unwrap();
        let h_chained = compute_entry_hash(&entry, Some([1u8; 32])).unwrap();
        assert_ne!(h_genesis, h_chained);
    }

    #[test]
    fn single_bit_context_change_changes_hash() {
        let mut e1 = sample_entry();
        let mut e2 = sample_entry();
        e1.context = serde_json::json!({"trial": 1});
        e2.context = serde_json::json!({"trial": 2});
        let h1 = compute_entry_hash(&e1, None).unwrap();
        let h2 = compute_entry_hash(&e2, None).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn sealed_entry_serde_round_trip() {
        let mut head = ChainHead::new();
        let sealed = head.link(sample_entry()).unwrap();
        let json = serde_json::to_string(&sealed).unwrap();
        let back: SealedAuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.entry_hash, sealed.entry_hash);
        assert_eq!(back.parent_hash, sealed.parent_hash);
        assert_eq!(back.base.seq, sealed.base.seq);
    }

    #[test]
    fn next_seq_accessor() {
        let head = ChainHead::new();
        assert_eq!(head.next_seq(), 0);
    }

    #[test]
    fn last_entry_hash_accessor() {
        let mut head = ChainHead::new();
        assert!(head.last_entry_hash().is_none());
        head.link(sample_entry()).unwrap();
        assert!(head.last_entry_hash().is_some());
    }

    #[test]
    fn domain_tag_is_included_in_hash() {
        let entry = sample_entry();
        let hash = compute_entry_hash(&entry, None).unwrap();
        use sha2::{Digest, Sha256};
        let canonical = crate::canonical::encode(&entry).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(b"mojave-audit-v1\x00");
        hasher.update(&canonical);
        hasher.update([0u8; 32]);
        let expected: [u8; 32] = hasher.finalize().into();
        assert_eq!(hash, expected);
    }
}
