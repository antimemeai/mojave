# eval-ingest Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** New crate `eval-ingest` with a runner-agnostic `IngestAdapter` trait, an `InspectAdapter` for UK AISI Inspect eval logs, a generic `JsonlAdapter` for BYO runners, and a validation layer.

**Architecture:** Trait-based adapter pattern. Each adapter reads a specific log format and produces `Vec<TrialRecord>` (from eval-core). Errors are non-fail-fast: malformed individual records produce warnings, structural failures produce errors. `SourceMeta` captures provenance (content hash, runner name/version) for the audit chain.

**Tech Stack:** Rust, serde/serde_json for JSON parsing, sha2 for content hashing, chrono for ISO 8601 timestamp parsing, eval-core for TrialRecord types.

---

## File Structure

```
crates/eval-ingest/
├── Cargo.toml
├── src/
│   ├── lib.rs              # pub mod declarations + re-exports
│   ├── types.rs            # IngestSource, IngestResult, SourceMeta, IngestError, IngestWarning, WarningKind
│   ├── validate.rs         # validate_record() → Result<TrialRecord, IngestWarning>
│   ├── id.rs               # deterministic ULID generation from string hashes
│   ├── inspect.rs          # InspectAdapter: IngestAdapter impl
│   ├── inspect_types.rs    # serde structs mirroring Inspect's EvalLog JSON schema
│   └── jsonl.rs            # JsonlAdapter + FieldMapping + OutcomeMapping
└── tests/
    ├── fixtures/
    │   ├── inspect_binary.json         # 10 samples, binary C/I scores
    │   ├── inspect_model_graded.json   # 5 samples, model_graded_fact scorer
    │   ├── inspect_multi_scorer.json   # 5 samples, 2 scorers
    │   ├── inspect_epochs.json         # 5 samples, 3 epochs
    │   ├── inspect_malformed.json      # 10 samples, sample 3 has null score
    │   ├── basic.jsonl                 # simple task_id/agent_id/score JSONL
    │   └── custom_fields.jsonl         # non-standard field names
    ├── inspect_tck.rs
    └── jsonl_tck.rs
```

---

### Task 1: Crate Skeleton + Core Types

**Files:**
- Create: `crates/eval-ingest/Cargo.toml`
- Create: `crates/eval-ingest/src/lib.rs`
- Create: `crates/eval-ingest/src/types.rs`
- Modify: `Cargo.toml` (workspace root — add member)

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "eval-ingest"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
publish = false
description = """
Runner-agnostic eval log ingestion — reads eval runner output and
produces TrialRecord streams for the mojave measurement engine.
"""

[lib]
path = "src/lib.rs"

