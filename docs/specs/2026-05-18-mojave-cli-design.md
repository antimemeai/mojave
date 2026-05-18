# mojave-cli Design Spec

## Goal

Single binary (`mojave`) that is the command-line entry point to the entire measurement engine. Wires eval-ingest → eval-orchestrator into three working subcommands (ingest, analyze, monitor), plus a sensitivity subcommand that delegates to the published `salib` crate. JSON to stdout, always machine-readable. Future BEADs extend the CLI as part of their acceptance criteria.

## Prerequisites: salib extraction

The 7 local `salib-*` crates and `salib-cli` must be removed from the workspace before building mojave-cli. They are already published on crates.io under `antimemeai` at version 0.1.1:

- `salib` (umbrella — re-exports core, samplers, estimators, validation, shapley, surrogate)
- `salib-cli` (published standalone binary)

**Actions:**

1. Delete `crates/salib-core`, `crates/salib-samplers`, `crates/salib-estimators`, `crates/salib-validation`, `crates/salib-shapley`, `crates/salib-surrogate`, `crates/salib-cli` from the workspace.
2. Remove all 7 from `Cargo.toml` workspace members.
3. Update `crates/spc-charts/Cargo.toml`: change the optional `salib-estimators` dep from `path = "../salib-estimators"` to `version = "0.1.1"`.
4. Delete `crates/metric-tck-harness` if it is only used by salib crates (verify first — seq-anytime-valid also uses it as a dev-dependency; if so, point that at a published version or vendor it).
5. Verify `cargo test --workspace` still passes.

**Post-extraction workspace:**

```
crates/
  eval-core/
  eval-ingest/
  eval-orchestrator/
  irr/
  seq-anytime-valid/
  spc-charts/
  metric-tck-harness/   # kept if still needed by non-salib crates
  mojave-cli/           # NEW
```

## Architecture

### Crate: `mojave-cli`

Thin binary (`src/main.rs`) + library (`src/lib.rs`). The binary does clap parsing and calls library functions. Library exposes command logic so integration tests can call it without spawning a process.

**Binary name:** `mojave`

**Dependencies:**

| Dep | Source | Purpose |
|-----|--------|---------|
| `eval-ingest` | workspace path | Ingest adapters |
| `eval-orchestrator` | workspace path | Batch analyze + streaming Monitor |
| `salib` | crates.io 0.1.1 | Sensitivity analysis (umbrella) |
| `clap` | crates.io | Arg parsing with derive |
| `serde` | crates.io | Config + output serialization |
| `serde_json` | crates.io | JSON output |
| `serde_yaml` | crates.io | Config file parsing |
| `tracing` | crates.io | Structured logging |
| `tracing-subscriber` | crates.io | stderr log output |

**Dev-dependencies:**

| Dep | Purpose |
|-----|---------|
| `assert_cmd` | CLI smoke tests (spawn binary, check stdout/exit code) |
| `predicates` | Assertion helpers for assert_cmd |
| `tempfile` | Temp dirs for test fixtures |

### Module structure

```
src/
  main.rs          # clap dispatch only
  lib.rs           # re-exports command modules
  config.rs        # config file loading + CLI flag merge
  format.rs        # JSON output + hint generation + optional pretty
  commands/
    mod.rs
    ingest.rs      # mojave ingest
    analyze.rs     # mojave analyze
    monitor.rs     # mojave monitor
    sensitivity.rs # mojave sensitivity (delegates to salib)
```

## Command Surface

### `mojave ingest <path>...`

Ingests eval runner output, normalizes to TrialRecords, writes JSON array to stdout.

**Arguments:**

| Arg | Type | Default | Description |
|-----|------|---------|-------------|
| `<path>...` | positional, required | — | One or more files or directories |
| `--format` | `auto\|inspect\|jsonl` | `auto` | Input format; auto-detect examines file structure |
| `--field-mapping` | path to YAML | — | Custom field mapping for JSONL adapter |

**Output (stdout):** JSON array of TrialRecord objects, plus `_meta` with source provenance:

```json
{
  "records": [ ... ],
  "warnings": [
    {"kind": "missing_field", "message": "...", "source": "file.jsonl:42"}
  ],
  "source_meta": {
    "runner_name": "inspect_ai",
    "content_hash": "sha256:...",
    "original_path": "logs/eval_001.json"
  }
}
```

**Exit codes:** 0 success, 1 ingest error, 2 usage error.

### `mojave analyze <path>...`

Ingests input (auto-detect), runs the full measurement battery, outputs analysis report as JSON.

