# Design: eval-ingest — Runner-Agnostic Eval Log Import

## Goal

New crate `eval-ingest` that reads eval runner output and produces
`Vec<TrialRecord>`. Runner-agnostic trait with concrete adapters.
Inspect adapter is first. Generic JSONL adapter for BYO runners.

## Architecture

```
eval runner logs (JSON, JSONL, etc.)
       │
       ▼
┌─────────────────────────────────────┐
│  eval-ingest                        │
│                                     │
│  IngestAdapter trait                │
│  ├── InspectAdapter                 │
│  ├── JsonlAdapter (generic, BYO)    │
│  └── (future: lm-harness, etc.)    │
│                                     │
│  Validation + normalization         │
│  Error collection (not fail-fast)   │
└─────────────────────────────────────┘
       │
       ▼
  Vec<TrialRecord>  (eval-core)
```

## Core Trait

```rust
pub trait IngestAdapter {
    fn ingest(&self, source: IngestSource) -> Result<IngestResult, IngestError>;
}

pub enum IngestSource {
    File(PathBuf),
    Dir(PathBuf),
    Reader(Box<dyn Read>),
}

pub struct IngestResult {
    pub records: Vec<TrialRecord>,
    pub warnings: Vec<IngestWarning>,
    pub source_meta: SourceMeta,
}

pub struct SourceMeta {
    pub runner_name: String,
    pub runner_version: Option<String>,
    pub log_format_version: Option<String>,
    pub original_path: Option<PathBuf>,
    pub content_hash: [u8; 32],
}
```

The trait returns `IngestResult`, not just `Vec<TrialRecord>`. Warnings
are collected, not swallowed. `SourceMeta` captures provenance for the
audit chain — what was read, from where, what was its hash.

Errors are **not fail-fast**. A log with 1000 samples where 3 have
malformed scores should produce 997 records + 3 warnings, not an error.
`IngestError` is for structural failures (file not found, not JSON,
completely unrecognizable format).

## InspectAdapter

Maps Inspect's `EvalLog` JSON to `Vec<TrialRecord>`.

### Inspect log structure (v2)

```json
{
  "version": 2,
  "status": "success",
  "eval": {
    "run_id": "abc123",
    "task_id": "security-guide",
    "task_version": 0,
    "model": "openai/gpt-4o",
    "config": { "temperature": 0.5, "seed": 42 },
    "revision": { "commit": "abc123" },
    "packages": { "inspect_ai": "0.3.80" }
  },
  "plan": { "config": { "temperature": 0.5 } },
  "results": { "scores": [...], "metrics": {...} },
  "samples": [
    {
      "id": "sample_1",
      "uuid": "shortuuid_here",
      "epoch": 1,
      "started_at": "2025-05-12T20:28:26-04:00",
      "scores": {
        "scorer_name": {
          "value": "C",
          "answer": "...",
          "metadata": { "grading": [...] }
        }
      },
      "metadata": { "custom_key": "value" }
    }
  ]
}
```

### Field mapping

| TrialRecord | Inspect source | Derivation |
|---|---|---|
| `trial_id` | `sample.uuid` | Hash into ULID. Fallback: hash `(run_id, sample.id, epoch)` for pre-0.3.70 logs |
| `run_id` | `eval.run_id` | Hash into ULID |
| `task_id` | `eval.task_id` | Direct |
| `task_version` | `eval.task_version` | `int` or `str`, stringify |
| `agent_id` | `eval.model` | e.g. `"openai/gpt-4o"` |
| `agent_version` | `eval.revision.commit` | Fallback: `eval.packages["inspect_ai"]` |
| `judge_config` | Scorer metadata | Only populated for model-graded scorers. See below. |
| `seed` | `eval.config.seed` | From `GenerateConfig` |
| `timestamp` | `sample.started_at` | ISO 8601 → Unix epoch seconds |
| `outcome` | `sample.scores[name].value` | See outcome mapping below |
| `metadata` | `sample.metadata` ∪ `eval.metadata` | Merged. Also: `epoch`, `scorer_name`, git `commit` |

### Outcome mapping

