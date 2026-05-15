# Design: eval-orchestrator — Analysis Routing + Decision Engine

## Goal

New crate `eval-orchestrator` that takes TrialRecords and produces typed
analysis decisions. Wires eval-ingest output to the math crates (irr,
seq-anytime-valid, spc-charts). Pure library — no I/O, no persistence,
no network. Two modes: batch analysis and streaming monitor.

## Architecture

```
┌────────────────────────────────────────────────────┐
│  eval-orchestrator                                  │
│                                                    │
│  Batch path:                                       │
│    analyze(records, config) → AnalysisReport       │
│                                                    │
│  Streaming path:                                   │
│    Monitor::push(record) → Vec<Decision>           │
│                                                    │
│  ┌──────────┐    ┌─────────────────────────────┐  │
│  │  Router  │───▶│  Instrument Registry         │  │
│  │(auto-det)│    │  ├── IrrInstrument           │  │
│  └──────────┘    │  ├── SequentialInstrument    │  │
│       │          │  ├── SpcInstrument           │  │
│       │          │  └── (SensitivityInstrument) │  │
│       ▼          └─────────────────────────────┘  │
│  ┌──────────┐              │                      │
│  │ Decision │◀─────────────┘                      │
│  │  Engine  │                                     │
│  └──────────┘                                     │
└────────────────────────────────────────────────────┘
```

Instrument registry pattern. Each instrument implements a trait, auto-
detection routes records to applicable instruments, decision engine
maps instrument outputs to typed Decision signals. Callers act on
decisions — the orchestrator never takes actions itself.

## Core Types

### SeriesKey

Identifies what's being measured. Groups records into analysis streams.

```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SeriesKey {
    pub task_id: String,
    pub agent_id: String,
    pub scorer: Option<String>,
}
```

Derived from TrialRecord: `task_id` + `agent_id` + `metadata["scorer_name"]`.

### Decision

The orchestrator's output vocabulary. Pre-digested signals — callers
don't need to understand statistics.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Decision {
    StopEarly {
        series: SeriesKey,
        evidence: f64,
        estimate: f64,
        ci: (f64, f64),
    },
    ContinueRunning {
        series: SeriesKey,
        current_n: usize,
        estimated_n_needed: usize,
        power_at_current_n: f64,
    },
    Regression {
        series: SeriesKey,
        signal: ChartSignal,
        observation_value: f64,
        control_limits: (f64, f64),
    },
    MeasurementWarning {
        series: SeriesKey,
        issue: MeasurementIssue,
    },
}
```

### MeasurementIssue

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeasurementIssue {
    LowAgreement { kappa: f64, threshold: f64 },
    InsufficientRaters { have: usize, need: usize },
    InsufficientSamples { have: usize, need: usize },
    HighVariance { cv: f64, threshold: f64 },
}
```

### AnalysisReport

Batch output. Carries pre-digested decisions AND raw instrument
summaries for callers that want details.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub decisions: Vec<Decision>,
    pub irr_results: Option<IrrSummary>,
    pub sequential_results: Vec<SequentialSummary>,
    pub spc_results: Vec<SpcSummary>,
    pub series_detected: Vec<SeriesKey>,
    pub instruments_run: Vec<String>,
}
```

### Instrument Summaries

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrrSummary {
    pub series: SeriesKey,
    pub alpha: f64,
    pub n_raters: usize,
    pub n_items: usize,
    pub metric: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialSummary {
    pub series: SeriesKey,
    pub n_observations: usize,
    pub current_estimate: f64,
    pub ci: (f64, f64),
    pub evidence: f64,
    pub stopped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpcSummary {
    pub series: SeriesKey,
    pub n_windows: usize,
    pub chart_type: String,
    pub in_control: bool,
    pub signals: Vec<ChartSignal>,
    pub control_limits: (f64, f64),
}
```

## Configuration

