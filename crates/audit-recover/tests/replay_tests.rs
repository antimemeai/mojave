#![allow(clippy::unwrap_used, clippy::expect_used)]

use audit_chain::entry::{AuditEntryBuilder, Principal};
use audit_chain::seal::ChainHead;
use audit_recover::replay;
use chrono::{TimeZone, Utc};
use std::io::Write;

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
        .at(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap())
        .detail(serde_json::json!({"trial": 1}))
        .build()
        .unwrap()
}

fn build_chain_jsonl(n: usize) -> String {
    let mut head = ChainHead::new();
    let mut lines = Vec::new();
    for _ in 0..n {
        let sealed = head.link(sample_entry()).unwrap();
        lines.push(serde_json::to_string(&sealed).unwrap());
    }
    lines.join("\n") + "\n"
}

#[test]
fn replay_empty_file() {
    let result = replay::replay_chain_str("").unwrap();
    assert_eq!(result.entry_count, 0);
    assert_eq!(result.chain_head.next_seq(), 0);
}

#[test]
fn replay_valid_chain() {
    let jsonl = build_chain_jsonl(5);
    let result = replay::replay_chain_str(&jsonl).unwrap();
    assert_eq!(result.entry_count, 5);
    assert_eq!(result.chain_head.next_seq(), 5);
    assert!(result.chain_head.last_entry_hash().is_some());
}

#[test]
fn replay_truncated_last_line_recovers() {
    let mut jsonl = build_chain_jsonl(3);
    jsonl.push_str("{\"truncated\": tr");
    let result = replay::replay_chain_str(&jsonl).unwrap();
    assert_eq!(result.entry_count, 3);
    assert_eq!(result.truncated_lines, 1);
}

#[test]
fn replay_corrupt_middle_line_fails() {
    let jsonl = build_chain_jsonl(3);
    let mut lines: Vec<&str> = jsonl.lines().collect();
    let corrupted = "not json at all";
    lines[1] = corrupted;
    let content = lines.join("\n");
    let result = replay::replay_chain_str(&content);
    assert!(result.is_err());
}

#[test]
fn replay_from_file() {
    let jsonl = build_chain_jsonl(4);
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "{jsonl}").unwrap();
    let result = replay::replay_chain_file(tmp.path()).unwrap();
    assert_eq!(result.entry_count, 4);
}
