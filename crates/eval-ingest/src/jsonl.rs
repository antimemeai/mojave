use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;

use eval_core::{Outcome, TrialRecord};
use sha2::{Digest, Sha256};
use ulid::Ulid;

use crate::id::ulid_from_parts;
use crate::types::{
    IngestAdapter, IngestError, IngestResult, IngestSource, IngestWarning, SourceMeta, WarningKind,
};
use crate::validate::validate_record;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FieldMapping {
    pub task_id: String,
    pub agent_id: String,
    pub outcome: OutcomeMapping,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub seed: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub task_version: Option<String>,
    #[serde(default)]
    pub agent_version: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum OutcomeMapping {
    BinaryField { path: String },
    ScoreField { path: String },
    GradedField { path: String, max: u8 },
    MultiCriterion { fields: Vec<(String, String)> },
    Auto { path: String },
}

pub struct JsonlAdapter {
    mapping: FieldMapping,
}

impl JsonlAdapter {
    pub fn new(mapping: FieldMapping) -> Self {
        Self { mapping }
    }

    pub fn with_auto_detect() -> Self {
        Self {
            mapping: FieldMapping {
                task_id: "task_id".into(),
                agent_id: "agent_id".into(),
                outcome: OutcomeMapping::Auto {
                    path: "score".into(),
                },
                timestamp: Some("timestamp".into()),
                seed: Some("seed".into()),
                run_id: Some("run_id".into()),
                task_version: Some("task_version".into()),
                agent_version: Some("agent_version".into()),
            },
        }
    }
}

impl IngestAdapter for JsonlAdapter {
    fn ingest(&self, source: IngestSource) -> Result<IngestResult, IngestError> {
        let (bytes, original_path) = read_source(source)?;

        let hash = Sha256::digest(&bytes);
        let content_hash = hex::encode(hash);

        let file_run_id = Ulid::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let reader = BufReader::new(&bytes[..]);
        let mut records = Vec::new();
        let mut warnings = Vec::new();

        for (line_idx, line_result) in reader.lines().enumerate() {
            let line = match line_result {
                Ok(l) if l.trim().is_empty() => continue,
                Ok(l) => l,
                Err(e) => {
                    warnings.push(IngestWarning {
                        source_index: Some(line_idx),
                        source_id: None,
                        kind: WarningKind::ParseError(format!("could not read line: {e}")),
                    });
                    continue;
                }
            };

            let obj: serde_json::Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(e) => {
                    warnings.push(IngestWarning {
                        source_index: Some(line_idx),
                        source_id: None,
                        kind: WarningKind::ParseError(format!("invalid JSON: {e}")),
                    });
                    continue;
                }
            };

            match self.map_record(&obj, line_idx, file_run_id, now) {
                Ok(record) => match validate_record(&record, Some(line_idx), None, now) {
                    Ok(()) => records.push(record),
                    Err(w) => warnings.push(w),
                },
                Err(w) => warnings.push(w),
            }
        }

        if records.is_empty() {
            return Err(IngestError::NoRecordsProduced);
        }

        Ok(IngestResult {
            records,
            warnings,
            source_meta: SourceMeta {
                runner_name: "jsonl".into(),
                runner_version: None,
                log_format_version: None,
                original_path,
                content_hash,
            },
        })
    }
}

impl JsonlAdapter {
    fn map_record(
        &self,
        obj: &serde_json::Value,
        line_idx: usize,
        file_run_id: Ulid,
        file_timestamp: i64,
    ) -> Result<TrialRecord, IngestWarning> {
        let m = &self.mapping;

        let task_id = extract_string(obj, &m.task_id).ok_or_else(|| IngestWarning {
            source_index: Some(line_idx),
            source_id: None,
            kind: WarningKind::ParseError(format!("missing required field: {}", m.task_id)),
        })?;

        let agent_id = extract_string(obj, &m.agent_id).ok_or_else(|| IngestWarning {
            source_index: Some(line_idx),
            source_id: None,
            kind: WarningKind::ParseError(format!("missing required field: {}", m.agent_id)),
        })?;

        let run_id = m
            .run_id
            .as_ref()
            .and_then(|p| extract_string(obj, p))
            .map(|s| crate::id::ulid_from_str(&s))
            .unwrap_or(file_run_id);

        let trial_id = ulid_from_parts(&[
            &run_id.to_string(),
            &task_id,
            &agent_id,
            &line_idx.to_string(),
        ]);

        let timestamp = m
            .timestamp
            .as_ref()
            .and_then(|p| extract_i64(obj, p))
            .unwrap_or(file_timestamp);

        let seed = m.seed.as_ref().and_then(|p| extract_u64(obj, p));
        let task_version = m.task_version.as_ref().and_then(|p| extract_string(obj, p));
        let agent_version = m
            .agent_version
            .as_ref()
            .and_then(|p| extract_string(obj, p));

        let outcome =
            map_outcome_from_mapping(obj, &m.outcome).map_err(|detail| IngestWarning {
                source_index: Some(line_idx),
                source_id: None,
                kind: WarningKind::ParseError(detail),
            })?;

        let metadata = match obj.as_object() {
            Some(map) => map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            None => BTreeMap::new(),
        };

        Ok(TrialRecord {
            trial_id,
            run_id,
            task_id,
            task_version,
            agent_id,
            agent_version,
            judge_config: None,
            seed,
            timestamp,
            outcome,
            metadata,
        })
    }
}

