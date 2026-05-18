//! Adapter that ingests Inspect AI `EvalLog` JSON files into [`TrialRecord`]s.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::DateTime;
use eval_core::{JudgeConfig, Outcome, TrialRecord};
use sha2::{Digest, Sha256};

use crate::id::{ulid_from_parts, ulid_from_str};
use crate::inspect_types::{InspectGradingMessage, InspectLog, InspectSample, InspectScore};
use crate::types::{
    IngestAdapter, IngestError, IngestResult, IngestSource, IngestWarning, SourceMeta, WarningKind,
};
use crate::validate::validate_record;

/// Adapter for Inspect AI (`inspect_ai`) evaluation logs.
pub struct InspectAdapter;

impl IngestAdapter for InspectAdapter {
    fn ingest(&self, source: IngestSource) -> Result<IngestResult, IngestError> {
        let (raw_bytes, original_path) = read_source(source)?;

        // Compute SHA-256 content hash.
        let hash_bytes = Sha256::digest(&raw_bytes);
        let content_hash = hex::encode(hash_bytes);

        // Deserialize the top-level log.
        let log: InspectLog = serde_json::from_slice(&raw_bytes)?;

        let now = current_unix_seconds();

        // Derive stable run_id from eval.run_id string (or fallback to hash).
        let run_id_str = log
            .eval
            .run_id
            .clone()
            .unwrap_or_else(|| content_hash.clone());
        let run_id = ulid_from_str(&run_id_str);

        // Resolve task_id and agent_id.
        let task_id = log
            .eval
            .task_id
            .clone()
            .or_else(|| log.eval.task.clone())
            .unwrap_or_default();
        let agent_id = log.eval.model.clone().unwrap_or_default();

        // Log-level metadata base (cloned once; merged into each record).
        let eval_meta = log.eval.metadata.clone();
        let log_meta = log.metadata.clone();

        let mut records: Vec<TrialRecord> = Vec::new();
        let mut warnings: Vec<IngestWarning> = Vec::new();

        let samples = log.samples.unwrap_or_default();

        for (sample_index, sample) in samples.iter().enumerate() {
            let sample_id_str = json_value_to_id_string(&sample.id);
            let epoch = sample.epoch.unwrap_or(0);

            // Ingest each scorer independently.
            let scores = sample.scores.clone().unwrap_or_default();
            if scores.is_empty() {
                // No scorers — nothing to produce for this sample.
                continue;
            }

            for (scorer_name, score) in &scores {
                let record = build_record(
                    &run_id_str,
                    run_id,
                    &task_id,
                    &agent_id,
                    sample,
                    &sample_id_str,
                    epoch,
                    scorer_name,
                    score,
                    &eval_meta,
                    &log_meta,
                );

                let record = match record {
                    Ok(r) => r,
                    Err(kind) => {
                        warnings.push(IngestWarning {
                            source_index: Some(sample_index),
                            source_id: Some(sample_id_str.clone()),
                            kind,
                        });
                        continue;
                    }
                };

                // Validate and collect warnings.
                match validate_record(
                    &record,
                    Some(sample_index),
                    Some(sample_id_str.clone()),
                    now,
                ) {
                    Ok(()) => records.push(record),
                    Err(w) => warnings.push(w),
                }
            }
        }

        if records.is_empty() {
            return Err(IngestError::NoRecordsProduced);
        }

        let runner_version = extract_runner_version(&log.metadata);

        Ok(IngestResult {
            records,
            warnings,
            source_meta: SourceMeta {
                runner_name: "inspect_ai".to_owned(),
                runner_version,
                log_format_version: log.version,
                original_path,
                content_hash,
            },
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read all bytes from an [`IngestSource`], returning (bytes, path).
fn read_source(source: IngestSource) -> Result<(Vec<u8>, Option<PathBuf>), IngestError> {
    match source {
        IngestSource::File(path) => {
            let bytes = std::fs::read(&path)?;
            Ok((bytes, Some(path)))
        }
        IngestSource::Dir(_) => {
            // Directory ingestion is not yet implemented for Inspect.
            Err(IngestError::UnrecognizedFormat)
        }
        IngestSource::Reader(mut r) => {
            let mut buf = Vec::new();
            r.read_to_end(&mut buf)?;
            Ok((buf, None))
        }
    }
}

/// Build a single [`TrialRecord`] from a (sample, scorer) pair.
///
/// Returns `Err(WarningKind)` for non-fatal mapping failures that should be
/// converted to warnings.
#[allow(clippy::too_many_arguments)]
fn build_record(
    run_id_str: &str,
    run_id: ulid::Ulid,
    task_id: &str,
    agent_id: &str,
    sample: &InspectSample,
    sample_id_str: &str,
    epoch: u32,
    scorer_name: &str,
    score: &InspectScore,
    eval_meta: &BTreeMap<String, serde_json::Value>,
    log_meta: &BTreeMap<String, serde_json::Value>,
) -> Result<TrialRecord, WarningKind> {
    // --- trial_id ---
    let trial_id = match sample.uuid.as_deref() {
        Some(uuid) => ulid_from_parts(&[uuid, scorer_name]),
        None => ulid_from_parts(&[run_id_str, sample_id_str, &epoch.to_string(), scorer_name]),
    };

    // --- outcome ---
    let outcome = map_score_value(&score.value)?;

    // --- judge_config ---
    let judge_config = extract_judge_config(score, &sample.messages);

    // --- timestamp ---
    let timestamp = parse_timestamp(sample.started_at.as_deref());

    // --- metadata ---
    let metadata = build_metadata(
        sample,
        eval_meta,
        log_meta,
        scorer_name,
        epoch,
        sample_id_str,
    );

    Ok(TrialRecord {
        trial_id,
        run_id,
        task_id: task_id.to_owned(),
        task_version: None,
        agent_id: agent_id.to_owned(),
        agent_version: None,
        judge_config,
        seed: None,
        timestamp,
        outcome,
        metadata,
    })
}

/// Map an Inspect `Score.value` JSON value to an [`Outcome`].
fn map_score_value(value: &serde_json::Value) -> Result<Outcome, WarningKind> {
    match value {
        // Boolean
        serde_json::Value::Bool(b) => Ok(Outcome::Binary(*b)),

        // String letter grades
        serde_json::Value::String(s) => match s.as_str() {
            "C" => Ok(Outcome::Binary(true)),
            "I" | "N" => Ok(Outcome::Binary(false)),
            "P" => Ok(Outcome::Score(0.5)),
            other => Err(WarningKind::ParseError(format!(
                "unrecognised string score value: {other:?}"
            ))),
        },

        // Integer or float number
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_u64() {
                match i {
                    0 => Ok(Outcome::Binary(false)),
                    1 => Ok(Outcome::Binary(true)),
                    2..=255 => Ok(Outcome::Graded(i as u8)),
                    _ => Err(WarningKind::ParseError(format!(
                        "integer score out of range [0, 255]: {i}"
                    ))),
                }
            } else if let Some(f) = n.as_f64() {
                if f.is_finite() {
                    Ok(Outcome::Score(f))
                } else {
                    Err(WarningKind::NonFiniteScore(f))
                }
            } else {
                Err(WarningKind::ParseError(format!(
                    "unrepresentable numeric score: {n}"
                )))
            }
        }

        // Object → MultiCriterion
        serde_json::Value::Object(map) => {
            let mut criteria: BTreeMap<String, f64> = BTreeMap::new();
            for (k, v) in map {
                let f = v.as_f64().ok_or_else(|| {
                    WarningKind::ParseError(format!(
                        "multi-criterion key {k:?} has non-numeric value: {v}"
                    ))
                })?;
                if !f.is_finite() {
                    return Err(WarningKind::NonFiniteCriterion {
                        key: k.clone(),
                        value: f,
                    });
                }
                criteria.insert(k.clone(), f);
            }
            Ok(Outcome::MultiCriterion(criteria))
        }

        other => Err(WarningKind::ParseError(format!(
            "unexpected score value type: {other}"
        ))),
    }
}

/// Attempt to extract a [`JudgeConfig`] from a scorer's metadata and messages.
///
/// - `model` comes from the last assistant message in the grading conversation,
///   OR from `score.metadata["grading"]["model"]`.
/// - `prompt_template_hash` is the SHA-256 of the first user message content.
fn extract_judge_config(
    score: &InspectScore,
    messages: &[InspectGradingMessage],
) -> Option<JudgeConfig> {
    // Look for grading metadata block.
    let grading_block = score
        .metadata
        .as_ref()
        .and_then(|m| m.get("grading"))
        .and_then(|v| v.as_object());

    // Model: prefer grading block, then last assistant message.
    let model = grading_block
        .and_then(|g| g.get("model"))
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .or_else(|| {
            messages
                .iter()
                .rev()
                .find(|m| m.role.as_deref() == Some("assistant"))
                .and_then(|m| m.content.as_ref())
                .and_then(|c| c.as_str())
                .map(str::to_owned)
        });

    let model = model?;

    // Prompt template hash: SHA-256 of the first user message content.
    let prompt_template_hash = messages
        .iter()
        .find(|m| m.role.as_deref() == Some("user"))
        .and_then(|m| m.content.as_ref())
        .map(|c| {
            let text = match c {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            let hash = Sha256::digest(text.as_bytes());
            hex::encode(hash)
        })
        .unwrap_or_default();

    // Temperature: prefer grading block.
    let temperature = grading_block
        .and_then(|g| g.get("temperature"))
        .and_then(|v| v.as_f64())
        .map(|f| f as f32)
        .unwrap_or(0.0);

    let seed = grading_block
        .and_then(|g| g.get("seed"))
        .and_then(|v| v.as_u64());

    // Family: simple heuristic — take the part before the first '/'.
    let family = model.split('/').next().unwrap_or(&model).to_owned();

    JudgeConfig::new(model, family, prompt_template_hash, temperature, seed).ok()
}

/// Parse an optional RFC 3339 timestamp string into Unix epoch seconds.
///
/// Falls back to 0 (which the validator will flag as too old) if absent or
/// unparseable, so the caller can collect a warning rather than hard-error.
fn parse_timestamp(started_at: Option<&str>) -> i64 {
    started_at
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp())
        .unwrap_or(0)
}

/// Merge sample + eval + log metadata, then inject ingestion provenance keys.
fn build_metadata(
    sample: &InspectSample,
    eval_meta: &BTreeMap<String, serde_json::Value>,
    log_meta: &BTreeMap<String, serde_json::Value>,
    scorer_name: &str,
    epoch: u32,
    sample_id_str: &str,
) -> BTreeMap<String, serde_json::Value> {
    let mut meta: BTreeMap<String, serde_json::Value> = BTreeMap::new();

    // Lowest-priority: log-level metadata.
    for (k, v) in log_meta {
        meta.insert(k.clone(), v.clone());
    }
    // Mid-priority: eval-level metadata.
    for (k, v) in eval_meta {
        meta.insert(k.clone(), v.clone());
    }
    // High-priority: sample-level metadata.
    for (k, v) in &sample.metadata {
        meta.insert(k.clone(), v.clone());
    }

    // Injected provenance.
    meta.insert(
        "scorer_name".to_owned(),
        serde_json::Value::String(scorer_name.to_owned()),
    );
    meta.insert("epoch".to_owned(), serde_json::Value::Number(epoch.into()));
    meta.insert(
        "sample_id".to_owned(),
        serde_json::Value::String(sample_id_str.to_owned()),
    );

    meta
}

/// Stringify a JSON `id` field (may be string or integer).
fn json_value_to_id_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

/// Extract an optional `inspect_ai_version` from top-level log metadata.
fn extract_runner_version(meta: &BTreeMap<String, serde_json::Value>) -> Option<String> {
    meta.get("inspect_version")
        .or_else(|| meta.get("version"))
        .and_then(|v| v.as_str())
        .map(str::to_owned)
}

/// Current Unix epoch seconds using `SystemTime`.
fn current_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use std::io::Cursor;

    use super::*;
    use crate::types::{IngestAdapter, IngestSource};

    fn minimal_log_json(score_value: serde_json::Value) -> Vec<u8> {
        let log = serde_json::json!({
            "version": "0.3",
            "eval": {
                "run_id": "run-abc123",
                "task": "my_task",
                "model": "openai/gpt-4o"
            },
            "samples": [
                {
                    "id": 1,
                    "uuid": "550e8400-e29b-41d4-a716-446655440000",
                    "epoch": 0,
                    "started_at": "2024-06-01T12:00:00Z",
                    "scores": {
                        "accuracy": {
                            "value": score_value
                        }
                    }
                }
            ]
        });
        log.to_string().into_bytes()
    }

    fn reader(bytes: Vec<u8>) -> IngestSource {
        IngestSource::Reader(Box::new(Cursor::new(bytes)))
    }

    #[test]
    fn ingests_binary_correct() {
        let adapter = InspectAdapter;
        let result = adapter
            .ingest(reader(minimal_log_json(serde_json::json!("C"))))
            .unwrap();
        assert_eq!(result.records.len(), 1);
        assert_eq!(result.records[0].outcome, Outcome::Binary(true));
        assert_eq!(result.records[0].task_id, "my_task");
        assert_eq!(result.records[0].agent_id, "openai/gpt-4o");
        assert_eq!(result.source_meta.runner_name, "inspect_ai");
    }

    #[test]
    fn ingests_binary_incorrect() {
        let adapter = InspectAdapter;
        let result = adapter
            .ingest(reader(minimal_log_json(serde_json::json!("I"))))
            .unwrap();
        assert_eq!(result.records[0].outcome, Outcome::Binary(false));
    }

    #[test]
    fn ingests_partial_score() {
        let adapter = InspectAdapter;
        let result = adapter
            .ingest(reader(minimal_log_json(serde_json::json!("P"))))
            .unwrap();
        assert_eq!(result.records[0].outcome, Outcome::Score(0.5));
    }

    #[test]
    fn ingests_float_score() {
        let adapter = InspectAdapter;
        let result = adapter
            .ingest(reader(minimal_log_json(serde_json::json!(0.75))))
            .unwrap();
        assert_eq!(result.records[0].outcome, Outcome::Score(0.75));
    }

    #[test]
    fn ingests_graded_score() {
        let adapter = InspectAdapter;
        let result = adapter
            .ingest(reader(minimal_log_json(serde_json::json!(3))))
            .unwrap();
        assert_eq!(result.records[0].outcome, Outcome::Graded(3));
    }

    #[test]
    fn ingests_bool_true_as_binary() {
        let adapter = InspectAdapter;
        let result = adapter
            .ingest(reader(minimal_log_json(serde_json::json!(true))))
            .unwrap();
        assert_eq!(result.records[0].outcome, Outcome::Binary(true));
    }

    #[test]
    fn ingests_int_zero_as_binary_false() {
        let adapter = InspectAdapter;
        let result = adapter
            .ingest(reader(minimal_log_json(serde_json::json!(0))))
            .unwrap();
        assert_eq!(result.records[0].outcome, Outcome::Binary(false));
    }

    #[test]
    fn ingests_int_one_as_binary_true() {
        let adapter = InspectAdapter;
        let result = adapter
            .ingest(reader(minimal_log_json(serde_json::json!(1))))
            .unwrap();
        assert_eq!(result.records[0].outcome, Outcome::Binary(true));
    }

    #[test]
    fn ingests_multi_criterion() {
        let adapter = InspectAdapter;
        let result = adapter
            .ingest(reader(minimal_log_json(
                serde_json::json!({"coherence": 0.8, "accuracy": 0.9}),
            )))
            .unwrap();
        let Outcome::MultiCriterion(map) = &result.records[0].outcome else {
            panic!("expected MultiCriterion");
        };
        assert_eq!(*map.get("accuracy").unwrap(), 0.9);
    }

    #[test]
    fn no_samples_returns_no_records_error() {
        let log_bytes = serde_json::json!({
            "version": "0.3",
            "eval": {
                "run_id": "run-xyz",
                "task": "empty_task",
                "model": "openai/gpt-4o"
            },
            "samples": []
        })
        .to_string()
        .into_bytes();
        let adapter = InspectAdapter;
        let err = adapter.ingest(reader(log_bytes)).unwrap_err();
        assert!(matches!(err, IngestError::NoRecordsProduced));
    }

    #[test]
    fn invalid_json_returns_error() {
        let adapter = InspectAdapter;
        let err = adapter.ingest(reader(b"not json".to_vec())).unwrap_err();
        assert!(matches!(err, IngestError::NotJson(_)));
    }

    #[test]
    fn deterministic_trial_id() {
        let adapter = InspectAdapter;
        let bytes = minimal_log_json(serde_json::json!("C"));
        let r1 = adapter.ingest(reader(bytes.clone())).unwrap();
        let r2 = adapter.ingest(reader(bytes)).unwrap();
        assert_eq!(r1.records[0].trial_id, r2.records[0].trial_id);
    }

    #[test]
    fn metadata_contains_scorer_name() {
        let adapter = InspectAdapter;
        let result = adapter
            .ingest(reader(minimal_log_json(serde_json::json!("C"))))
            .unwrap();
        let meta = &result.records[0].metadata;
        assert_eq!(
            meta.get("scorer_name"),
            Some(&serde_json::Value::String("accuracy".to_owned()))
        );
    }
}