**Arguments:**

| Arg | Type | Default | Description |
|-----|------|---------|-------------|
| `<path>...` | positional, required | — | Files/directories (Inspect, JSONL, or pre-ingested TrialRecord JSON) |
| `--config` | path to YAML | — | Full config override file |
| `--format` | `json\|pretty` | `json` | Output format |
| `--irr-threshold` | f64 | 0.67 | IRR agreement threshold |
| `--irr-metric` | `krippendorff\|fleiss\|gwet` | `krippendorff` | IRR metric |
| `--spc-chart` | `ewma\|cusum\|shewhart\|combined` | `ewma` | SPC chart type |
| `--spc-phase1-windows` | usize | 20 | SPC phase 1 calibration windows |
| `--sequential-alpha` | f64 | 0.05 | Sequential testing significance level |
| `--force-enable` | comma-separated list | — | Force-enable instruments (irr, sequential, spc) |
| `--force-disable` | comma-separated list | — | Force-disable instruments |

**Config resolution:** CLI flags > config file > AnalysisConfig::default().

**Output (stdout):** AnalysisReport JSON with `hint` fields on decisions:

```json
{
  "series_detected": [
    {"task_id": "arc-challenge", "agent_id": "gpt-4.1", "scorer": null}
  ],
  "instruments_run": ["irr", "sequential", "spc"],
  "decisions": [
    {
      "type": "stop_early",
      "series": {"task_id": "arc-challenge", "agent_id": "gpt-4.1"},
      "evidence": 47.2,
      "estimate": 0.82,
      "ci": [0.79, 0.85],
      "hint": "Effect stable at 0.82 ± 0.03. Evidence (47.2) exceeds threshold — safe to stop."
    },
    {
      "type": "regression",
      "series": {"task_id": "humaneval", "agent_id": "gpt-4.1"},
      "signal": {"variant": "below_lower", "statistic": 0.43},
      "observation_value": 0.43,
      "control_limits": [0.71, 0.89],
      "hint": "Observation 0.43 outside control limits [0.71, 0.89]."
    },
    {
      "type": "measurement_warning",
      "series": {"task_id": "mmlu", "agent_id": "claude-4"},
      "issue": {"kind": "low_agreement", "kappa": 0.31, "threshold": 0.67},
      "hint": "Inter-rater agreement (κ=0.31) below threshold (0.67)."
    }
  ],
  "summaries": {
    "irr": {
      "series": {"task_id": "arc-challenge", "agent_id": "gpt-4.1"},
      "alpha": 0.92,
      "n_raters": 3,
      "n_items": 50,
      "metric": "krippendorff_alpha"
    },
    "sequential": [ ... ],
    "spc": [ ... ]
  }
}
```

**Exit codes:** 0 success, 1 analysis error, 2 usage error.

### `mojave monitor`

Streaming analysis. Reads TrialRecords incrementally, emits Decision JSON lines to stdout as they fire.

**Arguments:**

| Arg | Type | Default | Description |
|-----|------|---------|-------------|
| `--watch` | path | — | File or directory to poll; omit for stdin mode |
| `--config` | path to YAML | — | Full config override |
| `--format` | `json\|pretty` | `json` | Output format |
| Same `--irr-*`, `--spc-*`, `--sequential-*` flags as analyze | | | |

**Stdin mode (default):** Reads newline-delimited TrialRecord JSON from stdin. Each line parsed, pushed into Monitor, decisions emitted immediately.

**Watch mode (`--watch=<path>`):** Uses the `notify` crate for OS-level filesystem events (inotify on Linux, FSEvents on macOS, ReadDirectoryChanges on Windows). For files, reacts to append events and reads new lines. For directories, watches for new files matching `*.json` or `*.jsonl`. Falls back to 1-second polling on platforms where native events are unavailable.

**Output (stdout):** One JSON object per line, per decision, as they fire:

```
{"type":"continue_running","series":{...},"current_n":12,"hint":"12 observations, insufficient evidence."}
{"type":"stop_early","series":{...},"evidence":23.1,"estimate":0.81,"ci":[0.78,0.84],"hint":"Safe to stop."}
```

**Graceful shutdown:** On EOF (stdin) or SIGINT, emit a final MonitorSummary JSON object with active series state, then exit 0.

**Exit codes:** 0 clean shutdown, 1 error, 2 usage error.

### `mojave sensitivity <subcommand>`

Calls the published `salib` crate as a library (not shelling out to the salib binary). Three subcommands matching the salib-cli surface:

