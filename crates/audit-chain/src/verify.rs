use crate::canonical::CanonicalEncodingError;
use crate::seal::{compute_entry_hash, SealedAuditEntry};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ChainFinding {
    EntryHashMismatch {
        index: usize,
        seq: u64,
    },
    ParentHashMismatch {
        index: usize,
        seq: u64,
    },
    SeqDiscontinuity {
        index: usize,
        expected: u64,
        actual: u64,
    },
    NonGenesisAtIndexZero,
}

#[derive(Debug)]
pub struct ChainFindings {
    findings: Vec<ChainFinding>,
}

impl ChainFindings {
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    pub fn findings(&self) -> &[ChainFinding] {
        &self.findings
    }

    pub fn has_entry_hash_mismatch_at_seq(&self, seq: u64) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f, ChainFinding::EntryHashMismatch { seq: s, .. } if *s == seq))
    }

    pub fn has_parent_hash_mismatch_at_seq(&self, seq: u64) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f, ChainFinding::ParentHashMismatch { seq: s, .. } if *s == seq))
    }

    pub fn has_seq_discontinuity_at_index(&self, index: usize) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f, ChainFinding::SeqDiscontinuity { index: i, .. } if *i == index))
    }

    pub fn has_non_genesis_at_index_zero(&self) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f, ChainFinding::NonGenesisAtIndexZero))
    }
}

pub struct ChainVerifier;

impl ChainVerifier {
    pub fn verify(entries: &[SealedAuditEntry]) -> ChainFindings {
        let mut findings = Vec::new();

        for (i, entry) in entries.iter().enumerate() {
            if i == 0 && entry.parent_hash.is_some() {
                findings.push(ChainFinding::NonGenesisAtIndexZero);
            }

            let expected_seq = if i == 0 {
                entries.first().map_or(0, |e| e.base.seq)
            } else {
                entries[i - 1].base.seq + 1
            };
            if i > 0 && entry.base.seq != expected_seq {
                findings.push(ChainFinding::SeqDiscontinuity {
                    index: i,
                    expected: expected_seq,
                    actual: entry.base.seq,
                });
            }

            if i > 0 {
                let expected_parent = entries[i - 1].entry_hash;
                if entry.parent_hash != Some(expected_parent) {
                    findings.push(ChainFinding::ParentHashMismatch {
                        index: i,
                        seq: entry.base.seq,
                    });
                }
            }

            if let Ok(recomputed) = compute_entry_hash(&entry.base, entry.parent_hash) {
                if recomputed != entry.entry_hash {
                    findings.push(ChainFinding::EntryHashMismatch {
                        index: i,
                        seq: entry.base.seq,
                    });
                }
            }
        }

        ChainFindings { findings }
    }

    pub fn recompute_entry_hash(
        entry: &SealedAuditEntry,
    ) -> Result<[u8; 32], CanonicalEncodingError> {
        compute_entry_hash(&entry.base, entry.parent_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{Action, AuditEntryBuilder, Decision, Principal};
    use crate::seal::ChainHead;
    use chrono::{TimeZone, Utc};

    fn sample_entry() -> crate::entry::AuditEntry {
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

    fn build_chain(n: usize) -> Vec<SealedAuditEntry> {
        let mut head = ChainHead::new();
        (0..n).map(|_| head.link(sample_entry()).unwrap()).collect()
    }

    #[test]
    fn clean_chain_verifies_clean() {
        let chain = build_chain(4);
        let result = ChainVerifier::verify(&chain);
        assert!(result.is_clean());
        assert_eq!(result.findings().len(), 0);
    }

    #[test]
    fn empty_chain_verifies_clean() {
        let result = ChainVerifier::verify(&[]);
        assert!(result.is_clean());
    }

    #[test]
    fn tampered_context_detected() {
        let mut chain = build_chain(4);
        chain[2].base.context = serde_json::json!({"tampered": true});
        let result = ChainVerifier::verify(&chain);
        assert!(!result.is_clean());
        assert!(result.has_entry_hash_mismatch_at_seq(2));
    }

    #[test]
    fn tampered_parent_hash_detected() {
        let mut chain = build_chain(4);
        chain[2].parent_hash = Some([0u8; 32]);
        let result = ChainVerifier::verify(&chain);
        assert!(!result.is_clean());
        assert!(result.has_parent_hash_mismatch_at_seq(2));
    }

    #[test]
    fn seq_discontinuity_detected() {
        let mut chain = build_chain(4);
        chain[2].base.seq = 99;
        let result = ChainVerifier::verify(&chain);
        assert!(!result.is_clean());
        assert!(result.has_seq_discontinuity_at_index(2));
    }

    #[test]
    fn non_genesis_at_zero_detected() {
        let mut chain = build_chain(2);
        chain[0].parent_hash = Some([1u8; 32]);
        let result = ChainVerifier::verify(&chain);
        assert!(result.has_non_genesis_at_index_zero());
    }

    #[test]
    fn recompute_entry_hash_matches() {
        let chain = build_chain(3);
        for entry in &chain {
            let recomputed = ChainVerifier::recompute_entry_hash(entry).unwrap();
            assert_eq!(recomputed, entry.entry_hash);
        }
    }

    #[test]
    fn multiple_findings_can_accumulate() {
        let mut chain = build_chain(4);
        chain[1].base.context = serde_json::json!({"tampered": true});
        chain[3].base.seq = 99;
        let result = ChainVerifier::verify(&chain);
        assert!(!result.is_clean());
        assert!(result.findings().len() >= 2);
    }

    #[test]
    fn genesis_with_sentinel_verifies_clean() {
        let chain = build_chain(1);
        assert!(chain[0].parent_hash.is_none());
        let result = ChainVerifier::verify(&chain);
        assert!(result.is_clean());
        let recomputed = ChainVerifier::recompute_entry_hash(&chain[0]).unwrap();
        assert_eq!(recomputed, chain[0].entry_hash);
    }
}
