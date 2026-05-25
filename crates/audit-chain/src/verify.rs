use crate::canonical::CanonicalEncodingError;
use crate::seal::{compute_chained_hash, compute_genesis_hash, SealedAuditEntry};

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
            if i == 0 && entry.parent_hash().is_some() {
                findings.push(ChainFinding::NonGenesisAtIndexZero);
            }

            let expected_seq = if i == 0 {
                entries.first().map_or(0, |e| e.seq())
            } else {
                entries[i - 1].seq() + 1
            };
            if i > 0 && entry.seq() != expected_seq {
                findings.push(ChainFinding::SeqDiscontinuity {
                    index: i,
                    expected: expected_seq,
                    actual: entry.seq(),
                });
            }

            if i > 0 {
                let expected_parent = entries[i - 1].entry_hash();
                if entry.parent_hash() != Some(expected_parent) {
                    findings.push(ChainFinding::ParentHashMismatch {
                        index: i,
                        seq: entry.seq(),
                    });
                }
            }

            let recomputed = match entry {
                SealedAuditEntry::Genesis {
                    base,
                    model_identity,
                    ..
                } => compute_genesis_hash(base, model_identity.hash),
                SealedAuditEntry::Chained {
                    base, parent_hash, ..
                } => compute_chained_hash(base, *parent_hash),
            };
            if let Ok(hash) = recomputed {
                if hash != entry.entry_hash() {
                    findings.push(ChainFinding::EntryHashMismatch {
                        index: i,
                        seq: entry.seq(),
                    });
                }
            }
        }

        ChainFindings { findings }
    }

    pub fn recompute_entry_hash(
        entry: &SealedAuditEntry,
    ) -> Result<[u8; 32], CanonicalEncodingError> {
        match entry {
            SealedAuditEntry::Genesis {
                base,
                model_identity,
                ..
            } => compute_genesis_hash(base, model_identity.hash),
            SealedAuditEntry::Chained {
                base, parent_hash, ..
            } => compute_chained_hash(base, *parent_hash),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{AuditEntryBuilder, Principal};
    use crate::model_identity::{ModelHashMethod, ModelIdentity};
    use crate::seal::ChainHead;
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

    fn sample_entry() -> crate::entry::AuditEntry {
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

    fn build_chain(n: usize) -> Vec<SealedAuditEntry> {
        assert!(n > 0, "chain must have at least 1 entry (genesis)");
        let (mut head, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let mut entries = vec![genesis];
        for _ in 1..n {
            entries.push(head.link(sample_entry()).unwrap());
        }
        entries
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
        if let SealedAuditEntry::Chained { ref mut base, .. } = chain[2] {
            base.detail = serde_json::json!({"tampered": true});
        }
        let result = ChainVerifier::verify(&chain);
        assert!(!result.is_clean());
        assert!(result.has_entry_hash_mismatch_at_seq(2));
    }

    #[test]
    fn tampered_parent_hash_detected() {
        let mut chain = build_chain(4);
        if let SealedAuditEntry::Chained {
            ref mut parent_hash,
            ..
        } = chain[2]
        {
            *parent_hash = [0u8; 32];
        }
        let result = ChainVerifier::verify(&chain);
        assert!(!result.is_clean());
        assert!(result.has_parent_hash_mismatch_at_seq(2));
    }

    #[test]
    fn seq_discontinuity_detected() {
        let mut chain = build_chain(4);
        if let SealedAuditEntry::Chained { ref mut base, .. } = chain[2] {
            base.seq = 99;
        }
        let result = ChainVerifier::verify(&chain);
        assert!(!result.is_clean());
        assert!(result.has_seq_discontinuity_at_index(2));
    }

    #[test]
    fn non_genesis_at_zero_detected() {
        let chain = build_chain(2);
        let forged = SealedAuditEntry::Chained {
            base: chain[0].base().clone(),
            parent_hash: [1u8; 32],
            entry_hash: chain[0].entry_hash(),
        };
        let tampered = vec![forged, chain[1].clone()];
        let result = ChainVerifier::verify(&tampered);
        assert!(result.has_non_genesis_at_index_zero());
    }

    #[test]
    fn recompute_entry_hash_matches() {
        let chain = build_chain(3);
        for entry in &chain {
            let recomputed = ChainVerifier::recompute_entry_hash(entry).unwrap();
            assert_eq!(recomputed, entry.entry_hash());
        }
    }

    #[test]
    fn multiple_findings_can_accumulate() {
        let mut chain = build_chain(4);
        if let SealedAuditEntry::Chained { ref mut base, .. } = chain[1] {
            base.detail = serde_json::json!({"tampered": true});
        }
        if let SealedAuditEntry::Chained { ref mut base, .. } = chain[3] {
            base.seq = 99;
        }
        let result = ChainVerifier::verify(&chain);
        assert!(!result.is_clean());
        assert!(result.findings().len() >= 2);
    }

    #[test]
    fn genesis_with_model_hash_verifies_clean() {
        let chain = build_chain(1);
        assert!(chain[0].is_genesis());
        let result = ChainVerifier::verify(&chain);
        assert!(result.is_clean());
        let recomputed = ChainVerifier::recompute_entry_hash(&chain[0]).unwrap();
        assert_eq!(recomputed, chain[0].entry_hash());
    }
}
