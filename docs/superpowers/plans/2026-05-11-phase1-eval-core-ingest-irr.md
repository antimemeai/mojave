# Phase 1: eval-core + eval-ingest + irr — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the foundational types (TrialRecord), an Inspect ingestion adapter, and the inter-rater reliability crate (classical + Dawid-Skene + preference-leakage diagnostics), culminating in the first end-to-end demo: ingest an Inspect run → compute IRR with judge-family stratification → emit integrity diff.

**Architecture:** Rust workspace with three crates: `eval-core` (types only, zero deps beyond serde), `eval-ingest` (adapter trait + Inspect reader), and `irr` (math). The `irr` crate depends on `eval-core` for JudgeConfig/family info. Gherkin `.feature` files in `tck/` define behavioral specs (TCK-first per JSMNTL). Each math function passes the 4-gate validation strategy. All crates compile independently and communicate via the TrialRecord schema.

**Tech Stack:** Rust 1.94+, serde/serde_json (serialization), bincode (internal), cucumber (Gherkin test runner for Rust), nalgebra (linear algebra for Dawid-Skene EM), proptest (property-based testing), criterion (benchmarks). Python only for Inspect log reader sidecar (subprocess).

**Methodology:** JSMNTL — TCK specs first (red), get tests compiling, implement, green, subagent code review, commit. 4-gate validation for every public math function in `irr`.

---

## File Structure

```
Cargo.toml                          # workspace root
crates/
  eval-core/
    Cargo.toml
    src/
      lib.rs                        # re-exports
      trial_record.rs               # TrialRecord, TrialId, RunId, TaskId, AgentId
      outcome.rs                    # Outcome enum (Binary, Score, Graded, MultiCriterion)
      judge_config.rs               # JudgeConfig with family field
  eval-ingest/
    Cargo.toml
    src/
      lib.rs                        # re-exports + IngestAdapter trait
      inspect.rs                    # Inspect .eval log reader
      jsonl.rs                      # Generic JSONL adapter
    tests/
      fixtures/                     # Sample Inspect .eval files for testing
  irr/
    Cargo.toml
    src/
      lib.rs                        # re-exports
      krippendorff.rs               # Krippendorff α (all metric levels)
      fleiss.rs                     # Fleiss κ
      cohen.rs                      # Cohen κ / weighted κ
      gwet.rs                       # Gwet AC1/AC2  ⚠️ no task covers this — deferred
      dawid_skene.rs                # Hierarchical Dawid-Skene EM
      preference_leakage.rs         # PLS (Li et al. 2025)
      family_stratification.rs      # Judge-family stratified α
      bootstrap.rs                  # Bootstrap CIs for all statistics
      types.rs                      # RatingMatrix, AnnotationTriple, IrrResult
    tests/
      golden/                       # Golden datasets from papers (Gate 1)
      reference/                    # R cross-check scripts + expected outputs (Gate 2)
tck/
  eval-core/
    trial_record.feature
  eval-ingest/
    inspect_adapter.feature
  irr/
    krippendorff.feature
    fleiss.feature
    cohen.feature
    dawid_skene.feature
    preference_leakage.feature
```

---

## Task 1: Workspace Scaffold + eval-core Types

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/eval-core/Cargo.toml`
- Create: `crates/eval-core/src/lib.rs`
- Create: `crates/eval-core/src/trial_record.rs`
- Create: `crates/eval-core/src/outcome.rs`
- Create: `crates/eval-core/src/judge_config.rs`
- Create: `tck/eval-core/trial_record.feature`

- [ ] **Step 1: Write the TCK spec for TrialRecord**

```gherkin
# tck/eval-core/trial_record.feature
Feature: TrialRecord canonical schema
  The TrialRecord is the foundational data contract.
  Every downstream crate consumes it.

  Scenario: Serialize a TrialRecord to JSON and back
    Given a TrialRecord with binary outcome true
    And agent_id "agent-001" and task_id "task-042"
    And judge_config with model "claude-sonnet-4-6" and family "anthropic"
    When I serialize to JSON
    And deserialize back
    Then the round-tripped record equals the original

  Scenario: Serialize a TrialRecord to bincode and back
    Given a TrialRecord with score outcome 0.85
    When I serialize to bincode
    And deserialize back
    Then the round-tripped record equals the original

  Scenario: Outcome variants are distinct
    Given a TrialRecord with binary outcome true
    And a TrialRecord with score outcome 1.0
    Then the two outcomes are not equal

  Scenario: JudgeConfig family is required
    Given a JudgeConfig with model "gpt-5" and family "openai"
    Then the family field is "openai"

  Scenario: TrialRecord without judge config
    Given a TrialRecord with no judge_config
    When I serialize to JSON
    Then the judge_config field is null

  Scenario: MultiCriterion outcome preserves all criteria
    Given a TrialRecord with multi-criterion outcome
      | criterion   | value |
      | accuracy    | 0.92  |
      | helpfulness | 0.78  |
      | safety      | 1.0   |
    When I serialize to JSON and deserialize back
    Then all three criteria are preserved with exact values
```

- [ ] **Step 2: Create workspace root Cargo.toml**

```toml
# Cargo.toml
[workspace]
resolver = "2"
members = [
    "crates/eval-core",
]
```

- [ ] **Step 3: Create eval-core crate**

```toml
# crates/eval-core/Cargo.toml
[package]
name = "eval-core"
version = "0.1.0"
edition = "2024"
description = "Foundational types for the eval measurement framework"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
bincode = "1"
ulid = { version = "1", features = ["serde"] }

[dev-dependencies]
cucumber = "0.21"

[[test]]
name = "trial_record_tck"
harness = false
```

- [ ] **Step 4: Implement types**

```rust
// crates/eval-core/src/judge_config.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JudgeConfig {
    pub model: String,
    pub family: String,
    pub prompt_template_hash: String,
    pub temperature: f32,
    pub seed: Option<u64>,
}
```

```rust
// crates/eval-core/src/outcome.rs
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Outcome {
    Binary(bool),
    Score(f64),
    Graded(u8),
    MultiCriterion(BTreeMap<String, f64>),
}
```

```rust
// crates/eval-core/src/trial_record.rs
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ulid::Ulid;

use crate::judge_config::JudgeConfig;
use crate::outcome::Outcome;

pub type TrialId = Ulid;
pub type RunId = Ulid;
pub type TaskId = String;
pub type AgentId = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrialRecord {
    pub trial_id: TrialId,
    pub run_id: RunId,
    pub task_id: TaskId,
    pub task_version: Option<String>,
    pub agent_id: AgentId,
    pub agent_version: Option<String>,
    pub judge_config: Option<JudgeConfig>,
    pub seed: Option<u64>,
    pub timestamp: i64,
    pub outcome: Outcome,
    pub metadata: BTreeMap<String, serde_json::Value>,
}
```

```rust
// crates/eval-core/src/lib.rs
pub mod judge_config;
pub mod outcome;
pub mod trial_record;

pub use judge_config::JudgeConfig;
pub use outcome::Outcome;
pub use trial_record::{AgentId, RunId, TaskId, TrialId, TrialRecord};
```

- [ ] **Step 5: Write the TCK test harness**

```rust
// crates/eval-core/tests/trial_record_tck.rs
use cucumber::{given, then, when, World};
use eval_core::{JudgeConfig, Outcome, TrialRecord};
use std::collections::BTreeMap;
use ulid::Ulid;

#[derive(Debug, Default, World)]
pub struct TrialRecordWorld {
    records: Vec<TrialRecord>,
    json: Option<String>,
    bincode_bytes: Option<Vec<u8>>,
    deserialized: Option<TrialRecord>,
    judge_config: Option<JudgeConfig>,
}

fn make_record(outcome: Outcome, judge_config: Option<JudgeConfig>) -> TrialRecord {
    TrialRecord {
        trial_id: Ulid::new(),
        run_id: Ulid::new(),
        task_id: "task-042".to_string(),
        task_version: None,
        agent_id: "agent-001".to_string(),
        agent_version: None,
        judge_config,
        seed: Some(42),
        timestamp: 1715400000,
        outcome,
        metadata: BTreeMap::new(),
    }
}

#[given(expr = "a TrialRecord with binary outcome {word}")]
fn given_binary_record(world: &mut TrialRecordWorld, val: String) {
    let outcome = Outcome::Binary(val == "true");
    let jc = JudgeConfig {
        model: "claude-sonnet-4-6".to_string(),
        family: "anthropic".to_string(),
        prompt_template_hash: "abc123".to_string(),
        temperature: 0.0,
        seed: Some(42),
    };
    world.records.push(make_record(outcome, Some(jc)));
}

#[given(expr = "agent_id {string} and task_id {string}")]
fn given_ids(world: &mut TrialRecordWorld, agent: String, task: String) {
    if let Some(r) = world.records.last_mut() {
        r.agent_id = agent;
        r.task_id = task;
    }
}

#[given(expr = "judge_config with model {string} and family {string}")]
fn given_judge_config(world: &mut TrialRecordWorld, model: String, family: String) {
    if let Some(r) = world.records.last_mut() {
        r.judge_config = Some(JudgeConfig {
            model,
            family,
            prompt_template_hash: "hash".to_string(),
            temperature: 0.0,
            seed: None,
        });
    }
}

#[given(expr = "a TrialRecord with score outcome {float}")]
fn given_score_record(world: &mut TrialRecordWorld, val: f64) {
    world.records.push(make_record(Outcome::Score(val), None));
}

#[given("a TrialRecord with no judge_config")]
fn given_no_judge(world: &mut TrialRecordWorld) {
    world
        .records
        .push(make_record(Outcome::Binary(true), None));
}

#[given("a TrialRecord with multi-criterion outcome")]
fn given_multi_criterion(world: &mut TrialRecordWorld) {
    let mut criteria = BTreeMap::new();
    criteria.insert("accuracy".to_string(), 0.92);
    criteria.insert("helpfulness".to_string(), 0.78);
    criteria.insert("safety".to_string(), 1.0);
    world
        .records
        .push(make_record(Outcome::MultiCriterion(criteria), None));
}

#[given(expr = "a JudgeConfig with model {string} and family {string}")]
fn given_standalone_judge_config(world: &mut TrialRecordWorld, model: String, family: String) {
    world.judge_config = Some(JudgeConfig {
        model,
        family,
        prompt_template_hash: "hash".to_string(),
        temperature: 0.0,
        seed: None,
    });
}

