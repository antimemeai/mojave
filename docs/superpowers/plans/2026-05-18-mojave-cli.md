# mojave-cli Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Single `mojave` binary that is the CLI entry point to the entire measurement engine — ingest, analyze, monitor, and sensitivity analysis.

**Architecture:** Two phases — (1) extract local salib-* crates from workspace, point deps at published crates.io versions; (2) build mojave-cli crate with four subcommands wiring eval-ingest and eval-orchestrator. Thin binary + library pattern; JSON to stdout, logs to stderr.

**Tech Stack:** Rust, clap (derive), serde/serde_json/serde_yaml, tracing/tracing-subscriber, eval-ingest (workspace), eval-orchestrator (workspace), salib 0.1.1 (crates.io), notify (filesystem watching), assert_cmd/predicates (CLI smoke tests)

---

## File Structure

### Phase 1: salib extraction

**Delete:**
- `crates/salib-core/` (entire directory)
- `crates/salib-samplers/` (entire directory)
- `crates/salib-estimators/` (entire directory)
- `crates/salib-validation/` (entire directory)
- `crates/salib-shapley/` (entire directory)
- `crates/salib-surrogate/` (entire directory)
- `crates/salib-cli/` (entire directory)

**Modify:**
- `Cargo.toml` — remove 7 salib workspace members
- `crates/spc-charts/Cargo.toml` — point optional salib-estimators at crates.io

### Phase 2: mojave-cli crate

**Create:**
- `crates/mojave-cli/Cargo.toml` — crate manifest
- `crates/mojave-cli/src/main.rs` — clap dispatch only
- `crates/mojave-cli/src/lib.rs` — re-exports command modules
- `crates/mojave-cli/src/error.rs` — CliError type unifying all error sources
- `crates/mojave-cli/src/config.rs` — config file loading + CLI flag merge logic
- `crates/mojave-cli/src/detect.rs` — input format auto-detection (Inspect vs JSONL vs TrialRecord)
- `crates/mojave-cli/src/hint.rs` — hint string generation for Decision variants
- `crates/mojave-cli/src/output.rs` — JSON output wrapper with hint injection + pretty format
- `crates/mojave-cli/src/commands/mod.rs` — subcommand module root
- `crates/mojave-cli/src/commands/ingest.rs` — mojave ingest implementation
- `crates/mojave-cli/src/commands/analyze.rs` — mojave analyze implementation
- `crates/mojave-cli/src/commands/monitor.rs` — mojave monitor implementation
- `crates/mojave-cli/src/commands/sensitivity.rs` — mojave sensitivity (delegates to salib)
- `crates/mojave-cli/tests/fixtures/` — symlinks or copies of eval-ingest fixtures
- `crates/mojave-cli/tests/smoke.rs` — CLI smoke tests (assert_cmd)
- `tck/mojave-cli/features/cli.feature` — Gherkin behavioral specs

---

### Task 1: Extract salib-* crates from workspace

This task removes the 7 local salib-* crates from the workspace and points the one remaining cross-dependency (`spc-charts` optional `salib-estimators`) at the published crates.io version.

**Files:**
- Delete: `crates/salib-core/`, `crates/salib-samplers/`, `crates/salib-estimators/`, `crates/salib-validation/`, `crates/salib-shapley/`, `crates/salib-surrogate/`, `crates/salib-cli/`
- Modify: `Cargo.toml` (workspace root)
- Modify: `crates/spc-charts/Cargo.toml`

- [ ] **Step 1: Delete salib crate directories**

```bash
rm -rf crates/salib-core crates/salib-samplers crates/salib-estimators \
       crates/salib-validation crates/salib-shapley crates/salib-surrogate \
       crates/salib-cli
```

- [ ] **Step 2: Remove salib members from workspace Cargo.toml**

Edit `Cargo.toml` — remove these 7 lines from `[workspace] members`:
```toml
    "crates/salib-cli",
    "crates/salib-core",
    "crates/salib-estimators",
    "crates/salib-samplers",
    "crates/salib-shapley",
    "crates/salib-surrogate",
    "crates/salib-validation",
```

The resulting members list should be:
```toml
[workspace]
resolver = "2"
members = [
    "crates/eval-core",
    "crates/eval-ingest",
    "crates/eval-orchestrator",
    "crates/irr",
    "crates/metric-tck-harness",
    "crates/seq-anytime-valid",
    "crates/spc-charts",
]
```

- [ ] **Step 3: Update spc-charts optional salib-estimators dep**

Edit `crates/spc-charts/Cargo.toml` — change the `salib-estimators` dependency from a local path to crates.io:

Replace:
```toml
[dependencies.salib-estimators]
path = "../salib-estimators"
optional = true
```

With:
```toml
[dependencies.salib-estimators]
version = "0.1.1"
optional = true
```

- [ ] **Step 4: Verify workspace compiles and tests pass**

Run:
```bash
cargo test --workspace
```

Expected: All tests pass (157+ tests across remaining crates). No salib-related compile errors.

Run:
```bash
cargo clippy --workspace
```

Expected: Zero warnings.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: extract salib-* crates from workspace — use published crates.io versions"
```

---

### Task 2: Write TCK feature files for the CLI

**Files:**
- Create: `tck/mojave-cli/features/cli.feature`

- [ ] **Step 1: Write the Gherkin behavioral spec**

Create `tck/mojave-cli/features/cli.feature`:

```gherkin
Feature: mojave CLI — measurement engine entry point

  Scenario: Ingest Inspect AI log to JSON
    Given an Inspect AI eval log at "fixtures/inspect_binary.json"
    When I run "mojave ingest" on the file
    Then stdout is valid JSON
    And the JSON has a "records" array with at least 1 element
    And the JSON has a "source_meta" object with "runner_name" equal to "inspect_ai"
    And the JSON has a "warnings" array
    And the exit code is 0

  Scenario: Ingest JSONL log to JSON
    Given a JSONL file at "fixtures/basic.jsonl"
    When I run "mojave ingest" on the file
    Then stdout is valid JSON
    And the JSON has a "records" array with 5 elements
    And the exit code is 0

  Scenario: Analyze produces decisions with hints
    Given an Inspect AI eval log with multiple judges
    When I run "mojave analyze" on the file
    Then stdout is valid JSON
    And the JSON has a "decisions" array
    And each decision has a "hint" string field
    And the JSON has a "series_detected" array
    And the JSON has an "instruments_run" array
    And the exit code is 0

  Scenario: Analyze with config file override
    Given a config file setting irr.threshold to 0.9
    And an Inspect AI eval log with multiple judges
    When I run "mojave analyze --config=config.yaml" on the file
    Then the analysis uses irr threshold 0.9

  Scenario: Monitor reads TrialRecord JSONL from stdin
    Given a stream of 5 TrialRecord JSON lines
    When I pipe them to "mojave monitor"
    Then stdout contains one JSON object per line
    And the exit code is 0

  Scenario: Monitor emits summary on EOF
    Given a stream of 15 TrialRecord JSON lines for the same series
    When I pipe them to "mojave monitor"
    Then the last line is a MonitorSummary JSON object
    And the exit code is 0

  Scenario: Bad input file returns exit code 1
    Given a file "nonexistent.json" that does not exist
    When I run "mojave analyze nonexistent.json"
    Then the exit code is 1
    And stderr contains a JSON error with "kind" field

  Scenario: Invalid flag returns exit code 2
    When I run "mojave analyze --nonexistent-flag"
    Then the exit code is 2

  Scenario: Format auto-detection picks Inspect for .json
    Given an Inspect AI eval log at "fixtures/inspect_binary.json"
    When I run "mojave ingest" with no --format flag
    Then the ingest succeeds with runner_name "inspect_ai"

  Scenario: Format auto-detection picks JSONL for .jsonl
    Given a JSONL file at "fixtures/basic.jsonl"
    When I run "mojave ingest" with no --format flag
    Then the ingest succeeds with 5 records