[dependencies]
eval-core = { path = "../eval-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
ulid = { version = "1", features = ["serde"] }
sha2 = "0.10"
chrono = { version = "0.4", default-features = false, features = ["std", "serde"] }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Add to workspace members**

In `Cargo.toml` (workspace root), add `"crates/eval-ingest"` to the `members` array, after `"crates/eval-core"`.

- [ ] **Step 3: Write types.rs**

```rust
use std::io::Read;
use std::path::PathBuf;

use eval_core::TrialRecord;
use serde::{Deserialize, Serialize};

pub trait IngestAdapter {
    fn ingest(&self, source: IngestSource) -> Result<IngestResult, IngestError>;
}

pub enum IngestSource {
    File(PathBuf),
    Dir(PathBuf),
    Reader(Box<dyn Read>),
}

#[derive(Debug, Clone)]
pub struct IngestResult {
    pub records: Vec<TrialRecord>,
    pub warnings: Vec<IngestWarning>,
    pub source_meta: SourceMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMeta {
    pub runner_name: String,
    pub runner_version: Option<String>,
    pub log_format_version: Option<String>,
    pub original_path: Option<PathBuf>,
    pub content_hash: [u8; 32],
}

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("not valid JSON at {path}: {detail}")]
    NotJson { path: PathBuf, detail: String },

    #[error("unrecognized format at {path}: {detail}")]
    UnrecognizedFormat { path: PathBuf, detail: String },

    #[error("no records produced from {path} ({} warnings)", warnings.len())]
    NoRecordsProduced {
        path: PathBuf,
        warnings: Vec<IngestWarning>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestWarning {
    pub source_index: Option<usize>,
    pub source_id: Option<String>,
    pub kind: WarningKind,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningKind {
    MalformedRecord,
    ValidationFailed,
    UnmappableOutcome,
    MissingRequiredField,
    SkippedSample,
}
```

- [ ] **Step 4: Write lib.rs**

```rust
#![forbid(unsafe_code)]

pub mod types;

pub use types::{
    IngestAdapter, IngestError, IngestResult, IngestSource, IngestWarning, SourceMeta, WarningKind,
};
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p eval-ingest`
Expected: compiles with zero errors, zero warnings.

- [ ] **Step 6: Commit**

```bash
git add crates/eval-ingest/ Cargo.toml
git commit -m "feat(eval-ingest): crate skeleton with IngestAdapter trait and core types"
```

---

### Task 2: Deterministic ID Generation + Validation Layer

**Files:**
- Create: `crates/eval-ingest/src/id.rs`
- Create: `crates/eval-ingest/src/validate.rs`
- Modify: `crates/eval-ingest/src/lib.rs`

- [ ] **Step 1: Write id.rs**

Deterministic ULID generation from string inputs. Used to convert Inspect's shortuuid IDs and composite keys into ULIDs.

```rust
use sha2::{Digest, Sha256};
use ulid::Ulid;

pub fn ulid_from_str(input: &str) -> Ulid {
    let hash = Sha256::digest(input.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    Ulid::from_bytes(bytes)
}

pub fn ulid_from_parts(parts: &[&str]) -> Ulid {
    let mut hasher = Sha256::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            hasher.update(b"\x00");
        }
        hasher.update(part.as_bytes());
    }
    let hash = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    Ulid::from_bytes(bytes)
}
```

- [ ] **Step 2: Write validate.rs**

```rust
use eval_core::{Outcome, TrialRecord};

use crate::types::{IngestWarning, WarningKind};

const MIN_TIMESTAMP: i64 = 1_577_836_800; // 2020-01-01T00:00:00Z
const MAX_FUTURE_SLACK: i64 = 86_400; // 1 day

pub fn validate_record(
    record: &TrialRecord,
    source_index: Option<usize>,
    source_id: Option<String>,
    now: i64,
) -> Result<(), IngestWarning> {
    if record.task_id.is_empty() {
        return Err(IngestWarning {
            source_index,
            source_id,
            kind: WarningKind::ValidationFailed,
            detail: "task_id is empty".into(),
        });
    }

    if record.agent_id.is_empty() {
        return Err(IngestWarning {
            source_index,
            source_id,
            kind: WarningKind::ValidationFailed,
            detail: "agent_id is empty".into(),
        });
    }

    if record.timestamp < MIN_TIMESTAMP {
        return Err(IngestWarning {
            source_index,
            source_id,
            kind: WarningKind::ValidationFailed,
            detail: format!("timestamp {} is before 2020-01-01", record.timestamp),
        });
    }

    if record.timestamp > now + MAX_FUTURE_SLACK {
        return Err(IngestWarning {
            source_index,
            source_id,
            kind: WarningKind::ValidationFailed,
            detail: format!("timestamp {} is in the future", record.timestamp),
        });
    }

    match &record.outcome {
        Outcome::Score(v) if !v.is_finite() => {
            return Err(IngestWarning {
                source_index,
                source_id,
                kind: WarningKind::ValidationFailed,
                detail: format!("score is not finite: {v}"),
            });
        }
        Outcome::MultiCriterion(map) => {
            for (key, val) in map {
                if !val.is_finite() {
                    return Err(IngestWarning {
                        source_index,
                        source_id,
                        kind: WarningKind::ValidationFailed,
                        detail: format!("multi-criterion value for '{key}' is not finite: {val}"),
                    });
                }
            }
        }
        _ => {}
    }

    Ok(())
}
```

- [ ] **Step 3: Update lib.rs**

```rust
#![forbid(unsafe_code)]

pub mod id;
pub mod types;
pub mod validate;

pub use types::{
    IngestAdapter, IngestError, IngestResult, IngestSource, IngestWarning, SourceMeta, WarningKind,
};
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p eval-ingest`
Expected: compiles clean.

- [ ] **Step 5: Commit**

```bash
git add crates/eval-ingest/src/id.rs crates/eval-ingest/src/validate.rs crates/eval-ingest/src/lib.rs
git commit -m "feat(eval-ingest): deterministic ULID generation and TrialRecord validation"
```

---

### Task 3: Inspect JSON Deserialization Types

**Files:**
- Create: `crates/eval-ingest/src/inspect_types.rs`
- Modify: `crates/eval-ingest/src/lib.rs`

These are serde structs that mirror Inspect's JSON schema. They are NOT our types — they're a deserialization target that we map FROM.

- [ ] **Step 1: Write inspect_types.rs**

```rust
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub struct InspectLog {
    pub version: u32,
    pub status: String,
    pub eval: InspectEvalSpec,
    pub plan: Option<InspectPlan>,
    pub results: Option<serde_json::Value>,
    pub samples: Option<Vec<InspectSample>>,
}

#[derive(Debug, Deserialize)]
pub struct InspectEvalSpec {
    pub run_id: String,
    pub task_id: String,
    pub task_version: Option<serde_json::Value>,
    pub model: String,
    #[serde(default)]
    pub config: InspectGenerateConfig,
    pub revision: Option<InspectRevision>,
    pub packages: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
pub struct InspectGenerateConfig {
    pub temperature: Option<f64>,
    pub seed: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct InspectRevision {
    pub commit: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InspectPlan {
    #[serde(default)]
    pub config: InspectGenerateConfig,
}

#[derive(Debug, Deserialize)]
pub struct InspectSample {
    pub id: serde_json::Value,
    pub uuid: Option<String>,
    #[serde(default = "default_epoch")]
    pub epoch: u32,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub scores: Option<BTreeMap<String, InspectScore>>,
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
    pub error: Option<String>,
}

fn default_epoch() -> u32 {
    1
}

#[derive(Debug, Deserialize)]
pub struct InspectScore {
    pub value: serde_json::Value,
    pub answer: Option<String>,
    pub explanation: Option<String>,
    pub metadata: Option<BTreeMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct InspectGradingMessage {
    pub role: String,
    pub content: serde_json::Value,
    pub model: Option<String>,
}
```

- [ ] **Step 2: Update lib.rs — add `pub mod inspect_types;`**

```rust
#![forbid(unsafe_code)]

pub mod id;
pub mod inspect_types;
pub mod types;
pub mod validate;

pub use types::{
    IngestAdapter, IngestError, IngestResult, IngestSource, IngestWarning, SourceMeta, WarningKind,
};
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p eval-ingest`
Expected: compiles clean.

- [ ] **Step 4: Commit**

```bash
git add crates/eval-ingest/src/inspect_types.rs crates/eval-ingest/src/lib.rs
git commit -m "feat(eval-ingest): Inspect EvalLog JSON deserialization types"
```

---

### Task 4: Inspect Adapter — Core Implementation

**Files:**
- Create: `crates/eval-ingest/src/inspect.rs`
- Modify: `crates/eval-ingest/src/lib.rs`

- [ ] **Step 1: Write inspect.rs**

```rust
use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;

use chrono::DateTime;
use eval_core::{JudgeConfig, Outcome, TrialRecord};
use sha2::{Digest, Sha256};

use crate::id::{ulid_from_parts, ulid_from_str};
use crate::inspect_types::{InspectLog, InspectSample, InspectScore};
use crate::types::{
    IngestAdapter, IngestError, IngestResult, IngestSource, IngestWarning, SourceMeta, WarningKind,
};
use crate::validate::validate_record;

pub struct InspectAdapter;

impl IngestAdapter for InspectAdapter {
    fn ingest(&self, source: IngestSource) -> Result<IngestResult, IngestError> {
        let (bytes, original_path) = read_source(source)?;
        let content_hash = sha256(&bytes);

        let log: InspectLog = serde_json::from_slice(&bytes).map_err(|e| {
            IngestError::NotJson {
                path: original_path.clone().unwrap_or_default(),
                detail: e.to_string(),
            }
        })?;

        if log.version < 1 {
            return Err(IngestError::UnrecognizedFormat {
                path: original_path.clone().unwrap_or_default(),
                detail: format!("unsupported Inspect log version: {}", log.version),
            });
        }

        let mut records = Vec::new();
        let mut warnings = Vec::new();

        let samples = log.samples.unwrap_or_default();
        let now = chrono::Utc::now().timestamp();

        for (sample_idx, sample) in samples.iter().enumerate() {
            if let Some(ref err) = sample.error {
                if sample.scores.is_none() {
                    warnings.push(IngestWarning {
                        source_index: Some(sample_idx),
                        source_id: Some(sample_id_str(&sample.id)),
                        kind: WarningKind::SkippedSample,
                        detail: format!("sample errored without scores: {err}"),
                    });
                    continue;
                }
            }

            let scores = match &sample.scores {
                Some(s) if !s.is_empty() => s,
                _ => {
                    warnings.push(IngestWarning {
                        source_index: Some(sample_idx),
                        source_id: Some(sample_id_str(&sample.id)),
                        kind: WarningKind::MissingRequiredField,
                        detail: "sample has no scores".into(),
                    });
                    continue;
                }
            };

            for (scorer_name, score) in scores {
                match build_record(&log, sample, sample_idx, scorer_name, score) {
                    Ok(record) => {
                        match validate_record(&record, Some(sample_idx), Some(sample_id_str(&sample.id)), now) {
                            Ok(()) => records.push(record),
                            Err(w) => warnings.push(w),
                        }
                    }
                    Err(w) => warnings.push(w),
                }
            }
        }

        if records.is_empty() {
            return Err(IngestError::NoRecordsProduced {
                path: original_path.clone().unwrap_or_default(),
                warnings,
            });
        }

        Ok(IngestResult {
            records,
            warnings,
            source_meta: SourceMeta {
                runner_name: "inspect_ai".into(),
                runner_version: log
                    .eval
                    .packages
                    .as_ref()
                    .and_then(|p| p.get("inspect_ai").cloned()),
                log_format_version: Some(log.version.to_string()),
                original_path,
                content_hash,
            },
        })
    }
}

fn build_record(
    log: &InspectLog,
    sample: &InspectSample,
    sample_idx: usize,
    scorer_name: &str,
    score: &InspectScore,
) -> Result<TrialRecord, IngestWarning> {
    let eval = &log.eval;
    let source_id = Some(sample_id_str(&sample.id));

    let trial_id = match &sample.uuid {
        Some(uuid) => ulid_from_parts(&[uuid, scorer_name]),
        None => ulid_from_parts(&[
            &eval.run_id,
            &sample_id_str(&sample.id),
            &sample.epoch.to_string(),
            scorer_name,
        ]),
    };

    let run_id = ulid_from_str(&eval.run_id);

    let task_version = eval.task_version.as_ref().map(|v| match v {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    });

    let agent_version = eval
        .revision
        .as_ref()
        .and_then(|r| r.commit.clone())
        .or_else(|| {
            eval.packages
                .as_ref()
                .and_then(|p| p.get("inspect_ai").cloned())
        });

    let temperature = log
        .plan
        .as_ref()
        .and_then(|p| p.config.temperature)
        .or(eval.config.temperature);

    let gen_seed = log
        .plan
        .as_ref()
        .and_then(|p| p.config.seed)
        .or(eval.config.seed);

    let judge_config = extract_judge_config(score, scorer_name, temperature, gen_seed);

    let timestamp = sample
        .started_at
        .as_ref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp())
        .unwrap_or(0);

    let outcome = map_outcome(&score.value).map_err(|detail| IngestWarning {
        source_index: Some(sample_idx),
        source_id: source_id.clone(),
        kind: WarningKind::UnmappableOutcome,
        detail,
    })?;

    let mut metadata = sample.metadata.clone();
    metadata.extend(eval.metadata.clone());
    metadata.insert("scorer_name".into(), serde_json::Value::String(scorer_name.to_string()));
    metadata.insert("epoch".into(), serde_json::Value::Number(sample.epoch.into()));
    metadata.insert(
        "sample_id".into(),
        serde_json::Value::String(sample_id_str(&sample.id)),
    );

    Ok(TrialRecord {
        trial_id,
        run_id,
        task_id: eval.task_id.clone(),
        task_version,
        agent_id: eval.model.clone(),
        agent_version,
        judge_config,
        seed: gen_seed,
        timestamp,
        outcome,
        metadata,
    })
}

fn extract_judge_config(
    score: &InspectScore,
    scorer_name: &str,
    temperature: Option<f64>,
    seed: Option<u64>,
) -> Option<JudgeConfig> {
    let meta = score.metadata.as_ref()?;
    let grading = meta.get("grading")?;
    let messages: Vec<crate::inspect_types::InspectGradingMessage> =
        serde_json::from_value(grading.clone()).ok()?;

    let grader_model = messages
        .iter()
        .rev()
        .find(|m| m.role == "assistant")
        .and_then(|m| m.model.clone())
        .unwrap_or_else(|| scorer_name.to_string());

    let prompt_content = messages
        .iter()
        .find(|m| m.role == "user")
        .map(|m| m.content.to_string())
        .unwrap_or_default();
    let prompt_hash = hex::encode(Sha256::digest(prompt_content.as_bytes()));

    let family = grader_model.clone();
    let temp = temperature.unwrap_or(0.0) as f32;

    JudgeConfig::new(grader_model, family, prompt_hash, temp, seed).ok()
}

fn map_outcome(value: &serde_json::Value) -> Result<Outcome, String> {
    match value {
        serde_json::Value::String(s) => match s.as_str() {
            "C" => Ok(Outcome::Binary(true)),
            "I" | "N" => Ok(Outcome::Binary(false)),
            "P" => Outcome::score(0.5).map_err(|e| e.to_string()),
            other => Err(format!("unmappable string outcome: {other:?}")),
        },
        serde_json::Value::Bool(b) => Ok(Outcome::Binary(*b)),
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f == 0.0 || f == 1.0 {
                    Ok(Outcome::Binary(f == 1.0))
                } else if f.fract() == 0.0 && f >= 0.0 && f <= 255.0 {
                    Ok(Outcome::Graded(f as u8))
                } else {
                    Outcome::score(f).map_err(|e| e.to_string())
                }
            } else {
                Err(format!("non-f64 number: {n}"))
            }
        }
        serde_json::Value::Object(map) => {
            let mut criteria = BTreeMap::new();
            for (k, v) in map {
                let f = v
                    .as_f64()
                    .ok_or_else(|| format!("multi-criterion value for '{k}' is not a number"))?;
                criteria.insert(k.clone(), f);
            }
            Outcome::multi_criterion(criteria).map_err(|e| e.to_string())
        }
        other => Err(format!("unmappable outcome type: {other}")),
    }
}

fn sample_id_str(id: &serde_json::Value) -> String {
    match id {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

fn read_source(source: IngestSource) -> Result<(Vec<u8>, Option<PathBuf>), IngestError> {
    match source {
        IngestSource::File(path) => {
            let bytes = std::fs::read(&path)?;
            Ok((bytes, Some(path)))
        }
        IngestSource::Dir(_path) => {
            Err(IngestError::Io(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "InspectAdapter reads single files, not directories",
            )))
        }
        IngestSource::Reader(mut reader) => {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes)?;
            Ok((bytes, None))
        }
    }
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let hash = Sha256::digest(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hash);
    out
}
```

- [ ] **Step 2: Add `hex` dependency to Cargo.toml**

Add to `[dependencies]`:
```toml
hex = "0.4"
```

- [ ] **Step 3: Update lib.rs**

```rust
#![forbid(unsafe_code)]

pub mod id;
pub mod inspect;
pub mod inspect_types;
pub mod types;
pub mod validate;

pub use inspect::InspectAdapter;
pub use types::{
    IngestAdapter, IngestError, IngestResult, IngestSource, IngestWarning, SourceMeta, WarningKind,
};
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p eval-ingest`
Expected: compiles clean.

- [ ] **Step 5: Commit**

```bash
git add crates/eval-ingest/
git commit -m "feat(eval-ingest): InspectAdapter implementation — maps Inspect EvalLog to TrialRecord"
```

---

### Task 5: Inspect Test Fixtures + TCK Tests

**Files:**
- Create: `crates/eval-ingest/tests/fixtures/inspect_binary.json`
- Create: `crates/eval-ingest/tests/fixtures/inspect_model_graded.json`
- Create: `crates/eval-ingest/tests/fixtures/inspect_multi_scorer.json`
- Create: `crates/eval-ingest/tests/fixtures/inspect_epochs.json`
- Create: `crates/eval-ingest/tests/fixtures/inspect_malformed.json`
- Create: `crates/eval-ingest/tests/inspect_tck.rs`

- [ ] **Step 1: Create inspect_binary.json fixture**

A minimal Inspect v2 log with 10 samples, binary C/I outcomes:

```json
{
  "version": 2,
  "status": "success",
  "eval": {
    "run_id": "run_abc123",
    "task_id": "security-guide",
    "task_version": 1,
    "model": "openai/gpt-4o",
    "config": { "temperature": 0.0, "seed": 42 },
    "revision": { "commit": "deadbeef" },
    "packages": { "inspect_ai": "0.3.80" },
    "metadata": {}
  },
  "plan": { "config": { "temperature": 0.0, "seed": 42 } },
  "results": null,
  "samples": [
    { "id": "s1",  "uuid": "uuid_s1",  "epoch": 1, "started_at": "2025-05-12T20:28:26+00:00", "scores": { "exact_match": { "value": "C" } }, "metadata": {} },
    { "id": "s2",  "uuid": "uuid_s2",  "epoch": 1, "started_at": "2025-05-12T20:28:27+00:00", "scores": { "exact_match": { "value": "I" } }, "metadata": {} },
    { "id": "s3",  "uuid": "uuid_s3",  "epoch": 1, "started_at": "2025-05-12T20:28:28+00:00", "scores": { "exact_match": { "value": "C" } }, "metadata": {} },
    { "id": "s4",  "uuid": "uuid_s4",  "epoch": 1, "started_at": "2025-05-12T20:28:29+00:00", "scores": { "exact_match": { "value": "C" } }, "metadata": {} },
    { "id": "s5",  "uuid": "uuid_s5",  "epoch": 1, "started_at": "2025-05-12T20:28:30+00:00", "scores": { "exact_match": { "value": "I" } }, "metadata": {} },
    { "id": "s6",  "uuid": "uuid_s6",  "epoch": 1, "started_at": "2025-05-12T20:28:31+00:00", "scores": { "exact_match": { "value": "C" } }, "metadata": {} },
    { "id": "s7",  "uuid": "uuid_s7",  "epoch": 1, "started_at": "2025-05-12T20:28:32+00:00", "scores": { "exact_match": { "value": "I" } }, "metadata": {} },
    { "id": "s8",  "uuid": "uuid_s8",  "epoch": 1, "started_at": "2025-05-12T20:28:33+00:00", "scores": { "exact_match": { "value": "C" } }, "metadata": {} },
    { "id": "s9",  "uuid": "uuid_s9",  "epoch": 1, "started_at": "2025-05-12T20:28:34+00:00", "scores": { "exact_match": { "value": "C" } }, "metadata": {} },
    { "id": "s10", "uuid": "uuid_s10", "epoch": 1, "started_at": "2025-05-12T20:28:35+00:00", "scores": { "exact_match": { "value": "I" } }, "metadata": {} }
  ]
}
```

- [ ] **Step 2: Create inspect_model_graded.json fixture**

5 samples with a model-graded scorer containing grading metadata:

```json
{
  "version": 2,
  "status": "success",
  "eval": {
    "run_id": "run_graded_001",
    "task_id": "factuality-check",
    "task_version": 0,
    "model": "openai/gpt-4o",
    "config": { "temperature": 0.5 },
    "metadata": {}
  },
  "plan": { "config": { "temperature": 0.5 } },
  "results": null,
  "samples": [
    { "id": 1, "uuid": "uuid_g1", "epoch": 1, "started_at": "2025-06-01T10:00:00+00:00", "scores": { "model_graded_fact": { "value": "C", "metadata": { "grading": [ { "role": "user", "content": "Is the following statement factually correct?" }, { "role": "assistant", "content": "Yes, the statement is correct.", "model": "gpt-4o-mini-2024-07-18" } ] } } }, "metadata": {} },
    { "id": 2, "uuid": "uuid_g2", "epoch": 1, "started_at": "2025-06-01T10:00:01+00:00", "scores": { "model_graded_fact": { "value": "I", "metadata": { "grading": [ { "role": "user", "content": "Is the following statement factually correct?" }, { "role": "assistant", "content": "No.", "model": "gpt-4o-mini-2024-07-18" } ] } } }, "metadata": {} },
    { "id": 3, "uuid": "uuid_g3", "epoch": 1, "started_at": "2025-06-01T10:00:02+00:00", "scores": { "model_graded_fact": { "value": "C", "metadata": { "grading": [ { "role": "user", "content": "Is the following statement factually correct?" }, { "role": "assistant", "content": "Yes.", "model": "gpt-4o-mini-2024-07-18" } ] } } }, "metadata": {} },
    { "id": 4, "uuid": "uuid_g4", "epoch": 1, "started_at": "2025-06-01T10:00:03+00:00", "scores": { "model_graded_fact": { "value": "C", "metadata": { "grading": [ { "role": "user", "content": "Is the following statement factually correct?" }, { "role": "assistant", "content": "Yes.", "model": "gpt-4o-mini-2024-07-18" } ] } } }, "metadata": {} },
    { "id": 5, "uuid": "uuid_g5", "epoch": 1, "started_at": "2025-06-01T10:00:04+00:00", "scores": { "model_graded_fact": { "value": "I", "metadata": { "grading": [ { "role": "user", "content": "Is the following statement factually correct?" }, { "role": "assistant", "content": "No.", "model": "gpt-4o-mini-2024-07-18" } ] } } }, "metadata": {} }
  ]
}
```

- [ ] **Step 3: Create inspect_multi_scorer.json fixture**

5 samples with 2 scorers each:

```json
{
  "version": 2,
  "status": "success",
  "eval": {
    "run_id": "run_multi_001",
    "task_id": "multi-scored-task",
    "task_version": 0,
    "model": "anthropic/claude-sonnet",
    "config": {},
    "metadata": {}
  },
  "plan": { "config": {} },
  "results": null,
  "samples": [
    { "id": "m1", "uuid": "uuid_m1", "epoch": 1, "started_at": "2025-06-01T12:00:00+00:00", "scores": { "exact_match": { "value": "C" }, "includes": { "value": "C" } }, "metadata": {} },
    { "id": "m2", "uuid": "uuid_m2", "epoch": 1, "started_at": "2025-06-01T12:00:01+00:00", "scores": { "exact_match": { "value": "I" }, "includes": { "value": "C" } }, "metadata": {} },
    { "id": "m3", "uuid": "uuid_m3", "epoch": 1, "started_at": "2025-06-01T12:00:02+00:00", "scores": { "exact_match": { "value": "C" }, "includes": { "value": "I" } }, "metadata": {} },
    { "id": "m4", "uuid": "uuid_m4", "epoch": 1, "started_at": "2025-06-01T12:00:03+00:00", "scores": { "exact_match": { "value": "I" }, "includes": { "value": "I" } }, "metadata": {} },
    { "id": "m5", "uuid": "uuid_m5", "epoch": 1, "started_at": "2025-06-01T12:00:04+00:00", "scores": { "exact_match": { "value": "C" }, "includes": { "value": "C" } }, "metadata": {} }
  ]
}
```

- [ ] **Step 4: Create inspect_epochs.json fixture**

5 samples with 3 epochs each:

```json
{
  "version": 2,
  "status": "success",
  "eval": {
    "run_id": "run_epoch_001",
    "task_id": "epoch-task",
    "task_version": 0,
    "model": "openai/gpt-4o",
    "config": { "seed": 123 },
    "metadata": {}
  },
  "plan": { "config": { "seed": 123 } },
  "results": null,
  "samples": [
    { "id": "e1", "uuid": "uuid_e1_ep1", "epoch": 1, "started_at": "2025-06-01T14:00:00+00:00", "scores": { "exact_match": { "value": "C" } }, "metadata": {} },
    { "id": "e1", "uuid": "uuid_e1_ep2", "epoch": 2, "started_at": "2025-06-01T14:00:01+00:00", "scores": { "exact_match": { "value": "I" } }, "metadata": {} },
    { "id": "e1", "uuid": "uuid_e1_ep3", "epoch": 3, "started_at": "2025-06-01T14:00:02+00:00", "scores": { "exact_match": { "value": "C" } }, "metadata": {} },
    { "id": "e2", "uuid": "uuid_e2_ep1", "epoch": 1, "started_at": "2025-06-01T14:00:03+00:00", "scores": { "exact_match": { "value": "C" } }, "metadata": {} },
    { "id": "e2", "uuid": "uuid_e2_ep2", "epoch": 2, "started_at": "2025-06-01T14:00:04+00:00", "scores": { "exact_match": { "value": "C" } }, "metadata": {} },
    { "id": "e2", "uuid": "uuid_e2_ep3", "epoch": 3, "started_at": "2025-06-01T14:00:05+00:00", "scores": { "exact_match": { "value": "I" } }, "metadata": {} },
    { "id": "e3", "uuid": "uuid_e3_ep1", "epoch": 1, "started_at": "2025-06-01T14:00:06+00:00", "scores": { "exact_match": { "value": "C" } }, "metadata": {} },
    { "id": "e3", "uuid": "uuid_e3_ep2", "epoch": 2, "started_at": "2025-06-01T14:00:07+00:00", "scores": { "exact_match": { "value": "C" } }, "metadata": {} },
    { "id": "e3", "uuid": "uuid_e3_ep3", "epoch": 3, "started_at": "2025-06-01T14:00:08+00:00", "scores": { "exact_match": { "value": "C" } }, "metadata": {} },
    { "id": "e4", "uuid": "uuid_e4_ep1", "epoch": 1, "started_at": "2025-06-01T14:00:09+00:00", "scores": { "exact_match": { "value": "I" } }, "metadata": {} },
    { "id": "e4", "uuid": "uuid_e4_ep2", "epoch": 2, "started_at": "2025-06-01T14:00:10+00:00", "scores": { "exact_match": { "value": "C" } }, "metadata": {} },
    { "id": "e4", "uuid": "uuid_e4_ep3", "epoch": 3, "started_at": "2025-06-01T14:00:11+00:00", "scores": { "exact_match": { "value": "I" } }, "metadata": {} },
    { "id": "e5", "uuid": "uuid_e5_ep1", "epoch": 1, "started_at": "2025-06-01T14:00:12+00:00", "scores": { "exact_match": { "value": "C" } }, "metadata": {} },
    { "id": "e5", "uuid": "uuid_e5_ep2", "epoch": 2, "started_at": "2025-06-01T14:00:13+00:00", "scores": { "exact_match": { "value": "I" } }, "metadata": {} },
    { "id": "e5", "uuid": "uuid_e5_ep3", "epoch": 3, "started_at": "2025-06-01T14:00:14+00:00", "scores": { "exact_match": { "value": "C" } }, "metadata": {} }
  ]
}
```

- [ ] **Step 5: Create inspect_malformed.json fixture**

10 samples, sample 3 (index 2) has null scores, sample 7 (index 6) has an unmappable value:

```json
{
  "version": 2,
  "status": "success",
  "eval": {
    "run_id": "run_malformed_001",
    "task_id": "malformed-task",
    "task_version": 0,
    "model": "openai/gpt-4o",
    "config": {},
    "metadata": {}
  },
  "plan": { "config": {} },
  "results": null,
  "samples": [
    { "id": "x1",  "uuid": "uuid_x1",  "epoch": 1, "started_at": "2025-06-01T16:00:00+00:00", "scores": { "s": { "value": "C" } }, "metadata": {} },
    { "id": "x2",  "uuid": "uuid_x2",  "epoch": 1, "started_at": "2025-06-01T16:00:01+00:00", "scores": { "s": { "value": "I" } }, "metadata": {} },
    { "id": "x3",  "uuid": "uuid_x3",  "epoch": 1, "started_at": "2025-06-01T16:00:02+00:00", "scores": null, "metadata": {} },
    { "id": "x4",  "uuid": "uuid_x4",  "epoch": 1, "started_at": "2025-06-01T16:00:03+00:00", "scores": { "s": { "value": "C" } }, "metadata": {} },
    { "id": "x5",  "uuid": "uuid_x5",  "epoch": 1, "started_at": "2025-06-01T16:00:04+00:00", "scores": { "s": { "value": "C" } }, "metadata": {} },
    { "id": "x6",  "uuid": "uuid_x6",  "epoch": 1, "started_at": "2025-06-01T16:00:05+00:00", "scores": { "s": { "value": "I" } }, "metadata": {} },
    { "id": "x7",  "uuid": "uuid_x7",  "epoch": 1, "started_at": "2025-06-01T16:00:06+00:00", "scores": { "s": { "value": [1, 2, 3] } }, "metadata": {} },
    { "id": "x8",  "uuid": "uuid_x8",  "epoch": 1, "started_at": "2025-06-01T16:00:07+00:00", "scores": { "s": { "value": "C" } }, "metadata": {} },
    { "id": "x9",  "uuid": "uuid_x9",  "epoch": 1, "started_at": "2025-06-01T16:00:08+00:00", "scores": { "s": { "value": "I" } }, "metadata": {} },
    { "id": "x10", "uuid": "uuid_x10", "epoch": 1, "started_at": "2025-06-01T16:00:09+00:00", "scores": { "s": { "value": "C" } }, "metadata": {} }
  ]
}
```

- [ ] **Step 6: Write inspect_tck.rs**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use eval_ingest::{IngestAdapter, IngestSource, InspectAdapter, WarningKind};
use eval_core::Outcome;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn binary_outcomes_produce_10_records() {
    let result = InspectAdapter.ingest(IngestSource::File(fixture("inspect_binary.json"))).unwrap();
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

    let correct_count = result.records.iter().filter(|r| r.outcome == Outcome::Binary(true)).count();
    let incorrect_count = result.records.iter().filter(|r| r.outcome == Outcome::Binary(false)).count();
    assert_eq!(correct_count, 6);
    assert_eq!(incorrect_count, 4);
}

#[test]
fn model_graded_populates_judge_config() {
    let result = InspectAdapter.ingest(IngestSource::File(fixture("inspect_model_graded.json"))).unwrap();
    assert_eq!(result.records.len(), 5);

    for record in &result.records {
        let jc = record.judge_config.as_ref().expect("judge_config should be populated");
        assert_eq!(jc.model, "gpt-4o-mini-2024-07-18");
        assert!(!jc.prompt_template_hash.is_empty());
        assert_eq!(jc.prompt_template_hash.len(), 64); // SHA-256 hex
    }
}

#[test]
fn multi_scorer_produces_record_per_scorer() {
    let result = InspectAdapter.ingest(IngestSource::File(fixture("inspect_multi_scorer.json"))).unwrap();
    assert_eq!(result.records.len(), 10); // 5 samples × 2 scorers

    for record in &result.records {
        let scorer = record.metadata.get("scorer_name").unwrap().as_str().unwrap();
        assert!(scorer == "exact_match" || scorer == "includes");
    }

    let exact_count = result.records.iter()
        .filter(|r| r.metadata.get("scorer_name").unwrap().as_str().unwrap() == "exact_match")
        .count();
    let includes_count = result.records.iter()
        .filter(|r| r.metadata.get("scorer_name").unwrap().as_str().unwrap() == "includes")
        .count();
    assert_eq!(exact_count, 5);
    assert_eq!(includes_count, 5);
}

#[test]
fn epochs_produce_distinct_trial_ids() {
    let result = InspectAdapter.ingest(IngestSource::File(fixture("inspect_epochs.json"))).unwrap();
    assert_eq!(result.records.len(), 15); // 5 samples × 3 epochs

    for record in &result.records {
        let epoch = record.metadata.get("epoch").unwrap().as_u64().unwrap();
        assert!(epoch >= 1 && epoch <= 3);
    }

    let sample_ids: Vec<&str> = result.records.iter()
        .map(|r| r.metadata.get("sample_id").unwrap().as_str().unwrap())
        .collect();
    for sid in &["e1", "e2", "e3", "e4", "e5"] {
        let count = sample_ids.iter().filter(|s| *s == sid).count();
        assert_eq!(count, 3, "sample {sid} should appear 3 times (once per epoch)");
    }

    // All trial_ids must be unique
    let mut trial_ids: Vec<_> = result.records.iter().map(|r| r.trial_id).collect();
    trial_ids.sort();
    trial_ids.dedup();
    assert_eq!(trial_ids.len(), 15);
}

#[test]
fn malformed_samples_produce_warnings() {
    let result = InspectAdapter.ingest(IngestSource::File(fixture("inspect_malformed.json"))).unwrap();
    assert_eq!(result.records.len(), 8); // 10 - 2 bad samples
    assert_eq!(result.warnings.len(), 2);

    let null_warning = result.warnings.iter().find(|w| w.source_id.as_deref() == Some("x3")).unwrap();
    assert_eq!(null_warning.kind, WarningKind::MissingRequiredField);

    let unmappable_warning = result.warnings.iter().find(|w| w.source_id.as_deref() == Some("x7")).unwrap();
    assert_eq!(unmappable_warning.kind, WarningKind::UnmappableOutcome);
}

#[test]
fn content_hash_is_sha256() {
    let path = fixture("inspect_binary.json");
    let result = InspectAdapter.ingest(IngestSource::File(path.clone())).unwrap();

    let file_bytes = std::fs::read(&path).unwrap();
    let expected_hash = <sha2::Sha256 as sha2::Digest>::digest(&file_bytes);
    assert_eq!(result.source_meta.content_hash, expected_hash.as_slice());
}

#[test]
fn deterministic_ids_are_reproducible() {
    let result1 = InspectAdapter.ingest(IngestSource::File(fixture("inspect_binary.json"))).unwrap();
    let result2 = InspectAdapter.ingest(IngestSource::File(fixture("inspect_binary.json"))).unwrap();

    for (r1, r2) in result1.records.iter().zip(result2.records.iter()) {
        assert_eq!(r1.trial_id, r2.trial_id);
        assert_eq!(r1.run_id, r2.run_id);
    }
}
```

- [ ] **Step 7: Run tests — expect them to fail initially, then pass**

Run: `cargo test -p eval-ingest -- --test-threads=1`
Expected: All 7 tests pass.

- [ ] **Step 8: Commit**

```bash
git add crates/eval-ingest/tests/
git commit -m "test(eval-ingest): Inspect adapter TCK — 7 tests covering binary, model-graded, multi-scorer, epochs, malformed, provenance, determinism"
```

---

### Task 6: JSONL Adapter — FieldMapping + Implementation

**Files:**
- Create: `crates/eval-ingest/src/jsonl.rs`
- Modify: `crates/eval-ingest/src/lib.rs`

- [ ] **Step 1: Write jsonl.rs**

```rust
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
    pub timestamp: Option<String>,
    pub seed: Option<String>,
    pub run_id: Option<String>,
    pub task_version: Option<String>,
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
                outcome: OutcomeMapping::Auto { path: "score".into() },
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
        let content_hash = sha256(&bytes);

        let file_run_id = Ulid::new();
        let file_timestamp = chrono::Utc::now().timestamp();
        let now = file_timestamp;

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
                        kind: WarningKind::MalformedRecord,
                        detail: format!("could not read line: {e}"),
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
                        kind: WarningKind::MalformedRecord,
                        detail: format!("invalid JSON: {e}"),
                    });
                    continue;
                }
            };

            match self.map_record(&obj, line_idx, file_run_id, file_timestamp) {
                Ok(record) => match validate_record(&record, Some(line_idx), None, now) {
                    Ok(()) => records.push(record),
                    Err(w) => warnings.push(w),
                },
                Err(w) => warnings.push(w),
            }
        }

        if records.is_empty() {
            return Err(IngestError::NoRecordsProduced {
                path: original_path.clone().unwrap_or_default(),
                warnings,
            });
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
            kind: WarningKind::MissingRequiredField,
            detail: format!("missing required field: {}", m.task_id),
        })?;

        let agent_id = extract_string(obj, &m.agent_id).ok_or_else(|| IngestWarning {
            source_index: Some(line_idx),
            source_id: None,
            kind: WarningKind::MissingRequiredField,
            detail: format!("missing required field: {}", m.agent_id),
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
        let agent_version = m.agent_version.as_ref().and_then(|p| extract_string(obj, p));

        let outcome = map_outcome_from_mapping(obj, &m.outcome).map_err(|detail| {
            IngestWarning {
                source_index: Some(line_idx),
                source_id: None,
                kind: WarningKind::UnmappableOutcome,
                detail,
            }
        })?;

        let metadata = match obj.as_object() {
            Some(map) => map
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
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
            let val = extract_value(obj, path)
                .ok_or_else(|| format!("missing field: {path}"))?;
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
            let val = extract_value(obj, path)
                .ok_or_else(|| format!("missing field: {path}"))?;
            let f = val.as_f64().ok_or_else(|| format!("not a number: {val}"))?;
            Outcome::score(f).map_err(|e| e.to_string())
        }
        OutcomeMapping::GradedField { path, max: _ } => {
            let val = extract_value(obj, path)
                .ok_or_else(|| format!("missing field: {path}"))?;
            let n = val.as_u64().ok_or_else(|| format!("not an integer: {val}"))?;
            Ok(Outcome::Graded(n as u8))
        }
        OutcomeMapping::MultiCriterion { fields } => {
            let mut criteria = BTreeMap::new();
            for (name, path) in fields {
                let val = extract_value(obj, path)
                    .ok_or_else(|| format!("missing field: {path}"))?;
                let f = val.as_f64().ok_or_else(|| format!("{name}: not a number: {val}"))?;
                criteria.insert(name.clone(), f);
            }
            Outcome::multi_criterion(criteria).map_err(|e| e.to_string())
        }
        OutcomeMapping::Auto { path } => {
            let val = extract_value(obj, path)
                .ok_or_else(|| format!("missing field: {path}"))?;
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
        IngestSource::Dir(_path) => Err(IngestError::Io(std::io::Error::new(
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

fn sha256(data: &[u8]) -> [u8; 32] {
    let hash = Sha256::digest(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hash);
    out
}
```

- [ ] **Step 2: Update lib.rs**

```rust
#![forbid(unsafe_code)]

pub mod id;
pub mod inspect;
pub mod inspect_types;
pub mod jsonl;
pub mod types;
pub mod validate;

pub use inspect::InspectAdapter;
pub use jsonl::{FieldMapping, JsonlAdapter, OutcomeMapping};
pub use types::{
    IngestAdapter, IngestError, IngestResult, IngestSource, IngestWarning, SourceMeta, WarningKind,
};
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p eval-ingest`
Expected: compiles clean.

- [ ] **Step 4: Commit**

```bash
git add crates/eval-ingest/src/jsonl.rs crates/eval-ingest/src/lib.rs
git commit -m "feat(eval-ingest): JsonlAdapter with FieldMapping and OutcomeMapping"
```

---

### Task 7: JSONL Test Fixtures + TCK Tests

**Files:**
- Create: `crates/eval-ingest/tests/fixtures/basic.jsonl`
- Create: `crates/eval-ingest/tests/fixtures/custom_fields.jsonl`
- Create: `crates/eval-ingest/tests/jsonl_tck.rs`

- [ ] **Step 1: Create basic.jsonl fixture**

```
{"task_id":"math-101","agent_id":"gpt-4o","score":0.95,"timestamp":1717200000}
{"task_id":"math-102","agent_id":"gpt-4o","score":0.80,"timestamp":1717200001}
{"task_id":"math-103","agent_id":"gpt-4o","score":0.60,"timestamp":1717200002}
{"task_id":"math-104","agent_id":"gpt-4o","score":1.0,"timestamp":1717200003}
{"task_id":"math-105","agent_id":"gpt-4o","score":0.0,"timestamp":1717200004}
```

- [ ] **Step 2: Create custom_fields.jsonl fixture**

```
{"item":"task-A","model":"claude-3","pass":true}
{"item":"task-B","model":"claude-3","pass":false}
{"item":"task-C","model":"claude-3","pass":true}
```

- [ ] **Step 3: Write jsonl_tck.rs**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use eval_core::Outcome;
use eval_ingest::{
    FieldMapping, IngestAdapter, IngestSource, JsonlAdapter, OutcomeMapping, WarningKind,
};

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

    // Verify scores
    if let Outcome::Score(v) = result.records[0].outcome {
        assert!((v - 0.95).abs() < 1e-10);
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
        timestamp: None,
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

    // All records share a run_id (generated per file)
    let run_ids: Vec<_> = result.records.iter().map(|r| r.run_id).collect();
    assert!(run_ids.windows(2).all(|w| w[0] == w[1]));

    // All trial_ids are unique
    let mut trial_ids: Vec<_> = result.records.iter().map(|r| r.trial_id).collect();
    trial_ids.sort();
    trial_ids.dedup();
    assert_eq!(trial_ids.len(), 5);
}

#[test]
fn mixed_valid_invalid_lines() {
    let data = b"
{\"task_id\":\"t1\",\"agent_id\":\"a1\",\"score\":0.5,\"timestamp\":1717200000}
this is not json
{\"task_id\":\"t2\",\"agent_id\":\"a1\",\"score\":0.8,\"timestamp\":1717200001}
";
    let adapter = JsonlAdapter::with_auto_detect();
    let result = adapter
        .ingest(IngestSource::Reader(Box::new(&data[..])))
        .unwrap();

    assert_eq!(result.records.len(), 2);
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0].kind, WarningKind::MalformedRecord);
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
    assert_eq!(result1.source_meta.content_hash, result2.source_meta.content_hash);
}
```

- [ ] **Step 4: Run all tests**

Run: `cargo test -p eval-ingest`
Expected: All 12 tests pass (7 inspect + 5 jsonl).

- [ ] **Step 5: Commit**

```bash
git add crates/eval-ingest/tests/
git commit -m "test(eval-ingest): JSONL adapter TCK — 5 tests covering auto-detect, custom mapping, defaults, mixed lines, determinism"
```

---

### Task 8: Workspace Integration + Full Test Suite

**Files:**
- Modify: none (verification task)

- [ ] **Step 1: Run full workspace build**

Run: `cargo build --workspace`
Expected: compiles clean, zero warnings.

- [ ] **Step 2: Run clippy on eval-ingest**

Run: `cargo clippy -p eval-ingest -- -D warnings`
Expected: zero warnings.

- [ ] **Step 3: Run full workspace tests**

Run: `cargo test --workspace`
Expected: all tests pass, including eval-ingest's 12 tests.

- [ ] **Step 4: Run rustfmt check**

Run: `cargo fmt --check -p eval-ingest`
Expected: no formatting issues.

- [ ] **Step 5: Fix any issues found, then commit**

If any issues were found in steps 1-4, fix them and commit:
```bash
git add -A
git commit -m "fix(eval-ingest): clippy/fmt fixes from workspace integration"
```

---

### Task 9: Update BEAD-0015 + TCK Feature Files

**Files:**
- Modify: `.context/beads/BEAD-0015-eval-runner-ingest-layer.md`
- Create: `tck/eval-ingest/features/inspect.feature`
- Create: `tck/eval-ingest/features/jsonl.feature`

- [ ] **Step 1: Write inspect.feature**

```gherkin
Feature: Inspect eval log ingestion

  Scenario: Ingest a v2 eval log with binary outcomes
    Given an Inspect v2 log file with 10 samples scored as "C"/"I"
    When the InspectAdapter ingests the file
    Then 10 TrialRecords are produced
    And each record has outcome Binary(true) or Binary(false)
    And each record has a valid trial_id and run_id
    And task_id matches the eval spec task_id
    And agent_id matches the eval spec model

  Scenario: Ingest a log with model-graded scorer
    Given an Inspect log with model_graded_fact scorer
    When the InspectAdapter ingests the file
    Then each record has judge_config populated
    And judge_config.model is the grading model name
    And judge_config.prompt_template_hash is a SHA-256 hex string

  Scenario: Ingest a log with multiple scorers
    Given an Inspect log with 5 samples and 2 scorers
    When the InspectAdapter ingests the file
    Then 10 TrialRecords are produced
    And metadata contains scorer_name for each record

  Scenario: Ingest a log with epochs
    Given an Inspect log with 5 samples and 3 epochs
    When the InspectAdapter ingests the file
    Then 15 TrialRecords are produced
    And records from different epochs share sample.id but differ in trial_id
    And metadata contains epoch number

  Scenario: Malformed samples produce warnings not errors
    Given an Inspect log where sample 3 has a null score
    When the InspectAdapter ingests the file
    Then the result contains records for all other samples
    And the result contains warnings for bad samples

  Scenario: Content hash is computed for provenance
    Given an Inspect log file
    When the InspectAdapter ingests the file
    Then source_meta.content_hash is the SHA-256 of the file
    And source_meta.runner_name is "inspect_ai"

  Scenario: Deterministic IDs across runs
    Given the same Inspect log file ingested twice
    Then trial_ids and run_ids match across both runs
```

- [ ] **Step 2: Write jsonl.feature**

```gherkin
Feature: Generic JSONL ingestion

  Scenario: Ingest JSONL with auto field name detection
    Given a JSONL file with fields "task_id", "agent_id", "score"
    When the JsonlAdapter ingests with auto-detect
    Then records are produced with correct field mapping

  Scenario: Ingest JSONL with explicit field mapping
    Given a JSONL file with fields "item", "model", "pass"
    And a FieldMapping mapping item->task_id, model->agent_id, pass->Binary
    When the JsonlAdapter ingests the file
    Then records have correct task_id, agent_id, and Binary outcome

  Scenario: Missing optional fields get defaults
    Given a JSONL file with only required fields
    When the JsonlAdapter ingests the file
    Then run_id is generated (one per file)
    And trial_id is generated per record

  Scenario: Mixed valid/invalid lines
    Given a JSONL file where one line is malformed JSON
    When the JsonlAdapter ingests the file
    Then records are produced for all valid lines
    And warnings are emitted for invalid lines

  Scenario: Content hash is deterministic
    Given the same JSONL file ingested twice
    Then content_hash matches across both runs
```

- [ ] **Step 3: Update BEAD-0015 to closed**

Update `.context/beads/BEAD-0015-eval-runner-ingest-layer.md`:
- Change `status: open` to `status: closed`
- Add `closed: 2026-05-14`
- Add completion notes documenting the components and test counts.

- [ ] **Step 4: Commit**

```bash
git add tck/eval-ingest/ .context/beads/BEAD-0015-eval-runner-ingest-layer.md
git commit -m "docs(eval-ingest): TCK feature files + close BEAD-0015"
```
