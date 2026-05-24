#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use audit_emit::config::EmitterConfig;
use audit_emit::emitter::Emitter;
use audit_events::*;
use chrono::Utc;
use tempfile::tempdir;

fn sample_event(kind: EventKind) -> AuditEvent {
    AuditEvent {
        envelope_version: 1,
        at: Utc::now(),
        monotonic_ns: Some(42),
        actor: Principal {
            kind: "System".into(),
            id: "test".into(),
        },
        trace_id: None,
        event: kind,
        resource: ResourceRef {
            kind: "eval".into(),
            id: "arc".into(),
        },
        authorization: Authorization::Allowed,
        outcome: Outcome::Succeeded,
        tags: BTreeMap::new(),
        detail: serde_json::json!({"run_id": "RUN-001"}),
        blob_ref: None,
    }
}

#[test]
fn emit_single_event() {
    let dir = tempdir().unwrap();
    let mut emitter = Emitter::open(dir.path()).unwrap();
    let sealed = emitter.emit(sample_event(EventKind::EvalStarted)).unwrap();
    assert_eq!(sealed.base.seq, 0);
    assert_eq!(sealed.base.event, "eval.started");
}

#[test]
fn emit_multiple_events_chain_correctly() {
    let dir = tempdir().unwrap();
    let mut emitter = Emitter::open(dir.path()).unwrap();

    let s0 = emitter.emit(sample_event(EventKind::EvalStarted)).unwrap();
    let s1 = emitter
        .emit(sample_event(EventKind::EvalCompleted))
        .unwrap();

    assert_eq!(s0.base.seq, 0);
    assert_eq!(s1.base.seq, 1);
    assert_eq!(s1.parent_hash, Some(s0.entry_hash));
}

#[test]
fn emit_persists_to_jsonl() {
    let dir = tempdir().unwrap();
    let mut emitter = Emitter::open(dir.path()).unwrap();
    emitter.emit(sample_event(EventKind::EvalStarted)).unwrap();
    emitter
        .emit(sample_event(EventKind::EvalCompleted))
        .unwrap();
    drop(emitter);

    let chain_path = dir.path().join("chain.jsonl");
    let contents = std::fs::read_to_string(&chain_path).unwrap();
    assert_eq!(contents.lines().count(), 2);
}

#[test]
fn reopen_continues_chain() {
    let dir = tempdir().unwrap();

    {
        let mut emitter = Emitter::open(dir.path()).unwrap();
        emitter.emit(sample_event(EventKind::EvalStarted)).unwrap();
    }

    {
        let mut emitter = Emitter::open(dir.path()).unwrap();
        let sealed = emitter
            .emit(sample_event(EventKind::EvalCompleted))
            .unwrap();
        assert_eq!(sealed.base.seq, 1);
    }
}

#[test]
fn detail_auto_promoted_to_blob() {
    let dir = tempdir().unwrap();
    let config = EmitterConfig {
        detail_max_bytes: 10,
        ..EmitterConfig::default()
    };
    let mut emitter = Emitter::open(dir.path()).unwrap().with_config(config);

    let mut event = sample_event(EventKind::EvalStarted);
    event.detail = serde_json::json!({"large_data": "x".repeat(100)});

    let sealed = emitter.emit(event).unwrap();
    assert!(sealed.base.blob_ref.is_some());
    assert!(sealed.base.detail.get("__promoted_to_blob").is_some());

    let blob_dir = dir.path().join("blobs");
    assert!(blob_dir.exists());
    assert!(std::fs::read_dir(&blob_dir).unwrap().count() > 0);
}

#[test]
fn emit_with_explicit_blob() {
    let dir = tempdir().unwrap();
    let mut emitter = Emitter::open(dir.path()).unwrap();
    let blob_data = b"large payload data here";

    let sealed = emitter
        .emit_with_blob(
            sample_event(EventKind::RunCardSealed),
            blob_data,
            "application/octet-stream",
        )
        .unwrap();

    assert!(sealed.base.blob_ref.is_some());
}

#[test]
fn tag_validation_rejects_too_many() {
    let dir = tempdir().unwrap();
    let mut emitter = Emitter::open(dir.path()).unwrap();

    let mut event = sample_event(EventKind::EvalStarted);
    for i in 0..33 {
        event.tags.insert(format!("key{i}"), "value".into());
    }

    let result = emitter.emit(event);
    assert!(result.is_err());
}