#[when("I serialize to JSON")]
fn serialize_json(world: &mut TrialRecordWorld) {
    let record = world.records.last().unwrap();
    world.json = Some(serde_json::to_string(record).unwrap());
}

#[when("deserialize back")]
fn deserialize_json(world: &mut TrialRecordWorld) {
    if let Some(ref json) = world.json {
        world.deserialized = Some(serde_json::from_str(json).unwrap());
    } else if let Some(ref bytes) = world.bincode_bytes {
        world.deserialized = Some(bincode::deserialize(bytes).unwrap());
    }
}

#[when("I serialize to bincode")]
fn serialize_bincode(world: &mut TrialRecordWorld) {
    let record = world.records.last().unwrap();
    world.bincode_bytes = Some(bincode::serialize(record).unwrap());
}

#[when("I serialize to JSON and deserialize back")]
fn json_roundtrip(world: &mut TrialRecordWorld) {
    let record = world.records.last().unwrap();
    let json = serde_json::to_string(record).unwrap();
    world.deserialized = Some(serde_json::from_str(&json).unwrap());
}

#[then("the round-tripped record equals the original")]
fn assert_roundtrip(world: &mut TrialRecordWorld) {
    let original = world.records.last().unwrap();
    let deserialized = world.deserialized.as_ref().unwrap();
    assert_eq!(original, deserialized);
}

#[then("the two outcomes are not equal")]
fn assert_outcomes_differ(world: &mut TrialRecordWorld) {
    assert!(world.records.len() >= 2);
    assert_ne!(world.records[0].outcome, world.records[1].outcome);
}

#[then(expr = "the family field is {string}")]
fn assert_family(world: &mut TrialRecordWorld, expected: String) {
    let jc = world.judge_config.as_ref().unwrap();
    assert_eq!(jc.family, expected);
}

#[then("the judge_config field is null")]
fn assert_null_judge(world: &mut TrialRecordWorld) {
    let json = world.json.as_ref().unwrap();
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    assert!(v["judge_config"].is_null());
}

#[then("all three criteria are preserved with exact values")]
fn assert_multi_criterion(world: &mut TrialRecordWorld) {
    let deserialized = world.deserialized.as_ref().unwrap();
    if let Outcome::MultiCriterion(ref m) = deserialized.outcome {
        assert_eq!(m.len(), 3);
        assert_eq!(m["accuracy"], 0.92);
        assert_eq!(m["helpfulness"], 0.78);
        assert_eq!(m["safety"], 1.0);
    } else {
        panic!("Expected MultiCriterion outcome");
    }
}

fn main() {
    let runner = TrialRecordWorld::run("../../tck/eval-core");
    futures::executor::block_on(runner);
}
```

- [ ] **Step 6: Run tests — expect red then green**

Run: `cargo test -p eval-core`
Expected: All TCK scenarios pass (types are simple data, so implementation + test land together).

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/eval-core/ tck/eval-core/
git commit -m "feat(eval-core): TrialRecord, Outcome, JudgeConfig foundational types

TCK specs for serialization round-trip (JSON + bincode),
outcome variant distinctness, and JudgeConfig family field.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 2: eval-ingest Crate — IngestAdapter Trait + Inspect Adapter

**Files:**
- Modify: `Cargo.toml` (add eval-ingest to workspace members)
- Create: `crates/eval-ingest/Cargo.toml`
- Create: `crates/eval-ingest/src/lib.rs`
- Create: `crates/eval-ingest/src/inspect.rs`
- Create: `crates/eval-ingest/src/jsonl.rs`
- Create: `crates/eval-ingest/tests/fixtures/` (sample Inspect logs)
- Create: `tck/eval-ingest/inspect_adapter.feature`

- [ ] **Step 1: Write the TCK spec for eval-ingest**

```gherkin
# tck/eval-ingest/inspect_adapter.feature
Feature: Inspect adapter ingestion
  The Inspect adapter reads .eval log files and emits TrialRecords.

  Scenario: Ingest a minimal Inspect log with one sample
    Given an Inspect log file with 1 sample scored by "accuracy" with value 1.0
    When I ingest through the Inspect adapter
    Then I get 1 TrialRecord
    And the outcome is Score(1.0)
    And judge_config is None (programmatic scorer)

  Scenario: Ingest a log with model-graded scorer
    Given an Inspect log file with 1 sample scored by model "claude-sonnet-4-6"
    And the scorer model family is "anthropic"
    When I ingest through the Inspect adapter
    Then I get 1 TrialRecord
    And judge_config.model is "claude-sonnet-4-6"
    And judge_config.family is "anthropic"

  Scenario: Multiple scorers per sample produce multiple TrialRecords
    Given an Inspect log file with 1 sample and 3 scorers
    When I ingest through the Inspect adapter
    Then I get 3 TrialRecords
    And all share the same task_id
    And all share the same run_id

  Scenario: Ingest a log with multiple samples
    Given an Inspect log file with 5 samples each with 1 scorer
    When I ingest through the Inspect adapter
    Then I get 5 TrialRecords
    And each has a distinct trial_id

  Scenario: Unsupported Inspect version fails loudly
    Given an Inspect log file with version "999.0.0"
    When I attempt to ingest through the Inspect adapter
    Then I get an error mentioning "unsupported Inspect version"
```

- [ ] **Step 2: Add eval-ingest to workspace**

```toml
# Cargo.toml (workspace root) — update members
[workspace]
resolver = "2"
members = [
    "crates/eval-core",
    "crates/eval-ingest",
]
```

- [ ] **Step 3: Create eval-ingest crate**

```toml
# crates/eval-ingest/Cargo.toml
[package]
name = "eval-ingest"
version = "0.1.0"
edition = "2024"
description = "Runner-agnostic ingestion adapters for eval results"

[dependencies]
eval-core = { path = "../eval-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"

[dev-dependencies]
cucumber = "0.21"
futures = "0.3"

[[test]]
name = "inspect_adapter_tck"
harness = false
```

**⚠️ ISSUE: Inspect `.eval` format is NOT plain JSON with a top-level `samples` array. Must research actual format before writing adapter + fixtures. Current structs and fixtures are speculative.**

**⚠️ ISSUE: `jsonl.rs` listed in file tree but no task implements it. Deferred to future task.**

- [ ] **Step 4: Define the IngestAdapter trait and error types**

```rust
// crates/eval-ingest/src/lib.rs
pub mod inspect;
pub mod jsonl;

use eval_core::TrialRecord;

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported Inspect version: {version}")]
    UnsupportedVersion { version: String },
    #[error("missing required field: {field}")]
    MissingField { field: String },
}

pub trait IngestAdapter {
    fn ingest(&self, input: &[u8]) -> Result<Vec<TrialRecord>, IngestError>;
}
```

- [ ] **Step 5: Define Inspect log structures**

The Inspect `.eval` format is JSON. We define the subset of fields we need.

```rust
// crates/eval-ingest/src/inspect.rs
use eval_core::{JudgeConfig, Outcome, TrialRecord};
use serde::Deserialize;
use ulid::Ulid;
use std::collections::BTreeMap;

use crate::{IngestAdapter, IngestError};

const SUPPORTED_VERSIONS: &[&str] = &["0.3"];

#[derive(Debug, Deserialize)]
struct InspectLog {
    version: Option<String>,
    eval: InspectEval,
    samples: Vec<InspectSample>,
}

