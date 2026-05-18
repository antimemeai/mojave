use std::path::{Path, PathBuf};

use eval_ingest::inspect::InspectAdapter;
use eval_ingest::types::{IngestAdapter, IngestResult, IngestSource};
use eval_ingest::{FieldMapping, JsonlAdapter};
use serde::Serialize;

use crate::detect::{detect_format, parse_format_flag, InputFormat};
use crate::error::CliError;

#[derive(Serialize)]
pub struct IngestOutput {
    pub records: Vec<eval_core::TrialRecord>,
    pub warnings: Vec<WarningOutput>,
    pub source_meta: SourceMetaOutput,
}

#[derive(Serialize)]
pub struct WarningOutput {
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_index: Option<usize>,
}

#[derive(Serialize)]
pub struct SourceMetaOutput {
    pub runner_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runner_version: Option<String>,
    pub content_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_path: Option<String>,
}

fn warning_kind_tag(kind: &eval_ingest::types::WarningKind) -> String {
    use eval_ingest::types::WarningKind;
    match kind {
        WarningKind::EmptyTaskId => "empty_task_id".into(),
        WarningKind::EmptyAgentId => "empty_agent_id".into(),
        WarningKind::TimestampTooOld(_) => "timestamp_too_old".into(),
        WarningKind::TimestampInFuture(_) => "timestamp_in_future".into(),
        WarningKind::NonFiniteScore(_) => "non_finite_score".into(),
        WarningKind::NonFiniteCriterion { .. } => "non_finite_criterion".into(),
        WarningKind::UnknownScorer => "unknown_scorer".into(),
        WarningKind::ParseError(_) => "parse_error".into(),
    }
}

fn warning_kind_message(kind: &eval_ingest::types::WarningKind) -> String {
    use eval_ingest::types::WarningKind;
    match kind {
        WarningKind::EmptyTaskId => "task_id field is empty".into(),
        WarningKind::EmptyAgentId => "agent_id field is empty".into(),
        WarningKind::TimestampTooOld(ts) => format!("timestamp {ts} is before 2020-01-01"),
        WarningKind::TimestampInFuture(ts) => format!("timestamp {ts} is more than 24h in future"),
        WarningKind::NonFiniteScore(v) => format!("score value {v} is not finite"),
        WarningKind::NonFiniteCriterion { key, value } => {
            format!("criterion '{key}' has non-finite value {value}")
        }
        WarningKind::UnknownScorer => "scorer name could not be determined".into(),
        WarningKind::ParseError(msg) => format!("parse error: {msg}"),
    }
}

pub fn run_ingest(
    paths: &[PathBuf],
    format_flag: &str,
    field_mapping_path: Option<&Path>,
) -> Result<IngestOutput, CliError> {
    let forced_format = parse_format_flag(format_flag)
        .map_err(|e| CliError::Config(crate::error::ConfigError::ParseError(e.to_string())))?;

    let field_mapping: Option<FieldMapping> = match field_mapping_path {
        Some(p) => {
            let contents = std::fs::read_to_string(p)?;
            let mapping: FieldMapping = serde_yaml::from_str(&contents).map_err(|e| {
                CliError::Config(crate::error::ConfigError::ParseError(e.to_string()))
            })?;
            Some(mapping)
        }
        None => None,
    };

    let mut all_records = Vec::new();
    let mut all_warnings = Vec::new();
    let mut last_source_meta = None;

    for path in paths {
        let format = match forced_format {
            Some(f) => f,
            None => detect_format(path).map_err(|e| {
                CliError::Config(crate::error::ConfigError::ParseError(e.to_string()))
            })?,
        };

        let source = if path.is_dir() {
            IngestSource::Dir(path.clone())
        } else {
            IngestSource::File(path.clone())
        };

        let result: IngestResult = match format {
            InputFormat::Inspect => InspectAdapter.ingest(source)?,
            InputFormat::Jsonl => {
                let adapter = match &field_mapping {
                    Some(fm) => JsonlAdapter::new(fm.clone()),
                    None => JsonlAdapter::with_auto_detect(),
                };
                adapter.ingest(source)?
            }
        };

        for w in &result.warnings {
            all_warnings.push(WarningOutput {
                kind: warning_kind_tag(&w.kind),
                message: warning_kind_message(&w.kind),
                source_index: w.source_index,
            });
        }

        all_records.extend(result.records);
        last_source_meta = Some(result.source_meta);
    }

    let meta = last_source_meta.unwrap_or_else(|| eval_ingest::types::SourceMeta {
        runner_name: "unknown".into(),
        runner_version: None,
        log_format_version: None,
        original_path: None,
        content_hash: String::new(),
    });

    Ok(IngestOutput {
        records: all_records,
        warnings: all_warnings,
        source_meta: SourceMetaOutput {
            runner_name: meta.runner_name,
            runner_version: meta.runner_version,
            content_hash: meta.content_hash,
            original_path: meta.original_path.map(|p| p.display().to_string()),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../eval-ingest/tests/fixtures")
            .join(name)
    }

    #[test]
    fn ingest_inspect_binary() {
        let paths = vec![fixture_path("inspect_binary.json")];
        let output = run_ingest(&paths, "auto", None).unwrap();
        assert!(!output.records.is_empty());
        assert_eq!(output.source_meta.runner_name, "inspect_ai");
    }

    #[test]
    fn ingest_jsonl_basic() {
        let paths = vec![fixture_path("basic.jsonl")];
        let output = run_ingest(&paths, "auto", None).unwrap();
        assert_eq!(output.records.len(), 5);
    }

    #[test]
    fn ingest_forced_format() {
        let paths = vec![fixture_path("basic.jsonl")];
        let output = run_ingest(&paths, "jsonl", None).unwrap();
        assert_eq!(output.records.len(), 5);
    }

    #[test]
    fn ingest_output_serializes_to_json() {
        let paths = vec![fixture_path("basic.jsonl")];
        let output = run_ingest(&paths, "auto", None).unwrap();
        let json = serde_json::to_string(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["records"].is_array());
        assert!(parsed["source_meta"]["runner_name"].is_string());
    }
}
