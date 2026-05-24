#![allow(clippy::unwrap_used, clippy::expect_used)]

use audit_chain::entry::{AuditEntryBuilder, Principal};
use audit_chain::seal::{ChainHead, SealedAuditEntry};
use audit_chain::verify::ChainVerifier;
use chrono::{TimeZone, Utc};

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
        .at(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap())
        .detail(serde_json::json!({"trial": 1}))
        .build()
        .unwrap()
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
    let mut head = ChainHead::new();
    let mut prev_seq = None;
    for _ in 0..100 {
        let sealed = head.link(sample_entry()).unwrap();
        if let Some(prev) = prev_seq {
            assert_eq!(sealed.base.seq, prev + 1);
        }
        prev_seq = Some(sealed.base.seq);
    }
}

#[test]
fn honestly_constructed_chain_always_verifies_clean() {
    for n in [1, 2, 5, 10, 50] {
        let mut head = ChainHead::new();
        let chain: Vec<SealedAuditEntry> =
            (0..n).map(|_| head.link(sample_entry()).unwrap()).collect();
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
    let mut genesis_head = ChainHead::new();
    let genesis = genesis_head.link(sample_entry()).unwrap();

    let mut resumed_head = ChainHead::resume([1u8; 32], 0);
    let with_parent = resumed_head.link(sample_entry()).unwrap();

    assert_ne!(
        genesis.entry_hash, with_parent.entry_hash,
        "genesis sentinel and arbitrary parent must produce different hashes"
    );
}

#[test]
fn every_entry_in_chain_has_unique_hash() {
    let mut head = ChainHead::new();
    let chain: Vec<SealedAuditEntry> = (0..20)
        .map(|_| head.link(sample_entry()).unwrap())
        .collect();
    let mut hashes: Vec<[u8; 32]> = chain.iter().map(|e| e.entry_hash).collect();
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
    let mut head = ChainHead::new();
    let chain: Vec<SealedAuditEntry> = (0..10)
        .map(|_| head.link(sample_entry()).unwrap())
        .collect();
    for i in 1..chain.len() {
        assert_eq!(
            chain[i].parent_hash,
            Some(chain[i - 1].entry_hash),
            "entry {i} parent_hash must equal entry {} entry_hash",
            i - 1
        );
    }
}
