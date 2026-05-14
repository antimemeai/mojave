#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use eval_core::Outcome;
use eval_ingest::{FieldMapping, IngestAdapter, IngestSource, JsonlAdapter, OutcomeMapping};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn basic_jsonl_with_auto_detect() {
    let adapter = JsonlAdapter::with_auto_detect();
    let result = adapter
        .ingest(IngestSource::File(fixture("basic.jsonl")))
        .unwrap();

    assert_eq!(result.records.len(), 5);
    assert!(result.warnings.is_empty());

    for record in &result.records {
        assert_eq!(record.agent_id, "gpt-4o");
        assert!(record.task_id.starts_with("math-"));
        assert!(matches!(record.outcome, Outcome::Score(_)));
    }

    if let Outcome::Score(v) = result.records[0].outcome {
        assert!((v - 0.95).abs() < 1e-10);
    } else {
        panic!("expected Score outcome");
    }
}

#[test]
fn custom_fields_with_explicit_mapping() {
    let mapping = FieldMapping {
        task_id: "item".into(),
        agent_id: "model".into(),
        outcome: OutcomeMapping::BinaryField {
            path: "pass".into(),
        },
        timestamp: Some("timestamp".into()),
        seed: None,
        run_id: None,
        task_version: None,
        agent_version: None,
    };
    let adapter = JsonlAdapter::new(mapping);
    let result = adapter
        .ingest(IngestSource::File(fixture("custom_fields.jsonl")))
        .unwrap();

    assert_eq!(result.records.len(), 3);
    assert_eq!(result.records[0].task_id, "task-A");
    assert_eq!(result.records[0].agent_id, "claude-3");
    assert_eq!(result.records[0].outcome, Outcome::Binary(true));
    assert_eq!(result.records[1].outcome, Outcome::Binary(false));
    assert_eq!(result.records[2].outcome, Outcome::Binary(true));
}

#[test]
fn missing_optional_fields_get_defaults() {
    let adapter = JsonlAdapter::with_auto_detect();
    let result = adapter
        .ingest(IngestSource::File(fixture("basic.jsonl")))
        .unwrap();

    let run_ids: Vec<_> = result.records.iter().map(|r| r.run_id).collect();
    assert!(run_ids.windows(2).all(|w| w[0] == w[1]));

    let mut trial_ids: Vec<_> = result.records.iter().map(|r| r.trial_id).collect();
    trial_ids.sort();
    trial_ids.dedup();
    assert_eq!(trial_ids.len(), 5);
}

#[test]
fn mixed_valid_invalid_lines() {
    let data = b"\n{\"task_id\":\"t1\",\"agent_id\":\"a1\",\"score\":0.5,\"timestamp\":1717200000}\nthis is not json\n{\"task_id\":\"t2\",\"agent_id\":\"a1\",\"score\":0.8,\"timestamp\":1717200001}\n";
    let adapter = JsonlAdapter::with_auto_detect();
    let result = adapter
        .ingest(IngestSource::Reader(Box::new(&data[..])))
        .unwrap();

    assert_eq!(result.records.len(), 2);
    assert_eq!(result.warnings.len(), 1);
    assert!(matches!(
        result.warnings[0].kind,
        eval_ingest::WarningKind::ParseError(_)
    ));
}

#[test]
fn jsonl_content_hash_is_deterministic() {
    let adapter = JsonlAdapter::with_auto_detect();
    let result1 = adapter
        .ingest(IngestSource::File(fixture("basic.jsonl")))
        .unwrap();
    let result2 = adapter
        .ingest(IngestSource::File(fixture("basic.jsonl")))
        .unwrap();
    assert_eq!(
        result1.source_meta.content_hash,
        result2.source_meta.content_hash
    );
}
