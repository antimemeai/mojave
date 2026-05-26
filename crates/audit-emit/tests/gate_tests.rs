#![allow(clippy::unwrap_used, clippy::expect_used)]

use audit_chain::model_identity::{ModelHashMethod, ModelIdentity};
use audit_emit::emitter::Emitter;
use audit_emit::gate::AuditGate;
use audit_events::*;
use std::collections::BTreeMap;
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

#[test]
fn gate_resolve_emits_event_and_returns_inner() {
    let dir = tempdir().unwrap();
    let mut emitter = Emitter::open(dir.path(), sample_model()).unwrap();

    let gate = AuditGate::new(
        42u64,
        EventKind::EvalCompleted,
        ResourceRef {
            kind: "eval".into(),
            id: "test".into(),
        },
        Outcome::Succeeded,
    );

    let value = gate
        .resolve(
            &mut emitter,
            Principal {
                kind: "System".into(),
                id: "test".into(),
            },
            BTreeMap::new(),
            serde_json::json!({}),
        )
        .unwrap();

    assert_eq!(value, 42);
    assert_eq!(emitter.chain_head().next_seq(), 2);
}

#[test]
#[should_panic(expected = "AuditGate dropped without resolution")]
fn gate_drop_without_resolve_panics_in_debug() {
    let _gate = AuditGate::new(
        "leaked value",
        EventKind::EvalStarted,
        ResourceRef {
            kind: "eval".into(),
            id: "test".into(),
        },
        Outcome::Succeeded,
    );
}

#[test]
fn gate_event_kind_accessor() {
    let gate = AuditGate::new(
        (),
        EventKind::PodCreated,
        ResourceRef {
            kind: "pod".into(),
            id: "test".into(),
        },
        Outcome::Succeeded,
    );
    assert_eq!(gate.event_kind(), EventKind::PodCreated);

    let dir = tempdir().unwrap();
    let mut emitter = Emitter::open(dir.path(), sample_model()).unwrap();
    gate.resolve(
        &mut emitter,
        Principal {
            kind: "System".into(),
            id: "test".into(),
        },
        BTreeMap::new(),
        serde_json::json!({}),
    )
    .unwrap();
}
