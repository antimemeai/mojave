#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use audit_chain::model_identity::{ModelHashMethod, ModelIdentity};
use audit_chain::verify::ChainVerifier;
use audit_emit::config::EmitterConfig;
use audit_emit::emitter::Emitter;
use audit_events::*;
use chrono::Utc;
use tempfile::tempdir;

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

fn event(kind: EventKind) -> AuditEvent {
    AuditEvent {
        envelope_version: 1,
        at: Utc::now(),
        monotonic_ns: Some(0),
        actor: Principal {
            kind: "System".into(),
            id: "integration-test".into(),
        },
        trace_id: None,
        event: kind,
        resource: ResourceRef {
            kind: "eval".into(),
            id: "arc_challenge".into(),
        },
        authorization: Authorization::Allowed,
        outcome: Outcome::Succeeded,
        tags: BTreeMap::from([("test".into(), "true".into())]),
        detail: serde_json::json!({"integration": true}),
        blob_ref: None,
    }
}

#[test]
fn full_lifecycle_emit_verify_reopen_gc() {
    let dir = tempdir().unwrap();

    // Phase 1: Emit a sequence of events (genesis is auto-created)
    {
        let mut emitter = Emitter::open(dir.path(), sample_model()).unwrap();
        emitter.emit(event(EventKind::EvalStarted)).unwrap();
        emitter.emit(event(EventKind::DatasetLoaded)).unwrap();
        emitter.emit(event(EventKind::ModelLoaded)).unwrap();
        emitter.emit(event(EventKind::ScoringCompleted)).unwrap();
        emitter.emit(event(EventKind::EvalCompleted)).unwrap();
    }

    // Phase 2: Verify the chain (genesis + 5 events = 6 entries)
    let chain_path = dir.path().join("chain.jsonl");
    let contents = std::fs::read_to_string(&chain_path).unwrap();
    let entries: Vec<audit_chain::seal::SealedAuditEntry> = contents
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();

    assert_eq!(entries.len(), 6);
    let findings = ChainVerifier::verify(&entries);
    assert!(
        findings.is_clean(),
        "chain should verify clean: {:?}",
        findings.findings()
    );

    // Phase 3: Reopen and continue
    {
        let mut emitter = Emitter::open(dir.path(), sample_model()).unwrap();
        let sealed = emitter.emit(event(EventKind::RunCardGenerated)).unwrap();
        assert_eq!(sealed.seq(), 6);
    }

    // Phase 4: Verify extended chain
    let contents = std::fs::read_to_string(&chain_path).unwrap();
    let entries: Vec<audit_chain::seal::SealedAuditEntry> = contents
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(entries.len(), 7);
    let findings = ChainVerifier::verify(&entries);
    assert!(findings.is_clean());

    // Phase 5: GC (no orphan blobs expected, but should run clean)
    let result = audit_recover::gc::gc_blobs(dir.path()).unwrap();
    assert_eq!(result.blobs_deleted, 0);
}

#[test]
fn auto_promotion_blob_survives_gc() {
    let dir = tempdir().unwrap();
    let config = EmitterConfig {
        detail_max_bytes: 10,
        ..EmitterConfig::default()
    };

    {
        let mut emitter = Emitter::open(dir.path(), sample_model())
            .unwrap()
            .with_config(config);
        let mut ev = event(EventKind::EvalCompleted);
        ev.detail = serde_json::json!({"large_payload": "x".repeat(200)});
        emitter.emit(ev).unwrap();
    }

    let result = audit_recover::gc::gc_blobs(dir.path()).unwrap();
    assert_eq!(result.blobs_deleted, 0);
    assert!(result.blobs_referenced > 0);
}
