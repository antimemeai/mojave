#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use eval_core::Outcome;
use eval_ingest::inspect::InspectAdapter;
use eval_ingest::{IngestAdapter, IngestSource};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn binary_outcomes_produce_10_records() {
    let result = InspectAdapter
        .ingest(IngestSource::File(fixture("inspect_binary.json")))
        .unwrap();
    assert_eq!(result.records.len(), 10);
    assert!(result.warnings.is_empty());
    assert_eq!(result.source_meta.runner_name, "inspect_ai");
    assert_eq!(result.source_meta.runner_version.as_deref(), Some("0.3.80"));
    assert_eq!(result.source_meta.log_format_version.as_deref(), Some("2"));

    for record in &result.records {
        assert_eq!(record.task_id, "security-guide");
        assert_eq!(record.agent_id, "openai/gpt-4o");
        assert!(matches!(record.outcome, Outcome::Binary(_)));
        assert!(!record.trial_id.is_nil());
        assert!(!record.run_id.is_nil());
    }

    let correct_count = result
        .records
        .iter()
        .filter(|r| r.outcome == Outcome::Binary(true))
        .count();
    let incorrect_count = result
        .records
        .iter()
        .filter(|r| r.outcome == Outcome::Binary(false))
        .count();
    assert_eq!(correct_count, 6);
    assert_eq!(incorrect_count, 4);
}

#[test]
fn model_graded_populates_judge_config() {
    let result = InspectAdapter
        .ingest(IngestSource::File(fixture("inspect_model_graded.json")))
        .unwrap();
    assert_eq!(result.records.len(), 5);

    for record in &result.records {
        let jc = record
            .judge_config
            .as_ref()
            .expect("judge_config should be populated");
        assert_eq!(jc.model, "gpt-4o-mini-2024-07-18");
        assert!(!jc.prompt_template_hash.is_empty());
        assert_eq!(jc.prompt_template_hash.len(), 64);
    }

    // All prompts are identical, so all hashes should match.
    let hashes: Vec<&str> = result
        .records
        .iter()
        .map(|r| {
            r.judge_config
                .as_ref()
                .unwrap()
                .prompt_template_hash
                .as_str()
        })
        .collect();
    assert!(hashes.windows(2).all(|w| w[0] == w[1]));
}

#[test]
fn multi_scorer_produces_record_per_scorer() {
    let result = InspectAdapter
        .ingest(IngestSource::File(fixture("inspect_multi_scorer.json")))
        .unwrap();
    assert_eq!(result.records.len(), 10);

    for record in &result.records {
        let scorer = record
            .metadata
            .get("scorer_name")
            .unwrap()
            .as_str()
            .unwrap();
        assert!(scorer == "exact_match" || scorer == "includes");
    }

    let exact_count = result
        .records
        .iter()
        .filter(|r| r.metadata.get("scorer_name").unwrap().as_str().unwrap() == "exact_match")
        .count();
    let includes_count = result
        .records
        .iter()
        .filter(|r| r.metadata.get("scorer_name").unwrap().as_str().unwrap() == "includes")
        .count();
    assert_eq!(exact_count, 5);
    assert_eq!(includes_count, 5);
}

#[test]
fn epochs_produce_distinct_trial_ids() {
    let result = InspectAdapter
        .ingest(IngestSource::File(fixture("inspect_epochs.json")))
        .unwrap();
    assert_eq!(result.records.len(), 15);

    for record in &result.records {
        let epoch = record.metadata.get("epoch").unwrap().as_u64().unwrap();
        assert!((1..=3).contains(&epoch));
    }

    let sample_ids: Vec<&str> = result
        .records
        .iter()
        .map(|r| r.metadata.get("sample_id").unwrap().as_str().unwrap())
        .collect();
    for sid in &["e1", "e2", "e3", "e4", "e5"] {
        let count = sample_ids.iter().filter(|s| *s == sid).count();
        assert_eq!(
            count, 3,
            "sample {sid} should appear 3 times (once per epoch)"
        );
    }

    let mut trial_ids: Vec<_> = result.records.iter().map(|r| r.trial_id).collect();
    trial_ids.sort();
    trial_ids.dedup();
    assert_eq!(trial_ids.len(), 15);
}

#[test]
fn malformed_samples_produce_warnings() {
    let result = InspectAdapter
        .ingest(IngestSource::File(fixture("inspect_malformed.json")))
        .unwrap();
    // x3 has null scores (silently skipped), x7 has array value (ParseError warning)
    assert_eq!(result.records.len(), 8);
    assert_eq!(result.warnings.len(), 1);

    let w = &result.warnings[0];
    assert_eq!(w.source_id.as_deref(), Some("x7"));
    assert!(matches!(w.kind, eval_ingest::WarningKind::ParseError(_)));
}

#[test]
fn content_hash_is_sha256() {
    let path = fixture("inspect_binary.json");
    let result = InspectAdapter
        .ingest(IngestSource::File(path.clone()))
        .unwrap();

    let file_bytes = std::fs::read(&path).unwrap();
    let expected_hash = hex::encode(<sha2::Sha256 as sha2::Digest>::digest(&file_bytes));
    assert_eq!(result.source_meta.content_hash, expected_hash);
}

#[test]
fn deterministic_ids_are_reproducible() {
    let result1 = InspectAdapter
        .ingest(IngestSource::File(fixture("inspect_binary.json")))
        .unwrap();
    let result2 = InspectAdapter
        .ingest(IngestSource::File(fixture("inspect_binary.json")))
        .unwrap();

    for (r1, r2) in result1.records.iter().zip(result2.records.iter()) {
        assert_eq!(r1.trial_id, r2.trial_id);
        assert_eq!(r1.run_id, r2.run_id);
    }
}