- `mojave sensitivity sample <problem.yaml> --sampler=<sobol|lhs|saltelli|morris|fast> --n=<N> --seed=<s>` — emit sample matrix as JSON.
- `mojave sensitivity analyze <samples> <outputs> --estimator=<saltelli2010|jansen|janon|owen|morris|...>` — compute sensitivity indices, output JSON.
- `mojave sensitivity run <experiment.yaml>` — drive end-to-end campaign.

Output format matches the rest of mojave: JSON to stdout.

## Output Contract

- **stdout:** Always machine-readable JSON. Default format for all commands.
- **stderr:** Tracing logs (when `--verbose` is set). Structured JSON error on failure: `{"error": "message", "kind": "ingest_error|orchestrator_error|config_error"}`.
- **`--format=pretty`:** Optional colored, human-readable tables and prose rendered from the same data. Informational — not a different data format, just a presentation layer.
- **`hint` field:** Present on every Decision object. One-sentence human-readable summary of what the decision means. Always accompanies the raw data, never replaces it.

## Config File Format

YAML, matching the `AnalysisConfig` structure:

```yaml
# mojave.yaml
irr:
  threshold: 0.67
  metric: krippendorff    # krippendorff | fleiss | gwet
  min_raters: 2

sequential:
  alpha: 0.05
  method: msprt
  mixing_variance: 1.0

spc:
  chart_type: ewma        # ewma | cusum | shewhart | combined
  phase1_windows: 20
  window_size: per_run    # per_run | fixed:<n>
  lambda: 0.2
  l_sigma: 2.962

monitor:
  irr_recompute_interval: 50
  auto_detect: true

force_enable: []           # irr, sequential, spc
force_disable: []
```

All fields optional — omitted fields use `AnalysisConfig::default()`. CLI flags override individual fields after file loading.

## Error Handling

All errors are thiserror-derived and caught at the top level in `main.rs`.

| Error source | Type | CLI behavior |
|---|---|---|
| Bad CLI args | clap | stderr message, exit 2 |
| Config file parse failure | `ConfigError` (new) | JSON error to stderr, exit 1 |
| Ingest failure | `IngestError` | JSON error to stderr, exit 1 |
| Orchestrator failure | `OrchestratorError` | JSON error to stderr, exit 1 |
| IO (file not found, stdin broken) | `std::io::Error` | JSON error to stderr, exit 1 |

No panics. `#![forbid(unsafe_code)]` on the crate. Workspace clippy lints (`unwrap_used = "deny"`, `expect_used = "deny"`) enforced.

## Testing

### Unit tests

- Config resolution: flags override file, file overrides defaults, missing fields fall through
- Format auto-detection: Inspect JSON vs JSONL vs pre-ingested TrialRecord JSON
- Hint generation: one test per Decision variant, verifying the hint string is present and contains key values

### Integration tests (library calls)

- Full pipeline: fixture file → ingest → analyze → assert JSON output shape and decision types
- Monitor pipeline: feed records one-at-a-time through library, assert decisions fire at correct points
- Config file: load YAML, verify merged config matches expectations
- Error paths: bad input file, malformed config, empty records

### CLI smoke tests (assert_cmd)

- `mojave ingest fixtures/inspect.json` → exit 0, stdout is valid JSON with `records` array
- `mojave analyze fixtures/inspect.json` → exit 0, stdout is valid JSON with `decisions` array
- `echo '...' | mojave monitor` → exit 0, stdout lines are valid JSON
- `mojave analyze --nonexistent-flag` → exit 2
- `mojave analyze missing_file.json` → exit 1, stderr has JSON error

### Not tested here

Math correctness (Gate 1–4) lives in irr, seq-anytime-valid, spc-charts. The CLI tests verify wiring and output shape, not statistical properties.

## Future Extension Discipline

Each future BEAD that adds capabilities must include in its acceptance criteria:

- [ ] Add/update mojave CLI subcommand or flags
- [ ] CLI integration tests for new surface
- [ ] Update CLI help text
- [ ] Update this spec or supersede with a new version

Expected extensions:

| BEAD | CLI surface |
|------|-------------|
| 0009 (audit chain) | `mojave verify <corpus>`, `--sealed` flag on analyze/monitor |
| 0012 (perturbation) | `mojave ablate <factor-spec>` subcommand |
| 0014 (git integration) | `--git-repo=<path>` flag on analyze/monitor |
| 0005/0006/0007 (IRT, factors, CAT) | Additional fields in analyze output, new summary sections |
| 0011 (construct validity) | `mojave report <run-id>` subcommand |
