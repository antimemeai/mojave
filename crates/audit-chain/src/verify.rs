use crate::canonical::CanonicalEncodingError;
use crate::model_identity::ModelIdentity;
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
    MissingGenesis,
    ChainedAtIndexZero,
    GenesisHashMismatch,
    DuplicateGenesis {
        index: usize,
    },
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

    pub fn has_missing_genesis(&self) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f, ChainFinding::MissingGenesis))
    }

    pub fn has_chained_at_index_zero(&self) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f, ChainFinding::ChainedAtIndexZero))
    }

    pub fn has_genesis_hash_mismatch(&self) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f, ChainFinding::GenesisHashMismatch))
    }

    pub fn has_duplicate_genesis_at_index(&self, index: usize) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f, ChainFinding::DuplicateGenesis { index: i } if *i == index))
    }
}

pub struct ChainVerifier;

impl ChainVerifier {
    pub fn verify(entries: &[SealedAuditEntry]) -> ChainFindings {
        let mut findings = Vec::new();

        if entries.is_empty() {
            findings.push(ChainFinding::MissingGenesis);
            return ChainFindings { findings };
        }

        for (i, entry) in entries.iter().enumerate() {
            if i == 0 {
                match entry {
                    SealedAuditEntry::Chained { .. } => {
                        findings.push(ChainFinding::ChainedAtIndexZero);
                    }
                    SealedAuditEntry::Genesis {
                        base,
                        model_identity,
                        entry_hash,
                    } => {
                        if let Ok(recomputed) = compute_genesis_hash(base, model_identity.hash) {
                            if recomputed != *entry_hash {
                                findings.push(ChainFinding::GenesisHashMismatch);
                            }
                        }
                    }
                }
            } else {
                match entry {
                    SealedAuditEntry::Genesis { .. } => {
                        findings.push(ChainFinding::DuplicateGenesis { index: i });
                    }
                    SealedAuditEntry::Chained {
                        base,
                        parent_hash,
                        entry_hash,
                    } => {
                        let expected_parent = entries[i - 1].entry_hash();
                        if *parent_hash != expected_parent {
                            findings.push(ChainFinding::ParentHashMismatch {
                                index: i,
                                seq: base.seq,
                            });
                        }

                        if let Ok(recomputed) = compute_chained_hash(base, *parent_hash) {
                            if recomputed != *entry_hash {
                                findings.push(ChainFinding::EntryHashMismatch {
                                    index: i,
                                    seq: base.seq,
                                });
                            }
                        }
                    }
                }
            }

            if i > 0 {
                let expected_seq = entries[i - 1].seq() + 1;
                if entry.seq() != expected_seq {
                    findings.push(ChainFinding::SeqDiscontinuity {
                        index: i,
                        expected: expected_seq,
                        actual: entry.seq(),
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

    pub fn model_identity(entries: &[SealedAuditEntry]) -> Option<&ModelIdentity> {
        match entries.first()? {
            SealedAuditEntry::Genesis { model_identity, .. } => Some(model_identity),
            _ => None,
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
    }

    #[test]
    fn empty_chain_has_missing_genesis() {
        let result = ChainVerifier::verify(&[]);
        assert!(!result.is_clean());
        assert!(result.has_missing_genesis());
    }

    #[test]
    fn chained_at_index_zero_detected() {
        let chain = build_chain(2);
        let forged = SealedAuditEntry::Chained {
            base: chain[0].base().clone(),
            parent_hash: [0u8; 32],
            entry_hash: chain[0].entry_hash(),
        };
        let tampered = vec![forged, chain[1].clone()];
        let result = ChainVerifier::verify(&tampered);
        assert!(result.has_chained_at_index_zero());
    }

    #[test]
    fn genesis_hash_mismatch_detected() {
        let mut chain = build_chain(2);
        if let SealedAuditEntry::Genesis {
            ref mut model_identity,
            ..
        } = chain[0]
        {
            model_identity.hash = [99u8; 32];
        }
        let result = ChainVerifier::verify(&chain);
        assert!(result.has_genesis_hash_mismatch());
    }

    #[test]
    fn duplicate_genesis_detected() {
        let mut chain = build_chain(3);
        let (_, extra_genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        chain[2] = extra_genesis;
        let result = ChainVerifier::verify(&chain);
        assert!(result.has_duplicate_genesis_at_index(2));
    }

    #[test]
    fn tampered_context_detected() {
        let mut chain = build_chain(4);
        if let SealedAuditEntry::Chained { ref mut base, .. } = chain[2] {
            base.detail = serde_json::json!({"tampered": true});
        }
        let result = ChainVerifier::verify(&chain);
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
        assert!(result.has_parent_hash_mismatch_at_seq(2));
    }

    #[test]
    fn seq_discontinuity_detected() {
        let mut chain = build_chain(4);
        if let SealedAuditEntry::Chained { ref mut base, .. } = chain[2] {
            base.seq = 99;
        }
        let result = ChainVerifier::verify(&chain);
        assert!(result.has_seq_discontinuity_at_index(2));
    }

    #[test]
    fn recompute_entry_hash_matches_all() {
        let chain = build_chain(4);
        for entry in &chain {
            let recomputed = ChainVerifier::recompute_entry_hash(entry).unwrap();
            assert_eq!(recomputed, entry.entry_hash());
        }
    }

    #[test]
    fn model_identity_accessor_works() {
        let chain = build_chain(3);
        let mi = ChainVerifier::model_identity(&chain).unwrap();
        assert_eq!(mi.name, "test-model");
        assert_eq!(mi.hash, [42u8; 32]);
    }

    #[test]
    fn model_identity_returns_none_for_empty() {
        assert!(ChainVerifier::model_identity(&[]).is_none());
    }

    #[test]
    fn model_identity_returns_none_when_no_genesis() {
        let chain = build_chain(2);
        let forged = SealedAuditEntry::Chained {
            base: chain[0].base().clone(),
            parent_hash: [0u8; 32],
            entry_hash: chain[0].entry_hash(),
        };
        assert!(ChainVerifier::model_identity(&[forged]).is_none());
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
    fn genesis_only_chain_verifies_clean() {
        let chain = build_chain(1);
        assert!(chain[0].is_genesis());
        let result = ChainVerifier::verify(&chain);
        assert!(result.is_clean());
    }
}