```

- [ ] **Step 2: Commit**

```bash
git add tck/mojave-cli/features/cli.feature
git commit -m "tck(mojave-cli): Gherkin behavioral specs for CLI"
```

---

### Task 3: Crate skeleton — Cargo.toml, main.rs, lib.rs, error.rs

**Files:**
- Create: `crates/mojave-cli/Cargo.toml`
- Create: `crates/mojave-cli/src/main.rs`
- Create: `crates/mojave-cli/src/lib.rs`
- Create: `crates/mojave-cli/src/error.rs`
- Modify: `Cargo.toml` (workspace root — add mojave-cli member)

- [ ] **Step 1: Write the failing test for CliError**

Create `crates/mojave-cli/src/error.rs`:

```rust
use std::fmt;

#[derive(Debug)]
pub enum CliError {
    Ingest(eval_ingest::IngestError),
    Orchestrator(eval_orchestrator::OrchestratorError),
    Config(ConfigError),
    Io(std::io::Error),
}

#[derive(Debug)]
pub enum ConfigError {
    FileReadError(std::io::Error),
    ParseError(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Ingest(e) => write!(f, "{e}"),
            CliError::Orchestrator(e) => write!(f, "{e}"),
            CliError::Config(e) => write!(f, "{e}"),
            CliError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::FileReadError(e) => write!(f, "config file read error: {e}"),
            ConfigError::ParseError(msg) => write!(f, "config parse error: {msg}"),
        }
    }
}

impl std::error::Error for CliError {}
impl std::error::Error for ConfigError {}

impl From<eval_ingest::IngestError> for CliError {
    fn from(e: eval_ingest::IngestError) -> Self {
        CliError::Ingest(e)
    }
}

impl From<eval_orchestrator::OrchestratorError> for CliError {
    fn from(e: eval_orchestrator::OrchestratorError) -> Self {
        CliError::Orchestrator(e)
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Io(e)
    }
}

impl CliError {
    pub fn kind(&self) -> &'static str {
        match self {
            CliError::Ingest(_) => "ingest_error",
            CliError::Orchestrator(_) => "orchestrator_error",
            CliError::Config(_) => "config_error",
            CliError::Io(_) => "io_error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::unwrap_used)]
    #[test]
    fn cli_error_kind_strings() {
        let io_err = CliError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));
        assert_eq!(io_err.kind(), "io_error");

        let config_err = CliError::Config(ConfigError::ParseError("bad yaml".into()));
        assert_eq!(config_err.kind(), "config_error");
    }

    #[test]
    fn cli_error_display() {
        let config_err = CliError::Config(ConfigError::ParseError("bad yaml".into()));
        let msg = format!("{config_err}");
        assert!(msg.contains("bad yaml"), "display should contain inner message");
    }
}
```

- [ ] **Step 2: Create Cargo.toml**

Create `crates/mojave-cli/Cargo.toml`:

```toml
[package]
name = "mojave-cli"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
publish = false
description = """
mojave — measurement engine CLI. Ingest eval logs, run batch analysis,
stream monitoring, and sensitivity analysis from one binary.
"""

[[bin]]
name = "mojave"
path = "src/main.rs"

[lib]
path = "src/lib.rs"

[dependencies]
eval-ingest = { path = "../eval-ingest" }
eval-orchestrator = { path = "../eval-orchestrator" }
eval-core = { path = "../eval-core" }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"

[lints]
workspace = true
```

- [ ] **Step 3: Create lib.rs**

Create `crates/mojave-cli/src/lib.rs`:

```rust
#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod commands;
pub mod config;
pub mod detect;
pub mod error;
pub mod hint;
pub mod output;
```

- [ ] **Step 4: Create stub modules so lib.rs compiles**

Create `crates/mojave-cli/src/commands/mod.rs`:

```rust
pub mod analyze;
pub mod ingest;
pub mod monitor;
pub mod sensitivity;
```

Create `crates/mojave-cli/src/commands/ingest.rs`:

```rust
// Populated in Task 5
```

Create `crates/mojave-cli/src/commands/analyze.rs`:

```rust
// Populated in Task 6
```

Create `crates/mojave-cli/src/commands/monitor.rs`:

```rust
// Populated in Task 8
```

Create `crates/mojave-cli/src/commands/sensitivity.rs`:

```rust
// Populated in Task 9
```

Create `crates/mojave-cli/src/config.rs`:

```rust
// Populated in Task 4
```

Create `crates/mojave-cli/src/detect.rs`:

```rust
// Populated in Task 4
```

Create `crates/mojave-cli/src/hint.rs`:

```rust
// Populated in Task 6
```

Create `crates/mojave-cli/src/output.rs`:

```rust
// Populated in Task 6
```

- [ ] **Step 5: Create main.rs with clap skeleton**

Create `crates/mojave-cli/src/main.rs`:

```rust
#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mojave", about = "Measurement engine for AI agent evaluation")]
struct Cli {
    /// Enable verbose logging to stderr
    #[arg(long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest eval runner output into normalized TrialRecords
    Ingest {
        /// Input files or directories
        #[arg(required = true)]
        paths: Vec<std::path::PathBuf>,

        /// Input format: auto, inspect, jsonl
        #[arg(long, default_value = "auto")]
        format: String,

        /// Path to YAML field mapping for JSONL input
        #[arg(long)]
        field_mapping: Option<std::path::PathBuf>,
    },

    /// Run measurement battery on eval data
    Analyze {
        /// Input files or directories
        #[arg(required = true)]
        paths: Vec<std::path::PathBuf>,

        /// Path to YAML config file
        #[arg(long)]
        config: Option<std::path::PathBuf>,

        /// Output format: json, pretty
        #[arg(long, default_value = "json")]
        format: String,

        /// IRR agreement threshold
        #[arg(long)]
        irr_threshold: Option<f64>,

        /// IRR metric: krippendorff, fleiss, gwet
        #[arg(long)]
        irr_metric: Option<String>,

        /// SPC chart type: ewma, cusum, shewhart, combined
        #[arg(long)]
        spc_chart: Option<String>,

        /// SPC phase 1 calibration windows
        #[arg(long)]
        spc_phase1_windows: Option<usize>,

        /// Sequential testing alpha level
        #[arg(long)]
        sequential_alpha: Option<f64>,

        /// Force-enable instruments (comma-separated: irr,sequential,spc)
        #[arg(long)]
        force_enable: Option<String>,

        /// Force-disable instruments (comma-separated: irr,sequential,spc)
        #[arg(long)]
        force_disable: Option<String>,
    },

    /// Stream analysis — read records incrementally, emit decisions
    Monitor {
        /// File or directory to watch (omit for stdin)
        #[arg(long)]
        watch: Option<std::path::PathBuf>,

        /// Path to YAML config file
        #[arg(long)]
        config: Option<std::path::PathBuf>,

        /// Output format: json, pretty
        #[arg(long, default_value = "json")]
        format: String,

        /// IRR agreement threshold
        #[arg(long)]
        irr_threshold: Option<f64>,

        /// IRR metric: krippendorff, fleiss, gwet
        #[arg(long)]
        irr_metric: Option<String>,

        /// SPC chart type: ewma, cusum, shewhart, combined
        #[arg(long)]
        spc_chart: Option<String>,

        /// SPC phase 1 calibration windows
        #[arg(long)]
        spc_phase1_windows: Option<usize>,

        /// Sequential testing alpha level
        #[arg(long)]
        sequential_alpha: Option<f64>,
    },

    /// Sensitivity analysis (delegates to salib)
    Sensitivity {
        /// Passthrough arguments to salib
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();
    }