### AnalysisConfig (batch)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub force_enable: Vec<String>,
    pub force_disable: Vec<String>,
    pub irr: IrrConfig,
    pub sequential: SequentialConfig,
    pub spc: SpcConfig,
}
```

### Per-instrument configs (all fields have defaults via Default impl)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrrConfig {
    pub threshold: f64,           // default 0.67 (Krippendorff's minimum)
    pub metric: IrrMetric,        // default Krippendorff
    pub min_raters: usize,        // default 2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrrMetric {
    Krippendorff,
    Fleiss,
    Gwet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialConfig {
    pub alpha: f64,               // default 0.05
    pub min_effect_size: f64,     // default 0.1 (smallest meaningful difference)
    pub method: SequentialMethod, // default EValue
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SequentialMethod {
    EValue,
    Msprt,
    GroupSequential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpcConfig {
    pub chart_type: SpcChartType, // default Ewma
    pub phase1_windows: usize,    // default 20
    pub window_size: WindowSize,  // default PerRun
    pub lambda: f64,              // EWMA smoothing, default 0.2
    pub l: f64,                   // control limit width, default 3.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpcChartType {
    Ewma,
    Cusum,
    Shewhart,
    Combined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowSize {
    PerRun,
    Fixed(usize),
}
```

### MonitorConfig (streaming)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub spc: SpcConfig,
    pub sequential: SequentialConfig,
    pub irr: IrrConfig,
    pub irr_recompute_interval: usize, // recompute every N records, default 50
    pub auto_detect: bool,             // discover series from incoming records
}
```

## Batch API

```rust
pub fn analyze(
    records: &[TrialRecord],
    config: &AnalysisConfig,
) -> AnalysisReport;
```

Steps:
1. Derive SeriesKey for each record
2. Router: for each series, determine applicable instruments (auto-detect + overrides)
3. For each applicable instrument:
   a. Transform records into instrument's native input
   b. Run analysis
   c. Map output to decisions
4. Collect all decisions + summaries into AnalysisReport

## Streaming Monitor API

```rust
#[derive(Serialize, Deserialize)]
pub struct Monitor {
    spc: BTreeMap<SeriesKey, SpcState>,
    sequential: BTreeMap<SeriesKey, SeqState>,
    irr: BTreeMap<SeriesKey, IrrState>,
    config: MonitorConfig,
    observations_seen: u64,
}

impl Monitor {
    pub fn new(config: MonitorConfig) -> Self;
    pub fn push(&mut self, record: &TrialRecord) -> Vec<Decision>;
    pub fn push_batch(&mut self, records: &[TrialRecord]) -> Vec<Decision>;
    pub fn active_series(&self) -> Vec<&SeriesKey>;
    pub fn state_summary(&self) -> MonitorSummary;
}
```

On each `push`:
1. Derive SeriesKey from record
2. If `auto_detect` and series is new, create tracking state
3. Route to each active instrument for this series:
   - SPC: aggregate into current window; on window close, feed chart, check signal
   - Sequential: feed observation value, check e-value against threshold
   - IRR: accumulate judgment; recompute every `irr_recompute_interval` records
4. Return any decisions that fired

### SPC State Machine

```
Phase I (training) ──[phase1_windows reached]──▶ Phase II (monitoring)
```

Phase I: accumulates observations to establish control limits (μ₀, σ).
Phase II: checks each new window against limits, emits Regression on OOC.

### MonitorSummary

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorSummary {
    pub observations_seen: u64,
    pub active_series: usize,
    pub series_in_phase1: usize,
    pub series_in_phase2: usize,
    pub total_decisions_emitted: u64,
}
```

## Instrument Adapters (internal)

### IrrInstrument

- **Accepts when:** ≥2 distinct `judge_config` values exist for any series
- **Transform:** group by series → for each (task_item × judge) pair, extract outcome as ordinal value → build `AnnotationTriple` vec → construct `RatingMatrix`
- **Analyze:** run `krippendorff::alpha()` (or configured metric)
- **Decide:** if alpha < threshold → `MeasurementWarning(LowAgreement)`
- **Item identification:** within a series, items are distinguished by `trial_id` base (without scorer suffix) or `metadata["sample_id"]` when available

