# eval-orchestrator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the eval-orchestrator crate — the ingest→analyze→decide loop that wires eval-ingest output to the math crates (irr, seq-anytime-valid, spc-charts). Batch + streaming modes, auto-detect routing, typed Decision signals.

**Architecture:** Instrument registry pattern. Each math crate gets an adapter (IrrInstrument, SequentialInstrument, SpcInstrument) behind an `Instrument` trait. A Router auto-detects which instruments apply (multiple judges → IRR, repeated observations → sequential, multiple runs → SPC) with force-enable/disable overrides. Batch `analyze()` runs all applicable instruments and collects decisions. Streaming `Monitor` maintains per-series state, feeds observations to instruments, and returns decisions on each `push()`. Monitor is `Serialize`/`Deserialize` but stores raw observations + configs (not chart objects, since `EwmaChart`/`CusumChart`/`AnytimeMonitor` lack serde derives).

**Tech Stack:** Rust, serde, serde_json, thiserror, eval-core, irr, seq-anytime-valid, spc-charts

---

## File Structure

```
crates/eval-orchestrator/
├── Cargo.toml
├── src/
│   ├── lib.rs              # pub API, re-exports
│   ├── types.rs            # Decision, SeriesKey, MeasurementIssue, summaries
│   ├── config.rs           # AnalysisConfig, MonitorConfig, per-instrument configs
│   ├── router.rs           # auto-detection logic
│   ├── instrument.rs       # Instrument trait definition
│   ├── instruments/
│   │   ├── mod.rs          # re-exports
│   │   ├── irr.rs          # IrrInstrument adapter
│   │   ├── sequential.rs   # SequentialInstrument adapter
│   │   └── spc.rs          # SpcInstrument adapter
│   ├── analyze.rs          # analyze() batch entry point
│   ├── monitor.rs          # Monitor streaming struct
│   └── outcome_ext.rs      # Outcome→f64 conversion helper
└── tests/
    ├── fixtures/            # hand-crafted record sets
    ├── batch_tck.rs         # batch analysis TCK tests
    └── monitor_tck.rs       # streaming monitor TCK tests

tck/eval-orchestrator/
└── features/
    ├── batch.feature
    └── monitor.feature
```

---

### Task 1: Crate Skeleton + Core Types

**Files:**
- Create: `crates/eval-orchestrator/Cargo.toml`
- Create: `crates/eval-orchestrator/src/lib.rs`
- Create: `crates/eval-orchestrator/src/types.rs`
- Create: `crates/eval-orchestrator/src/outcome_ext.rs`
- Modify: `Cargo.toml` (workspace members)
- Test: `crates/eval-orchestrator/src/types.rs` (unit tests)
- Test: `crates/eval-orchestrator/src/outcome_ext.rs` (unit tests)

- [ ] **Step 1: Write failing tests for SeriesKey and outcome conversion**

In `crates/eval-orchestrator/src/outcome_ext.rs`:

```rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use eval_core::Outcome;
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn binary_true_converts_to_1() {
        assert!((outcome_to_f64(&Outcome::Binary(true)) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn binary_false_converts_to_0() {
        assert!(outcome_to_f64(&Outcome::Binary(false)).abs() < f64::EPSILON);
    }

    #[test]
    fn score_passes_through() {
        assert!((outcome_to_f64(&Outcome::Score(0.75)) - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn graded_normalizes_to_unit() {
        assert!((outcome_to_f64(&Outcome::Graded(128)) - 128.0 / 255.0).abs() < f64::EPSILON);
    }

    #[test]
    fn multi_criterion_takes_mean() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), 0.8);
        m.insert("b".into(), 0.4);
        let val = outcome_to_f64(&Outcome::MultiCriterion(m));
        assert!((val - 0.6).abs() < 1e-10);
    }

    #[test]
    fn empty_multi_criterion_is_zero() {
        let val = outcome_to_f64(&Outcome::MultiCriterion(BTreeMap::new()));
        assert!(val.abs() < f64::EPSILON);
    }
}
```

In `crates/eval-orchestrator/src/types.rs`:

```rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn series_key_from_record_no_scorer() {
        let record = eval_core::TrialRecord {
            trial_id: ulid::Ulid::new(),
            run_id: ulid::Ulid::new(),
            task_id: "math".into(),
            task_version: None,
            agent_id: "gpt-4o".into(),
            agent_version: None,
            judge_config: None,
            seed: None,
            timestamp: 1000,
            outcome: eval_core::Outcome::Binary(true),
            metadata: std::collections::BTreeMap::new(),
        };
        let key = SeriesKey::from_record(&record);
        assert_eq!(key.task_id, "math");
        assert_eq!(key.agent_id, "gpt-4o");
        assert_eq!(key.scorer, None);
    }

    #[test]
    fn series_key_from_record_with_scorer() {
        let mut metadata = std::collections::BTreeMap::new();
        metadata.insert(
            "scorer_name".into(),
            serde_json::Value::String("exact_match".into()),
        );
        let record = eval_core::TrialRecord {
            trial_id: ulid::Ulid::new(),
            run_id: ulid::Ulid::new(),
            task_id: "code".into(),
            task_version: None,
            agent_id: "claude".into(),
            agent_version: None,
            judge_config: None,
            seed: None,
            timestamp: 2000,
            outcome: eval_core::Outcome::Score(0.9),
            metadata,
        };
        let key = SeriesKey::from_record(&record);
        assert_eq!(key.scorer, Some("exact_match".into()));
    }

    #[test]
    fn series_key_equality_and_hash() {
        let a = SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        };
        let b = SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        };
        assert_eq!(a, b);

        let mut set = std::collections::HashSet::new();
        set.insert(a.clone());
        set.insert(b);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn series_key_roundtrip_serde() {
        let key = SeriesKey {
            task_id: "math".into(),
            agent_id: "gpt-4o".into(),
            scorer: Some("exact".into()),
        };
        let json = serde_json::to_string(&key).unwrap();
        let back: SeriesKey = serde_json::from_str(&json).unwrap();
        assert_eq!(key, back);
    }

    #[test]
    fn decision_roundtrip_serde() {
        let d = Decision::StopEarly {
            series: SeriesKey {
                task_id: "t".into(),
                agent_id: "a".into(),
                scorer: None,
            },
            evidence: 25.0,
            estimate: 0.85,
            ci: (0.7, 1.0),
        };
        let json = serde_json::to_string(&d).unwrap();
        let back: Decision = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, Decision::StopEarly { evidence, .. } if (evidence - 25.0).abs() < f64::EPSILON));
    }

    #[test]
    fn measurement_issue_roundtrip_serde() {
        let issue = MeasurementIssue::LowAgreement {
            kappa: 0.3,
            threshold: 0.67,
        };
        let json = serde_json::to_string(&issue).unwrap();
        let back: MeasurementIssue = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, MeasurementIssue::LowAgreement { kappa, .. } if (kappa - 0.3).abs() < f64::EPSILON));
    }
}
```

- [ ] **Step 2: Create Cargo.toml and add to workspace**

`crates/eval-orchestrator/Cargo.toml`:

```toml
[package]
name = "eval-orchestrator"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
publish = false
description = """
Analysis routing + decision engine — wires eval-ingest output to
the math crates (irr, seq-anytime-valid, spc-charts). Batch + streaming.
"""

[lib]
path = "src/lib.rs"

[dependencies]
eval-core = { path = "../eval-core" }
irr = { path = "../irr" }
seq-anytime-valid = { path = "../seq-anytime-valid" }
spc-charts = { path = "../spc-charts" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"

[dev-dependencies]
ulid = { version = "1", features = ["serde"] }
```

Add `"crates/eval-orchestrator"` to the workspace members in root `Cargo.toml`.

- [ ] **Step 3: Implement types.rs**

```rust
use eval_core::TrialRecord;
use serde::{Deserialize, Serialize};
use spc_charts::ChartSignal;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SeriesKey {
    pub task_id: String,
    pub agent_id: String,
    pub scorer: Option<String>,
}

impl SeriesKey {
    pub fn from_record(record: &TrialRecord) -> Self {
        let scorer = record
            .metadata
            .get("scorer_name")
            .and_then(|v| v.as_str())
            .map(String::from);
        Self {
            task_id: record.task_id.clone(),
            agent_id: record.agent_id.clone(),
            scorer,
        }
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeasurementIssue {
    LowAgreement { kappa: f64, threshold: f64 },
    InsufficientRaters { have: usize, need: usize },
    InsufficientSamples { have: usize, need: usize },
    HighVariance { cv: f64, threshold: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub decisions: Vec<Decision>,
    pub irr_results: Option<IrrSummary>,
    pub sequential_results: Vec<SequentialSummary>,
    pub spc_results: Vec<SpcSummary>,
    pub series_detected: Vec<SeriesKey>,
    pub instruments_run: Vec<String>,
}

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

#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("no records provided")]
    EmptyInput,
    #[error("no instruments applicable and none force-enabled")]
    NoInstrumentsApplicable,
}
```

- [ ] **Step 4: Implement outcome_ext.rs**

```rust
use eval_core::Outcome;

pub fn outcome_to_f64(outcome: &Outcome) -> f64 {
    match outcome {
        Outcome::Binary(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Outcome::Score(f) => *f,
        Outcome::Graded(g) => *g as f64 / 255.0,
        Outcome::MultiCriterion(m) => {
            if m.is_empty() {
                0.0
            } else {
                m.values().sum::<f64>() / m.len() as f64
            }
        }
    }
}

pub fn outcome_to_ordinal(outcome: &Outcome) -> u32 {
    match outcome {
        Outcome::Binary(b) => u32::from(*b),
        Outcome::Score(f) => (*f * 1000.0) as u32,
        Outcome::Graded(g) => *g as u32,
        Outcome::MultiCriterion(m) => {
            if m.is_empty() {
                0
            } else {
                (m.values().sum::<f64>() / m.len() as f64 * 1000.0) as u32
            }
        }
    }
}
```

- [ ] **Step 5: Implement lib.rs (minimal, just modules + re-exports)**

```rust
#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod outcome_ext;
pub mod types;

pub use types::{
    AnalysisReport, Decision, IrrSummary, MeasurementIssue, OrchestratorError, SequentialSummary,
    SeriesKey, SpcSummary,
};
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p eval-orchestrator --lib -- --nocapture`
Expected: All 12 tests pass (6 in types, 6 in outcome_ext).

- [ ] **Step 7: Run clippy and fmt**

Run: `cargo clippy -p eval-orchestrator -- -D warnings && cargo fmt -p eval-orchestrator --check`
Expected: Zero warnings, clean format.

- [ ] **Step 8: Commit**

```bash
git add crates/eval-orchestrator/Cargo.toml crates/eval-orchestrator/src/lib.rs \
  crates/eval-orchestrator/src/types.rs crates/eval-orchestrator/src/outcome_ext.rs \
  Cargo.toml
git commit -m "feat(eval-orchestrator): crate skeleton, core types, outcome conversion"
```

---

### Task 2: Configuration Types

**Files:**
- Create: `crates/eval-orchestrator/src/config.rs`
- Modify: `crates/eval-orchestrator/src/lib.rs` (add module)
- Test: `crates/eval-orchestrator/src/config.rs` (unit tests)

- [ ] **Step 1: Write failing tests for config defaults and serde**