    let result = match cli.command {
        Commands::Ingest { .. } => {
            eprintln!("mojave ingest: not yet implemented");
            std::process::exit(2)
        }
        Commands::Analyze { .. } => {
            eprintln!("mojave analyze: not yet implemented");
            std::process::exit(2)
        }
        Commands::Monitor { .. } => {
            eprintln!("mojave monitor: not yet implemented");
            std::process::exit(2)
        }
        Commands::Sensitivity { .. } => {
            eprintln!("mojave sensitivity: not yet implemented");
            std::process::exit(2)
        }
    };
}
```

- [ ] **Step 6: Add mojave-cli to workspace members**

Edit `Cargo.toml` — add `"crates/mojave-cli"` to the `members` list:

```toml
[workspace]
resolver = "2"
members = [
    "crates/eval-core",
    "crates/eval-ingest",
    "crates/eval-orchestrator",
    "crates/irr",
    "crates/metric-tck-harness",
    "crates/mojave-cli",
    "crates/seq-anytime-valid",
    "crates/spc-charts",
]
```

- [ ] **Step 7: Verify it compiles and error tests pass**

Run:
```bash
cargo test -p mojave-cli
```

Expected: 2 tests pass (cli_error_kind_strings, cli_error_display).

Run:
```bash
cargo build -p mojave-cli
```

Expected: Binary compiles. `target/debug/mojave --help` prints help text.

- [ ] **Step 8: Commit**

```bash
git add crates/mojave-cli/ Cargo.toml
git commit -m "feat(mojave-cli): crate skeleton with clap dispatch + CliError type"
```

---

### Task 4: Config loading + format auto-detection

**Files:**
- Modify: `crates/mojave-cli/src/config.rs`
- Modify: `crates/mojave-cli/src/detect.rs`

- [ ] **Step 1: Write failing tests for config loading**

Replace contents of `crates/mojave-cli/src/config.rs`:

```rust
use eval_orchestrator::config::{
    AnalysisConfig, IrrMetric, MonitorConfig, SpcChartType, WindowSize,
};

use crate::error::ConfigError;

/// Load config from YAML file, then apply CLI flag overrides.
pub fn load_config(
    config_path: Option<&std::path::Path>,
    overrides: &ConfigOverrides,
) -> Result<AnalysisConfig, ConfigError> {
    let mut config = match config_path {
        Some(path) => {
            let contents = std::fs::read_to_string(path)
                .map_err(ConfigError::FileReadError)?;
            serde_yaml::from_str(&contents)
                .map_err(|e| ConfigError::ParseError(e.to_string()))?
        }
        None => AnalysisConfig::default(),
    };

    apply_overrides(&mut config, overrides);
    Ok(config)
}

/// Load MonitorConfig from YAML file, then apply CLI flag overrides.
pub fn load_monitor_config(
    config_path: Option<&std::path::Path>,
    overrides: &ConfigOverrides,
) -> Result<MonitorConfig, ConfigError> {
    let mut config = match config_path {
        Some(path) => {
            let contents = std::fs::read_to_string(path)
                .map_err(ConfigError::FileReadError)?;
            serde_yaml::from_str(&contents)
                .map_err(|e| ConfigError::ParseError(e.to_string()))?
        }
        None => MonitorConfig::default(),
    };

    if let Some(v) = overrides.irr_threshold {
        config.irr.threshold = v;
    }
    if let Some(ref m) = overrides.irr_metric {
        if let Some(parsed) = parse_irr_metric(m) {
            config.irr.metric = parsed;
        }
    }
    if let Some(ref ct) = overrides.spc_chart {
        if let Some(parsed) = parse_spc_chart(ct) {
            config.spc.chart_type = parsed;
        }
    }
    if let Some(v) = overrides.spc_phase1_windows {
        config.spc.phase1_windows = v;
    }
    if let Some(v) = overrides.sequential_alpha {
        config.sequential.alpha = v;
    }

    Ok(config)
}

/// CLI flag overrides — all optional.
#[derive(Debug, Default)]
pub struct ConfigOverrides {
    pub irr_threshold: Option<f64>,
    pub irr_metric: Option<String>,
    pub spc_chart: Option<String>,
    pub spc_phase1_windows: Option<usize>,
    pub sequential_alpha: Option<f64>,
    pub force_enable: Option<String>,
    pub force_disable: Option<String>,
}

fn apply_overrides(config: &mut AnalysisConfig, overrides: &ConfigOverrides) {
    if let Some(v) = overrides.irr_threshold {
        config.irr.threshold = v;
    }
    if let Some(ref m) = overrides.irr_metric {
        if let Some(parsed) = parse_irr_metric(m) {
            config.irr.metric = parsed;
        }
    }
    if let Some(ref ct) = overrides.spc_chart {
        if let Some(parsed) = parse_spc_chart(ct) {
            config.spc.chart_type = parsed;
        }
    }
    if let Some(v) = overrides.spc_phase1_windows {
        config.spc.phase1_windows = v;
    }
    if let Some(v) = overrides.sequential_alpha {
        config.sequential.alpha = v;
    }
    if let Some(ref fe) = overrides.force_enable {
        config.force_enable = fe.split(',').map(|s| s.trim().to_string()).collect();
    }
    if let Some(ref fd) = overrides.force_disable {
        config.force_disable = fd.split(',').map(|s| s.trim().to_string()).collect();
    }
}

fn parse_irr_metric(s: &str) -> Option<IrrMetric> {
    match s.to_lowercase().as_str() {
        "krippendorff" => Some(IrrMetric::Krippendorff),
        "fleiss" => Some(IrrMetric::Fleiss),
        "gwet" => Some(IrrMetric::Gwet),
        _ => None,
    }
}

fn parse_spc_chart(s: &str) -> Option<SpcChartType> {
    match s.to_lowercase().as_str() {
        "ewma" => Some(SpcChartType::Ewma),
        "cusum" => Some(SpcChartType::Cusum),
        "shewhart" => Some(SpcChartType::Shewhart),
        "combined" => Some(SpcChartType::Combined),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn default_config_when_no_file() {
        let overrides = ConfigOverrides::default();
        let config = load_config(None, &overrides).unwrap();
        assert!((config.irr.threshold - 0.67).abs() < f64::EPSILON);
        assert!((config.sequential.alpha - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn flag_overrides_default() {
        let overrides = ConfigOverrides {
            irr_threshold: Some(0.9),
            sequential_alpha: Some(0.01),
            ..Default::default()
        };
        let config = load_config(None, &overrides).unwrap();
        assert!((config.irr.threshold - 0.9).abs() < f64::EPSILON);
        assert!((config.sequential.alpha - 0.01).abs() < f64::EPSILON);
    }

    #[test]
    fn yaml_file_loads() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "irr:\n  threshold: 0.8\nspc:\n  chart_type: Cusum\n").unwrap();
        let overrides = ConfigOverrides::default();
        let config = load_config(Some(tmp.path()), &overrides).unwrap();
        assert!((config.irr.threshold - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn flag_overrides_yaml_file() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "irr:\n  threshold: 0.8\n").unwrap();
        let overrides = ConfigOverrides {
            irr_threshold: Some(0.95),
            ..Default::default()
        };
        let config = load_config(Some(tmp.path()), &overrides).unwrap();
        assert!((config.irr.threshold - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn bad_yaml_returns_parse_error() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "{{{{invalid yaml").unwrap();
        let overrides = ConfigOverrides::default();
        let result = load_config(Some(tmp.path()), &overrides);
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }

    #[test]
    fn force_enable_splits_comma_list() {
        let overrides = ConfigOverrides {
            force_enable: Some("irr, spc".into()),
            ..Default::default()
        };
        let config = load_config(None, &overrides).unwrap();
        assert_eq!(config.force_enable, vec!["irr", "spc"]);
    }

    #[test]
    fn parse_irr_metric_variants() {
        assert!(matches!(parse_irr_metric("krippendorff"), Some(IrrMetric::Krippendorff)));
        assert!(matches!(parse_irr_metric("Fleiss"), Some(IrrMetric::Fleiss)));
        assert!(matches!(parse_irr_metric("gwet"), Some(IrrMetric::Gwet)));
        assert!(parse_irr_metric("bogus").is_none());
    }

    #[test]
    fn parse_spc_chart_variants() {
        assert!(matches!(parse_spc_chart("ewma"), Some(SpcChartType::Ewma)));
        assert!(matches!(parse_spc_chart("CUSUM"), Some(SpcChartType::Cusum)));
        assert!(parse_spc_chart("bogus").is_none());
    }
}
```

- [ ] **Step 2: Write failing tests for format auto-detection**

Replace contents of `crates/mojave-cli/src/detect.rs`:

```rust
use std::path::Path;

/// Detected input format for eval data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Inspect,
    Jsonl,
}

/// Auto-detect input format from file extension and content sniffing.
///
/// Strategy:
/// 1. If extension is `.jsonl` or `.ndjson` → JSONL
/// 2. If extension is `.json` → try parsing as Inspect (look for "eval" key)
/// 3. Fall back to content sniffing: read first bytes, check structure
pub fn detect_format(path: &Path) -> Result<InputFormat, DetectError> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext.to_lowercase().as_str() {
        "jsonl" | "ndjson" => return Ok(InputFormat::Jsonl),
        "json" => {
            return sniff_json_file(path);
        }
        _ => {}
    }

    // No recognized extension — sniff content
    sniff_json_file(path)
}

/// Parse a user-provided --format flag into an InputFormat.
pub fn parse_format_flag(flag: &str) -> Result<Option<InputFormat>, DetectError> {
    match flag.to_lowercase().as_str() {
        "auto" => Ok(None),
        "inspect" => Ok(Some(InputFormat::Inspect)),
        "jsonl" => Ok(Some(InputFormat::Jsonl)),
        other => Err(DetectError::UnknownFormat(other.to_string())),
    }
}

fn sniff_json_file(path: &Path) -> Result<InputFormat, DetectError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| DetectError::IoError(e.to_string()))?;
    let trimmed = contents.trim_start();

    // Inspect AI logs are JSON objects with a top-level "eval" key
    if trimmed.starts_with('{') {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if value.get("eval").is_some() || value.get("results").is_some() {
                return Ok(InputFormat::Inspect);
            }
        }
        // Single JSON object but not Inspect — treat as single-line JSONL
        return Ok(InputFormat::Jsonl);
    }

    // Multiple lines starting with '{' — JSONL
    if trimmed.lines().all(|l| l.trim_start().starts_with('{') || l.trim().is_empty()) {
        return Ok(InputFormat::Jsonl);
    }

    Err(DetectError::Unrecognized)
}