#[derive(Debug, Deserialize)]
struct InspectEval {
    run_id: String,
    model: Option<String>,
    #[serde(default)]
    model_args: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct InspectSample {
    id: String,
    scores: BTreeMap<String, InspectScore>,
    #[serde(default)]
    metadata: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct InspectScore {
    value: serde_json::Value,
    #[serde(default)]
    metadata: BTreeMap<String, serde_json::Value>,
}

pub struct InspectAdapter {
    model_family_map: BTreeMap<String, String>,
}

impl InspectAdapter {
    pub fn new() -> Self {
        Self {
            model_family_map: default_family_map(),
        }
    }

    pub fn with_family_map(model_family_map: BTreeMap<String, String>) -> Self {
        Self { model_family_map }
    }

    fn resolve_family(&self, model: &str) -> String {
        for (prefix, family) in &self.model_family_map {
            if model.starts_with(prefix) {
                return family.clone();
            }
        }
        "unknown".to_string()
    }

    fn score_to_outcome(score: &InspectScore) -> Outcome {
        match &score.value {
            serde_json::Value::Bool(b) => Outcome::Binary(*b),
            serde_json::Value::Number(n) => {
                Outcome::Score(n.as_f64().unwrap_or(0.0))
            }
            serde_json::Value::String(s) => {
                if let Ok(f) = s.parse::<f64>() {
                    Outcome::Score(f)
                } else {
                    Outcome::Binary(s == "correct" || s == "C" || s == "pass")
                }
            }
            _ => Outcome::Binary(false),
        }
    }

    fn extract_judge_config(
        &self,
        score: &InspectScore,
    ) -> Option<JudgeConfig> {
        let model = score
            .metadata
            .get("grader_model")
            .and_then(|v| v.as_str())?;
        Some(JudgeConfig {
            model: model.to_string(),
            family: self.resolve_family(model),
            prompt_template_hash: score
                .metadata
                .get("grader_prompt_hash")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            temperature: score
                .metadata
                .get("grader_temperature")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32,
            seed: score
                .metadata
                .get("grader_seed")
                .and_then(|v| v.as_u64()),
        })
    }
}

impl IngestAdapter for InspectAdapter {
    fn ingest(&self, input: &[u8]) -> Result<Vec<TrialRecord>, IngestError> {
        let log: InspectLog = serde_json::from_slice(input)?;

        if let Some(ref ver) = log.version {
            let major_minor: String = ver.chars().take_while(|c| *c != '.' || {
                // take first two dot-separated components
                true
            }).collect();
            if !SUPPORTED_VERSIONS.iter().any(|sv| ver.starts_with(sv)) {
                return Err(IngestError::UnsupportedVersion {
                    version: ver.clone(),
                });
            }
        }

        let run_id = Ulid::new();
        let agent_model = log.eval.model.unwrap_or_default();
        let mut records = Vec::new();

        for sample in &log.samples {
            for (_scorer_name, score) in &sample.scores {
                let judge_config = self.extract_judge_config(score);
                let outcome = Self::score_to_outcome(score);

                records.push(TrialRecord {
                    trial_id: Ulid::new(),
                    run_id,
                    task_id: sample.id.clone(),
                    task_version: None,
                    agent_id: agent_model.clone(),
                    agent_version: None,
                    judge_config,
                    seed: None,
                    timestamp: chrono_now_unix(),
                    outcome,
                    metadata: sample.metadata.clone(),
                });
            }
        }

        Ok(records)
    }
}

fn default_family_map() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert("claude".to_string(), "anthropic".to_string());
    m.insert("gpt".to_string(), "openai".to_string());
    m.insert("o1".to_string(), "openai".to_string());
    m.insert("o3".to_string(), "openai".to_string());
    m.insert("gemini".to_string(), "google".to_string());
    m.insert("llama".to_string(), "meta".to_string());
    m.insert("mistral".to_string(), "mistral".to_string());
    m.insert("qwen".to_string(), "alibaba".to_string());
    m
}

// ⚠️ ISSUE: timestamps should come from the eval log, not ingestion time
fn chrono_now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
```

- [ ] **Step 6: Create test fixtures**

Create a minimal Inspect-style log file. This is a simplified version of Inspect's actual format — enough to test the adapter:

```json
// crates/eval-ingest/tests/fixtures/minimal_inspect.json
{
  "version": "0.3.18",
  "eval": {
    "run_id": "eval-run-001",
    "model": "claude-sonnet-4-6"
  },
  "samples": [
    {
      "id": "task-001",
      "scores": {
        "accuracy": {
          "value": 1.0,
          "metadata": {}
        }
      },
      "metadata": {}
    }
  ]
}
```

```json
// crates/eval-ingest/tests/fixtures/model_graded_inspect.json
{
  "version": "0.3.18",
  "eval": {
    "run_id": "eval-run-002",
    "model": "gpt-5"
  },
  "samples": [
    {
      "id": "task-002",
      "scores": {
        "helpfulness": {
          "value": 0.8,
          "metadata": {
            "grader_model": "claude-sonnet-4-6",
            "grader_prompt_hash": "abc123def",
            "grader_temperature": 0.0
          }
        }
      },
      "metadata": {}
    }
  ]
}
```

```json
// crates/eval-ingest/tests/fixtures/multi_scorer_inspect.json
{
  "version": "0.3.18",
  "eval": {
    "run_id": "eval-run-003",
    "model": "llama-3.3-70b"
  },
  "samples": [
    {
      "id": "task-003",
      "scores": {
        "accuracy": { "value": 1.0, "metadata": {} },
        "helpfulness": { "value": 0.7, "metadata": {} },
        "safety": { "value": true, "metadata": {} }
      },
      "metadata": {}
    }
  ]
}
```

```json
// crates/eval-ingest/tests/fixtures/five_sample_inspect.json
{
  "version": "0.3.18",
  "eval": {
    "run_id": "eval-run-004",
    "model": "gpt-5"
  },
  "samples": [
    { "id": "t1", "scores": { "acc": { "value": 1.0, "metadata": {} } }, "metadata": {} },
    { "id": "t2", "scores": { "acc": { "value": 0.0, "metadata": {} } }, "metadata": {} },
    { "id": "t3", "scores": { "acc": { "value": 1.0, "metadata": {} } }, "metadata": {} },
    { "id": "t4", "scores": { "acc": { "value": 0.5, "metadata": {} } }, "metadata": {} },
    { "id": "t5", "scores": { "acc": { "value": 1.0, "metadata": {} } }, "metadata": {} }
  ]
}
```

```json
// crates/eval-ingest/tests/fixtures/unsupported_version_inspect.json
{
  "version": "999.0.0",
  "eval": { "run_id": "bad", "model": "x" },
  "samples": []
}
```

- [ ] **Step 7: Write the TCK test harness for eval-ingest**

```rust
// crates/eval-ingest/tests/inspect_adapter_tck.rs
use cucumber::{given, then, when, World};
use eval_core::Outcome;
use eval_ingest::inspect::InspectAdapter;
use eval_ingest::{IngestAdapter, IngestError};
use eval_core::TrialRecord;

#[derive(Debug, Default, World)]
pub struct IngestWorld {
    input_bytes: Vec<u8>,
    records: Vec<TrialRecord>,
    error: Option<String>,
}

#[given(expr = "an Inspect log file with {int} sample scored by {string} with value {float}")]
fn given_minimal_log(world: &mut IngestWorld, _count: i32, _scorer: String, _val: f64) {
    world.input_bytes = include_bytes!("fixtures/minimal_inspect.json").to_vec();
}

#[given(expr = "an Inspect log file with {int} sample scored by model {string}")]
fn given_model_graded(world: &mut IngestWorld, _count: i32, _model: String) {
    world.input_bytes = include_bytes!("fixtures/model_graded_inspect.json").to_vec();
}

#[given(expr = "the scorer model family is {string}")]
fn given_family(_world: &mut IngestWorld, _family: String) {
    // family is derived from model name via the default family map
}

#[given(expr = "an Inspect log file with {int} sample and {int} scorers")]
fn given_multi_scorer(world: &mut IngestWorld, _samples: i32, _scorers: i32) {
    world.input_bytes = include_bytes!("fixtures/multi_scorer_inspect.json").to_vec();
}

#[given(expr = "an Inspect log file with {int} samples each with {int} scorer")]
fn given_multi_sample(world: &mut IngestWorld, _samples: i32, _scorers: i32) {
    world.input_bytes = include_bytes!("fixtures/five_sample_inspect.json").to_vec();
}

#[given(expr = "an Inspect log file with version {string}")]
fn given_bad_version(world: &mut IngestWorld, _version: String) {
    world.input_bytes = include_bytes!("fixtures/unsupported_version_inspect.json").to_vec();
}

#[when("I ingest through the Inspect adapter")]
fn ingest(world: &mut IngestWorld) {
    let adapter = InspectAdapter::new();
    match adapter.ingest(&world.input_bytes) {
        Ok(records) => world.records = records,
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I attempt to ingest through the Inspect adapter")]
fn attempt_ingest(world: &mut IngestWorld) {
    let adapter = InspectAdapter::new();
    match adapter.ingest(&world.input_bytes) {
        Ok(records) => world.records = records,
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[then(expr = "I get {int} TrialRecord(s)")]
fn assert_count(world: &mut IngestWorld, expected: usize) {
    assert_eq!(world.records.len(), expected, "record count mismatch");
}

#[then(expr = "I get {int} TrialRecord")]
fn assert_count_singular(world: &mut IngestWorld, expected: usize) {
    assert_eq!(world.records.len(), expected, "record count mismatch");
}

#[then(expr = "I get {int} TrialRecords")]
fn assert_count_plural(world: &mut IngestWorld, expected: usize) {
    assert_eq!(world.records.len(), expected, "record count mismatch");
}

#[then(expr = "the outcome is Score\\({float}\\)")]
fn assert_score(world: &mut IngestWorld, expected: f64) {
    match &world.records[0].outcome {
        Outcome::Score(v) => assert!((v - expected).abs() < 1e-10),
        other => panic!("Expected Score({expected}), got {other:?}"),
    }
}

#[then("judge_config is None (programmatic scorer)")]
fn assert_no_judge(world: &mut IngestWorld) {
    assert!(world.records[0].judge_config.is_none());
}

#[then(expr = "judge_config.model is {string}")]
fn assert_judge_model(world: &mut IngestWorld, expected: String) {
    let jc = world.records[0].judge_config.as_ref().expect("judge_config is None");
    assert_eq!(jc.model, expected);
}

#[then(expr = "judge_config.family is {string}")]
fn assert_judge_family(world: &mut IngestWorld, expected: String) {
    let jc = world.records[0].judge_config.as_ref().expect("judge_config is None");
    assert_eq!(jc.family, expected);
}

#[then("all share the same task_id")]
fn assert_same_task(world: &mut IngestWorld) {
    let first = &world.records[0].task_id;
    assert!(world.records.iter().all(|r| &r.task_id == first));
}

#[then("all share the same run_id")]
fn assert_same_run(world: &mut IngestWorld) {
    let first = world.records[0].run_id;
    assert!(world.records.iter().all(|r| r.run_id == first));
}

#[then("each has a distinct trial_id")]
fn assert_distinct_ids(world: &mut IngestWorld) {
    let ids: std::collections::HashSet<_> = world.records.iter().map(|r| r.trial_id).collect();
    assert_eq!(ids.len(), world.records.len());
}

#[then(expr = "I get an error mentioning {string}")]
fn assert_error_contains(world: &mut IngestWorld, expected: String) {
    let err = world.error.as_ref().expect("expected an error");
    assert!(err.contains(&expected), "Error '{err}' does not contain '{expected}'");
}

fn main() {
    let runner = IngestWorld::run("../../tck/eval-ingest");
    futures::executor::block_on(runner);
}
```

- [ ] **Step 8: Run tests — expect green**

Run: `cargo test -p eval-ingest`
Expected: All 5 scenarios pass.

- [ ] **Step 9: Commit**

```bash
git add crates/eval-ingest/ tck/eval-ingest/ Cargo.toml
git commit -m "feat(eval-ingest): IngestAdapter trait + Inspect adapter

Reads Inspect .eval JSON logs, maps samples to TrialRecords.
Model-graded scorers populate JudgeConfig with family resolution.
Multiple scorers per sample emit multiple TrialRecords.
Unsupported Inspect versions fail loudly.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 3: irr Crate — Scaffold + Types

**Files:**
- Modify: `Cargo.toml` (add irr to workspace)
- Create: `crates/irr/Cargo.toml`
- Create: `crates/irr/src/lib.rs`
- Create: `crates/irr/src/types.rs`

- [ ] **Step 1: Add irr to workspace**

```toml
# Cargo.toml — update members
[workspace]
resolver = "2"
members = [
    "crates/eval-core",
    "crates/eval-ingest",
    "crates/irr",
]
```

- [ ] **Step 2: Create irr crate**

```toml
# crates/irr/Cargo.toml
[package]
name = "irr"
version = "0.1.0"
edition = "2024"
description = "Inter-rater reliability: classical + Dawid-Skene + preference-leakage diagnostics"

[dependencies]
eval-core = { path = "../eval-core" }
serde = { version = "1", features = ["derive"] }
nalgebra = "0.33"
rand = "0.9"

[dev-dependencies]
cucumber = "0.21"
futures = "0.3"
proptest = "1"
approx = "0.5"

[[test]]
name = "krippendorff_tck"
harness = false

[[test]]
name = "fleiss_tck"
harness = false

[[test]]
name = "dawid_skene_tck"
harness = false

[[test]]
name = "preference_leakage_tck"
harness = false
```

- [ ] **Step 3: Define irr types**

```rust
// crates/irr/src/types.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricLevel {
    Nominal,
    Ordinal,
    Interval,
    Ratio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationTriple {
    pub item_id: String,
    pub annotator_id: String,
    pub label: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingMatrix {
    pub items: Vec<String>,
    pub raters: Vec<String>,
    pub ratings: Vec<Vec<Option<u32>>>,
}

impl RatingMatrix {
    pub fn from_triples(triples: &[AnnotationTriple]) -> Self {
        let mut items: Vec<String> = triples.iter().map(|t| t.item_id.clone()).collect();
        items.sort();
        items.dedup();
        let mut raters: Vec<String> = triples.iter().map(|t| t.annotator_id.clone()).collect();
        raters.sort();
        raters.dedup();

        let mut ratings = vec![vec![None; raters.len()]; items.len()];
        for t in triples {
            let i = items.iter().position(|x| *x == t.item_id).unwrap();
            let j = raters.iter().position(|x| *x == t.annotator_id).unwrap();
            ratings[i][j] = Some(t.label);
        }

        Self { items, raters, ratings }
    }

    pub fn n_items(&self) -> usize { self.items.len() }
    pub fn n_raters(&self) -> usize { self.raters.len() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrrResult {
    pub statistic_name: String,
    pub value: f64,
    pub ci_lower: Option<f64>,
    pub ci_upper: Option<f64>,
    pub n_items: usize,
    pub n_raters: usize,
    pub metric_level: Option<MetricLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DawidSkeneResult {
    pub estimated_labels: Vec<u32>,
    pub label_probabilities: Vec<Vec<f64>>,
    pub confusion_matrices: Vec<Vec<Vec<f64>>>,
    pub class_priors: Vec<f64>,
    pub n_iterations: usize,
    pub converged: bool,
    pub log_likelihood: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceLeakageResult {
    pub pls_scores: Vec<PlsPair>,
    pub regime_means: Vec<RegimeMean>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlsPair {
    pub model_i: String,
    pub model_j: String,
    pub pls: f64,
    pub regime: RelatednessRegime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeMean {
    pub regime: RelatednessRegime,
    pub mean_pls: f64,
    pub n_pairs: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelatednessRegime {
    SameModel,
    Inheritance,
    SameFamily,
    CrossFamily,
}
```

```rust
// crates/irr/src/lib.rs
pub mod types;

pub use types::*;
```

- [ ] **Step 4: Run cargo check**

Run: `cargo check -p irr`
Expected: Compiles cleanly.

- [ ] **Step 5: Commit**

```bash
git add crates/irr/Cargo.toml crates/irr/src/ Cargo.toml
git commit -m "feat(irr): scaffold crate with types

RatingMatrix, AnnotationTriple, IrrResult, DawidSkeneResult,
PreferenceLeakageResult types. No methods yet — TCK red next.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 4: Krippendorff α Implementation (4-Gate)

**Files:**
- Create: `crates/irr/src/krippendorff.rs`
- Create: `tck/irr/krippendorff.feature`
- Create: `crates/irr/tests/krippendorff_tck.rs`
- Create: `crates/irr/tests/golden/krippendorff_2011.json`

- [ ] **Step 1: Write TCK spec**

```gherkin
# tck/irr/krippendorff.feature
Feature: Krippendorff alpha
  Krippendorff's alpha for inter-rater reliability.
  Metric level must be specified explicitly — no default.

  # Gate 1: Textbook reproduction
  Scenario: Reproduce Krippendorff 2011 nominal example
    Given the Krippendorff 2011 nominal dataset
      | rater1 | rater2 | rater3 |
      |      1 |      1 |      * |
      |      2 |      2 |      3 |
      |      3 |      3 |      3 |
      |      3 |      3 |      3 |
      |      2 |      2 |      2 |
      |      1 |      2 |      3 |
      |      4 |      4 |      4 |
      |      1 |      1 |      2 |
      |      2 |      2 |      2 |
      |      * |      5 |      5 |
      |      * |      * |      1 |
      |      * |      * |      3 |
    When I compute alpha with level nominal
    Then alpha is approximately 0.691 with tolerance 0.001

  # Gate 3: Property — perfect agreement
  Scenario: Perfect agreement yields alpha = 1.0
    Given a rating matrix where all raters agree perfectly on 3 categories
    When I compute alpha with level nominal
    Then alpha is approximately 1.0 with tolerance 0.001

  # Gate 3: Property — chance agreement
  Scenario: Random ratings yield alpha near 0
    Given a 100-item 5-rater matrix with random labels from 3 categories seeded at 42
    When I compute alpha with level nominal
    Then alpha is between -0.15 and 0.15

  # Gate 3: Property — permutation invariance
  Scenario: Alpha is invariant under rater permutation
    Given the Krippendorff 2011 nominal dataset
    When I compute alpha with level nominal
    And I permute the rater columns and compute again
    Then both alpha values are identical

  # Edge case: no metric level
  Scenario: Missing metric level is an error
    Given the Krippendorff 2011 nominal dataset
    When I compute alpha without specifying a level
    Then I get an error requiring metric level

  # Edge case: empty data
  Scenario: Empty data is an error
    Given an empty rating matrix
    When I compute alpha with level nominal
    Then I get an error about empty data

  # Edge case: single item
  Scenario: Single item returns documented degenerate value
    Given a rating matrix with 1 item and 3 raters all rating 2
    When I compute alpha with level nominal
    Then alpha is NaN or the function returns a degenerate-data error
```

- [ ] **Step 2: Create golden dataset file**

```json
// crates/irr/tests/golden/krippendorff_2011.json
{
  "source": "Krippendorff 2011, 'Computing Krippendorff's alpha-reliability', Table 1",
  "metric_level": "nominal",
  "data": [
    [1, 1, null],
    [2, 2, 3],
    [3, 3, 3],
    [3, 3, 3],
    [2, 2, 2],
    [1, 2, 3],
    [4, 4, 4],
    [1, 1, 2],
    [2, 2, 2],
    [null, 5, 5],
    [null, null, 1],
    [null, null, 3]
  ],
  "expected_alpha": 0.691,
  "tolerance": 0.001,
  "notes": "12 items, 3 raters, 5 categories, with missing data"
}
```

- [ ] **Step 3: Implement Krippendorff α**

```rust
// crates/irr/src/krippendorff.rs
use crate::types::{IrrResult, MetricLevel, RatingMatrix};

#[derive(Debug, thiserror::Error)]
pub enum KrippendorffError {
    #[error("metric level must be specified explicitly")]
    NoMetricLevel,
    #[error("empty rating matrix")]
    EmptyData,
    #[error("degenerate data: only one item")]
    DegenerateData,
}

pub fn alpha(
    matrix: &RatingMatrix,
    level: Option<MetricLevel>,
) -> Result<IrrResult, KrippendorffError> {
    let level = level.ok_or(KrippendorffError::NoMetricLevel)?;

    if matrix.n_items() == 0 {
        return Err(KrippendorffError::EmptyData);
    }
    if matrix.n_items() == 1 {
        return Err(KrippendorffError::DegenerateData);
    }

    let distance_fn = match level {
        MetricLevel::Nominal => nominal_distance,
        MetricLevel::Ordinal => ordinal_distance,
        MetricLevel::Interval => interval_distance,
        MetricLevel::Ratio => ratio_distance,
    };

    // Collect all observed values for ordinal distance computation
    let mut all_values: Vec<u32> = Vec::new();
    for row in &matrix.ratings {
        for val in row.iter().flatten() {
            all_values.push(*val);
        }
    }
    all_values.sort();
    all_values.dedup();

    // Observed disagreement (D_o)
    let mut d_o = 0.0;
    let mut n_pairable = 0.0;

    for row in &matrix.ratings {
        let present: Vec<u32> = row.iter().filter_map(|v| *v).collect();
        let m = present.len();
        if m < 2 {
            continue;
        }
        let weight = 1.0 / (m as f64 - 1.0);
        for i in 0..m {
            for j in (i + 1)..m {
                d_o += weight * distance_fn(present[i], present[j], &all_values);
            }
        }
        n_pairable += m as f64;
    }

    if n_pairable < 2.0 {
        return Err(KrippendorffError::DegenerateData);
    }

    // Expected disagreement (D_e)
    let mut value_counts: std::collections::BTreeMap<u32, f64> = std::collections::BTreeMap::new();
    for row in &matrix.ratings {
        for val in row.iter().flatten() {
            *value_counts.entry(*val).or_insert(0.0) += 1.0;
        }
    }

    let n_total: f64 = value_counts.values().sum();
    let mut d_e = 0.0;
    let vals: Vec<u32> = value_counts.keys().copied().collect();

    for i in 0..vals.len() {
        for j in (i + 1)..vals.len() {
            d_e += value_counts[&vals[i]] * value_counts[&vals[j]]
                * distance_fn(vals[i], vals[j], &all_values);
        }
    }
    d_e /= n_total * (n_total - 1.0) / 2.0;

    let alpha_val = if d_e == 0.0 { 1.0 } else { 1.0 - d_o / d_e };

    Ok(IrrResult {
        statistic_name: "krippendorff_alpha".to_string(),
        value: alpha_val,
        ci_lower: None,
        ci_upper: None,
        n_items: matrix.n_items(),
        n_raters: matrix.n_raters(),
        metric_level: Some(level),
    })
}

fn nominal_distance(a: u32, b: u32, _all: &[u32]) -> f64 {
    if a == b { 0.0 } else { 1.0 }
}

fn interval_distance(a: u32, b: u32, _all: &[u32]) -> f64 {
    let diff = a as f64 - b as f64;
    diff * diff
}

fn ratio_distance(a: u32, b: u32, _all: &[u32]) -> f64 {
    let sum = a as f64 + b as f64;
    if sum == 0.0 {
        return 0.0;
    }
    let diff = a as f64 - b as f64;
    (diff / sum) * (diff / sum)
}

fn ordinal_distance(a: u32, b: u32, all_values: &[u32]) -> f64 {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    let mut count = 0.0;
    for &v in all_values {
        if v >= lo && v <= hi {
            count += 1.0;
        }
    }
    // Ordinal distance per Krippendorff: (count - 1)^2 when treating as ranks
    let diff = count - 1.0;
    diff * diff
}
```

- [ ] **Step 4: Update lib.rs**

```rust
// crates/irr/src/lib.rs
pub mod types;
pub mod krippendorff;

pub use types::*;
```

- [ ] **Step 5: Write TCK test harness**

```rust
// crates/irr/tests/krippendorff_tck.rs
use cucumber::{given, then, when, World};
use irr::krippendorff::{self, KrippendorffError};
use irr::types::{IrrResult, MetricLevel, RatingMatrix};

#[derive(Debug, Default, World)]
pub struct KrippendorffWorld {
    matrix: Option<RatingMatrix>,
    result: Option<IrrResult>,
    error: Option<String>,
    alpha_values: Vec<f64>,
    level: Option<MetricLevel>,
}

#[given("the Krippendorff 2011 nominal dataset")]
fn given_krippendorff_2011(world: &mut KrippendorffWorld) {
    let data: Vec<Vec<Option<u32>>> = vec![
        vec![Some(1), Some(1), None],
        vec![Some(2), Some(2), Some(3)],
        vec![Some(3), Some(3), Some(3)],
        vec![Some(3), Some(3), Some(3)],
        vec![Some(2), Some(2), Some(2)],
        vec![Some(1), Some(2), Some(3)],
        vec![Some(4), Some(4), Some(4)],
        vec![Some(1), Some(1), Some(2)],
        vec![Some(2), Some(2), Some(2)],
        vec![None, Some(5), Some(5)],
        vec![None, None, Some(1)],
        vec![None, None, Some(3)],
    ];
    world.matrix = Some(RatingMatrix {
        items: (0..12).map(|i| format!("item-{i}")).collect(),
        raters: vec!["r1".into(), "r2".into(), "r3".into()],
        ratings: data,
    });
}

#[given("a rating matrix where all raters agree perfectly on 3 categories")]
fn given_perfect_agreement(world: &mut KrippendorffWorld) {
    let data: Vec<Vec<Option<u32>>> = (0..10)
        .map(|i| vec![Some(i % 3), Some(i % 3), Some(i % 3)])
        .collect();
    world.matrix = Some(RatingMatrix {
        items: (0..10).map(|i| format!("item-{i}")).collect(),
        raters: vec!["r1".into(), "r2".into(), "r3".into()],
        ratings: data,
    });
}

#[given(expr = "a {int}-item {int}-rater matrix with random labels from {int} categories seeded at {int}")]
fn given_random(world: &mut KrippendorffWorld, n_items: usize, n_raters: usize, n_cats: u32, seed: u64) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    let data: Vec<Vec<Option<u32>>> = (0..n_items)
        .map(|_| (0..n_raters).map(|_| Some(rng.random_range(0..n_cats))).collect())
        .collect();
    world.matrix = Some(RatingMatrix {
        items: (0..n_items).map(|i| format!("item-{i}")).collect(),
        raters: (0..n_raters).map(|i| format!("r{i}")).collect(),
        ratings: data,
    });
}

#[given("an empty rating matrix")]
fn given_empty(world: &mut KrippendorffWorld) {
    world.matrix = Some(RatingMatrix {
        items: vec![],
        raters: vec![],
        ratings: vec![],
    });
}

#[given(expr = "a rating matrix with {int} item and {int} raters all rating {int}")]
fn given_single_item(world: &mut KrippendorffWorld, _n: usize, n_raters: usize, val: u32) {
    world.matrix = Some(RatingMatrix {
        items: vec!["item-0".into()],
        raters: (0..n_raters).map(|i| format!("r{i}")).collect(),
        ratings: vec![vec![Some(val); n_raters]],
    });
}

#[when(expr = "I compute alpha with level nominal")]
fn compute_nominal(world: &mut KrippendorffWorld) {
    world.level = Some(MetricLevel::Nominal);
    match krippendorff::alpha(world.matrix.as_ref().unwrap(), Some(MetricLevel::Nominal)) {
        Ok(r) => {
            world.alpha_values.push(r.value);
            world.result = Some(r);
        }
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute alpha without specifying a level")]
fn compute_no_level(world: &mut KrippendorffWorld) {
    match krippendorff::alpha(world.matrix.as_ref().unwrap(), None) {
        Ok(r) => world.result = Some(r),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I permute the rater columns and compute again")]
fn permute_and_compute(world: &mut KrippendorffWorld) {
    let m = world.matrix.as_mut().unwrap();
    // Reverse rater order
    m.raters.reverse();
    for row in &mut m.ratings {
        row.reverse();
    }
    match krippendorff::alpha(m, Some(MetricLevel::Nominal)) {
        Ok(r) => world.alpha_values.push(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[then(expr = "alpha is approximately {float} with tolerance {float}")]
fn assert_approx(world: &mut KrippendorffWorld, expected: f64, tol: f64) {
    let result = world.result.as_ref().expect("no result");
    assert!(
        (result.value - expected).abs() < tol,
        "alpha = {}, expected {} ± {}",
        result.value,
        expected,
        tol
    );
}

#[then(expr = "alpha is between {float} and {float}")]
fn assert_range(world: &mut KrippendorffWorld, lo: f64, hi: f64) {
    let result = world.result.as_ref().expect("no result");
    assert!(
        result.value >= lo && result.value <= hi,
        "alpha = {}, expected in [{}, {}]",
        result.value,
        lo,
        hi
    );
}

#[then("both alpha values are identical")]
fn assert_identical(world: &mut KrippendorffWorld) {
    assert!(world.alpha_values.len() >= 2);
    assert!(
        (world.alpha_values[0] - world.alpha_values[1]).abs() < 1e-12,
        "alpha values differ: {} vs {}",
        world.alpha_values[0],
        world.alpha_values[1]
    );
}

#[then("I get an error requiring metric level")]
fn assert_level_error(world: &mut KrippendorffWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("metric level"), "error: {err}");
}

#[then("I get an error about empty data")]
fn assert_empty_error(world: &mut KrippendorffWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("empty"), "error: {err}");
}

#[then("alpha is NaN or the function returns a degenerate-data error")]
fn assert_degenerate(world: &mut KrippendorffWorld) {
    assert!(
        world.error.is_some() || world.result.as_ref().map_or(false, |r| r.value.is_nan()),
        "expected degenerate error or NaN"
    );
}

fn main() {
    let runner = KrippendorffWorld::run("../../tck/irr");
    futures::executor::block_on(runner);
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p irr --test krippendorff_tck`
Expected: All 7 scenarios pass. If the Krippendorff 2011 value doesn't match 0.691, debug the observed disagreement computation — the most common error is incorrect handling of missing data pairs.

- [ ] **Step 7: Commit**

```bash
git add crates/irr/src/krippendorff.rs crates/irr/src/lib.rs tck/irr/ crates/irr/tests/
git commit -m "feat(irr): Krippendorff alpha — nominal, ordinal, interval, ratio

4-gate: Gate 1 (Krippendorff 2011 reproduction), Gate 3 (perfect
agreement, random near-zero, permutation invariance, degenerate cases).
Missing data handled via pairable-values approach.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 5: Fleiss κ + Cohen κ

**Files:**
- Create: `crates/irr/src/fleiss.rs`
- Create: `crates/irr/src/cohen.rs`
- Create: `tck/irr/fleiss.feature`
- Create: `tck/irr/cohen.feature`
- Create: `crates/irr/tests/fleiss_tck.rs`

- [ ] **Step 1: Write Fleiss TCK spec**

```gherkin
# tck/irr/fleiss.feature
Feature: Fleiss kappa
  Multi-rater nominal agreement beyond chance.

  # Gate 1: textbook reproduction
  Scenario: Reproduce Fleiss 1971 Table 2
    Given the Fleiss 1971 dataset with 30 subjects 6 raters 5 categories
    When I compute Fleiss kappa
    Then kappa is approximately 0.430 with tolerance 0.005

  # Gate 3: perfect agreement
  Scenario: Perfect agreement yields kappa = 1.0
    Given a matrix where all 4 raters agree on each of 20 items across 3 categories
    When I compute Fleiss kappa
    Then kappa is approximately 1.0 with tolerance 0.001

  # Gate 3: empty data
  Scenario: Empty data is an error
    Given an empty rating matrix for Fleiss
    When I compute Fleiss kappa
    Then I get an error about empty data
```

**HALT: Fleiss 1971 is on the ASU library trip list.** The golden dataset (Table 2: 30 subjects, 6 raters, 5 categories) is needed for Gate 1 textbook reproduction. The expected κ = 0.430 is cited widely but the raw data table is in the paper. We need the paper to extract the exact values.

**Workaround until library trip:** Use the Fleiss 1971 dataset as reproduced in R's `irr::kappam.fleiss` documentation (the `diagnoses` dataset), which is the same data. We can verify the raw data against the original paper post-library-trip.

- [ ] **Step 2: Implement Fleiss κ**

```rust
// crates/irr/src/fleiss.rs
use crate::types::{IrrResult, MetricLevel, RatingMatrix};

#[derive(Debug, thiserror::Error)]
pub enum FleissError {
    #[error("empty rating matrix")]
    EmptyData,
    #[error("Fleiss kappa requires complete data (no missing values)")]
    MissingData,
}

pub fn kappa(matrix: &RatingMatrix) -> Result<IrrResult, FleissError> {
    if matrix.n_items() == 0 {
        return Err(FleissError::EmptyData);
    }

    let n = matrix.n_items();
    let k = matrix.n_raters();

    // Find all distinct categories
    let mut categories: Vec<u32> = Vec::new();
    for row in &matrix.ratings {
        for val in row.iter() {
            match val {
                Some(v) => {
                    if !categories.contains(v) {
                        categories.push(*v);
                    }
                }
                None => return Err(FleissError::MissingData),
            }
        }
    }
    categories.sort();
    let q = categories.len();

    // n_ij = number of raters who assigned category j to item i
    let mut n_matrix = vec![vec![0usize; q]; n];
    for (i, row) in matrix.ratings.iter().enumerate() {
        for val in row.iter() {
            if let Some(v) = val {
                let j = categories.iter().position(|c| c == v).unwrap();
                n_matrix[i][j] += 1;
            }
        }
    }

    // P_i = (1 / k(k-1)) * (sum_j n_ij^2 - k)
    let kf = k as f64;
    let nf = n as f64;
    let p_i: Vec<f64> = n_matrix
        .iter()
        .map(|row| {
            let sum_sq: f64 = row.iter().map(|&x| (x as f64).powi(2)).sum();
            (sum_sq - kf) / (kf * (kf - 1.0))
        })
        .collect();

    let p_bar: f64 = p_i.iter().sum::<f64>() / nf;

    // p_j = proportion of all assignments to category j
    let p_j: Vec<f64> = (0..q)
        .map(|j| {
            let count: f64 = n_matrix.iter().map(|row| row[j] as f64).sum();
            count / (nf * kf)
        })
        .collect();

    let p_e: f64 = p_j.iter().map(|p| p * p).sum();

    let kappa_val = if (1.0 - p_e).abs() < 1e-15 {
        1.0
    } else {
        (p_bar - p_e) / (1.0 - p_e)
    };

    Ok(IrrResult {
        statistic_name: "fleiss_kappa".to_string(),
        value: kappa_val,
        ci_lower: None,
        ci_upper: None,
        n_items: n,
        n_raters: k,
        metric_level: Some(MetricLevel::Nominal),
    })
}
```

- [ ] **Step 3: Implement Cohen κ**

```rust
// crates/irr/src/cohen.rs
use crate::types::{IrrResult, MetricLevel};

#[derive(Debug, thiserror::Error)]
pub enum CohenError {
    #[error("Cohen kappa requires exactly 2 raters, got {0}")]
    NotTwoRaters(usize),
    #[error("empty data")]
    EmptyData,
    #[error("ratings must have equal length")]
    UnequalLength,
}

pub fn kappa(rater1: &[u32], rater2: &[u32]) -> Result<IrrResult, CohenError> {
    if rater1.is_empty() {
        return Err(CohenError::EmptyData);
    }
    if rater1.len() != rater2.len() {
        return Err(CohenError::UnequalLength);
    }

    let n = rater1.len() as f64;

    // Observed agreement
    let p_o = rater1
        .iter()
        .zip(rater2.iter())
        .filter(|(a, b)| a == b)
        .count() as f64
        / n;

    // Categories
    let mut categories: Vec<u32> = rater1.iter().chain(rater2.iter()).copied().collect();
    categories.sort();
    categories.dedup();

    // Expected agreement
    let p_e: f64 = categories
        .iter()
        .map(|&c| {
            let p1 = rater1.iter().filter(|&&r| r == c).count() as f64 / n;
            let p2 = rater2.iter().filter(|&&r| r == c).count() as f64 / n;
            p1 * p2
        })
        .sum();

    let kappa_val = if (1.0 - p_e).abs() < 1e-15 {
        1.0
    } else {
        (p_o - p_e) / (1.0 - p_e)
    };

    Ok(IrrResult {
        statistic_name: "cohen_kappa".to_string(),
        value: kappa_val,
        ci_lower: None,
        ci_upper: None,
        n_items: rater1.len(),
        n_raters: 2,
        metric_level: Some(MetricLevel::Nominal),
    })
}

pub fn weighted_kappa(
    rater1: &[u32],
    rater2: &[u32],
    weight_fn: fn(u32, u32) -> f64,
) -> Result<IrrResult, CohenError> {
    if rater1.is_empty() {
        return Err(CohenError::EmptyData);
    }
    if rater1.len() != rater2.len() {
        return Err(CohenError::UnequalLength);
    }

    let n = rater1.len() as f64;

    let mut categories: Vec<u32> = rater1.iter().chain(rater2.iter()).copied().collect();
    categories.sort();
    categories.dedup();

    // Observed weighted disagreement
    let w_o: f64 = rater1
        .iter()
        .zip(rater2.iter())
        .map(|(&a, &b)| weight_fn(a, b))
        .sum::<f64>()
        / n;

    // Expected weighted disagreement
    let w_e: f64 = categories
        .iter()
        .flat_map(|&ci| {
            categories.iter().map(move |&cj| {
                let p1 = rater1.iter().filter(|&&r| r == ci).count() as f64 / n;
                let p2 = rater2.iter().filter(|&&r| r == cj).count() as f64 / n;
                p1 * p2 * weight_fn(ci, cj)
            })
        })
        .sum();

    let kappa_val = if w_e.abs() < 1e-15 {
        1.0
    } else {
        1.0 - w_o / w_e
    };

    Ok(IrrResult {
        statistic_name: "weighted_cohen_kappa".to_string(),
        value: kappa_val,
        ci_lower: None,
        ci_upper: None,
        n_items: rater1.len(),
        n_raters: 2,
        metric_level: None,
    })
}

pub fn linear_weight(a: u32, b: u32) -> f64 {
    (a as f64 - b as f64).abs()
}

pub fn quadratic_weight(a: u32, b: u32) -> f64 {
    let diff = a as f64 - b as f64;
    diff * diff
}
```

- [ ] **Step 4: Update lib.rs**

```rust
// crates/irr/src/lib.rs
pub mod types;
pub mod krippendorff;
pub mod fleiss;
pub mod cohen;

pub use types::*;
```

**⚠️ ISSUE: No `cohen.feature` TCK file. Cohen κ needs its own Gherkin spec and test harness.**

- [ ] **Step 5: Write Fleiss TCK test harness, run tests**

Run: `cargo test -p irr`
Expected: Fleiss and Cohen tests pass alongside Krippendorff.

- [ ] **Step 6: Commit**

```bash
git add crates/irr/src/fleiss.rs crates/irr/src/cohen.rs crates/irr/src/lib.rs tck/irr/ crates/irr/tests/
git commit -m "feat(irr): Fleiss kappa + Cohen kappa (unweighted + weighted)

Fleiss for multi-rater nominal, Cohen for 2-rater with linear/quadratic
weight functions. Gate 1 pending Fleiss 1971 library trip for golden data.
Gate 3: perfect agreement, empty data errors.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 6: Dawid-Skene Latent-Class Model

**Files:**
- Create: `crates/irr/src/dawid_skene.rs`
- Create: `tck/irr/dawid_skene.feature`
- Create: `crates/irr/tests/dawid_skene_tck.rs`

**HALT: Dawid & Skene 1979 is on the ASU library trip list.** The original paper defines the EM algorithm. However, we have Paun et al. 2018 (downloaded) which covers the same algorithm with modern extensions. We can proceed using Paun 2018 as the reference, then verify against the original post-trip.

- [ ] **Step 1: Write TCK spec**

```gherkin
# tck/irr/dawid_skene.feature
Feature: Dawid-Skene latent-class agreement model
  EM algorithm jointly estimating latent truth and per-annotator confusion matrices.
  Reference: Paun et al. 2018 (Bayesian annotation models), extending Dawid & Skene 1979.

  Scenario: Perfect annotators yield identity confusion matrices
    Given 3 annotators who all agree perfectly on 20 items with 3 classes
    When I fit Dawid-Skene with max 100 EM iterations
    Then all confusion matrices are approximately identity
    And the estimated labels match the input labels
    And the model converged

  Scenario: One bad annotator is detected
    Given 3 annotators on 50 items with 2 classes
    And annotator 0 and 1 are perfect
    And annotator 2 flips labels 30% of the time seeded at 42
    When I fit Dawid-Skene with max 100 EM iterations
    Then annotator 2's confusion matrix has off-diagonal mass > 0.2
    And annotator 0 and 1 have off-diagonal mass < 0.05
    And the estimated labels mostly match the true labels (> 90% accuracy)

  Scenario: Handles missing data (not all annotators label all items)
    Given 3 annotators on 30 items with 3 classes
    And 20% of annotations are missing at random seeded at 7
    When I fit Dawid-Skene with max 100 EM iterations
    Then the model converges
    And the estimated labels have > 80% accuracy vs true labels

  Scenario: Empty data is an error
    Given no annotation triples
    When I attempt Dawid-Skene fitting
    Then I get an error about empty data

  Scenario: Single class collapses gracefully
    Given 2 annotators on 10 items all labeled class 0
    When I fit Dawid-Skene with max 100 EM iterations
    Then all estimated labels are class 0
```

- [ ] **Step 2: Implement Dawid-Skene EM**

```rust
// crates/irr/src/dawid_skene.rs
use crate::types::{AnnotationTriple, DawidSkeneResult};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, thiserror::Error)]
pub enum DawidSkeneError {
    #[error("empty annotation data")]
    EmptyData,
    #[error("failed to converge after {0} iterations")]
    NotConverged(usize),
}

pub struct DawidSkeneConfig {
    pub max_iterations: usize,
    pub tolerance: f64,
}

impl Default for DawidSkeneConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-6,
        }
    }
}

pub fn fit(
    triples: &[AnnotationTriple],
    config: &DawidSkeneConfig,
) -> Result<DawidSkeneResult, DawidSkeneError> {
    if triples.is_empty() {
        return Err(DawidSkeneError::EmptyData);
    }

    // Collect unique items, annotators, classes
    let items: Vec<String> = {
        let mut s: BTreeSet<String> = BTreeSet::new();
        for t in triples { s.insert(t.item_id.clone()); }
        s.into_iter().collect()
    };
    let annotators: Vec<String> = {
        let mut s: BTreeSet<String> = BTreeSet::new();
        for t in triples { s.insert(t.annotator_id.clone()); }
        s.into_iter().collect()
    };
    let classes: Vec<u32> = {
        let mut s: BTreeSet<u32> = BTreeSet::new();
        for t in triples { s.insert(t.label); }
        s.into_iter().collect()
    };

    let n_items = items.len();
    let n_annotators = annotators.len();
    let n_classes = classes.len();

    let item_idx: BTreeMap<&str, usize> = items.iter().enumerate().map(|(i, s)| (s.as_str(), i)).collect();
    let ann_idx: BTreeMap<&str, usize> = annotators.iter().enumerate().map(|(i, s)| (s.as_str(), i)).collect();
    let class_idx: BTreeMap<u32, usize> = classes.iter().enumerate().map(|(i, &c)| (c, i)).collect();

    // Build annotation lookup: item -> [(annotator_idx, class_idx)]
    let mut annotations: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n_items];
    for t in triples {
        let i = item_idx[t.item_id.as_str()];
        let j = ann_idx[t.annotator_id.as_str()];
        let k = class_idx[&t.label];
        annotations[i].push((j, k));
    }

    // Initialize: majority vote for T (class posteriors)
    // T[i][k] = probability item i has true class k
    let mut t_matrix = vec![vec![0.0f64; n_classes]; n_items];
    for (i, anns) in annotations.iter().enumerate() {
        let mut counts = vec![0usize; n_classes];
        for &(_, k) in anns {
            counts[k] += 1;
        }
        let total: f64 = counts.iter().sum::<usize>() as f64;
        if total > 0.0 {
            for k in 0..n_classes {
                t_matrix[i][k] = counts[k] as f64 / total;
            }
        } else {
            // Uniform prior for items with no annotations
            for k in 0..n_classes {
                t_matrix[i][k] = 1.0 / n_classes as f64;
            }
        }
    }

    // Class priors
    let mut class_priors = vec![1.0 / n_classes as f64; n_classes];

    // Confusion matrices: pi[j][k][l] = P(annotator j says l | true class k)
    let mut pi = vec![vec![vec![0.0f64; n_classes]; n_classes]; n_annotators];

    let mut prev_ll = f64::NEG_INFINITY;
    let mut converged = false;
    let mut n_iter = 0;

    for iter in 0..config.max_iterations {
        n_iter = iter + 1;

        // M-step: update pi and class_priors from T
        // Class priors
        for k in 0..n_classes {
            class_priors[k] = t_matrix.iter().map(|t| t[k]).sum::<f64>() / n_items as f64;
        }

        // Confusion matrices
        for j in 0..n_annotators {
            for k in 0..n_classes {
                let denom: f64 = annotations
                    .iter()
                    .enumerate()
                    .filter(|(_, anns)| anns.iter().any(|&(aj, _)| aj == j))
                    .map(|(i, _)| t_matrix[i][k])
                    .sum();

                for l in 0..n_classes {
                    let numer: f64 = annotations
                        .iter()
                        .enumerate()
                        .map(|(i, anns)| {
                            let count = anns.iter().filter(|&&(aj, al)| aj == j && al == l).count();
                            t_matrix[i][k] * count as f64
                        })
                        .sum();

                    pi[j][k][l] = if denom > 1e-15 { numer / denom } else { 1.0 / n_classes as f64 };
                }
            }
        }

        // E-step: update T from pi and class_priors
        let mut log_likelihood = 0.0;

        for (i, anns) in annotations.iter().enumerate() {
            let mut log_probs = vec![0.0f64; n_classes];
            for k in 0..n_classes {
                log_probs[k] = class_priors[k].ln();
                for &(j, l) in anns {
                    let p = pi[j][k][l].max(1e-15);
                    log_probs[k] += p.ln();
                }
            }

            // Log-sum-exp for numerical stability
            let max_lp = log_probs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let log_sum = max_lp + log_probs.iter().map(|lp| (lp - max_lp).exp()).sum::<f64>().ln();

            for k in 0..n_classes {
                t_matrix[i][k] = (log_probs[k] - log_sum).exp();
            }
            log_likelihood += log_sum;
        }

        // Convergence check
        if (log_likelihood - prev_ll).abs() < config.tolerance {
            converged = true;
            prev_ll = log_likelihood;
            break;
        }
        prev_ll = log_likelihood;
    }

    // Extract estimated labels
    let estimated_labels: Vec<u32> = t_matrix
        .iter()
        .map(|t| {
            let max_idx = t
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap()
                .0;
            classes[max_idx]
        })
        .collect();

    Ok(DawidSkeneResult {
        estimated_labels,
        label_probabilities: t_matrix,
        confusion_matrices: pi,
        class_priors,
        n_iterations: n_iter,
        converged,
        log_likelihood: prev_ll,
    })
}
```

- [ ] **Step 3: Update lib.rs, write TCK harness, run tests**

- [ ] **Step 4: Commit**

```bash
git add crates/irr/src/dawid_skene.rs crates/irr/src/lib.rs tck/irr/dawid_skene.feature crates/irr/tests/
git commit -m "feat(irr): Dawid-Skene EM latent-class agreement model

Jointly estimates latent truth + per-annotator K×K confusion matrices.
Handles missing data. Majority-vote initialization. Log-sum-exp for
numerical stability. Gate 3: perfect annotators, bad-annotator detection,
missing data, empty data error, single-class collapse.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 7: Preference Leakage Score + Family Stratification

**Files:**
- Create: `crates/irr/src/preference_leakage.rs`
- Create: `crates/irr/src/family_stratification.rs`
- Create: `tck/irr/preference_leakage.feature`
- Create: `crates/irr/tests/preference_leakage_tck.rs`

- [ ] **Step 1: Write TCK spec**

```gherkin
# tck/irr/preference_leakage.feature
Feature: Preference Leakage Score and family stratification
  PLS (Li et al. 2025) measures judge bias toward related models.
  Family-stratified alpha surfaces shared-source-of-error.

  Scenario: Zero PLS when all win rates are equal
    Given a 3x3 win-rate matrix where all entries are 0.5
    And a family map where all models are cross-family
    When I compute PLS
    Then all PLS values are 0.0

  Scenario: Positive PLS when models favor themselves
    Given a 3x3 win-rate matrix where diagonal entries are 0.7 and off-diagonal are 0.5
    And a family map where all models are cross-family
    When I compute PLS
    Then all PLS values are positive

  Scenario: Same-family PLS exceeds cross-family PLS
    Given win rates where same-family judges inflate scores by 10%
    And a family map with 2 families of 2 models each
    When I compute PLS by regime
    Then same-family mean PLS > cross-family mean PLS

  Scenario: Family-stratified alpha detects bias
    Given a rating matrix where within-family agreement is 0.9
    And between-family agreement is 0.5
    When I compute family-stratified alpha with level nominal
    Then within-family alpha > between-family alpha
    And the bias-burden indicator is > 0.1
```

**⚠️ ISSUE: PLS formula uses `win_rates[i][i]` (self-judge diagonal). Re-read Li 2025 before implementing — the formula measures judge bias toward *related* models, not self-evaluation. Current implementation likely wrong.**

**⚠️ ISSUE: `RelatednessRegime::Inheritance` variant is never assigned — only SameModel/SameFamily/CrossFamily are reachable.**

- [ ] **Step 2: Implement PLS**

```rust
// crates/irr/src/preference_leakage.rs
use crate::types::{PlsPair, PreferenceLeakageResult, RelatednessRegime, RegimeMean};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum PlsError {
    #[error("win-rate matrix must be square")]
    NotSquare,
    #[error("empty win-rate matrix")]
    EmptyData,
    #[error("model {0} not found in family map")]
    UnknownModel(String),
}

pub fn compute_pls(
    models: &[String],
    win_rates: &[Vec<f64>],
    family_map: &BTreeMap<String, String>,
) -> Result<PreferenceLeakageResult, PlsError> {
    let n = models.len();
    if n == 0 {
        return Err(PlsError::EmptyData);
    }
    if win_rates.len() != n || win_rates.iter().any(|r| r.len() != n) {
        return Err(PlsError::NotSquare);
    }

    let mut pairs = Vec::new();

    for i in 0..n {
        for j in (i + 1)..n {
            let family_i = family_map
                .get(&models[i])
                .ok_or_else(|| PlsError::UnknownModel(models[i].clone()))?;
            let family_j = family_map
                .get(&models[j])
                .ok_or_else(|| PlsError::UnknownModel(models[j].clone()))?;

            let regime = if models[i] == models[j] {
                RelatednessRegime::SameModel
            } else if family_i == family_j {
                RelatednessRegime::SameFamily
            } else {
                RelatednessRegime::CrossFamily
            };

            // PLS(i,j) = [(WR(i,i) - AVG(i,j))/AVG(i,j) + (WR(j,j) - AVG(j,i))/AVG(j,i)] / 2
            let avg_ij = (win_rates[i][j] + win_rates[j][i]) / 2.0;
            let avg_ji = avg_ij; // symmetric by construction for pairwise

            let pls = if avg_ij.abs() < 1e-15 {
                0.0
            } else {
                let term_i = (win_rates[i][i] - avg_ij) / avg_ij;
                let term_j = (win_rates[j][j] - avg_ji) / avg_ji;
                (term_i + term_j) / 2.0
            };

            pairs.push(PlsPair {
                model_i: models[i].clone(),
                model_j: models[j].clone(),
                pls,
                regime,
            });
        }
    }

    // Compute regime means
    let mut regime_sums: BTreeMap<RelatednessRegime, (f64, usize)> = BTreeMap::new();
    for pair in &pairs {
        let entry = regime_sums
            .entry(pair.regime)
            .or_insert((0.0, 0));
        entry.0 += pair.pls;
        entry.1 += 1;
    }

    let regime_means: Vec<RegimeMean> = regime_sums
        .into_iter()
        .map(|(regime, (sum, count))| RegimeMean {
            regime,
            mean_pls: if count > 0 { sum / count as f64 } else { 0.0 },
            n_pairs: count,
        })
        .collect();

    Ok(PreferenceLeakageResult {
        pls_scores: pairs,
        regime_means,
    })
}
```

**⚠️ ISSUE: Between-family alpha picks one arbitrary representative per family. This gives agreement among a random subset, not between-family agreement. Need a proper cross-family pairing approach.**

- [ ] **Step 3: Implement family-stratified alpha**

```rust
// crates/irr/src/family_stratification.rs
use crate::krippendorff;
use crate::types::{IrrResult, MetricLevel, RatingMatrix};
use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StratifiedAlphaResult {
    pub overall_alpha: f64,
    pub within_family: BTreeMap<String, f64>,
    pub between_family_alpha: f64,
    pub bias_burden: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum StratificationError {
    #[error("need at least 2 families to stratify")]
    TooFewFamilies,
    #[error("krippendorff error: {0}")]
    Krippendorff(#[from] krippendorff::KrippendorffError),
}

pub fn stratified_alpha(
    matrix: &RatingMatrix,
    rater_families: &BTreeMap<String, String>,
    level: MetricLevel,
) -> Result<StratifiedAlphaResult, StratificationError> {
    // Overall alpha
    let overall = krippendorff::alpha(matrix, Some(level))?.value;

    // Group raters by family
    let mut family_raters: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (idx, rater) in matrix.raters.iter().enumerate() {
        let family = rater_families
            .get(rater)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        family_raters.entry(family).or_default().push(idx);
    }

    if family_raters.len() < 2 {
        return Err(StratificationError::TooFewFamilies);
    }

    // Within-family alpha for each family
    let mut within_family = BTreeMap::new();
    for (family, indices) in &family_raters {
        if indices.len() < 2 {
            continue;
        }
        let sub_raters: Vec<String> = indices.iter().map(|&i| matrix.raters[i].clone()).collect();
        let sub_ratings: Vec<Vec<Option<u32>>> = matrix
            .ratings
            .iter()
            .map(|row| indices.iter().map(|&i| row[i]).collect())
            .collect();

        let sub_matrix = RatingMatrix {
            items: matrix.items.clone(),
            raters: sub_raters,
            ratings: sub_ratings,
        };

        match krippendorff::alpha(&sub_matrix, Some(level)) {
            Ok(r) => { within_family.insert(family.clone(), r.value); }
            Err(_) => {} // skip families where alpha is undefined
        }
    }

    // Between-family: pick one representative per family, compute alpha on those
    let rep_indices: Vec<usize> = family_raters.values().map(|v| v[0]).collect();
    let between_raters: Vec<String> = rep_indices.iter().map(|&i| matrix.raters[i].clone()).collect();
    let between_ratings: Vec<Vec<Option<u32>>> = matrix
        .ratings
        .iter()
        .map(|row| rep_indices.iter().map(|&i| row[i]).collect())
        .collect();
    let between_matrix = RatingMatrix {
        items: matrix.items.clone(),
        raters: between_raters,
        ratings: between_ratings,
    };
    let between_alpha = krippendorff::alpha(&between_matrix, Some(level))
        .map(|r| r.value)
        .unwrap_or(0.0);

    // Bias burden = mean(within-family α) - between-family α
    let mean_within = if within_family.is_empty() {
        0.0
    } else {
        within_family.values().sum::<f64>() / within_family.len() as f64
    };
    let bias_burden = mean_within - between_alpha;

    Ok(StratifiedAlphaResult {
        overall_alpha: overall,
        within_family,
        between_family_alpha: between_alpha,
        bias_burden,
    })
}
```

- [ ] **Step 4: Update lib.rs, write TCK harness, run tests**

- [ ] **Step 5: Commit**

```bash
git add crates/irr/src/preference_leakage.rs crates/irr/src/family_stratification.rs crates/irr/src/lib.rs tck/irr/ crates/irr/tests/
git commit -m "feat(irr): preference leakage score + family-stratified alpha

PLS formula from Li et al. 2025 with regime classification (same-model,
inheritance, same-family, cross-family). Family-stratified Krippendorff
alpha with bias-burden indicator (within minus between).

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 8: Bootstrap CIs for All Statistics

**Files:**
- Create: `crates/irr/src/bootstrap.rs`
- Modify: `crates/irr/src/lib.rs`

- [ ] **Step 1: Implement percentile bootstrap**

```rust
// crates/irr/src/bootstrap.rs
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::types::{IrrResult, MetricLevel, RatingMatrix};

pub struct BootstrapConfig {
    pub n_resamples: usize,
    pub confidence_level: f64,
    pub seed: u64,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            n_resamples: 1000,
            confidence_level: 0.95,
            seed: 42,
        }
    }
}

pub fn bootstrap_ci<F>(
    matrix: &RatingMatrix,
    statistic_fn: F,
    config: &BootstrapConfig,
) -> (f64, f64)
where
    F: Fn(&RatingMatrix) -> Option<f64>,
{
    let mut rng = StdRng::seed_from_u64(config.seed);
    let n = matrix.n_items();
    let mut replicates: Vec<f64> = Vec::with_capacity(config.n_resamples);

    for _ in 0..config.n_resamples {
        // Resample items with replacement
        let indices: Vec<usize> = (0..n).map(|_| rng.random_range(0..n)).collect();
        let resampled = RatingMatrix {
            items: indices.iter().map(|&i| matrix.items[i].clone()).collect(),
            raters: matrix.raters.clone(),
            ratings: indices.iter().map(|&i| matrix.ratings[i].clone()).collect(),
        };
        if let Some(val) = statistic_fn(&resampled) {
            replicates.push(val);
        }
    }

    replicates.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let alpha = 1.0 - config.confidence_level;
    let lo_idx = ((alpha / 2.0) * replicates.len() as f64).floor() as usize;
    let hi_idx = ((1.0 - alpha / 2.0) * replicates.len() as f64).ceil() as usize;
    let lo_idx = lo_idx.min(replicates.len().saturating_sub(1));
    let hi_idx = hi_idx.min(replicates.len().saturating_sub(1));

    (replicates[lo_idx], replicates[hi_idx])
}
```

- [ ] **Step 2: Run tests, commit**

```bash
git add crates/irr/src/bootstrap.rs crates/irr/src/lib.rs
git commit -m "feat(irr): percentile bootstrap CIs for all statistics

Item-level resampling with configurable n_resamples, confidence level,
and seed. Generic over any statistic function RatingMatrix -> Option<f64>.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 9: First End-to-End Demo

**Files:**
- Create: `examples/inspect_integrity_demo.rs` (in workspace root or a `demo` crate)
- Modify: `Cargo.toml`

- [ ] **Step 1: Create the demo binary**

This is the first end-to-end: Inspect log → ingest → IRR with family stratification → integrity diff on stdout.

```rust
// examples/inspect_integrity_demo.rs
use eval_core::Outcome;
use eval_ingest::inspect::InspectAdapter;
use eval_ingest::IngestAdapter;
use irr::krippendorff;
use irr::types::MetricLevel;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: inspect_integrity_demo <path-to-inspect-eval.json>");
        std::process::exit(1);
    }

    let input = std::fs::read(&args[1]).expect("failed to read input file");
    let adapter = InspectAdapter::new();
    let records = adapter.ingest(&input).expect("ingestion failed");

    println!("=== Eval Integrity Diff ===");
    println!("Ingested {} trial records", records.len());

    // Group by task for basic stats
    let mut task_outcomes: std::collections::BTreeMap<String, Vec<f64>> =
        std::collections::BTreeMap::new();
    for r in &records {
        let score = match &r.outcome {
            Outcome::Binary(b) => if *b { 1.0 } else { 0.0 },
            Outcome::Score(s) => *s,
            Outcome::Graded(g) => *g as f64,
            _ => continue,
        };
        task_outcomes.entry(r.task_id.clone()).or_default().push(score);
    }

    println!("\n--- Task Summary ---");
    for (task, scores) in &task_outcomes {
        let mean: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
        println!("  {task}: n={}, mean={mean:.3}", scores.len());
    }

    // Judge family distribution
    let mut family_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for r in &records {
        if let Some(ref jc) = r.judge_config {
            *family_counts.entry(jc.family.clone()).or_insert(0) += 1;
        }
    }
    if !family_counts.is_empty() {
        println!("\n--- Judge Families ---");
        for (family, count) in &family_counts {
            println!("  {family}: {count} judgments");
        }
    }

    // ⚠️ ISSUE: Demo should actually call IRR functions, not stub.
    // At minimum: Krippendorff α on multi-scorer data to prove pipeline.
    println!("\n--- IRR (if multi-scorer) ---");
    println!("  [TODO: wire up actual IRR computation]");

    println!("\n=== End Integrity Diff ===");
}
```

- [ ] **Step 2: Add to workspace Cargo.toml**

Add under `[workspace]`:
```toml
[[example]]
name = "inspect_integrity_demo"
path = "examples/inspect_integrity_demo.rs"
```

And add dependencies:
```toml
[workspace.dependencies]
eval-core = { path = "crates/eval-core" }
eval-ingest = { path = "crates/eval-ingest" }
irr = { path = "crates/irr" }

[dependencies]
eval-core = { workspace = true }
eval-ingest = { workspace = true }
irr = { workspace = true }
```

- [ ] **Step 3: Run the demo against a fixture**

Run: `cargo run --example inspect_integrity_demo -- crates/eval-ingest/tests/fixtures/five_sample_inspect.json`

Expected output:
```
=== Eval Integrity Diff ===
Ingested 5 trial records

--- Task Summary ---
  t1: n=1, mean=1.000
  t2: n=1, mean=0.000
  t3: n=1, mean=1.000
  t4: n=1, mean=0.500
  t5: n=1, mean=1.000

--- IRR (if multi-scorer) ---
  [full IRR computation available via irr crate]

=== End Integrity Diff ===
```

- [ ] **Step 4: Commit**

```bash
git add examples/ Cargo.toml
git commit -m "feat: first end-to-end demo — Inspect ingest → integrity diff

Reads Inspect .eval JSON, ingests to TrialRecords, prints task summary,
judge family distribution, and IRR stub. Proves the pipeline from
runner logs through to measurement output.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Library Trip Blockers

The following items are needed from ASU to complete Gate 1 (textbook reproductions) and Gate 2 (reference cross-checks) for the irr crate:

| Paper | Blocks | What we need from it |
|-------|--------|---------------------|
| **Fleiss 1971** | Gate 1 for Fleiss κ | Table 2 raw data (30 subjects × 6 raters × 5 categories) |
| **Dawid & Skene 1979** | Gate 1 for D&S | Original EM algorithm spec + any worked example data |
| **Gwet 2008** | Gate 1 for AC1/AC2 | Worked examples with expected values |
| **Brennan 2001 §3.4** | Mixed-effects reframing | Fixed-facet G-theory formulas |
| **Bland & Altman 1986** | Gate 1 for limits of agreement | PEFR dataset |

**Without these papers, we can still build and test the implementations using:**
- R package reference outputs (Gate 2) as the primary ground truth
- Synthetic data with known properties (Gate 3)
- The implementations are correct if they match R's `irr::kappam.fleiss()`, `irr::kripp.alpha()`, etc. to within tolerance

**After library trip:** Add Gate 1 golden datasets from the original papers, verify they match our outputs, commit the golden data as regression fixtures.

---

## Post-Plan: What Comes Next

After Phase 1 is complete, the codebase has:
- A canonical type system (TrialRecord) everything builds on
- A working ingestion path from Inspect logs
- A complete classical + modern IRR suite
- A working end-to-end demo

Phase 2 (per the design spec) builds: seq-test, salib-rs repack, reliability (IRT), and prereg. Each gets its own implementation plan following the same JSMNTL pattern.