In `crates/eval-orchestrator/src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn analysis_config_default_has_sane_values() {
        let c = AnalysisConfig::default();
        assert!(c.force_enable.is_empty());
        assert!(c.force_disable.is_empty());
        assert!((c.irr.threshold - 0.67).abs() < 0.01);
        assert!((c.sequential.alpha - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn irr_config_default() {
        let c = IrrConfig::default();
        assert!(matches!(c.metric, IrrMetric::Krippendorff));
        assert_eq!(c.min_raters, 2);
    }

    #[test]
    fn sequential_config_default() {
        let c = SequentialConfig::default();
        assert!((c.alpha - 0.05).abs() < f64::EPSILON);
        assert!((c.min_effect_size - 0.1).abs() < f64::EPSILON);
        assert!(matches!(c.method, SequentialMethod::Msprt));
    }

    #[test]
    fn spc_config_default() {
        let c = SpcConfig::default();
        assert!(matches!(c.chart_type, SpcChartType::Ewma));
        assert_eq!(c.phase1_windows, 20);
        assert!((c.lambda - 0.2).abs() < f64::EPSILON);
        assert!((c.l_sigma - 2.962).abs() < 0.001);
    }

    #[test]
    fn monitor_config_default() {
        let c = MonitorConfig::default();
        assert_eq!(c.irr_recompute_interval, 50);
        assert!(c.auto_detect);
    }

    #[test]
    fn analysis_config_roundtrip_serde() {
        let c = AnalysisConfig::default();
        let json = serde_json::to_string(&c).unwrap();
        let back: AnalysisConfig = serde_json::from_str(&json).unwrap();
        assert!((back.irr.threshold - c.irr.threshold).abs() < f64::EPSILON);
    }

    #[test]
    fn monitor_config_roundtrip_serde() {
        let c = MonitorConfig::default();
        let json = serde_json::to_string(&c).unwrap();
        let back: MonitorConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.irr_recompute_interval, c.irr_recompute_interval);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p eval-orchestrator --lib -- config::tests --nocapture`
Expected: FAIL — no config module.

- [ ] **Step 3: Implement config.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub force_enable: Vec<String>,
    pub force_disable: Vec<String>,
    pub irr: IrrConfig,
    pub sequential: SequentialConfig,
    pub spc: SpcConfig,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            force_enable: Vec::new(),
            force_disable: Vec::new(),
            irr: IrrConfig::default(),
            sequential: SequentialConfig::default(),
            spc: SpcConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrrConfig {
    pub threshold: f64,
    pub metric: IrrMetric,
    pub min_raters: usize,
}

