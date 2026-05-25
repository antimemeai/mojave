#![allow(clippy::unwrap_used, clippy::expect_used)]

use audit_chain::entry::{AuditEntryBuilder, Principal};
use audit_chain::model_identity::{ModelHashMethod, ModelIdentity};
use audit_chain::seal::{ChainHead, SealedAuditEntry};
use audit_chain::verify::ChainVerifier;
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
            id: "prop-test".into(),
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
fn canonical_encoding_is_deterministic() {
    let entry = sample_entry();
    let b1 = audit_chain::canonical::encode(&entry).unwrap();
    let b2 = audit_chain::canonical::encode(&entry).unwrap();
    assert_eq!(b1, b2);
    let entry2 = sample_entry();
    let b3 = audit_chain::canonical::encode(&entry2).unwrap();
    assert_eq!(b1, b3);
}

#[test]
fn chain_monotonicity_for_100_entries() {
    let (mut head, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
    assert_eq!(genesis.seq(), 0);
    let mut prev_seq = 0u64;
    for _ in 1..100 {
        let sealed = head.link(sample_entry()).unwrap();
        assert_eq!(sealed.seq(), prev_seq + 1);
        prev_seq = sealed.seq();
    }
}

#[test]
fn honestly_constructed_chain_always_verifies_clean() {
    for n in [1, 2, 5, 10, 50] {
        let chain = build_chain(n);
        let findings = ChainVerifier::verify(&chain);
        assert!(
            findings.is_clean(),
            "chain of {n} entries should verify clean, got {:?}",
            findings.findings()
        );
    }
}

#[test]
fn genesis_sentinel_differs_from_arbitrary_parent() {
    let (_, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();

    let mut resumed_head = ChainHead::resume([1u8; 32], 0);
    let with_parent = resumed_head.link(sample_entry()).unwrap();

    assert_ne!(
        genesis.entry_hash(),
        with_parent.entry_hash(),
        "genesis sentinel and arbitrary parent must produce different hashes"
    );
}

#[test]
fn every_entry_in_chain_has_unique_hash() {
    let chain = build_chain(20);
    let mut hashes: Vec<[u8; 32]> = chain.iter().map(|e| e.entry_hash()).collect();
    let original_len = hashes.len();
    hashes.sort();
    hashes.dedup();
    assert_eq!(
        hashes.len(),
        original_len,
        "all entry hashes must be unique"
    );
}

#[test]
fn parent_hash_links_are_consistent() {
    let chain = build_chain(10);
    for i in 1..chain.len() {
        assert_eq!(
            chain[i].parent_hash(),
            Some(chain[i - 1].entry_hash()),
            "entry {i} parent_hash must equal entry {} entry_hash",
            i - 1
        );
    }
}