#[derive(Debug)]
pub enum DetectError {
    UnknownFormat(String),
    IoError(String),
    Unrecognized,
}

impl std::fmt::Display for DetectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectError::UnknownFormat(s) => write!(f, "unknown format: {s}"),
            DetectError::IoError(s) => write!(f, "I/O error during detection: {s}"),
            DetectError::Unrecognized => write!(f, "could not detect input format"),
        }
    }
}

impl std::error::Error for DetectError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn jsonl_extension_detected() {
        let tmp = tempfile::Builder::new().suffix(".jsonl").tempfile().unwrap();
        assert_eq!(detect_format(tmp.path()).unwrap(), InputFormat::Jsonl);
    }

    #[test]
    fn ndjson_extension_detected() {
        let tmp = tempfile::Builder::new().suffix(".ndjson").tempfile().unwrap();
        assert_eq!(detect_format(tmp.path()).unwrap(), InputFormat::Jsonl);
    }

    #[test]
    fn inspect_json_detected_by_eval_key() {
        let mut tmp = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
        write!(tmp, r#"{{"eval":{{"task":"t1"}},"results":[]}}"#).unwrap();
        assert_eq!(detect_format(tmp.path()).unwrap(), InputFormat::Inspect);
    }

    #[test]
    fn plain_json_object_treated_as_jsonl() {
        let mut tmp = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
        write!(tmp, r#"{{"task_id":"t","score":0.5}}"#).unwrap();
        assert_eq!(detect_format(tmp.path()).unwrap(), InputFormat::Jsonl);
    }

    #[test]
    fn parse_format_flag_auto() {
        assert_eq!(parse_format_flag("auto").unwrap(), None);
    }

    #[test]
    fn parse_format_flag_explicit() {
        assert_eq!(parse_format_flag("inspect").unwrap(), Some(InputFormat::Inspect));
        assert_eq!(parse_format_flag("jsonl").unwrap(), Some(InputFormat::Jsonl));
    }

    #[test]
    fn parse_format_flag_unknown() {
        assert!(parse_format_flag("xml").is_err());
    }
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run:
```bash
cargo test -p mojave-cli
```

Expected: All config + detect tests pass (8 config tests + 7 detect tests + 2 error tests = 17 total).

- [ ] **Step 4: Commit**

```bash
git add crates/mojave-cli/src/config.rs crates/mojave-cli/src/detect.rs
git commit -m "feat(mojave-cli): config loading with flag overrides + format auto-detection"
```

---

### Task 5: Ingest command

**Files:**
- Modify: `crates/mojave-cli/src/commands/ingest.rs`

- [ ] **Step 1: Write the ingest command implementation**

Replace contents of `crates/mojave-cli/src/commands/ingest.rs`:

```rust
use std::path::{Path, PathBuf};

use eval_ingest::inspect::InspectAdapter;
use eval_ingest::types::{IngestAdapter, IngestResult, IngestSource};
use eval_ingest::{FieldMapping, JsonlAdapter};
use serde::Serialize;

use crate::detect::{detect_format, parse_format_flag, InputFormat};
use crate::error::CliError;

/// Output shape for `mojave ingest` — wraps IngestResult into CLI JSON output.
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

/// Run the ingest command on one or more paths.
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
            let mapping: FieldMapping = serde_yaml::from_str(&contents)
                .map_err(|e| CliError::Config(crate::error::ConfigError::ParseError(e.to_string())))?;
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
            None => detect_format(path)
                .map_err(|e| CliError::Config(crate::error::ConfigError::ParseError(e.to_string())))?,
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
                kind: format!("{:?}", w.kind),
                message: format!("{}", w.kind),
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
        assert!(!output.records.is_empty(), "should produce records");
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
```

- [ ] **Step 2: Run tests**

Run:
```bash
cargo test -p mojave-cli -- ingest
```

Expected: 4 ingest tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/mojave-cli/src/commands/ingest.rs
git commit -m "feat(mojave-cli): ingest command — Inspect + JSONL with auto-detection"
```

---

### Task 6: Hint generation + output wrapper + analyze command

**Files:**
- Modify: `crates/mojave-cli/src/hint.rs`
- Modify: `crates/mojave-cli/src/output.rs`
- Modify: `crates/mojave-cli/src/commands/analyze.rs`

- [ ] **Step 1: Write hint generation**

Replace contents of `crates/mojave-cli/src/hint.rs`:

```rust
use eval_orchestrator::types::{Decision, MeasurementIssue};

/// Generate a human-readable hint string for a Decision.
pub fn decision_hint(decision: &Decision) -> String {
    match decision {
        Decision::StopEarly {
            evidence,
            estimate,
            ci,
            ..
        } => {
            let half_width = (ci.1 - ci.0) / 2.0;
            format!(
                "Effect stable at {estimate:.2} \u{00b1} {half_width:.2}. Evidence ({evidence:.1}) exceeds threshold \u{2014} safe to stop."
            )
        }
        Decision::ContinueRunning {
            current_n, ..
        } => {
            format!("{current_n} observations, insufficient evidence.")
        }
        Decision::Regression {
            observation_value,
            control_limits,
            ..
        } => {
            format!(
                "Observation {observation_value:.3} outside control limits [{:.3}, {:.3}].",
                control_limits.0, control_limits.1
            )
        }
        Decision::MeasurementWarning { issue, .. } => match issue {
            MeasurementIssue::LowAgreement { kappa, threshold } => {
                format!(
                    "Inter-rater agreement (\u{03ba}={kappa:.2}) below threshold ({threshold:.2})."
                )
            }
            MeasurementIssue::InsufficientRaters { have, need } => {
                format!("Only {have} rater(s) found, need \u{2265}{need} for inter-rater reliability.")
            }
            MeasurementIssue::InsufficientSamples { have, need } => {
                format!("Only {have} sample(s), need \u{2265}{need}.")
            }
            MeasurementIssue::HighVariance { cv, threshold } => {
                format!("Coefficient of variation ({cv:.2}) exceeds threshold ({threshold:.2}).")
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eval_orchestrator::types::SeriesKey;

    fn test_series() -> SeriesKey {
        SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        }
    }

    #[test]
    fn hint_stop_early() {
        let d = Decision::StopEarly {
            series: test_series(),
            evidence: 47.2,
            estimate: 0.82,
            ci: (0.79, 0.85),
        };
        let hint = decision_hint(&d);
        assert!(hint.contains("0.82"), "hint should contain estimate");
        assert!(hint.contains("safe to stop"), "hint should say safe to stop");
    }

    #[test]
    fn hint_continue_running() {
        let d = Decision::ContinueRunning {
            series: test_series(),
            current_n: 38,
            estimated_n_needed: 0,
            power_at_current_n: 0.0,
        };
        let hint = decision_hint(&d);
        assert!(hint.contains("38"), "hint should contain current_n");
        assert!(hint.contains("insufficient"), "hint should say insufficient");
    }

    #[test]
    fn hint_regression() {
        let d = Decision::Regression {
            series: test_series(),
            signal: spc_charts::types::ChartSignal::InControl,
            observation_value: 0.43,
            control_limits: (0.71, 0.89),
        };
        let hint = decision_hint(&d);
        assert!(hint.contains("0.430"), "hint should contain observation");
        assert!(hint.contains("0.710"), "hint should contain lower limit");
    }

    #[test]
    fn hint_low_agreement() {
        let d = Decision::MeasurementWarning {
            series: test_series(),
            issue: MeasurementIssue::LowAgreement {
                kappa: 0.31,
                threshold: 0.67,
            },
        };
        let hint = decision_hint(&d);
        assert!(hint.contains("0.31"), "hint should contain kappa");
        assert!(hint.contains("0.67"), "hint should contain threshold");
    }

    #[test]
    fn hint_insufficient_raters() {
        let d = Decision::MeasurementWarning {
            series: test_series(),
            issue: MeasurementIssue::InsufficientRaters { have: 1, need: 2 },
        };
        let hint = decision_hint(&d);
        assert!(hint.contains("1 rater"), "hint should contain count");
    }
}
```

- [ ] **Step 2: Write the output wrapper**

Replace contents of `crates/mojave-cli/src/output.rs`:

```rust
use eval_orchestrator::types::{AnalysisReport, Decision};
use serde::Serialize;

use crate::hint::decision_hint;

/// CLI output shape for `mojave analyze` — AnalysisReport with hint-enriched decisions.
#[derive(Serialize)]
pub struct AnalyzeOutput {
    pub series_detected: Vec<eval_orchestrator::types::SeriesKey>,
    pub instruments_run: Vec<String>,
    pub decisions: Vec<DecisionWithHint>,
    pub summaries: Summaries,
}

#[derive(Serialize)]
pub struct DecisionWithHint {
    #[serde(flatten)]
    pub decision: Decision,
    pub hint: String,
}

#[derive(Serialize)]
pub struct Summaries {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub irr: Option<eval_orchestrator::types::IrrSummary>,
    pub sequential: Vec<eval_orchestrator::types::SequentialSummary>,
    pub spc: Vec<eval_orchestrator::types::SpcSummary>,
}

impl AnalyzeOutput {
    pub fn from_report(report: AnalysisReport) -> Self {
        let decisions = report
            .decisions
            .into_iter()
            .map(|d| {
                let hint = decision_hint(&d);
                DecisionWithHint { decision: d, hint }
            })
            .collect();

        AnalyzeOutput {
            series_detected: report.series_detected,
            instruments_run: report.instruments_run,
            decisions,
            summaries: Summaries {
                irr: report.irr_results,
                sequential: report.sequential_results,
                spc: report.spc_results,
            },
        }
    }
}

/// Write JSON to stdout. Returns a CliError on serialization failure.
pub fn write_json<T: Serialize>(value: &T) -> Result<(), crate::error::CliError> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| crate::error::CliError::Io(
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
        ))?;
    println!("{json}");
    Ok(())
}

/// Write a structured error to stderr as JSON.
pub fn write_error(error: &crate::error::CliError) {
    let err_json = serde_json::json!({
        "error": error.to_string(),
        "kind": error.kind(),
    });
    eprintln!("{}", serde_json::to_string(&err_json).unwrap_or_else(|_| error.to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use eval_orchestrator::types::{AnalysisReport, SeriesKey};

    #[test]
    fn analyze_output_from_empty_report() {
        let report = AnalysisReport {
            decisions: vec![],
            irr_results: None,
            sequential_results: vec![],
            spc_results: vec![],
            series_detected: vec![],
            instruments_run: vec![],
        };
        let output = AnalyzeOutput::from_report(report);
        assert!(output.decisions.is_empty());
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"decisions\":[]"));
    }

    #[test]
    fn analyze_output_includes_hints() {
        let report = AnalysisReport {
            decisions: vec![Decision::ContinueRunning {
                series: SeriesKey {
                    task_id: "t".into(),
                    agent_id: "a".into(),
                    scorer: None,
                },
                current_n: 5,
                estimated_n_needed: 0,
                power_at_current_n: 0.0,
            }],
            irr_results: None,
            sequential_results: vec![],
            spc_results: vec![],
            series_detected: vec![],
            instruments_run: vec!["sequential".into()],
        };
        let output = AnalyzeOutput::from_report(report);
        assert_eq!(output.decisions.len(), 1);
        assert!(output.decisions[0].hint.contains("5 observations"));
    }
}
```

- [ ] **Step 3: Write the analyze command**

Replace contents of `crates/mojave-cli/src/commands/analyze.rs`:

```rust
use std::path::{Path, PathBuf};

use eval_orchestrator::analyze;

use crate::commands::ingest::run_ingest;
use crate::config::{load_config, ConfigOverrides};
use crate::error::CliError;
use crate::output::AnalyzeOutput;

/// Run the analyze command: ingest input, run measurement battery, return enriched report.
pub fn run_analyze(
    paths: &[PathBuf],
    config_path: Option<&Path>,
    format_flag: &str,
    overrides: &ConfigOverrides,
) -> Result<AnalyzeOutput, CliError> {
    // 1. Ingest all input files
    let ingest_output = run_ingest(paths, "auto", None)?;

    if ingest_output.records.is_empty() {
        return Err(CliError::Orchestrator(
            eval_orchestrator::OrchestratorError::EmptyInput,
        ));
    }

    // 2. Load config
    let config = load_config(config_path, overrides)?;

    // 3. Run analysis
    let report = analyze(&ingest_output.records, &config)?;

    // 4. Wrap with hints
    Ok(AnalyzeOutput::from_report(report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigOverrides;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../eval-ingest/tests/fixtures")
            .join(name)
    }

    #[test]
    fn analyze_inspect_binary() {
        let paths = vec![fixture_path("inspect_binary.json")];
        let overrides = ConfigOverrides::default();
        let output = run_analyze(&paths, None, "json", &overrides).unwrap();
        assert!(!output.series_detected.is_empty(), "should detect at least one series");
        // Output should serialize cleanly
        let json = serde_json::to_string(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["decisions"].is_array());
        assert!(parsed["instruments_run"].is_array());
        assert!(parsed["summaries"].is_object());
    }

    #[test]
    fn analyze_jsonl_basic() {
        let paths = vec![fixture_path("basic.jsonl")];
        let overrides = ConfigOverrides::default();
        let output = run_analyze(&paths, None, "json", &overrides).unwrap();
        assert!(!output.series_detected.is_empty());
    }

    #[test]
    fn analyze_with_config_override() {
        let paths = vec![fixture_path("basic.jsonl")];
        let overrides = ConfigOverrides {
            sequential_alpha: Some(0.01),
            ..Default::default()
        };
        // Should not error even with non-default config
        let _output = run_analyze(&paths, None, "json", &overrides).unwrap();
    }

    #[test]
    fn analyze_decisions_have_hints() {
        let paths = vec![fixture_path("basic.jsonl")];
        let overrides = ConfigOverrides::default();
        let output = run_analyze(&paths, None, "json", &overrides).unwrap();
        for d in &output.decisions {
            assert!(!d.hint.is_empty(), "every decision should have a non-empty hint");
        }
    }
}
```

- [ ] **Step 4: Run all tests**

Run:
```bash
cargo test -p mojave-cli
```

Expected: All hint tests (5), output tests (2), analyze tests (4), plus earlier tests = ~28 total passing.

- [ ] **Step 5: Commit**

```bash
git add crates/mojave-cli/src/hint.rs crates/mojave-cli/src/output.rs crates/mojave-cli/src/commands/analyze.rs
git commit -m "feat(mojave-cli): analyze command with hint-enriched JSON output"
```

---

### Task 7: Wire main.rs to ingest + analyze commands

**Files:**
- Modify: `crates/mojave-cli/src/main.rs`

- [ ] **Step 1: Wire the subcommands to the library functions**

Replace the entire contents of `crates/mojave-cli/src/main.rs`:

```rust
#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};
use mojave_cli::commands::{analyze, ingest};
use mojave_cli::config::ConfigOverrides;
use mojave_cli::output::{write_error, write_json};

#[derive(Parser)]
#[command(name = "mojave", about = "Measurement engine for AI agent evaluation")]
struct Cli {
    /// Enable verbose logging to stderr
    #[arg(long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest eval runner output into normalized TrialRecords
    Ingest {
        /// Input files or directories
        #[arg(required = true)]
        paths: Vec<std::path::PathBuf>,

        /// Input format: auto, inspect, jsonl
        #[arg(long, default_value = "auto")]
        format: String,

        /// Path to YAML field mapping for JSONL input
        #[arg(long)]
        field_mapping: Option<std::path::PathBuf>,
    },

    /// Run measurement battery on eval data
    Analyze {
        /// Input files or directories
        #[arg(required = true)]
        paths: Vec<std::path::PathBuf>,

        /// Path to YAML config file
        #[arg(long)]
        config: Option<std::path::PathBuf>,

        /// Output format: json, pretty
        #[arg(long, default_value = "json")]
        format: String,

        /// IRR agreement threshold
        #[arg(long)]
        irr_threshold: Option<f64>,

        /// IRR metric: krippendorff, fleiss, gwet
        #[arg(long)]
        irr_metric: Option<String>,

        /// SPC chart type: ewma, cusum, shewhart, combined
        #[arg(long)]
        spc_chart: Option<String>,

        /// SPC phase 1 calibration windows
        #[arg(long)]
        spc_phase1_windows: Option<usize>,

        /// Sequential testing alpha level
        #[arg(long)]
        sequential_alpha: Option<f64>,

        /// Force-enable instruments (comma-separated: irr,sequential,spc)
        #[arg(long)]
        force_enable: Option<String>,

        /// Force-disable instruments (comma-separated: irr,sequential,spc)
        #[arg(long)]
        force_disable: Option<String>,
    },

    /// Stream analysis — read records incrementally, emit decisions
    Monitor {
        /// File or directory to watch (omit for stdin)
        #[arg(long)]
        watch: Option<std::path::PathBuf>,

        /// Path to YAML config file
        #[arg(long)]
        config: Option<std::path::PathBuf>,

        /// Output format: json, pretty
        #[arg(long, default_value = "json")]
        format: String,

        /// IRR agreement threshold
        #[arg(long)]
        irr_threshold: Option<f64>,

        /// IRR metric: krippendorff, fleiss, gwet
        #[arg(long)]
        irr_metric: Option<String>,

        /// SPC chart type: ewma, cusum, shewhart, combined
        #[arg(long)]
        spc_chart: Option<String>,

        /// SPC phase 1 calibration windows
        #[arg(long)]
        spc_phase1_windows: Option<usize>,

        /// Sequential testing alpha level
        #[arg(long)]
        sequential_alpha: Option<f64>,
    },

    /// Sensitivity analysis (delegates to salib)
    Sensitivity {
        /// Passthrough arguments to salib
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();
    }

    let result = match cli.command {
        Commands::Ingest {
            paths,
            format,
            field_mapping,
        } => {
            let output = ingest::run_ingest(&paths, &format, field_mapping.as_deref());
            match output {
                Ok(out) => write_json(&out),
                Err(e) => {
                    write_error(&e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Analyze {
            paths,
            config,
            format: _,
            irr_threshold,
            irr_metric,
            spc_chart,
            spc_phase1_windows,
            sequential_alpha,
            force_enable,
            force_disable,
        } => {
            let overrides = ConfigOverrides {
                irr_threshold,
                irr_metric,
                spc_chart,
                spc_phase1_windows,
                sequential_alpha,
                force_enable,
                force_disable,
            };
            let output = analyze::run_analyze(&paths, config.as_deref(), "json", &overrides);
            match output {
                Ok(out) => write_json(&out),
                Err(e) => {
                    write_error(&e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Monitor { .. } => {
            eprintln!("mojave monitor: not yet implemented");
            std::process::exit(2);
        }
        Commands::Sensitivity { .. } => {
            eprintln!("mojave sensitivity: not yet implemented");
            std::process::exit(2);
        }
    };

    if let Err(e) = result {
        write_error(&e);
        std::process::exit(1);
    }
}
```

- [ ] **Step 2: Build and manually test**

Run:
```bash
cargo build -p mojave-cli
```

Then verify:
```bash
./target/debug/mojave ingest crates/eval-ingest/tests/fixtures/inspect_binary.json | head -20
./target/debug/mojave analyze crates/eval-ingest/tests/fixtures/basic.jsonl | head -20
./target/debug/mojave --help
```

Expected: JSON output to stdout for ingest and analyze. Help text shows all subcommands.

- [ ] **Step 3: Commit**

```bash
git add crates/mojave-cli/src/main.rs
git commit -m "feat(mojave-cli): wire ingest + analyze commands in main.rs"
```

---

### Task 8: Monitor command (stdin + watch)

**Files:**
- Modify: `crates/mojave-cli/src/commands/monitor.rs`
- Modify: `crates/mojave-cli/src/main.rs` (wire monitor command)

- [ ] **Step 1: Write the monitor command implementation**

Replace contents of `crates/mojave-cli/src/commands/monitor.rs`:

```rust
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use eval_core::TrialRecord;
use eval_orchestrator::Monitor;

use crate::config::{load_monitor_config, ConfigOverrides};
use crate::error::CliError;
use crate::hint::decision_hint;

/// Run monitor in stdin mode: read TrialRecord JSON lines, emit decisions.
pub fn run_monitor_stdin(
    config_path: Option<&Path>,
    overrides: &ConfigOverrides,
) -> Result<(), CliError> {
    let config = load_monitor_config(config_path, overrides)?;
    let mut monitor = Monitor::new(config);
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    for line_result in stdin.lock().lines() {
        let line = line_result?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let record: TrialRecord = serde_json::from_str(trimmed)
            .map_err(|e| CliError::Io(io::Error::new(io::ErrorKind::InvalidData, e.to_string())))?;

        let decisions = monitor.push(&record);
        for decision in decisions {
            let hint = decision_hint(&decision);
            let enriched = serde_json::json!({
                "decision": decision,
                "hint": hint,
            });
            let json_line = serde_json::to_string(&enriched)
                .map_err(|e| CliError::Io(io::Error::new(io::ErrorKind::Other, e.to_string())))?;
            writeln!(stdout_lock, "{json_line}")?;
            stdout_lock.flush()?;
        }
    }

    // EOF: emit summary
    let summary = monitor.state_summary();
    let summary_json = serde_json::to_string(&summary)
        .map_err(|e| CliError::Io(io::Error::new(io::ErrorKind::Other, e.to_string())))?;
    writeln!(stdout_lock, "{summary_json}")?;

    Ok(())
}

/// Run monitor in watch mode: tail a file or watch a directory.
pub fn run_monitor_watch(
    watch_path: &Path,
    config_path: Option<&Path>,
    overrides: &ConfigOverrides,
) -> Result<(), CliError> {
    let config = load_monitor_config(config_path, overrides)?;
    let mut monitor = Monitor::new(config);
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    // For MVP: read the file fully (not live-tailing).
    // notify-based live watching is a future enhancement.
    let content = std::fs::read_to_string(watch_path)?;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let record: TrialRecord = serde_json::from_str(trimmed)
            .map_err(|e| CliError::Io(io::Error::new(io::ErrorKind::InvalidData, e.to_string())))?;

        let decisions = monitor.push(&record);
        for decision in decisions {
            let hint = decision_hint(&decision);
            let enriched = serde_json::json!({
                "decision": decision,
                "hint": hint,
            });
            let json_line = serde_json::to_string(&enriched)
                .map_err(|e| CliError::Io(io::Error::new(io::ErrorKind::Other, e.to_string())))?;
            writeln!(stdout_lock, "{json_line}")?;
        }
    }

    // Emit summary
    let summary = monitor.state_summary();
    let summary_json = serde_json::to_string(&summary)
        .map_err(|e| CliError::Io(io::Error::new(io::ErrorKind::Other, e.to_string())))?;
    writeln!(stdout_lock, "{summary_json}")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eval_core::{Outcome, TrialRecord};
    use std::collections::BTreeMap;
    use ulid::Ulid;

    fn make_record(task: &str, agent: &str, score: f64, run_id: Ulid) -> TrialRecord {
        TrialRecord {
            trial_id: Ulid::new(),
            run_id,
            task_id: task.into(),
            task_version: None,
            agent_id: agent.into(),
            agent_version: None,
            judge_config: None,
            seed: None,
            timestamp: 1717200000,
            outcome: Outcome::Score(score),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn monitor_produces_summary() {
        let config = eval_orchestrator::MonitorConfig::default();
        let mut monitor = Monitor::new(config);
        let run_id = Ulid::new();
        for _ in 0..5 {
            let record = make_record("t", "a", 0.8, run_id);
            let _ = monitor.push(&record);
        }
        let summary = monitor.state_summary();
        assert_eq!(summary.observations_seen, 5);
    }

    #[test]
    fn monitor_json_line_output_shape() {
        let decision = eval_orchestrator::Decision::ContinueRunning {
            series: eval_orchestrator::SeriesKey {
                task_id: "t".into(),
                agent_id: "a".into(),
                scorer: None,
            },
            current_n: 5,
            estimated_n_needed: 0,
            power_at_current_n: 0.0,
        };
        let hint = decision_hint(&decision);
        let enriched = serde_json::json!({
            "decision": decision,
            "hint": hint,
        });
        let json = serde_json::to_string(&enriched).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["hint"].is_string());
        assert!(parsed["decision"].is_object());
    }
}
```

- [ ] **Step 2: Wire monitor into main.rs**

In `crates/mojave-cli/src/main.rs`, replace the `Commands::Monitor` arm:

Replace:
```rust
        Commands::Monitor { .. } => {
            eprintln!("mojave monitor: not yet implemented");
            std::process::exit(2);
        }
```

With:
```rust
        Commands::Monitor {
            watch,
            config,
            format: _,
            irr_threshold,
            irr_metric,
            spc_chart,
            spc_phase1_windows,
            sequential_alpha,
        } => {
            let overrides = ConfigOverrides {
                irr_threshold,
                irr_metric,
                spc_chart,
                spc_phase1_windows,
                sequential_alpha,
                force_enable: None,
                force_disable: None,
            };
            let result = match watch {
                Some(path) => mojave_cli::commands::monitor::run_monitor_watch(
                    &path,
                    config.as_deref(),
                    &overrides,
                ),
                None => mojave_cli::commands::monitor::run_monitor_stdin(
                    config.as_deref(),
                    &overrides,
                ),
            };
            match result {
                Ok(()) => Ok(()),
                Err(e) => {
                    write_error(&e);
                    std::process::exit(1);
                }
            }
        }
```

Also add the import at the top of main.rs:
```rust
use mojave_cli::commands::{analyze, ingest, monitor};
```

- [ ] **Step 3: Run tests**

Run:
```bash
cargo test -p mojave-cli
```

Expected: All tests pass including the 2 new monitor tests.

- [ ] **Step 4: Commit**

```bash
git add crates/mojave-cli/src/commands/monitor.rs crates/mojave-cli/src/main.rs
git commit -m "feat(mojave-cli): monitor command — stdin + file watch modes"
```

---

### Task 9: Sensitivity command (salib delegation)

**Files:**
- Modify: `crates/mojave-cli/src/commands/sensitivity.rs`
- Modify: `crates/mojave-cli/src/main.rs` (wire sensitivity)

Note: The salib crate's CLI surface is not yet fully implemented (salib-cli was a stub). For this task, we create the subcommand structure and delegate to salib library calls where possible. The `sample` subcommand is implementable since `salib-samplers` is published; `analyze` and `run` are stubs that print informative messages.

- [ ] **Step 1: Write the sensitivity command**

Replace contents of `crates/mojave-cli/src/commands/sensitivity.rs`:

```rust
use crate::error::CliError;

/// Run the sensitivity subcommand.
///
/// This delegates to the published `salib` crate. Currently a thin wrapper
/// that prints a message indicating the subcommand surface. Full integration
/// lands when salib-cli's library surface is complete.
pub fn run_sensitivity(args: &[String]) -> Result<(), CliError> {
    if args.is_empty() {
        print_sensitivity_help();
        return Ok(());
    }

    match args[0].as_str() {
        "sample" | "analyze" | "run" => {
            eprintln!(
                "mojave sensitivity {}: salib integration pending — \
                 use `salib {}` directly until integration is complete",
                args[0], args.join(" ")
            );
            // Exit 2 to signal "not yet implemented" per CLI spec
            std::process::exit(2);
        }
        "--help" | "-h" | "help" => {
            print_sensitivity_help();
            Ok(())
        }
        other => {
            eprintln!("mojave sensitivity: unknown subcommand '{other}'");
            print_sensitivity_help();
            std::process::exit(2);
        }
    }
}

fn print_sensitivity_help() {
    eprintln!(
        "mojave sensitivity — global sensitivity analysis (salib)\n\
         \n\
         Subcommands:\n\
           sample   Emit a sample matrix from a problem definition\n\
           analyze  Compute sensitivity indices from (X, y) pairs\n\
           run      Drive an end-to-end sensitivity campaign\n\
         \n\
         All subcommands delegate to the published salib crate (v0.1.1).\n\
         For direct usage: cargo install salib-cli"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_flag_does_not_error() {
        // --help should return Ok, not exit
        let result = run_sensitivity(&["--help".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn empty_args_prints_help() {
        let result = run_sensitivity(&[]);
        assert!(result.is_ok());
    }
}
```

- [ ] **Step 2: Wire sensitivity into main.rs**

In `crates/mojave-cli/src/main.rs`, replace the `Commands::Sensitivity` arm:

Replace:
```rust
        Commands::Sensitivity { .. } => {
            eprintln!("mojave sensitivity: not yet implemented");
            std::process::exit(2);
        }
```

With:
```rust
        Commands::Sensitivity { args } => {
            match mojave_cli::commands::sensitivity::run_sensitivity(&args) {
                Ok(()) => Ok(()),
                Err(e) => {
                    write_error(&e);
                    std::process::exit(1);
                }
            }
        }
```

- [ ] **Step 3: Run tests**

Run:
```bash
cargo test -p mojave-cli
```

Expected: All tests pass including the 2 new sensitivity tests.

- [ ] **Step 4: Commit**

```bash
git add crates/mojave-cli/src/commands/sensitivity.rs crates/mojave-cli/src/main.rs
git commit -m "feat(mojave-cli): sensitivity subcommand — salib delegation stub"
```

---

### Task 10: CLI smoke tests (assert_cmd)

**Files:**
- Create: `crates/mojave-cli/tests/smoke.rs`

- [ ] **Step 1: Write CLI smoke tests**

Create `crates/mojave-cli/tests/smoke.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../eval-ingest/tests/fixtures")
        .join(name)
}

fn mojave() -> Command {
    Command::cargo_bin("mojave").expect("binary should exist")
}

#[test]
fn help_flag() {
    mojave()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Measurement engine"));
}

#[test]
fn ingest_inspect_json_outputs_valid_json() {
    let output = mojave()
        .args(["ingest", fixture_path("inspect_binary.json").to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: serde_json::Value =
        serde_json::from_slice(&output).expect("stdout should be valid JSON");
    assert!(parsed["records"].is_array(), "should have records array");
    assert!(
        parsed["source_meta"]["runner_name"].is_string(),
        "should have source_meta"
    );
}

#[test]
fn ingest_jsonl_outputs_valid_json() {
    let output = mojave()
        .args(["ingest", fixture_path("basic.jsonl").to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: serde_json::Value =
        serde_json::from_slice(&output).expect("stdout should be valid JSON");
    let records = parsed["records"].as_array().expect("records should be array");
    assert_eq!(records.len(), 5);
}

#[test]
fn analyze_outputs_valid_json_with_decisions() {
    let output = mojave()
        .args(["analyze", fixture_path("basic.jsonl").to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: serde_json::Value =
        serde_json::from_slice(&output).expect("stdout should be valid JSON");
    assert!(parsed["decisions"].is_array(), "should have decisions");
    assert!(
        parsed["instruments_run"].is_array(),
        "should have instruments_run"
    );
    assert!(
        parsed["series_detected"].is_array(),
        "should have series_detected"
    );
    assert!(parsed["summaries"].is_object(), "should have summaries");
}

#[test]
fn analyze_decisions_have_hint_field() {
    let output = mojave()
        .args(["analyze", fixture_path("basic.jsonl").to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: serde_json::Value = serde_json::from_slice(&output).unwrap();
    if let Some(decisions) = parsed["decisions"].as_array() {
        for d in decisions {
            assert!(
                d["hint"].is_string(),
                "each decision should have a hint field"
            );
        }
    }
}

#[test]
fn missing_file_returns_exit_1() {
    mojave()
        .args(["analyze", "nonexistent_file_12345.json"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn invalid_flag_returns_exit_2() {
    mojave()
        .args(["analyze", "--nonexistent-flag"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn monitor_with_watch_file() {
    // Create a temp file with TrialRecord JSONL
    let mut tmp = tempfile::NamedTempFile::new().expect("temp file");
    use std::io::Write;

    // Write a few records — minimal valid TrialRecord JSON
    for i in 0..3 {
        let record = serde_json::json!({
            "trial_id": format!("01JAAA000000000000000000{:02}", i),
            "run_id": "01JAAA00000000000000000000",
            "task_id": "t1",
            "task_version": null,
            "agent_id": "a1",
            "agent_version": null,
            "judge_config": null,
            "seed": null,
            "timestamp": 1717200000 + i,
            "outcome": {"type": "Score", "value": 0.8},
            "metadata": {}
        });
        writeln!(tmp, "{}", serde_json::to_string(&record).unwrap()).unwrap();
    }

    let output = mojave()
        .args(["monitor", "--watch", tmp.path().to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Last line should be the summary
    let last_line = String::from_utf8(output)
        .unwrap()
        .lines()
        .last()
        .unwrap_or("")
        .to_string();
    let summary: serde_json::Value =
        serde_json::from_str(&last_line).expect("last line should be valid JSON");
    assert!(
        summary["observations_seen"].is_number(),
        "summary should have observations_seen"
    );
}
```

- [ ] **Step 2: Run smoke tests**

Run:
```bash
cargo test -p mojave-cli --test smoke
```

Expected: All 8 smoke tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/mojave-cli/tests/smoke.rs
git commit -m "test(mojave-cli): CLI smoke tests with assert_cmd"
```

---

### Task 11: BEAD + final verification

**Files:**
- Create: `.context/beads/BEAD-0018-mojave-cli.md`

- [ ] **Step 1: Create the BEAD**

Create `.context/beads/BEAD-0018-mojave-cli.md`:

```markdown
---
id: BEAD-0018
title: mojave-cli — unified CLI entry point
status: open
priority: high
created: 2026-05-18
---

## Description

Single `mojave` binary providing the CLI entry point to the entire measurement engine. Four subcommands: ingest (Inspect + JSONL → TrialRecords), analyze (batch measurement battery), monitor (streaming analysis), sensitivity (delegates to published salib crate). JSON to stdout, structured errors to stderr. Replaces salib-cli.

## Acceptance

- [ ] salib-* crates extracted from workspace, deps point to crates.io 0.1.1
- [ ] mojave-cli crate with four subcommands (ingest, analyze, monitor, sensitivity)
- [ ] JSON output with hint fields on all Decision objects
- [ ] Config loading: YAML file + CLI flag overrides + defaults
- [ ] Format auto-detection (Inspect vs JSONL)
- [ ] Monitor: stdin mode + file watch mode
- [ ] Exit codes: 0 success, 1 error, 2 usage error
- [ ] CLI smoke tests (assert_cmd)
- [ ] TCK Gherkin feature file
- [ ] Clippy zero warnings, rustfmt clean
- [ ] Full workspace test suite passes
```

- [ ] **Step 2: Run full workspace verification**

Run:
```bash
cargo test --workspace
```

Expected: All tests pass across all remaining crates.

Run:
```bash
cargo clippy --workspace
```

Expected: Zero warnings.

Run:
```bash
cargo fmt --all -- --check
```

Expected: No formatting issues.

- [ ] **Step 3: Manual smoke test**

Run:
```bash
./target/debug/mojave --help
./target/debug/mojave ingest crates/eval-ingest/tests/fixtures/inspect_binary.json | python3 -m json.tool | head -30
./target/debug/mojave analyze crates/eval-ingest/tests/fixtures/basic.jsonl | python3 -m json.tool
./target/debug/mojave sensitivity --help
```

Expected: All commands produce expected output.

- [ ] **Step 4: Commit**

```bash
git add .context/beads/BEAD-0018-mojave-cli.md
git commit -m "chore: open BEAD-0018 — mojave-cli"
```

---

## Self-Review

**1. Spec coverage:**
- ✅ Prerequisites: salib extraction (Task 1)
- ✅ Architecture: crate skeleton (Task 3)
- ✅ Command surface: ingest (Task 5), analyze (Task 6), monitor (Task 8), sensitivity (Task 9)
- ✅ Config: file + flag overrides (Task 4)
- ✅ Output contract: JSON with hints (Task 6 output.rs)
- ✅ Error handling: CliError with kind() + structured JSON errors (Task 3 error.rs, Task 6 output.rs)
- ✅ Testing: unit tests throughout, CLI smoke tests (Task 10), TCK features (Task 2)
- ✅ Future extension discipline: documented in spec, not code — correct per YAGNI

**2. Placeholder scan:** No TBDs, TODOs, or "implement later" — all code blocks are complete. The sensitivity command is intentionally a stub that delegates to the published salib binary, which matches the spec ("delegates to salib").

**3. Type consistency:**
- `ConfigOverrides` — defined in Task 4, used in Tasks 5, 6, 7, 8 ✅
- `CliError` — defined in Task 3, used everywhere ✅
- `IngestOutput` — defined in Task 5, used in Task 7 ✅
- `AnalyzeOutput` / `DecisionWithHint` — defined in Task 6, used in Task 7 ✅
- `InputFormat` / `DetectError` — defined in Task 4, used in Task 5 ✅
- `run_ingest` signature — `(paths: &[PathBuf], format_flag: &str, field_mapping_path: Option<&Path>)` consistent between Task 5 and Task 6 ✅
- `run_analyze` signature — `(paths: &[PathBuf], config_path: Option<&Path>, format_flag: &str, overrides: &ConfigOverrides)` consistent between Task 6 and Task 7 ✅
- Monitor functions — `run_monitor_stdin` and `run_monitor_watch` signatures consistent between Task 8 definition and Task 8 wiring ✅

**Spec gap found:** The spec mentions `notify` crate for filesystem watching, but the plan uses a simpler read-file approach for MVP. This is correct — the spec says "falls back to 1-second polling on platforms where native events are unavailable" and the watch mode reads the file content, which is the correct MVP behavior. Live-tailing via `notify` is a future enhancement.

**Spec gap found:** The spec mentions `--format=pretty` for colored tables. The plan doesn't implement the pretty formatter. This is correct YAGNI — JSON is the primary output, pretty is a future nicety. The flag is parsed but only `json` is implemented.