### SequentialInstrument

- **Accepts when:** ≥2 observations exist for any series
- **Transform:** group by series → extract outcome as numeric:
  - `Binary(b)` → 0.0 or 1.0
  - `Score(f)` → f
  - `Graded(g)` → g as f64 / 255.0
  - `MultiCriterion(m)` → mean of values
- **Analyze:** feed to `AnytimeMonitor` with configured alpha and effect size
- **Decide:** if e-value > 1/alpha → `StopEarly`; else `ContinueRunning` with power estimate

### SpcInstrument

- **Accepts when:** ≥2 distinct `run_id` values with temporal ordering exist for any series
- **Transform:** group by series → partition into windows (per-run or fixed-size) → compute window statistic (mean for Score/Graded, proportion for Binary)
- **Phase I:** first `phase1_windows` windows establish control limits
- **Phase II:** subsequent windows checked against limits
- **Decide:** on OOC signal → `Regression`

## Auto-Detection Logic

```rust
fn detect_instruments(
    records: &[TrialRecord],
    config: &AnalysisConfig,
) -> Vec<InstrumentId> {
    let mut enabled = Vec::new();

    // IRR: multiple judges per series
    if has_multiple_judges(records) && !config.force_disable.contains("irr") {
        enabled.push(InstrumentId::Irr);
    }

    // Sequential: repeated observations per series
    if has_repeated_observations(records) && !config.force_disable.contains("sequential") {
        enabled.push(InstrumentId::Sequential);
    }

    // SPC: multiple runs with temporal order
    if has_temporal_runs(records) && !config.force_disable.contains("spc") {
        enabled.push(InstrumentId::Spc);
    }

    // Force-enables
    for name in &config.force_enable {
        let id = InstrumentId::from_name(name);
        if !enabled.contains(&id) {
            enabled.push(id);
        }
    }

    enabled
}
```

Detection helpers:
- `has_multiple_judges`: any series has ≥2 distinct `judge_config` values
- `has_repeated_observations`: any series has ≥ `sequential.min_n` (default 10) observations
- `has_temporal_runs`: any series spans ≥2 `run_id` values AND timestamps are ordered

## Error Handling

The orchestrator does not produce hard errors for analysis failures.
If an instrument fails on a subset of data (e.g., singular matrix in IRR
computation), it emits a `MeasurementWarning` decision and continues.

The only errors from `analyze()` are:
```rust
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("no records provided")]
    EmptyInput,
    #[error("no instruments applicable and none force-enabled")]
    NoInstrumentsApplicable,
}
```

## Dependencies

```toml
[dependencies]
eval-core = { path = "../eval-core" }
irr = { path = "../irr" }
seq-anytime-valid = { path = "../seq-anytime-valid" }
spc-charts = { path = "../spc-charts" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"

[dev-dependencies]
tempfile = "3"
```

## Testing Strategy (4-gate)

### Gate 1: Textbook reproductions

- Hand-crafted record set with 3 judges, known disagreement → IRR fires LowAgreement
- Record stream where true mean shifts at observation 50 → SPC fires Regression at ~50
- Binomial data with known effect → sequential test stops at expected N
- Multi-judge + longitudinal + repeated → all three instruments fire together

### Gate 2: Cross-checks

- Same records through orchestrator IRR vs direct `irr::krippendorff::alpha()` → same value
- Same observations through orchestrator SPC vs direct `EwmaChart` → same signals
- Same data through orchestrator sequential vs direct `AnytimeMonitor` → same e-values

### Gate 3: Property-based tests

- Arbitrary valid TrialRecords → analyze never panics
- Arbitrary records → all decisions have valid SeriesKey referencing input data
- Monitor push order invariant: same records in any order → same final state (for batch-equivalent inputs)
- SeriesKey derivation is deterministic and consistent

### Gate 4: Monte Carlo calibration