| `Score.value` | `Outcome` |
|---|---|
| `"C"` / `true` / `1` | `Binary(true)` |
| `"I"` / `false` / `0` / `"N"` | `Binary(false)` |
| `"P"` | `Score(0.5)` |
| `float` (continuous) | `Score(value)` |
| `int` (0-10 scale, etc.) | `Graded(value as u8)` |
| `dict` | `MultiCriterion(map)` |

### Judge config extraction

For model-graded scorers (identified by `score.metadata.grading`
being present):

- `model`: grading model name from the last assistant message
  in `score.metadata.grading`
- `family`: derive from model name (e.g. `"gpt-4o-mini-2024-07-18"`)
- `prompt_template_hash`: SHA-256 of the first user message content
  in `score.metadata.grading` (the rendered grading prompt)
- `temperature`: from `eval.config.temperature` or `plan.config.temperature`
- `seed`: from `eval.config.seed`

For non-model scorers (exact match, regex, etc.): `judge_config = None`.

### Multi-scorer handling

Inspect supports multiple scorers per sample (`scores` is a dict).
Each scorer produces a separate `TrialRecord`. A sample with 3
scorers produces 3 records sharing the same `trial_id` but with
different `outcome` values and `scorer_name` in metadata.

Decision rationale: the alternative (MultiCriterion for all scorers)
collapses distinct measurement instruments into one record. IRR
analysis needs per-judge records. Keep them separate.

### Epochs

Inspect's epoch system repeats each sample N times. Each epoch
produces a distinct `TrialRecord` with the same `task_id` and
`sample.id` but a different `trial_id` (derived from epoch number).
The `epoch` field goes into metadata.

## JsonlAdapter (Generic)

For customers with their own runners. Reads JSONL where each line
is a JSON object. Requires a `FieldMapping` configuration that
tells it which JSON fields map to which TrialRecord fields.

```rust
pub struct FieldMapping {
    pub task_id: String,          // JSON path, e.g. "task.name"
    pub agent_id: String,         // JSON path
    pub outcome: OutcomeMapping,  // how to interpret the score field
    pub timestamp: Option<String>,
    pub seed: Option<String>,
    pub run_id: Option<String>,
    // ... optional fields with sensible defaults
}

pub enum OutcomeMapping {
    BinaryField(String),             // JSON path → bool
    ScoreField(String),              // JSON path → f64
    GradedField { path: String, max: u8 },
    MultiCriterion(Vec<(String, String)>),  // name → JSON path pairs
    Auto(String),                    // infer from value type
}
```

`FieldMapping` is provided as YAML/JSON config alongside the data.
Reasonable defaults: if a field mapping is omitted, try common names
(`task_id`, `task`, `item_id`, `item` for task_id, etc.).

When `run_id` is not provided, generate one per file (all records
in a file share a run).

When `timestamp` is not provided, use file modification time.

## Validation

Every `TrialRecord` produced by any adapter goes through validation:

1. `trial_id` is set (generated if not derivable)
2. `run_id` is set (generated if not derivable)
3. `task_id` is non-empty
4. `agent_id` is non-empty
5. `timestamp` is a valid Unix epoch (not before 2020, not in the future)
6. `outcome` passes `Outcome::score()` / `Outcome::multi_criterion()` validation (finite values)
7. `metadata` values are valid JSON (no circular refs, no excessively large values)

Validation failures on individual records produce `IngestWarning`,
not `IngestError`. The record is dropped and the warning captures
what went wrong and which source record caused it.

## Error Types

```rust
pub enum IngestError {
    Io(std::io::Error),
    NotJson { path: PathBuf, detail: String },
    UnrecognizedFormat { path: PathBuf, detail: String },
    NoRecordsProduced { path: PathBuf, warnings: Vec<IngestWarning> },
}

pub struct IngestWarning {
    pub source_index: Option<usize>,  // line number or sample index
    pub source_id: Option<String>,    // sample ID if available
    pub kind: WarningKind,
    pub detail: String,
}

pub enum WarningKind {
    MalformedRecord,
    ValidationFailed,
    UnmappableOutcome,
    MissingRequiredField,
    SkippedSample,
}
```

