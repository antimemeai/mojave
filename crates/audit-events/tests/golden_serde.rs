#![allow(clippy::unwrap_used, clippy::expect_used)]

use audit_events::EventKind;

#[test]
fn all_event_kinds_serialize_to_dot_notation() {
    let expected = vec![
        (EventKind::EvalStarted, "eval.started"),
        (EventKind::EvalCompleted, "eval.completed"),
        (EventKind::EvalFailed, "eval.failed"),
        (EventKind::PodCreated, "pod.created"),
        (EventKind::PodReady, "pod.ready"),
        (EventKind::PodTerminated, "pod.terminated"),
        (EventKind::EndpointVerified, "endpoint.verified"),
        (EventKind::DatasetLoaded, "dataset.loaded"),
        (EventKind::DatasetCached, "dataset.cached"),
        (EventKind::ModelLoaded, "model.loaded"),
        (EventKind::ScoringCompleted, "scoring.completed"),
        (EventKind::RunCardGenerated, "run_card.generated"),
        (EventKind::RunCardSealed, "run_card.sealed"),
        (EventKind::KeyGenerated, "key.generated"),
        (EventKind::KeyLoaded, "key.loaded"),
        (EventKind::ChainVerified, "chain.verified"),
        (EventKind::ChainGenesis, "chain.genesis"),
        (EventKind::AttestationCreated, "attestation.created"),
        (EventKind::ConfigChanged, "config.changed"),
        (EventKind::CircuitBreakerTripped, "circuit_breaker.tripped"),
        (EventKind::CircuitBreakerReset, "circuit_breaker.reset"),
    ];

    for (kind, expected_str) in &expected {
        let json = serde_json::to_string(kind).unwrap();
        assert_eq!(json, format!("\"{expected_str}\""), "serialize {kind:?}");

        let back: EventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(&back, kind, "round-trip {kind:?}");
    }
}

#[test]
fn all_variants_covered_by_all() {
    assert_eq!(EventKind::all().len(), 21);
}

#[test]
fn unknown_event_kind_rejected() {
    let result: Result<EventKind, _> = serde_json::from_str("\"bogus.event\"");
    assert!(result.is_err());
}

#[test]
fn audit_event_round_trip() {
    use audit_events::{AuditEvent, Authorization, Outcome, Principal, ResourceRef};
    use std::collections::BTreeMap;

    let event = AuditEvent {
        envelope_version: 1,
        at: chrono::Utc::now(),
        monotonic_ns: Some(123456789),
        actor: Principal {
            kind: "System".into(),
            id: "test-agent".into(),
        },
        trace_id: None,
        event: EventKind::EvalStarted,
        resource: ResourceRef {
            kind: "eval".into(),
            id: "arc_challenge".into(),
        },
        authorization: Authorization::Allowed,
        outcome: Outcome::Succeeded,
        tags: BTreeMap::from([("run_id".into(), "RUN-001".into())]),
        detail: serde_json::json!({"model": "Qwen2.5-7B"}),
        blob_ref: None,
    };

    let json = serde_json::to_string(&event).unwrap();
    let back: AuditEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(back.event, EventKind::EvalStarted);
    assert_eq!(back.envelope_version, 1);
}

#[test]
fn validate_tags_enforces_limits() {
    use audit_events::validate_tags;
    use std::collections::BTreeMap;

    let mut tags = BTreeMap::new();
    for i in 0..33 {
        tags.insert(format!("key{i}"), "value".into());
    }
    assert!(validate_tags(&tags, 32, 256).is_err());

    let mut tags = BTreeMap::new();
    tags.insert("ok".into(), "x".repeat(257));
    assert!(validate_tags(&tags, 32, 256).is_err());

    let mut tags = BTreeMap::new();
    tags.insert("k\u{00e9}y".into(), "value".into());
    assert!(validate_tags(&tags, 32, 256).is_err());
}

#[test]
fn outcome_failed_carries_error() {
    use audit_events::Outcome;
    let o = Outcome::Failed {
        error: "disk full".into(),
    };
    let json = serde_json::to_string(&o).unwrap();
    assert!(json.contains("disk full"));
    let back: Outcome = serde_json::from_str(&json).unwrap();
    assert_eq!(back, o);
}