impl Default for IrrConfig {
    fn default() -> Self {
        Self {
            threshold: 0.67,
            metric: IrrMetric::Krippendorff,
            min_raters: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrrMetric {
    Krippendorff,
    Fleiss,
    Gwet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialConfig {
    pub alpha: f64,
    pub min_effect_size: f64,
    pub method: SequentialMethod,
    pub mixing_variance: f64,
}

impl Default for SequentialConfig {
    fn default() -> Self {
        Self {
            alpha: 0.05,
            min_effect_size: 0.1,
            method: SequentialMethod::Msprt,
            mixing_variance: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SequentialMethod {
    Msprt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpcConfig {
    pub chart_type: SpcChartType,
    pub phase1_windows: usize,
    pub window_size: WindowSize,
    pub lambda: f64,
    pub l_sigma: f64,
}

impl Default for SpcConfig {
    fn default() -> Self {
        Self {
            chart_type: SpcChartType::Ewma,
            phase1_windows: 20,
            window_size: WindowSize::PerRun,
            lambda: 0.2,
            l_sigma: 2.962,
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub spc: SpcConfig,
    pub sequential: SequentialConfig,
    pub irr: IrrConfig,
    pub irr_recompute_interval: usize,
    pub auto_detect: bool,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            spc: SpcConfig::default(),
            sequential: SequentialConfig::default(),
            irr: IrrConfig::default(),
            irr_recompute_interval: 50,
            auto_detect: true,
        }
    }
}
```

- [ ] **Step 4: Add config module to lib.rs**

Add `pub mod config;` and re-export:

```rust
pub use config::{
    AnalysisConfig, IrrConfig, IrrMetric, MonitorConfig, SequentialConfig, SequentialMethod,
    SpcChartType, SpcConfig, WindowSize,
};
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p eval-orchestrator --lib -- --nocapture`
Expected: All config tests + previous types tests pass.

- [ ] **Step 6: Run clippy and fmt**

Run: `cargo clippy -p eval-orchestrator -- -D warnings && cargo fmt -p eval-orchestrator --check`
Expected: Clean.

- [ ] **Step 7: Commit**

```bash
git add crates/eval-orchestrator/src/config.rs crates/eval-orchestrator/src/lib.rs
git commit -m "feat(eval-orchestrator): configuration types with defaults"
```

---

### Task 3: Instrument Trait + Router

**Files:**
- Create: `crates/eval-orchestrator/src/instrument.rs`
- Create: `crates/eval-orchestrator/src/router.rs`
- Modify: `crates/eval-orchestrator/src/lib.rs` (add modules)
- Test: `crates/eval-orchestrator/src/router.rs` (unit tests)

- [ ] **Step 1: Write failing tests for router auto-detection**

In `crates/eval-orchestrator/src/router.rs`:

```rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::config::AnalysisConfig;
    use eval_core::{JudgeConfig, Outcome, TrialRecord};
    use std::collections::BTreeMap;
    use ulid::Ulid;

    fn make_record(task: &str, agent: &str, run: Ulid, judge: Option<JudgeConfig>, ts: i64) -> TrialRecord {
        TrialRecord {
            trial_id: Ulid::new(),
            run_id: run,
            task_id: task.into(),
            task_version: None,
            agent_id: agent.into(),
            agent_version: None,
            judge_config: judge,
            seed: None,
            timestamp: ts,
            outcome: Outcome::Binary(true),
            metadata: BTreeMap::new(),
        }
    }

    fn judge(model: &str) -> JudgeConfig {
        JudgeConfig::new(model.into(), "gpt".into(), "abc123".repeat(10), 0.0, None).unwrap()
    }

    #[test]
    fn detects_irr_from_multiple_judges() {
        let run = Ulid::new();
        let records: Vec<_> = (0..10)
            .map(|i| {
                let j = if i % 2 == 0 { judge("j1") } else { judge("j2") };
                make_record("t", "a", run, Some(j), 1000 + i)
            })
            .collect();
        let config = AnalysisConfig::default();
        let ids = detect_instruments(&records, &config);
        assert!(ids.contains(&InstrumentId::Irr));
    }

    #[test]
    fn detects_sequential_from_repeated_obs() {
        let run = Ulid::new();
        let records: Vec<_> = (0..50)
            .map(|i| make_record("t", "a", run, None, 1000 + i))
            .collect();
        let config = AnalysisConfig::default();
        let ids = detect_instruments(&records, &config);
        assert!(ids.contains(&InstrumentId::Sequential));
    }

    #[test]
    fn detects_spc_from_multiple_runs() {
        let runs: Vec<Ulid> = (0..5).map(|_| Ulid::new()).collect();
        let records: Vec<_> = runs
            .iter()
            .enumerate()
            .flat_map(|(ri, run)| {
                (0..10).map(move |i| make_record("t", "a", *run, None, (ri * 100 + i) as i64))
            })
            .collect();
        let config = AnalysisConfig::default();
        let ids = detect_instruments(&records, &config);
        assert!(ids.contains(&InstrumentId::Spc));
    }

    #[test]
    fn force_disable_overrides_detection() {
        let run = Ulid::new();
        let records: Vec<_> = (0..10)
            .map(|i| {
                let j = if i % 2 == 0 { judge("j1") } else { judge("j2") };
                make_record("t", "a", run, Some(j), 1000 + i)
            })
            .collect();
        let mut config = AnalysisConfig::default();
        config.force_disable.push("irr".into());
        let ids = detect_instruments(&records, &config);
        assert!(!ids.contains(&InstrumentId::Irr));
    }

    #[test]
    fn force_enable_adds_instrument() {
        let run = Ulid::new();
        let records: Vec<_> = (0..3)
            .map(|i| make_record("t", "a", run, None, 1000 + i))
            .collect();
        let mut config = AnalysisConfig::default();
        config.force_enable.push("spc".into());
        let ids = detect_instruments(&records, &config);
        assert!(ids.contains(&InstrumentId::Spc));
    }

    #[test]
    fn no_instruments_for_tiny_dataset() {
        let run = Ulid::new();
        let records = vec![make_record("t", "a", run, None, 1000)];
        let config = AnalysisConfig::default();
        let ids = detect_instruments(&records, &config);
        assert!(ids.is_empty());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p eval-orchestrator --lib -- router::tests --nocapture`
Expected: FAIL — no router module.

- [ ] **Step 3: Implement instrument.rs**

```rust
use crate::config::AnalysisConfig;
use crate::types::{Decision, SeriesKey};
use eval_core::TrialRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstrumentId {
    Irr,
    Sequential,
    Spc,
}

impl InstrumentId {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "irr" => Some(Self::Irr),
            "sequential" => Some(Self::Sequential),
            "spc" => Some(Self::Spc),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Irr => "irr",
            Self::Sequential => "sequential",
            Self::Spc => "spc",
        }
    }
}

pub trait Instrument {
    fn id(&self) -> InstrumentId;

    fn analyze(
        &self,
        series: &SeriesKey,
        records: &[TrialRecord],
        config: &AnalysisConfig,
    ) -> Vec<Decision>;
}
```

- [ ] **Step 4: Implement router.rs**

```rust
use std::collections::{BTreeMap, BTreeSet};

use crate::config::AnalysisConfig;
use crate::instrument::InstrumentId;
use crate::types::SeriesKey;
use eval_core::TrialRecord;

pub fn detect_instruments(records: &[TrialRecord], config: &AnalysisConfig) -> Vec<InstrumentId> {
    let mut enabled = Vec::new();

    if !config.force_disable.iter().any(|s| s == "irr") && has_multiple_judges(records) {
        enabled.push(InstrumentId::Irr);
    }

    if !config.force_disable.iter().any(|s| s == "sequential") && has_repeated_observations(records)
    {
        enabled.push(InstrumentId::Sequential);
    }

    if !config.force_disable.iter().any(|s| s == "spc") && has_temporal_runs(records) {
        enabled.push(InstrumentId::Spc);
    }

    for name in &config.force_enable {
        if let Some(id) = InstrumentId::from_name(name) {
            if !enabled.contains(&id) {
                enabled.push(id);
            }
        }
    }

    enabled
}

pub fn group_by_series(records: &[TrialRecord]) -> BTreeMap<SeriesKey, Vec<&TrialRecord>> {
    let mut groups: BTreeMap<SeriesKey, Vec<&TrialRecord>> = BTreeMap::new();
    for record in records {
        let key = SeriesKey::from_record(record);
        groups.entry(key).or_default().push(record);
    }
    groups
}

fn has_multiple_judges(records: &[TrialRecord]) -> bool {
    let mut by_series: BTreeMap<SeriesKey, BTreeSet<String>> = BTreeMap::new();
    for record in records {
        if let Some(ref jc) = record.judge_config {
            let key = SeriesKey::from_record(record);
            by_series
                .entry(key)
                .or_default()
                .insert(jc.model.clone());
        }
    }
    by_series.values().any(|judges| judges.len() >= 2)
}

fn has_repeated_observations(records: &[TrialRecord]) -> bool {
    let mut counts: BTreeMap<SeriesKey, usize> = BTreeMap::new();
    for record in records {
        let key = SeriesKey::from_record(record);
        *counts.entry(key).or_default() += 1;
    }
    counts.values().any(|&n| n >= 10)
}

fn has_temporal_runs(records: &[TrialRecord]) -> bool {
    let mut by_series: BTreeMap<SeriesKey, BTreeSet<ulid::Ulid>> = BTreeMap::new();
    for record in records {
        let key = SeriesKey::from_record(record);
        by_series.entry(key).or_default().insert(record.run_id);
    }
    by_series.values().any(|runs| runs.len() >= 2)
}
```

- [ ] **Step 5: Add modules to lib.rs**

Add:
```rust
pub mod instrument;
pub mod router;
```

And re-export:
```rust
pub use instrument::InstrumentId;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p eval-orchestrator --lib -- --nocapture`
Expected: All router tests + previous tests pass.

- [ ] **Step 7: Run clippy and fmt**

Run: `cargo clippy -p eval-orchestrator -- -D warnings && cargo fmt -p eval-orchestrator --check`
Expected: Clean.

- [ ] **Step 8: Commit**

```bash
git add crates/eval-orchestrator/src/instrument.rs crates/eval-orchestrator/src/router.rs \
  crates/eval-orchestrator/src/lib.rs
git commit -m "feat(eval-orchestrator): instrument trait and auto-detection router"
```

---

### Task 4: IRR Instrument Adapter

**Files:**
- Create: `crates/eval-orchestrator/src/instruments/mod.rs`
- Create: `crates/eval-orchestrator/src/instruments/irr.rs`
- Modify: `crates/eval-orchestrator/src/lib.rs` (add instruments module)
- Test: `crates/eval-orchestrator/src/instruments/irr.rs` (unit tests)

- [ ] **Step 1: Write failing tests for IRR instrument**

In `crates/eval-orchestrator/src/instruments/irr.rs`:

```rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::config::{AnalysisConfig, IrrMetric};
    use crate::types::{Decision, SeriesKey};
    use eval_core::{JudgeConfig, Outcome, TrialRecord};
    use std::collections::BTreeMap;
    use ulid::Ulid;

    fn make_irr_record(task: &str, agent: &str, judge_model: &str, item_id: &str, label: bool) -> TrialRecord {
        let jc = JudgeConfig::new(
            judge_model.into(),
            "gpt".into(),
            "a".repeat(64),
            0.0,
            None,
        )
        .unwrap();
        let mut metadata = BTreeMap::new();
        metadata.insert("sample_id".into(), serde_json::Value::String(item_id.into()));
        TrialRecord {
            trial_id: Ulid::new(),
            run_id: Ulid::new(),
            task_id: task.into(),
            task_version: None,
            agent_id: agent.into(),
            agent_version: None,
            judge_config: Some(jc),
            seed: None,
            timestamp: 1000,
            outcome: Outcome::Binary(label),
            metadata,
        }
    }

    #[test]
    fn perfect_agreement_no_warning() {
        let mut records = Vec::new();
        for item in &["q1", "q2", "q3", "q4", "q5"] {
            records.push(make_irr_record("t", "a", "judge_a", item, true));
            records.push(make_irr_record("t", "a", "judge_b", item, true));
        }
        let series = SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        };
        let config = AnalysisConfig::default();
        let (decisions, summary) = IrrInstrument.run(&series, &records, &config);
        assert!(
            decisions.is_empty() || decisions.iter().all(|d| !matches!(d, Decision::MeasurementWarning { .. })),
            "perfect agreement should not produce LowAgreement warning"
        );
        let s = summary.unwrap();
        assert_eq!(s.n_raters, 2);
        assert_eq!(s.n_items, 5);
    }

    #[test]
    fn low_agreement_emits_warning() {
        let mut records = Vec::new();
        for (i, item) in ["q1", "q2", "q3", "q4", "q5"].iter().enumerate() {
            records.push(make_irr_record("t", "a", "judge_a", item, true));
            records.push(make_irr_record("t", "a", "judge_b", item, i % 2 == 0));
        }
        let series = SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        };
        let config = AnalysisConfig::default();
        let (decisions, _summary) = IrrInstrument.run(&series, &records, &config);
        let has_low_agreement = decisions
            .iter()
            .any(|d| matches!(d, Decision::MeasurementWarning { issue: MeasurementIssue::LowAgreement { .. }, .. }));
        assert!(has_low_agreement, "disagreement should produce LowAgreement warning");
    }

    #[test]
    fn insufficient_raters_emits_warning() {
        let records = vec![
            make_irr_record("t", "a", "judge_a", "q1", true),
        ];
        let series = SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        };
        let mut config = AnalysisConfig::default();
        config.irr.min_raters = 2;
        let (decisions, summary) = IrrInstrument.run(&series, &records, &config);
        assert!(summary.is_none());
        let has_insufficient = decisions
            .iter()
            .any(|d| matches!(d, Decision::MeasurementWarning { issue: MeasurementIssue::InsufficientRaters { .. }, .. }));
        assert!(has_insufficient);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p eval-orchestrator --lib -- instruments::irr::tests --nocapture`
Expected: FAIL — no module.

- [ ] **Step 3: Implement instruments/mod.rs**

```rust
pub mod irr;
pub mod sequential;
pub mod spc;
```

- [ ] **Step 4: Implement instruments/irr.rs**

```rust
use std::collections::{BTreeMap, BTreeSet};

use crate::config::{AnalysisConfig, IrrMetric};
use crate::instrument::{Instrument, InstrumentId};
use crate::outcome_ext::outcome_to_ordinal;
use crate::types::{Decision, IrrSummary, MeasurementIssue, SeriesKey};
use eval_core::TrialRecord;
use irr::types::{AnnotationTriple, MetricLevel, RatingMatrix};

pub struct IrrInstrument;

impl IrrInstrument {
    pub fn run(
        &self,
        series: &SeriesKey,
        records: &[TrialRecord],
        config: &AnalysisConfig,
    ) -> (Vec<Decision>, Option<IrrSummary>) {
        let mut judges: BTreeSet<String> = BTreeSet::new();
        for r in records {
            if let Some(ref jc) = r.judge_config {
                judges.insert(jc.model.clone());
            }
        }

        if judges.len() < config.irr.min_raters {
            return (
                vec![Decision::MeasurementWarning {
                    series: series.clone(),
                    issue: MeasurementIssue::InsufficientRaters {
                        have: judges.len(),
                        need: config.irr.min_raters,
                    },
                }],
                None,
            );
        }

        let triples = build_triples(records);
        if triples.is_empty() {
            return (Vec::new(), None);
        }

        let matrix = match RatingMatrix::from_triples(&triples) {
            Ok(m) => m,
            Err(_) => return (Vec::new(), None),
        };

        let metric_name = match config.irr.metric {
            IrrMetric::Krippendorff => "krippendorff_alpha",
            IrrMetric::Fleiss => "fleiss_kappa",
            IrrMetric::Gwet => "gwet_ac",
        };

        let result = match config.irr.metric {
            IrrMetric::Krippendorff => {
                irr::krippendorff::alpha(&matrix, Some(MetricLevel::Nominal))
            }
            IrrMetric::Fleiss => irr::fleiss::kappa(&matrix).map_err(|e| {
                irr::krippendorff::KrippendorffError::DegenerateData
            }),
            IrrMetric::Gwet => irr::gwet::ac(&matrix, None).map_err(|e| {
                irr::krippendorff::KrippendorffError::DegenerateData
            }),
        };

        match result {
            Ok(irr_result) => {
                let summary = IrrSummary {
                    series: series.clone(),
                    alpha: irr_result.value,
                    n_raters: irr_result.n_raters,
                    n_items: irr_result.n_items,
                    metric: metric_name.to_string(),
                };

                let mut decisions = Vec::new();
                if irr_result.value < config.irr.threshold {
                    decisions.push(Decision::MeasurementWarning {
                        series: series.clone(),
                        issue: MeasurementIssue::LowAgreement {
                            kappa: irr_result.value,
                            threshold: config.irr.threshold,
                        },
                    });
                }

                (decisions, Some(summary))
            }
            Err(_) => (Vec::new(), None),
        }
    }
}

impl Instrument for IrrInstrument {
    fn id(&self) -> InstrumentId {
        InstrumentId::Irr
    }

    fn analyze(
        &self,
        series: &SeriesKey,
        records: &[TrialRecord],
        config: &AnalysisConfig,
    ) -> Vec<Decision> {
        self.run(series, records, config).0
    }
}

fn build_triples(records: &[TrialRecord]) -> Vec<AnnotationTriple> {
    let mut triples = Vec::new();
    for record in records {
        let annotator_id = match &record.judge_config {
            Some(jc) => jc.model.clone(),
            None => continue,
        };
        let item_id = record
            .metadata
            .get("sample_id")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| record.trial_id.to_string());
        let label = outcome_to_ordinal(&record.outcome);
        triples.push(AnnotationTriple {
            item_id,
            annotator_id,
            label,
        });
    }
    triples
}
```

**Important note on error mapping:** The `fleiss::kappa` and `gwet::ac` functions return their own error types (`FleissError`, `GwetError`). Rather than importing those and matching variants, we do a simple map_err to treat all failures as analysis failures (returns empty decisions). The implementer should check the actual error types in `irr::fleiss` and `irr::gwet` and produce a proper mapping. If `map_err` with a KrippendorffError feels wrong, instead match on each error type separately:

```rust
let irr_value = match config.irr.metric {
    IrrMetric::Krippendorff => {
        match irr::krippendorff::alpha(&matrix, Some(MetricLevel::Nominal)) {
            Ok(r) => r,
            Err(_) => return (Vec::new(), None),
        }
    }
    IrrMetric::Fleiss => {
        match irr::fleiss::kappa(&matrix) {
            Ok(r) => r,
            Err(_) => return (Vec::new(), None),
        }
    }
    IrrMetric::Gwet => {
        match irr::gwet::ac(&matrix, None) {
            Ok(r) => r,
            Err(_) => return (Vec::new(), None),
        }
    }
};
```

Use this pattern instead — it avoids cross-type error coercion.

- [ ] **Step 5: Add instruments module to lib.rs**

Add `pub mod instruments;` to lib.rs.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p eval-orchestrator --lib -- --nocapture`
Expected: IRR instrument tests pass.

- [ ] **Step 7: Run clippy and fmt**

Run: `cargo clippy -p eval-orchestrator -- -D warnings && cargo fmt -p eval-orchestrator --check`
Expected: Clean.

- [ ] **Step 8: Commit**

```bash
git add crates/eval-orchestrator/src/instruments/ crates/eval-orchestrator/src/lib.rs
git commit -m "feat(eval-orchestrator): IRR instrument adapter"
```

---

### Task 5: Sequential Instrument Adapter

**Files:**
- Create: `crates/eval-orchestrator/src/instruments/sequential.rs`
- Test: `crates/eval-orchestrator/src/instruments/sequential.rs` (unit tests)

- [ ] **Step 1: Write failing tests for Sequential instrument**

In `crates/eval-orchestrator/src/instruments/sequential.rs`:

```rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::config::AnalysisConfig;
    use crate::types::{Decision, SeriesKey};
    use eval_core::{Outcome, TrialRecord};
    use std::collections::BTreeMap;
    use ulid::Ulid;

    fn make_seq_record(value: f64) -> TrialRecord {
        TrialRecord {
            trial_id: Ulid::new(),
            run_id: Ulid::new(),
            task_id: "t".into(),
            task_version: None,
            agent_id: "a".into(),
            agent_version: None,
            judge_config: None,
            seed: None,
            timestamp: 1000,
            outcome: Outcome::Score(value),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn strong_signal_produces_stop_early() {
        let records: Vec<_> = (0..100).map(|_| make_seq_record(2.0)).collect();
        let series = SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        };
        let config = AnalysisConfig::default();
        let (decisions, summary) = SequentialInstrument.run(&series, &records, &config);
        let has_stop = decisions
            .iter()
            .any(|d| matches!(d, Decision::StopEarly { .. }));
        assert!(has_stop, "strong consistent signal should trigger StopEarly");
        let s = summary.unwrap();
        assert!(s.stopped);
        assert!(s.evidence > 1.0);
    }

    #[test]
    fn weak_signal_produces_continue() {
        let records: Vec<_> = (0..5).map(|i| make_seq_record(if i % 2 == 0 { 0.01 } else { -0.01 })).collect();
        let series = SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        };
        let mut config = AnalysisConfig::default();
        config.sequential.alpha = 0.05;
        let (decisions, summary) = SequentialInstrument.run(&series, &records, &config);
        let has_continue = decisions
            .iter()
            .any(|d| matches!(d, Decision::ContinueRunning { .. }));
        assert!(has_continue, "weak signal should produce ContinueRunning");
        let s = summary.unwrap();
        assert!(!s.stopped);
    }

    #[test]
    fn binary_outcomes_convert_correctly() {
        let mut records = Vec::new();
        for _ in 0..50 {
            let mut r = make_seq_record(0.0);
            r.outcome = Outcome::Binary(true);
            records.push(r);
        }
        let series = SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        };
        let config = AnalysisConfig::default();
        let (_decisions, summary) = SequentialInstrument.run(&series, &records, &config);
        let s = summary.unwrap();
        assert_eq!(s.n_observations, 50);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p eval-orchestrator --lib -- instruments::sequential::tests --nocapture`
Expected: FAIL — empty module.

- [ ] **Step 3: Implement instruments/sequential.rs**

```rust
use crate::config::AnalysisConfig;
use crate::instrument::{Instrument, InstrumentId};
use crate::outcome_ext::outcome_to_f64;
use crate::types::{Decision, SequentialSummary, SeriesKey};
use eval_core::TrialRecord;
use seq_anytime_valid::monitor::anytime::AnytimeMonitor;
use seq_anytime_valid::types::{DataFamily, MsprtConfig};
use seq_anytime_valid::evidence::e_value::threshold_decision;

pub struct SequentialInstrument;

impl SequentialInstrument {
    pub fn run(
        &self,
        series: &SeriesKey,
        records: &[TrialRecord],
        config: &AnalysisConfig,
    ) -> (Vec<Decision>, Option<SequentialSummary>) {
        if records.is_empty() {
            return (Vec::new(), None);
        }

        let msprt_config = MsprtConfig {
            theta_0: 0.0,
            mixing_variance: config.sequential.mixing_variance,
            family: DataFamily::Normal {
                known_variance: None,
            },
            max_samples: None,
        };

        let mut monitor = match AnytimeMonitor::new(msprt_config, config.sequential.alpha) {
            Ok(m) => m,
            Err(_) => return (Vec::new(), None),
        };

        let mut last_snapshot = None;
        for record in records {
            let value = outcome_to_f64(&record.outcome);
            match monitor.update(value) {
                Ok(snap) => last_snapshot = Some(snap),
                Err(_) => continue,
            }
        }

        let snap = match last_snapshot {
            Some(s) => s,
            None => return (Vec::new(), None),
        };

        let e_value = snap.e_value.unwrap_or(snap.log_likelihood_ratio.exp());
        let threshold = 1.0 / config.sequential.alpha;
        let stopped = e_value >= threshold;

        let ci = snap.confidence_interval.unwrap_or((f64::NEG_INFINITY, f64::INFINITY));
        let estimate = (ci.0 + ci.1) / 2.0;

        let summary = SequentialSummary {
            series: series.clone(),
            n_observations: snap.n_observations,
            current_estimate: estimate,
            ci,
            evidence: e_value,
            stopped,
        };

        let mut decisions = Vec::new();
        if stopped {
            decisions.push(Decision::StopEarly {
                series: series.clone(),
                evidence: e_value,
                estimate,
                ci,
            });
        } else {
            decisions.push(Decision::ContinueRunning {
                series: series.clone(),
                current_n: snap.n_observations,
                estimated_n_needed: 0,
                power_at_current_n: 0.0,
            });
        }

        (decisions, Some(summary))
    }
}

impl Instrument for SequentialInstrument {
    fn id(&self) -> InstrumentId {
        InstrumentId::Sequential
    }

    fn analyze(
        &self,
        series: &SeriesKey,
        records: &[TrialRecord],
        config: &AnalysisConfig,
    ) -> Vec<Decision> {
        self.run(series, records, config).0
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p eval-orchestrator --lib -- instruments::sequential --nocapture`
Expected: All sequential tests pass.

- [ ] **Step 5: Run clippy and fmt**

Run: `cargo clippy -p eval-orchestrator -- -D warnings && cargo fmt -p eval-orchestrator --check`
Expected: Clean.

- [ ] **Step 6: Commit**

```bash
git add crates/eval-orchestrator/src/instruments/sequential.rs
git commit -m "feat(eval-orchestrator): sequential testing instrument adapter"
```

---

### Task 6: SPC Instrument Adapter

**Files:**
- Create: `crates/eval-orchestrator/src/instruments/spc.rs`
- Test: `crates/eval-orchestrator/src/instruments/spc.rs` (unit tests)

- [ ] **Step 1: Write failing tests for SPC instrument**

In `crates/eval-orchestrator/src/instruments/spc.rs`:

```rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::config::{AnalysisConfig, SpcChartType};
    use crate::types::{Decision, SeriesKey};
    use eval_core::{Outcome, TrialRecord};
    use std::collections::BTreeMap;
    use ulid::Ulid;

    fn make_spc_records(n_runs: usize, per_run: usize, mean: f64) -> Vec<TrialRecord> {
        let mut records = Vec::new();
        for ri in 0..n_runs {
            let run_id = Ulid::new();
            for si in 0..per_run {
                records.push(TrialRecord {
                    trial_id: Ulid::new(),
                    run_id,
                    task_id: "t".into(),
                    task_version: None,
                    agent_id: "a".into(),
                    agent_version: None,
                    judge_config: None,
                    seed: None,
                    timestamp: (ri * 1000 + si) as i64,
                    outcome: Outcome::Score(mean),
                    metadata: BTreeMap::new(),
                });
            }
        }
        records
    }

    #[test]
    fn stable_process_no_regression() {
        let records = make_spc_records(25, 10, 0.8);
        let series = SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        };
        let config = AnalysisConfig::default();
        let (decisions, summary) = SpcInstrument.run(&series, &records, &config);
        let regressions: Vec<_> = decisions
            .iter()
            .filter(|d| matches!(d, Decision::Regression { .. }))
            .collect();
        assert!(
            regressions.is_empty(),
            "stable process should not produce regressions"
        );
        let s = summary.unwrap();
        assert!(s.in_control);
    }

    #[test]
    fn mean_shift_detects_regression() {
        let mut records = make_spc_records(20, 10, 0.8);
        // phase 2: shift mean down
        records.extend(make_spc_records(10, 10, 0.2));
        let series = SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        };
        let config = AnalysisConfig::default();
        let (decisions, summary) = SpcInstrument.run(&series, &records, &config);
        let regressions: Vec<_> = decisions
            .iter()
            .filter(|d| matches!(d, Decision::Regression { .. }))
            .collect();
        assert!(
            !regressions.is_empty(),
            "mean shift should produce at least one Regression"
        );
        let s = summary.unwrap();
        assert!(!s.in_control);
    }

    #[test]
    fn insufficient_windows_no_summary() {
        let records = make_spc_records(3, 10, 0.8);
        let series = SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        };
        let mut config = AnalysisConfig::default();
        config.spc.phase1_windows = 20;
        let (_decisions, summary) = SpcInstrument.run(&series, &records, &config);
        // Not enough data for phase 2; summary should reflect phase 1 only
        if let Some(s) = &summary {
            assert!(s.in_control, "phase 1 only should be in_control");
        }
    }

    #[test]
    fn per_run_windowing_groups_by_run_id() {
        let records = make_spc_records(25, 5, 0.8);
        let series = SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        };
        let config = AnalysisConfig::default();
        let (_decisions, summary) = SpcInstrument.run(&series, &records, &config);
        let s = summary.unwrap();
        assert_eq!(s.n_windows, 25);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p eval-orchestrator --lib -- instruments::spc::tests --nocapture`
Expected: FAIL — empty module.

- [ ] **Step 3: Implement instruments/spc.rs**

```rust
use std::collections::BTreeMap;

use crate::config::{AnalysisConfig, SpcChartType, WindowSize};
use crate::instrument::{Instrument, InstrumentId};
use crate::outcome_ext::outcome_to_f64;
use crate::types::{Decision, SeriesKey, SpcSummary};
use eval_core::TrialRecord;
use spc_charts::types::{ChartSignal, ControlLimits};
use spc_charts::{CombinedChart, CombinedConfig, CusumChart, CusumConfig, EwmaChart, EwmaConfig, ShewhartChart, ShewhartConfig};
use ulid::Ulid;

pub struct SpcInstrument;

impl SpcInstrument {
    pub fn run(
        &self,
        series: &SeriesKey,
        records: &[TrialRecord],
        config: &AnalysisConfig,
    ) -> (Vec<Decision>, Option<SpcSummary>) {
        let windows = compute_windows(records, &config.spc.window_size);
        if windows.is_empty() {
            return (Vec::new(), None);
        }

        let window_stats: Vec<f64> = windows.iter().map(|w| w.mean).collect();
        let phase1_n = config.spc.phase1_windows.min(window_stats.len());

        let phase1_data = &window_stats[..phase1_n];
        let n_p1 = phase1_data.len();
        if n_p1 < 2 {
            return (Vec::new(), None);
        }

        let mu_0 = phase1_data.iter().sum::<f64>() / n_p1 as f64;
        let variance = phase1_data.iter().map(|x| (x - mu_0).powi(2)).sum::<f64>() / (n_p1 - 1) as f64;
        let sigma = variance.sqrt();

        if sigma <= 0.0 || !sigma.is_finite() {
            let summary = SpcSummary {
                series: series.clone(),
                n_windows: window_stats.len(),
                chart_type: chart_type_name(&config.spc.chart_type),
                in_control: true,
                signals: Vec::new(),
                control_limits: (mu_0, mu_0),
            };
            return (Vec::new(), Some(summary));
        }

        let limits = match ControlLimits::new(mu_0, sigma) {
            Ok(l) => l,
            Err(_) => return (Vec::new(), None),
        };

        let mut signals = Vec::new();
        let mut decisions = Vec::new();

        match config.spc.chart_type {
            SpcChartType::Ewma => {
                let chart_config = EwmaConfig {
                    limits: limits.clone(),
                    lambda: config.spc.lambda,
                    l_sigma: config.spc.l_sigma,
                };
                let mut chart = match EwmaChart::new(chart_config) {
                    Ok(c) => c,
                    Err(_) => return (Vec::new(), None),
                };
                // Feed phase 1 to warm up
                for &x in phase1_data {
                    chart.observe(x);
                }
                chart.reset();
                // Re-create chart for phase 2 monitoring
                let chart_config2 = EwmaConfig {
                    limits: limits.clone(),
                    lambda: config.spc.lambda,
                    l_sigma: config.spc.l_sigma,
                };
                let mut chart2 = match EwmaChart::new(chart_config2) {
                    Ok(c) => c,
                    Err(_) => return (Vec::new(), None),
                };
                for (i, &x) in window_stats[phase1_n..].iter().enumerate() {
                    let signal = chart2.observe(x);
                    if signal.is_out_of_control() {
                        signals.push(signal.clone());
                        decisions.push(Decision::Regression {
                            series: series.clone(),
                            signal,
                            observation_value: x,
                            control_limits: (mu_0 - config.spc.l_sigma * sigma, mu_0 + config.spc.l_sigma * sigma),
                        });
                    }
                }
            }
            SpcChartType::Cusum => {
                let chart_config = CusumConfig::default_for(limits.clone());
                let mut chart = match CusumChart::new(chart_config) {
                    Ok(c) => c,
                    Err(_) => return (Vec::new(), None),
                };
                for &x in &window_stats[phase1_n..] {
                    let signal = chart.observe(x);
                    if signal.is_out_of_control() {
                        signals.push(signal.clone());
                        decisions.push(Decision::Regression {
                            series: series.clone(),
                            signal,
                            observation_value: x,
                            control_limits: (mu_0 - 3.0 * sigma, mu_0 + 3.0 * sigma),
                        });
                    }
                }
            }
            SpcChartType::Shewhart => {
                let chart_config = ShewhartConfig::default_for(limits.clone());
                let mut chart = match ShewhartChart::new(chart_config) {
                    Ok(c) => c,
                    Err(_) => return (Vec::new(), None),
                };
                for &x in &window_stats[phase1_n..] {
                    let signal = chart.observe(x);
                    if signal.is_out_of_control() {
                        signals.push(signal.clone());
                        decisions.push(Decision::Regression {
                            series: series.clone(),
                            signal,
                            observation_value: x,
                            control_limits: (mu_0 - 3.0 * sigma, mu_0 + 3.0 * sigma),
                        });
                    }
                }
            }
            SpcChartType::Combined => {
                let chart_config = CombinedConfig::default_for(limits.clone());
                let mut chart = match CombinedChart::new(chart_config) {
                    Ok(c) => c,
                    Err(_) => return (Vec::new(), None),
                };
                for &x in &window_stats[phase1_n..] {
                    let signal = chart.observe(x);
                    if signal.is_out_of_control() {
                        signals.push(signal.clone());
                        decisions.push(Decision::Regression {
                            series: series.clone(),
                            signal,
                            observation_value: x,
                            control_limits: (mu_0 - 3.0 * sigma, mu_0 + 3.0 * sigma),
                        });
                    }
                }
            }
        }

        let in_control = signals.is_empty();
        let summary = SpcSummary {
            series: series.clone(),
            n_windows: window_stats.len(),
            chart_type: chart_type_name(&config.spc.chart_type),
            in_control,
            signals,
            control_limits: (mu_0 - config.spc.l_sigma * sigma, mu_0 + config.spc.l_sigma * sigma),
        };

        (decisions, Some(summary))
    }
}

impl Instrument for SpcInstrument {
    fn id(&self) -> InstrumentId {
        InstrumentId::Spc
    }

    fn analyze(
        &self,
        series: &SeriesKey,
        records: &[TrialRecord],
        config: &AnalysisConfig,
    ) -> Vec<Decision> {
        self.run(series, records, config).0
    }
}

struct WindowStat {
    mean: f64,
}

fn compute_windows(records: &[TrialRecord], window_size: &WindowSize) -> Vec<WindowStat> {
    match window_size {
        WindowSize::PerRun => {
            let mut by_run: BTreeMap<Ulid, Vec<f64>> = BTreeMap::new();
            for record in records {
                by_run
                    .entry(record.run_id)
                    .or_default()
                    .push(outcome_to_f64(&record.outcome));
            }
            // Sort runs by earliest timestamp
            let mut run_order: Vec<(Ulid, i64)> = Vec::new();
            for record in records {
                if !run_order.iter().any(|(id, _)| *id == record.run_id) {
                    run_order.push((record.run_id, record.timestamp));
                }
            }
            run_order.sort_by_key(|(_, ts)| *ts);

            run_order
                .iter()
                .filter_map(|(run_id, _)| {
                    let values = by_run.get(run_id)?;
                    if values.is_empty() {
                        return None;
                    }
                    let mean = values.iter().sum::<f64>() / values.len() as f64;
                    Some(WindowStat { mean })
                })
                .collect()
        }
        WindowSize::Fixed(size) => {
            let values: Vec<f64> = records.iter().map(|r| outcome_to_f64(&r.outcome)).collect();
            values
                .chunks(*size)
                .filter(|chunk| !chunk.is_empty())
                .map(|chunk| {
                    let mean = chunk.iter().sum::<f64>() / chunk.len() as f64;
                    WindowStat { mean }
                })
                .collect()
        }
    }
}

fn chart_type_name(ct: &SpcChartType) -> String {
    match ct {
        SpcChartType::Ewma => "ewma".to_string(),
        SpcChartType::Cusum => "cusum".to_string(),
        SpcChartType::Shewhart => "shewhart".to_string(),
        SpcChartType::Combined => "combined".to_string(),
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p eval-orchestrator --lib -- instruments::spc --nocapture`
Expected: All SPC tests pass.

- [ ] **Step 5: Run clippy and fmt**

Run: `cargo clippy -p eval-orchestrator -- -D warnings && cargo fmt -p eval-orchestrator --check`
Expected: Clean.

- [ ] **Step 6: Commit**

```bash
git add crates/eval-orchestrator/src/instruments/spc.rs
git commit -m "feat(eval-orchestrator): SPC instrument adapter"
```

---

### Task 7: Batch analyze() Entry Point

**Files:**
- Create: `crates/eval-orchestrator/src/analyze.rs`
- Modify: `crates/eval-orchestrator/src/lib.rs` (add module + re-export)
- Test: `crates/eval-orchestrator/src/analyze.rs` (unit tests)

- [ ] **Step 1: Write failing tests for analyze()**

In `crates/eval-orchestrator/src/analyze.rs`:

```rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::config::AnalysisConfig;
    use eval_core::{JudgeConfig, Outcome, TrialRecord};
    use std::collections::BTreeMap;
    use ulid::Ulid;

    fn make_record(task: &str, agent: &str, run: Ulid, judge: Option<JudgeConfig>, ts: i64, outcome: Outcome) -> TrialRecord {
        TrialRecord {
            trial_id: Ulid::new(),
            run_id: run,
            task_id: task.into(),
            task_version: None,
            agent_id: agent.into(),
            agent_version: None,
            judge_config: judge,
            seed: None,
            timestamp: ts,
            outcome,
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn empty_input_returns_error() {
        let config = AnalysisConfig::default();
        let result = analyze(&[], &config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OrchestratorError::EmptyInput
        ));
    }

    #[test]
    fn no_instruments_applicable_returns_error() {
        let run = Ulid::new();
        let records = vec![make_record("t", "a", run, None, 1000, Outcome::Binary(true))];
        let config = AnalysisConfig::default();
        let result = analyze(&records, &config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OrchestratorError::NoInstrumentsApplicable
        ));
    }

    #[test]
    fn multi_judge_triggers_irr() {
        let run = Ulid::new();
        let mut records = Vec::new();
        let j1 = JudgeConfig::new("j1".into(), "gpt".into(), "a".repeat(64), 0.0, None).unwrap();
        let j2 = JudgeConfig::new("j2".into(), "gpt".into(), "b".repeat(64), 0.0, None).unwrap();
        for i in 0..10 {
            let mut r = make_record("t", "a", run, Some(if i % 2 == 0 { j1.clone() } else { j2.clone() }), 1000 + i, Outcome::Binary(true));
            r.metadata.insert("sample_id".into(), serde_json::Value::String(format!("q{}", i / 2)));
            records.push(r);
        }
        let config = AnalysisConfig::default();
        let report = analyze(&records, &config).unwrap();
        assert!(report.instruments_run.contains(&"irr".to_string()));
        assert!(report.irr_results.is_some());
    }

    #[test]
    fn many_observations_triggers_sequential() {
        let run = Ulid::new();
        let records: Vec<_> = (0..50)
            .map(|i| make_record("t", "a", run, None, 1000 + i, Outcome::Score(2.0)))
            .collect();
        let config = AnalysisConfig::default();
        let report = analyze(&records, &config).unwrap();
        assert!(report.instruments_run.contains(&"sequential".to_string()));
        assert!(!report.sequential_results.is_empty());
    }

    #[test]
    fn multi_run_triggers_spc() {
        let runs: Vec<Ulid> = (0..25).map(|_| Ulid::new()).collect();
        let records: Vec<_> = runs
            .iter()
            .enumerate()
            .flat_map(|(ri, run)| {
                (0..10).map(move |i| {
                    make_record("t", "a", *run, None, (ri * 100 + i) as i64, Outcome::Score(0.8))
                })
            })
            .collect();
        let config = AnalysisConfig::default();
        let report = analyze(&records, &config).unwrap();
        assert!(report.instruments_run.contains(&"spc".to_string()));
        assert!(!report.spc_results.is_empty());
    }

    #[test]
    fn force_disable_prevents_instrument() {
        let run = Ulid::new();
        let j1 = JudgeConfig::new("j1".into(), "gpt".into(), "a".repeat(64), 0.0, None).unwrap();
        let j2 = JudgeConfig::new("j2".into(), "gpt".into(), "b".repeat(64), 0.0, None).unwrap();
        let mut records = Vec::new();
        for i in 0..10 {
            let mut r = make_record("t", "a", run, Some(if i % 2 == 0 { j1.clone() } else { j2.clone() }), 1000 + i, Outcome::Binary(true));
            r.metadata.insert("sample_id".into(), serde_json::Value::String(format!("q{}", i / 2)));
            records.push(r);
        }
        let mut config = AnalysisConfig::default();
        config.force_disable.push("irr".into());
        let result = analyze(&records, &config);
        // Should fail because IRR was the only applicable instrument
        assert!(result.is_err() || !result.unwrap().instruments_run.contains(&"irr".to_string()));
    }

    #[test]
    fn report_contains_detected_series() {
        let run = Ulid::new();
        let records: Vec<_> = (0..50)
            .map(|i| make_record("math", "gpt-4o", run, None, 1000 + i, Outcome::Score(0.9)))
            .collect();
        let config = AnalysisConfig::default();
        let report = analyze(&records, &config).unwrap();
        assert_eq!(report.series_detected.len(), 1);
        assert_eq!(report.series_detected[0].task_id, "math");
        assert_eq!(report.series_detected[0].agent_id, "gpt-4o");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p eval-orchestrator --lib -- analyze::tests --nocapture`
Expected: FAIL — no module.

- [ ] **Step 3: Implement analyze.rs**

```rust
use crate::config::AnalysisConfig;
use crate::instrument::InstrumentId;
use crate::instruments::irr::IrrInstrument;
use crate::instruments::sequential::SequentialInstrument;
use crate::instruments::spc::SpcInstrument;
use crate::router::{detect_instruments, group_by_series};
use crate::types::{AnalysisReport, IrrSummary, OrchestratorError, SequentialSummary, SpcSummary};
use eval_core::TrialRecord;

pub fn analyze(
    records: &[TrialRecord],
    config: &AnalysisConfig,
) -> Result<AnalysisReport, OrchestratorError> {
    if records.is_empty() {
        return Err(OrchestratorError::EmptyInput);
    }

    let instrument_ids = detect_instruments(records, config);
    if instrument_ids.is_empty() {
        return Err(OrchestratorError::NoInstrumentsApplicable);
    }

    let groups = group_by_series(records);
    let series_detected: Vec<_> = groups.keys().cloned().collect();

    let mut all_decisions = Vec::new();
    let mut irr_results = None;
    let mut sequential_results = Vec::new();
    let mut spc_results = Vec::new();
    let mut instruments_run = Vec::new();

    for &id in &instrument_ids {
        instruments_run.push(id.name().to_string());

        for (series, series_records) in &groups {
            let owned_records: Vec<TrialRecord> = series_records.iter().copied().cloned().collect();

            match id {
                InstrumentId::Irr => {
                    let (decisions, summary) =
                        IrrInstrument.run(series, &owned_records, config);
                    all_decisions.extend(decisions);
                    if let Some(s) = summary {
                        irr_results = Some(s);
                    }
                }
                InstrumentId::Sequential => {
                    let (decisions, summary) =
                        SequentialInstrument.run(series, &owned_records, config);
                    all_decisions.extend(decisions);
                    if let Some(s) = summary {
                        sequential_results.push(s);
                    }
                }
                InstrumentId::Spc => {
                    let (decisions, summary) =
                        SpcInstrument.run(series, &owned_records, config);
                    all_decisions.extend(decisions);
                    if let Some(s) = summary {
                        spc_results.push(s);
                    }
                }
            }
        }
    }

    Ok(AnalysisReport {
        decisions: all_decisions,
        irr_results,
        sequential_results,
        spc_results,
        series_detected,
        instruments_run,
    })
}
```

- [ ] **Step 4: Add analyze module to lib.rs and re-export**

Add `pub mod analyze;` and:
```rust
pub use analyze::analyze;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p eval-orchestrator --lib -- --nocapture`
Expected: All analyze tests + previous tests pass.

- [ ] **Step 6: Run clippy and fmt**

Run: `cargo clippy -p eval-orchestrator -- -D warnings && cargo fmt -p eval-orchestrator --check`
Expected: Clean.

- [ ] **Step 7: Commit**

```bash
git add crates/eval-orchestrator/src/analyze.rs crates/eval-orchestrator/src/lib.rs
git commit -m "feat(eval-orchestrator): batch analyze() entry point"
```

---

### Task 8: Streaming Monitor

**Files:**
- Create: `crates/eval-orchestrator/src/monitor.rs`
- Modify: `crates/eval-orchestrator/src/lib.rs` (add module + re-export)
- Test: `crates/eval-orchestrator/src/monitor.rs` (unit tests)

Critical design note: `EwmaChart`, `CusumChart`, `ShewhartChart`, `CombinedChart`, and `AnytimeMonitor` do NOT implement `Serialize`/`Deserialize`. The Monitor struct must be serializable, so it stores configs + accumulated raw observations per series, and reconstructs chart objects on demand from that data.

- [ ] **Step 1: Write failing tests for Monitor**

In `crates/eval-orchestrator/src/monitor.rs`:

```rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::config::MonitorConfig;
    use eval_core::{Outcome, TrialRecord};
    use std::collections::BTreeMap;
    use ulid::Ulid;

    fn make_record(task: &str, agent: &str, run: Ulid, value: f64, ts: i64) -> TrialRecord {
        TrialRecord {
            trial_id: Ulid::new(),
            run_id: run,
            task_id: task.into(),
            task_version: None,
            agent_id: agent.into(),
            agent_version: None,
            judge_config: None,
            seed: None,
            timestamp: ts,
            outcome: Outcome::Score(value),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn new_monitor_is_empty() {
        let m = Monitor::new(MonitorConfig::default());
        assert!(m.active_series().is_empty());
        let summary = m.state_summary();
        assert_eq!(summary.observations_seen, 0);
        assert_eq!(summary.active_series, 0);
    }

    #[test]
    fn push_auto_detects_series() {
        let mut m = Monitor::new(MonitorConfig::default());
        let run = Ulid::new();
        let record = make_record("t", "a", run, 0.8, 1000);
        m.push(&record);
        assert_eq!(m.active_series().len(), 1);
        let summary = m.state_summary();
        assert_eq!(summary.observations_seen, 1);
    }

    #[test]
    fn push_batch_equivalent_to_sequential() {
        let config = MonitorConfig::default();
        let run = Ulid::new();
        let records: Vec<_> = (0..20)
            .map(|i| make_record("t", "a", run, 0.8, 1000 + i))
            .collect();

        let mut m1 = Monitor::new(config.clone());
        for r in &records {
            m1.push(r);
        }

        let mut m2 = Monitor::new(config);
        m2.push_batch(&records);

        assert_eq!(
            m1.state_summary().observations_seen,
            m2.state_summary().observations_seen
        );
    }

    #[test]
    fn monitor_serde_roundtrip() {
        let mut m = Monitor::new(MonitorConfig::default());
        let run = Ulid::new();
        for i in 0..30 {
            let r = make_record("t", "a", run, 0.8, 1000 + i);
            m.push(&r);
        }

        let json = serde_json::to_string(&m).unwrap();
        let m2: Monitor = serde_json::from_str(&json).unwrap();

        assert_eq!(m.state_summary().observations_seen, m2.state_summary().observations_seen);
        assert_eq!(m.active_series().len(), m2.active_series().len());

        // Push one more record to both — should produce same state
        let r = make_record("t", "a", run, 0.8, 2000);
        let d1 = m.push(&r);
        let d2 = m2.push(&r);
        assert_eq!(d1.len(), d2.len());
    }

    #[test]
    fn spc_regression_in_streaming() {
        let mut m = Monitor::new(MonitorConfig::default());
        // Phase 1: 20 runs, stable at 0.8
        for ri in 0..20 {
            let run = Ulid::new();
            for si in 0..10 {
                let r = make_record("t", "a", run, 0.8, (ri * 100 + si) as i64);
                m.push(&r);
            }
        }
        // Phase 2: 10 runs, shifted to 0.2
        let mut found_regression = false;
        for ri in 20..30 {
            let run = Ulid::new();
            for si in 0..10 {
                let r = make_record("t", "a", run, 0.2, (ri * 100 + si) as i64);
                let decisions = m.push(&r);
                if decisions.iter().any(|d| matches!(d, Decision::Regression { .. })) {
                    found_regression = true;
                }
            }
        }
        assert!(found_regression, "SPC should detect regression in streaming mode");
    }

    #[test]
    fn sequential_stop_in_streaming() {
        let mut m = Monitor::new(MonitorConfig::default());
        let run = Ulid::new();
        let mut found_stop = false;
        for i in 0..200 {
            let r = make_record("t", "a", run, 2.0, 1000 + i);
            let decisions = m.push(&r);
            if decisions.iter().any(|d| matches!(d, Decision::StopEarly { .. })) {
                found_stop = true;
                break;
            }
        }
        assert!(found_stop, "strong signal should trigger StopEarly in streaming");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p eval-orchestrator --lib -- monitor::tests --nocapture`
Expected: FAIL — no module.

- [ ] **Step 3: Implement monitor.rs**

```rust
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::config::MonitorConfig;
use crate::outcome_ext::outcome_to_f64;
use crate::types::{Decision, MonitorSummary, SeriesKey};
use eval_core::TrialRecord;
use seq_anytime_valid::monitor::anytime::AnytimeMonitor;
use seq_anytime_valid::types::{DataFamily, MsprtConfig};
use spc_charts::types::{ChartSignal, ControlLimits};
use spc_charts::{EwmaChart, EwmaConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monitor {
    config: MonitorConfig,
    spc_state: BTreeMap<SeriesKey, SpcStreamState>,
    seq_state: BTreeMap<SeriesKey, SeqStreamState>,
    observations_seen: u64,
    total_decisions_emitted: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpcStreamState {
    window_values: Vec<f64>,
    current_window_run: Option<String>,
    completed_window_means: Vec<f64>,
    phase1_complete: bool,
    mu_0: f64,
    sigma: f64,
    chart_observations: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SeqStreamState {
    observations: Vec<f64>,
    stopped: bool,
}

impl Monitor {
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            config,
            spc_state: BTreeMap::new(),
            seq_state: BTreeMap::new(),
            observations_seen: 0,
            total_decisions_emitted: 0,
        }
    }

    pub fn push(&mut self, record: &TrialRecord) -> Vec<Decision> {
        self.observations_seen += 1;
        let series = SeriesKey::from_record(record);
        let value = outcome_to_f64(&record.outcome);
        let run_id_str = record.run_id.to_string();

        let mut decisions = Vec::new();

        // Sequential instrument
        decisions.extend(self.push_sequential(&series, value));

        // SPC instrument
        decisions.extend(self.push_spc(&series, value, &run_id_str));

        self.total_decisions_emitted += decisions.len() as u64;
        decisions
    }

    pub fn push_batch(&mut self, records: &[TrialRecord]) -> Vec<Decision> {
        let mut all_decisions = Vec::new();
        for record in records {
            all_decisions.extend(self.push(record));
        }
        all_decisions
    }

    pub fn active_series(&self) -> Vec<&SeriesKey> {
        let mut keys: BTreeMap<&SeriesKey, ()> = BTreeMap::new();
        for k in self.spc_state.keys() {
            keys.insert(k, ());
        }
        for k in self.seq_state.keys() {
            keys.insert(k, ());
        }
        keys.into_keys().collect()
    }

    pub fn state_summary(&self) -> MonitorSummary {
        let spc_phase1 = self
            .spc_state
            .values()
            .filter(|s| !s.phase1_complete)
            .count();
        let spc_phase2 = self
            .spc_state
            .values()
            .filter(|s| s.phase1_complete)
            .count();

        MonitorSummary {
            observations_seen: self.observations_seen,
            active_series: self.active_series().len(),
            series_in_phase1: spc_phase1,
            series_in_phase2: spc_phase2,
            total_decisions_emitted: self.total_decisions_emitted,
        }
    }

    fn push_sequential(&mut self, series: &SeriesKey, value: f64) -> Vec<Decision> {
        let state = self.seq_state.entry(series.clone()).or_insert_with(|| {
            SeqStreamState {
                observations: Vec::new(),
                stopped: false,
            }
        });

        if state.stopped {
            return Vec::new();
        }

        state.observations.push(value);

        let msprt_config = MsprtConfig {
            theta_0: 0.0,
            mixing_variance: self.config.sequential.mixing_variance,
            family: DataFamily::Normal {
                known_variance: None,
            },
            max_samples: None,
        };

        let mut monitor = match AnytimeMonitor::new(msprt_config, self.config.sequential.alpha) {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };

        let mut last_snap = None;
        for &obs in &state.observations {
            match monitor.update(obs) {
                Ok(snap) => last_snap = Some(snap),
                Err(_) => {}
            }
        }

        let snap = match last_snap {
            Some(s) => s,
            None => return Vec::new(),
        };

        let e_value = snap.e_value.unwrap_or(snap.log_likelihood_ratio.exp());
        let threshold = 1.0 / self.config.sequential.alpha;

        if e_value >= threshold {
            state.stopped = true;
            let ci = snap
                .confidence_interval
                .unwrap_or((f64::NEG_INFINITY, f64::INFINITY));
            let estimate = (ci.0 + ci.1) / 2.0;
            vec![Decision::StopEarly {
                series: series.clone(),
                evidence: e_value,
                estimate,
                ci,
            }]
        } else {
            Vec::new()
        }
    }

    fn push_spc(&mut self, series: &SeriesKey, value: f64, run_id: &str) -> Vec<Decision> {
        let phase1_windows = self.config.spc.phase1_windows;
        let lambda = self.config.spc.lambda;
        let l_sigma = self.config.spc.l_sigma;

        let state = self
            .spc_state
            .entry(series.clone())
            .or_insert_with(|| SpcStreamState {
                window_values: Vec::new(),
                current_window_run: None,
                completed_window_means: Vec::new(),
                phase1_complete: false,
                mu_0: 0.0,
                sigma: 0.0,
                chart_observations: Vec::new(),
            });

        let run_changed = state
            .current_window_run
            .as_ref()
            .map_or(true, |r| r != run_id);

        if run_changed && !state.window_values.is_empty() {
            let mean =
                state.window_values.iter().sum::<f64>() / state.window_values.len() as f64;
            state.completed_window_means.push(mean);
            state.window_values.clear();

            if !state.phase1_complete
                && state.completed_window_means.len() >= phase1_windows
            {
                let p1 = &state.completed_window_means[..phase1_windows];
                let mu = p1.iter().sum::<f64>() / p1.len() as f64;
                let var =
                    p1.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / (p1.len() - 1) as f64;
                state.mu_0 = mu;
                state.sigma = var.sqrt();
                state.phase1_complete = true;
            }
        }

        state.current_window_run = Some(run_id.to_string());
        state.window_values.push(value);

        if !state.phase1_complete {
            return Vec::new();
        }

        // Check: did we just complete a new window while in phase 2?
        if run_changed && state.phase1_complete && state.sigma > 0.0 && state.sigma.is_finite() {
            let latest_mean = match state.completed_window_means.last() {
                Some(&m) => m,
                None => return Vec::new(),
            };

            state.chart_observations.push(latest_mean);

            let limits = match ControlLimits::new(state.mu_0, state.sigma) {
                Ok(l) => l,
                Err(_) => return Vec::new(),
            };
            let config = EwmaConfig {
                limits,
                lambda,
                l_sigma,
            };
            let mut chart = match EwmaChart::new(config) {
                Ok(c) => c,
                Err(_) => return Vec::new(),
            };

            let mut last_signal = ChartSignal::InControl;
            for &obs in &state.chart_observations {
                last_signal = chart.observe(obs);
            }

            if last_signal.is_out_of_control() {
                return vec![Decision::Regression {
                    series: series.clone(),
                    signal: last_signal,
                    observation_value: latest_mean,
                    control_limits: (
                        state.mu_0 - l_sigma * state.sigma,
                        state.mu_0 + l_sigma * state.sigma,
                    ),
                }];
            }
        }

        Vec::new()
    }
}
```

- [ ] **Step 4: Add MonitorSummary to types.rs**

In `crates/eval-orchestrator/src/types.rs`, add:

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

- [ ] **Step 5: Add monitor module to lib.rs and re-export**

Add `pub mod monitor;` and:
```rust
pub use monitor::Monitor;
pub use types::MonitorSummary;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p eval-orchestrator --lib -- --nocapture`
Expected: All monitor tests + previous tests pass.

- [ ] **Step 7: Run clippy and fmt**

Run: `cargo clippy -p eval-orchestrator -- -D warnings && cargo fmt -p eval-orchestrator --check`
Expected: Clean.

- [ ] **Step 8: Commit**

```bash
git add crates/eval-orchestrator/src/monitor.rs crates/eval-orchestrator/src/types.rs \
  crates/eval-orchestrator/src/lib.rs
git commit -m "feat(eval-orchestrator): streaming Monitor with serde roundtrip"
```

---

### Task 9: TCK Feature Files + Integration Tests (Batch)

**Files:**
- Create: `tck/eval-orchestrator/features/batch.feature`
- Create: `crates/eval-orchestrator/tests/batch_tck.rs`
- Test: Integration tests

- [ ] **Step 1: Write batch.feature**

Create `tck/eval-orchestrator/features/batch.feature`:

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

- [ ] **Step 2: Write batch_tck.rs integration tests**

`crates/eval-orchestrator/tests/batch_tck.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

use eval_core::{JudgeConfig, Outcome, TrialRecord};
use eval_orchestrator::config::AnalysisConfig;
use eval_orchestrator::types::OrchestratorError;
use eval_orchestrator::{analyze, Decision};
use std::collections::BTreeMap;
use ulid::Ulid;

fn make_record(
    task: &str,
    agent: &str,
    run: Ulid,
    judge: Option<JudgeConfig>,
    ts: i64,
    outcome: Outcome,
) -> TrialRecord {
    TrialRecord {
        trial_id: Ulid::new(),
        run_id: run,
        task_id: task.into(),
        task_version: None,
        agent_id: agent.into(),
        agent_version: None,
        judge_config: judge,
        seed: None,
        timestamp: ts,
        outcome,
        metadata: BTreeMap::new(),
    }
}

fn judge(model: &str) -> JudgeConfig {
    JudgeConfig::new(model.into(), "gpt".into(), "a".repeat(64), 0.0, None).unwrap()
}

#[test]
fn auto_detect_irr_from_multi_judge() {
    let run = Ulid::new();
    let judges = [judge("j1"), judge("j2"), judge("j3")];
    let mut records = Vec::new();
    for i in 0..100 {
        let j = judges[i % 3].clone();
        let mut r = make_record("math", "gpt-4o", run, Some(j), i as i64, Outcome::Binary(i % 2 == 0));
        r.metadata.insert(
            "sample_id".into(),
            serde_json::Value::String(format!("q{}", i / 3)),
        );
        records.push(r);
    }
    let report = analyze(&records, &AnalysisConfig::default()).unwrap();
    assert!(report.instruments_run.contains(&"irr".to_string()));
    assert!(report.irr_results.is_some());
    let irr = report.irr_results.unwrap();
    assert!(irr.alpha.is_finite());
    assert_eq!(irr.n_raters, 3);
}

#[test]
fn auto_detect_sequential_from_repeated_obs() {
    let run = Ulid::new();
    let records: Vec<_> = (0..50)
        .map(|i| make_record("code", "claude", run, None, i, Outcome::Score(2.0)))
        .collect();
    let report = analyze(&records, &AnalysisConfig::default()).unwrap();
    assert!(report.instruments_run.contains(&"sequential".to_string()));
    let has_seq_decision = report.decisions.iter().any(|d| {
        matches!(d, Decision::StopEarly { .. } | Decision::ContinueRunning { .. })
    });
    assert!(has_seq_decision);
}

#[test]
fn auto_detect_spc_from_multi_run() {
    let runs: Vec<Ulid> = (0..25).map(|_| Ulid::new()).collect();
    let records: Vec<_> = runs
        .iter()
        .enumerate()
        .flat_map(|(ri, run)| {
            (0..10).map(move |i| {
                make_record("safety", "gpt-4o", *run, None, (ri * 100 + i) as i64, Outcome::Score(0.8))
            })
        })
        .collect();
    let report = analyze(&records, &AnalysisConfig::default()).unwrap();
    assert!(report.instruments_run.contains(&"spc".to_string()));
    assert!(!report.spc_results.is_empty());
}

#[test]
fn force_disable_overrides_detection() {
    let run = Ulid::new();
    let mut records = Vec::new();
    for i in 0..20 {
        let j = if i % 2 == 0 { judge("j1") } else { judge("j2") };
        let mut r = make_record("t", "a", run, Some(j), i, Outcome::Binary(true));
        r.metadata.insert("sample_id".into(), serde_json::Value::String(format!("q{}", i / 2)));
        records.push(r);
    }
    let mut config = AnalysisConfig::default();
    config.force_disable.push("irr".into());
    let result = analyze(&records, &config);
    match result {
        Ok(report) => assert!(!report.instruments_run.contains(&"irr".to_string())),
        Err(OrchestratorError::NoInstrumentsApplicable) => {} // also fine
        Err(e) => panic!("unexpected error: {e}"),
    }
}

#[test]
fn force_enable_overrides_detection() {
    let run = Ulid::new();
    let records: Vec<_> = (0..50)
        .map(|i| make_record("t", "a", run, None, i, Outcome::Score(0.8)))
        .collect();
    let mut config = AnalysisConfig::default();
    config.force_enable.push("spc".into());
    let report = analyze(&records, &config).unwrap();
    assert!(report.instruments_run.contains(&"spc".to_string()));
}

#[test]
fn empty_input_error() {
    let result = analyze(&[], &AnalysisConfig::default());
    assert!(matches!(result, Err(OrchestratorError::EmptyInput)));
}
```

- [ ] **Step 3: Run integration tests**

Run: `cargo test -p eval-orchestrator --test batch_tck -- --nocapture`
Expected: All 6 tests pass.

- [ ] **Step 4: Run clippy and fmt on the whole crate**

Run: `cargo clippy -p eval-orchestrator -- -D warnings && cargo fmt -p eval-orchestrator --check`
Expected: Clean.

- [ ] **Step 5: Commit**

```bash
git add tck/eval-orchestrator/features/batch.feature crates/eval-orchestrator/tests/batch_tck.rs
git commit -m "test(eval-orchestrator): batch TCK integration tests"
```

---

### Task 10: TCK Feature Files + Integration Tests (Monitor)

**Files:**
- Create: `tck/eval-orchestrator/features/monitor.feature`
- Create: `crates/eval-orchestrator/tests/monitor_tck.rs`
- Test: Integration tests

- [ ] **Step 1: Write monitor.feature**

Create `tck/eval-orchestrator/features/monitor.feature`:

```gherkin
Feature: Streaming monitor

  Scenario: SPC detects regression in streaming mode
    Given a Monitor with default config
    When 20 runs at mean=0.8 are pushed (phase I)
    And 10 runs at mean=0.2 are pushed (phase II)
    Then at least one Regression decision is emitted

  Scenario: Sequential test stops early
    Given a Monitor with sequential alpha=0.05
    When records with value=2.0 are pushed one at a time
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

- [ ] **Step 2: Write monitor_tck.rs integration tests**

`crates/eval-orchestrator/tests/monitor_tck.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

use eval_core::{Outcome, TrialRecord};
use eval_orchestrator::config::MonitorConfig;
use eval_orchestrator::{Decision, Monitor, SeriesKey};
use std::collections::BTreeMap;
use ulid::Ulid;

fn make_record(task: &str, agent: &str, run: Ulid, value: f64, ts: i64) -> TrialRecord {
    TrialRecord {
        trial_id: Ulid::new(),
        run_id: run,
        task_id: task.into(),
        task_version: None,
        agent_id: agent.into(),
        agent_version: None,
        judge_config: None,
        seed: None,
        timestamp: ts,
        outcome: Outcome::Score(value),
        metadata: BTreeMap::new(),
    }
}

#[test]
fn spc_detects_regression_streaming() {
    let mut monitor = Monitor::new(MonitorConfig::default());
    // Phase 1: 20 runs stable at 0.8
    for ri in 0..20 {
        let run = Ulid::new();
        for si in 0..10 {
            monitor.push(&make_record("t", "a", run, 0.8, (ri * 100 + si) as i64));
        }
    }
    // Phase 2: 10 runs shifted to 0.2
    let mut found = false;
    for ri in 20..30 {
        let run = Ulid::new();
        for si in 0..10 {
            let decisions =
                monitor.push(&make_record("t", "a", run, 0.2, (ri * 100 + si) as i64));
            if decisions
                .iter()
                .any(|d| matches!(d, Decision::Regression { .. }))
            {
                found = true;
            }
        }
    }
    assert!(found, "SPC should detect regression after mean shift");
}

#[test]
fn sequential_stops_early_streaming() {
    let mut monitor = Monitor::new(MonitorConfig::default());
    let run = Ulid::new();
    let mut found = false;
    for i in 0..200 {
        let decisions = monitor.push(&make_record("t", "a", run, 2.0, 1000 + i));
        if decisions
            .iter()
            .any(|d| matches!(d, Decision::StopEarly { .. }))
        {
            found = true;
            break;
        }
    }
    assert!(found, "strong signal should produce StopEarly");
}

#[test]
fn auto_detect_discovers_series() {
    let mut monitor = Monitor::new(MonitorConfig::default());
    let run = Ulid::new();
    monitor.push(&make_record("task1", "agent1", run, 0.8, 1000));
    let series = monitor.active_series();
    assert_eq!(series.len(), 1);

    monitor.push(&make_record("task2", "agent2", run, 0.5, 2000));
    let series = monitor.active_series();
    assert_eq!(series.len(), 2);
}

#[test]
fn monitor_serde_roundtrip_produces_same_decisions() {
    let config = MonitorConfig::default();
    let mut m = Monitor::new(config);
    let run = Ulid::new();
    for i in 0..100 {
        m.push(&make_record("t", "a", run, 0.8, 1000 + i));
    }

    let json = serde_json::to_string(&m).unwrap();
    let mut m2: Monitor = serde_json::from_str(&json).unwrap();

    let next = make_record("t", "a", run, 0.8, 2000);
    let d1 = m.push(&next);
    let d2 = m2.push(&next);
    assert_eq!(d1.len(), d2.len());
}

#[test]
fn push_batch_matches_sequential_push() {
    let config = MonitorConfig::default();
    let run = Ulid::new();
    let records: Vec<_> = (0..50)
        .map(|i| make_record("t", "a", run, 0.8, 1000 + i))
        .collect();

    let mut m1 = Monitor::new(config.clone());
    let mut all_d1 = Vec::new();
    for r in &records {
        all_d1.extend(m1.push(r));
    }

    let mut m2 = Monitor::new(config);
    let all_d2 = m2.push_batch(&records);

    assert_eq!(all_d1.len(), all_d2.len());
    assert_eq!(
        m1.state_summary().observations_seen,
        m2.state_summary().observations_seen,
    );
}
```

- [ ] **Step 3: Run integration tests**

Run: `cargo test -p eval-orchestrator --test monitor_tck -- --nocapture`
Expected: All 5 tests pass.

- [ ] **Step 4: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: All tests pass across all crates (eval-core, eval-ingest, eval-orchestrator, irr, spc-charts, seq-anytime-valid, salib-*).

- [ ] **Step 5: Run clippy and fmt**

Run: `cargo clippy -p eval-orchestrator -- -D warnings && cargo fmt -p eval-orchestrator --check`
Expected: Clean.

- [ ] **Step 6: Commit**

```bash
git add tck/eval-orchestrator/features/monitor.feature crates/eval-orchestrator/tests/monitor_tck.rs
git commit -m "test(eval-orchestrator): streaming Monitor TCK integration tests"
```

---

### Task 11: Cross-Check Tests (Gate 2) + BEAD Closure

**Files:**
- Create: `crates/eval-orchestrator/tests/cross_check.rs`
- Create: `.context/beads/BEAD-0017-eval-orchestrator.md`
- Test: Cross-check integration tests

These tests verify that the orchestrator's instrument adapters produce the same results as calling the underlying math crates directly.

- [ ] **Step 1: Write cross-check tests**

`crates/eval-orchestrator/tests/cross_check.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

use eval_core::{JudgeConfig, Outcome, TrialRecord};
use eval_orchestrator::config::AnalysisConfig;
use eval_orchestrator::analyze;
use irr::krippendorff;
use irr::types::{AnnotationTriple, MetricLevel, RatingMatrix};
use std::collections::BTreeMap;
use ulid::Ulid;

#[test]
fn irr_cross_check_matches_direct_krippendorff() {
    let run = Ulid::new();
    let judges = ["j1", "j2", "j3"];
    let items = ["q1", "q2", "q3", "q4", "q5"];
    let labels = [
        [1, 1, 1],
        [1, 0, 1],
        [0, 0, 0],
        [1, 1, 0],
        [0, 0, 1],
    ];

    // Build orchestrator records
    let mut records = Vec::new();
    for (ii, item) in items.iter().enumerate() {
        for (ji, judge_name) in judges.iter().enumerate() {
            let jc = JudgeConfig::new(
                (*judge_name).into(),
                "gpt".into(),
                "a".repeat(64),
                0.0,
                None,
            )
            .unwrap();
            let mut metadata = BTreeMap::new();
            metadata.insert("sample_id".into(), serde_json::Value::String((*item).into()));
            records.push(TrialRecord {
                trial_id: Ulid::new(),
                run_id: run,
                task_id: "t".into(),
                task_version: None,
                agent_id: "a".into(),
                agent_version: None,
                judge_config: Some(jc),
                seed: None,
                timestamp: (ii * 10 + ji) as i64,
                outcome: Outcome::Binary(labels[ii][ji] == 1),
                metadata,
            });
        }
    }

    // Orchestrator path
    let report = analyze(&records, &AnalysisConfig::default()).unwrap();
    let orch_alpha = report.irr_results.unwrap().alpha;

    // Direct path
    let mut triples = Vec::new();
    for (ii, item) in items.iter().enumerate() {
        for (ji, judge_name) in judges.iter().enumerate() {
            triples.push(AnnotationTriple {
                item_id: (*item).into(),
                annotator_id: (*judge_name).into(),
                label: labels[ii][ji],
            });
        }
    }
    let matrix = RatingMatrix::from_triples(&triples).unwrap();
    let direct = krippendorff::alpha(&matrix, Some(MetricLevel::Nominal)).unwrap();

    assert!(
        (orch_alpha - direct.value).abs() < 1e-10,
        "orchestrator alpha {orch_alpha} != direct alpha {}",
        direct.value
    );
}

#[test]
fn sequential_cross_check_matches_direct_monitor() {
    let run = Ulid::new();
    let values: Vec<f64> = (0..50).map(|_| 2.0).collect();

    // Orchestrator path
    let records: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(i, &v)| TrialRecord {
            trial_id: Ulid::new(),
            run_id: run,
            task_id: "t".into(),
            task_version: None,
            agent_id: "a".into(),
            agent_version: None,
            judge_config: None,
            seed: None,
            timestamp: i as i64,
            outcome: Outcome::Score(v),
            metadata: BTreeMap::new(),
        })
        .collect();

    let report = analyze(&records, &AnalysisConfig::default()).unwrap();
    let orch_evidence = report.sequential_results[0].evidence;

    // Direct path
    use seq_anytime_valid::monitor::anytime::AnytimeMonitor;
    use seq_anytime_valid::types::{DataFamily, MsprtConfig};
    let config = MsprtConfig {
        theta_0: 0.0,
        mixing_variance: 1.0,
        family: DataFamily::Normal {
            known_variance: None,
        },
        max_samples: None,
    };
    let mut monitor = AnytimeMonitor::new(config, 0.05).unwrap();
    let mut last_snap = None;
    for &v in &values {
        last_snap = Some(monitor.update(v).unwrap());
    }
    let snap = last_snap.unwrap();
    let direct_evidence = snap.e_value.unwrap_or(snap.log_likelihood_ratio.exp());

    assert!(
        (orch_evidence - direct_evidence).abs() < 1e-10,
        "orchestrator evidence {orch_evidence} != direct evidence {direct_evidence}"
    );
}
```

- [ ] **Step 2: Run cross-check tests**

Run: `cargo test -p eval-orchestrator --test cross_check -- --nocapture`
Expected: Both tests pass.

- [ ] **Step 3: Create BEAD**

`.context/beads/BEAD-0017-eval-orchestrator.md`:

```markdown
---
id: BEAD-0017
title: eval-orchestrator — analysis routing + decision engine
status: open
priority: high
created: 2026-05-15
---

## Description

eval-orchestrator crate wiring eval-ingest output to the math crates (irr, seq-anytime-valid, spc-charts). Instrument registry pattern, batch analyze() + streaming Monitor, auto-detect routing, typed Decision vocabulary.

## Acceptance

- [ ] Core types: SeriesKey, Decision, MeasurementIssue, summaries
- [ ] Configuration with sane defaults
- [ ] Instrument trait + 3 adapters (IRR, Sequential, SPC)
- [ ] Auto-detection router with force-enable/disable
- [ ] Batch analyze() producing AnalysisReport
- [ ] Streaming Monitor with serde roundtrip
- [ ] TCK batch + monitor integration tests
- [ ] Gate 2 cross-checks against direct math crate calls
- [ ] Clippy zero warnings, rustfmt clean
- [ ] Full workspace test suite passes
```

- [ ] **Step 4: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/eval-orchestrator/tests/cross_check.rs .context/beads/BEAD-0017-eval-orchestrator.md
git commit -m "test(eval-orchestrator): Gate 2 cross-check tests + BEAD-0017"
```

---

## Implementation Notes

### Serialization Workaround

`EwmaChart`, `CusumChart`, `ShewhartChart`, `CombinedChart` (from spc-charts) and `AnytimeMonitor` (from seq-anytime-valid) do NOT have `Serialize`/`Deserialize` derives. The Monitor works around this by:

1. Storing configs + raw accumulated observations in serializable state structs
2. Reconstructing chart objects from stored data on each `push()` call
3. Replaying all stored phase-2 observations through a fresh chart to recover state

This is correct but O(n) per push where n = number of stored phase-2 observations. For typical eval workloads (hundreds to low thousands of windows), this is fine. If it becomes a bottleneck, add Serialize derives to the upstream chart types.

### Sequential Monitor Replay

Same pattern: `AnytimeMonitor` lacks serde, so `SeqStreamState` stores all observations and replays them through a fresh `AnytimeMonitor` on each push. This is O(n) per push. For the expected ~100-1000 observations per series, this is acceptable.

### Import Paths

- `irr::krippendorff::alpha(matrix, level)` — not `irr::alpha`
- `irr::fleiss::kappa(matrix)` — not `irr::kappa`  
- `irr::gwet::ac(matrix, weights)` — not `irr::ac`
- `seq_anytime_valid::monitor::anytime::AnytimeMonitor` — full path, not re-exported at top
- `spc_charts::EwmaChart` — re-exported at crate root
- `eval_core::TrialRecord` — re-exported at crate root