## Dependencies

```toml
[dependencies]
eval-core = { path = "../eval-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
ulid = "1"
sha2 = "0.10"          # content hashing for SourceMeta
chrono = "0.4"          # ISO 8601 parsing for Inspect timestamps

[dev-dependencies]
tempfile = "3"
```

No heavy deps. `chrono` is the heaviest and it's widely used. `sha2`
for content hashing. All pinned and vendored per build discipline.

## Testing Strategy (4-gate)

### Gate 1: Textbook reproductions
- Download a real Inspect eval log (from their test suite or our own
  run) and pin it as a fixture. Assert exact TrialRecord output.
- Same for JSONL: pin a fixture file with known content.

### Gate 2: Reference impl cross-checks
- Run Inspect's own log reader on the same file, compare extracted
  fields to our mapping. Every field we extract must match.

### Gate 3: Property-based tests
- Arbitrary valid Inspect JSON → ingest → every record has valid
  trial_id, run_id, non-empty task_id, non-empty agent_id
- Roundtrip: generate TrialRecords, serialize to JSONL, read back
  with JsonlAdapter, assert equality
- Permutation invariance: sample order in the log doesn't affect
  the set of records produced
- Epoch faithfulness: N epochs × M scorers = N×M records

### Gate 4: Error handling
- Malformed samples produce warnings, not errors
- Empty logs produce `NoRecordsProduced` error
- Mixed valid/invalid samples: valid ones survive
- Completely garbled input: `NotJson` or `UnrecognizedFormat`
- Future Inspect log versions (version > 2): warning, best-effort

## File Structure

```
crates/eval-ingest/
├── Cargo.toml
├── src/
│   ├── lib.rs           # trait, types, re-exports
│   ├── types.rs         # IngestSource, IngestResult, errors, warnings
│   ├── validate.rs      # TrialRecord validation
│   ├── inspect.rs       # InspectAdapter
│   ├── inspect_types.rs # Inspect JSON deserialization types
│   └── jsonl.rs         # JsonlAdapter + FieldMapping
└── tests/
    ├── fixtures/        # pinned log files
    ├── inspect_tck.rs
    └── jsonl_tck.rs
```

## TCK Gherkin Features

### inspect.feature

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
    And the result contains 1 warning for sample 3

  Scenario: Content hash is computed for provenance
    Given an Inspect log file
    When the InspectAdapter ingests the file
    Then source_meta.content_hash is the SHA-256 of the file
    And source_meta.runner_name is "inspect_ai"
```

### jsonl.feature

```gherkin
Feature: Generic JSONL ingestion

  Scenario: Ingest JSONL with explicit field mapping
    Given a JSONL file with fields "item", "model", "pass"
    And a FieldMapping mapping item→task_id, model→agent_id, pass→Binary
    When the JsonlAdapter ingests the file
    Then records have correct task_id, agent_id, and Binary outcome

  Scenario: Ingest JSONL with auto field name detection
    Given a JSONL file with fields "task_id", "agent_id", "score"
    And no explicit FieldMapping
    When the JsonlAdapter ingests the file with auto-detect
    Then records are produced with correct field mapping

  Scenario: Missing optional fields get defaults
    Given a JSONL file with only "task_id", "agent_id", "score"
    When the JsonlAdapter ingests the file
    Then run_id is generated (one per file)
    And timestamp is derived from file modification time
    And trial_id is generated per record

  Scenario: Mixed valid/invalid lines
    Given a JSONL file where line 5 is malformed JSON
    When the JsonlAdapter ingests the file
    Then records are produced for all other lines
    And 1 warning is emitted for line 5
```

## Non-goals

- No streaming ingest in this crate. Batch only. Streaming is the
  orchestrator's job (it wraps ingest + analysis in a monitor loop).
- No network I/O. Reads files from disk. If someone wants to pull
  logs from S3/GCS, they download first.
- No format detection heuristics beyond basic JSON structure.
  The caller specifies which adapter to use (or the CLI auto-detects
  based on file content).