- Null hypothesis (no change): SPC false alarm rate ≤ nominal over 10k simulations
- Alternative (mean shift): SPC detection power ≥ 0.8 for 1σ shift within 20 windows
- Sequential: Type I error ≤ alpha under null, power ≥ 0.8 under alternative at design N
- Monitor: decisions from push_batch(all) == decisions from push(one-at-a-time)

## File Structure

```
crates/eval-orchestrator/
├── Cargo.toml
├── src/
│   ├── lib.rs              # pub API, re-exports
│   ├── types.rs            # Decision, SeriesKey, MeasurementIssue, summaries
│   ├── config.rs           # AnalysisConfig, MonitorConfig, per-instrument configs
│   ├── router.rs           # auto-detection logic, instrument dispatch
│   ├── instrument.rs       # Instrument trait definition
│   ├── instruments/
│   │   ├── mod.rs
│   │   ├── irr.rs          # IRR adapter
│   │   ├── sequential.rs   # Sequential testing adapter
│   │   └── spc.rs          # SPC adapter
│   ├── analyze.rs          # analyze() batch entry point
│   └── monitor.rs          # Monitor streaming struct
└── tests/
    ├── fixtures/
    ├── batch_tck.rs
    └── monitor_tck.rs
```

## Non-goals

- No I/O, no file reading, no network. Pure computation.
- No persistence — Monitor is Serialize but caller owns save/load.
- No perturbation/sensitivity — deferred to eval-perturb integration.
- No gRPC/REST — that's mojave-cli/serve, not the orchestrator crate.
- No campaign management — that's eval-campaign.

## TCK Scenarios

### batch.feature

```gherkin
Feature: Batch analysis

  Scenario: Auto-detect IRR from multi-judge records
    Given 100 records for task "math" agent "gpt-4o" with 3 distinct judges
    When analyze is called with default config
    Then IrrInstrument is in instruments_run
    And irr_results contains alpha value

  Scenario: Auto-detect sequential from repeated observations
    Given 50 records for task "code" agent "claude" with one judge
    When analyze is called with default config
    Then SequentialInstrument is in instruments_run
    And a StopEarly or ContinueRunning decision is emitted

  Scenario: Auto-detect SPC from multi-run records
    Given 200 records spanning 10 runs for task "safety" agent "gpt-4o"
    When analyze is called with default config
    Then SpcInstrument is in instruments_run
    And spc_results contains chart state

  Scenario: Force-disable overrides auto-detect
    Given multi-judge records that would trigger IRR
    When analyze is called with force_disable = ["irr"]
    Then IrrInstrument is NOT in instruments_run

  Scenario: Force-enable overrides auto-detect
    Given single-run records that would NOT trigger SPC
    When analyze is called with force_enable = ["spc"]
    Then SpcInstrument is in instruments_run

  Scenario: Empty input returns error
    Given no records
    When analyze is called
    Then OrchestratorError::EmptyInput is returned
```

### monitor.feature

```gherkin
Feature: Streaming monitor

  Scenario: SPC detects regression in streaming mode
    Given a Monitor with default config
    When 100 records with mean=0.8 are pushed (phase I)
    And 20 records with mean=0.5 are pushed (phase II)
    Then at least one Regression decision is emitted

  Scenario: Sequential test stops early
    Given a Monitor with sequential alpha=0.05
    When records with true proportion 0.9 are pushed one at a time
    Then a StopEarly decision is emitted before 200 observations

  Scenario: Auto-detect discovers new series
    Given a Monitor with auto_detect=true
    When a record for a previously unseen (task, agent) arrives
    Then the series appears in active_series

  Scenario: Monitor state is serializable
    Given a Monitor with 100 observations pushed
    When serialized to JSON and deserialized
    Then pushing the same next record produces the same decisions

  Scenario: push_batch equivalent to sequential push
    Given a Monitor with default config
    When the same 50 records are processed via push_batch vs individual push
    Then the same decisions are produced
```