fn map_outcome_from_mapping(
    obj: &serde_json::Value,
    mapping: &OutcomeMapping,
) -> Result<Outcome, String> {
    match mapping {
        OutcomeMapping::BinaryField { path } => {
            let val = extract_value(obj, path).ok_or_else(|| format!("missing field: {path}"))?;
            match val {
                serde_json::Value::Bool(b) => Ok(Outcome::Binary(*b)),
                serde_json::Value::Number(n) => {
                    let f = n.as_f64().ok_or_else(|| format!("not a number: {n}"))?;
                    Ok(Outcome::Binary(f != 0.0))
                }
                serde_json::Value::String(s) => match s.to_lowercase().as_str() {
                    "true" | "pass" | "correct" | "1" | "c" => Ok(Outcome::Binary(true)),
                    "false" | "fail" | "incorrect" | "0" | "i" => Ok(Outcome::Binary(false)),
                    other => Err(format!("cannot parse as binary: {other:?}")),
                },
                other => Err(format!("unexpected type for binary field: {other}")),
            }
        }
        OutcomeMapping::ScoreField { path } => {
            let val = extract_value(obj, path).ok_or_else(|| format!("missing field: {path}"))?;
            let f = val.as_f64().ok_or_else(|| format!("not a number: {val}"))?;
            Outcome::score(f).map_err(|e| e.to_string())
        }
        OutcomeMapping::GradedField { path, max: _ } => {
            let val = extract_value(obj, path).ok_or_else(|| format!("missing field: {path}"))?;
            let n = val
                .as_u64()
                .ok_or_else(|| format!("not an integer: {val}"))?;
            Ok(Outcome::Graded(n as u8))
        }
        OutcomeMapping::MultiCriterion { fields } => {
            let mut criteria = BTreeMap::new();
            for (name, path) in fields {
                let val =
                    extract_value(obj, path).ok_or_else(|| format!("missing field: {path}"))?;
                let f = val
                    .as_f64()
                    .ok_or_else(|| format!("{name}: not a number: {val}"))?;
                criteria.insert(name.clone(), f);
            }
            Outcome::multi_criterion(criteria).map_err(|e| e.to_string())
        }
        OutcomeMapping::Auto { path } => {
            let val = extract_value(obj, path).ok_or_else(|| format!("missing field: {path}"))?;
            match val {
                serde_json::Value::Bool(b) => Ok(Outcome::Binary(*b)),
                serde_json::Value::Number(n) => {
                    let f = n.as_f64().ok_or_else(|| format!("not f64: {n}"))?;
                    Outcome::score(f).map_err(|e| e.to_string())
                }
                serde_json::Value::String(s) => match s.to_lowercase().as_str() {
                    "true" | "pass" | "correct" | "1" | "c" => Ok(Outcome::Binary(true)),
                    "false" | "fail" | "incorrect" | "0" | "i" => Ok(Outcome::Binary(false)),
                    _ => Err(format!("cannot auto-map string: {s:?}")),
                },
                other => Err(format!("cannot auto-map type: {other}")),
            }
        }
    }
}

fn extract_value<'a>(obj: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = obj;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

fn extract_string(obj: &serde_json::Value, path: &str) -> Option<String> {
    let val = extract_value(obj, path)?;
    match val {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn extract_i64(obj: &serde_json::Value, path: &str) -> Option<i64> {
    extract_value(obj, path)?.as_i64()
}

fn extract_u64(obj: &serde_json::Value, path: &str) -> Option<u64> {
    extract_value(obj, path)?.as_u64()
}

fn read_source(source: IngestSource) -> Result<(Vec<u8>, Option<PathBuf>), IngestError> {
    match source {
        IngestSource::File(path) => {
            let bytes = std::fs::read(&path)?;
            Ok((bytes, Some(path)))
        }
        IngestSource::Dir(_) => Err(IngestError::Io(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "JsonlAdapter reads single files, not directories",
        ))),
        IngestSource::Reader(mut reader) => {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes)?;
            Ok((bytes, None))
        }
    }
}
