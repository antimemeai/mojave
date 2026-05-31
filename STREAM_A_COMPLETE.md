# Stream A: Statistical Correctness -- COMPLETE

**Branch:** `stream-a/statistical-correctness`
**Date:** 2026-05-30
**Peer:** A (Statistical Correctness)

## Summary

All Tier 0 blocking tasks (A1-A4) and the Tier 2 convergence diagnostics task (A5) are complete, tested, and committed. The confidence sequence pipeline now produces valid coverage for Bernoulli data.

## Tasks Completed

### A1: Fix AnytimeMonitor sigma for Bernoulli (Tier 0 -- BLOCKING)

**Commits:** `a90e3cb`, `5cf66d8`

**Bug:** AnytimeMonitor computed sigma via Welford's online variance unconditionally, voiding the anytime-valid coverage guarantee for Bernoulli data (46% coverage instead of 95%).

**Fix:** AnytimeMonitor now dispatches on `DataFamily`:
- `Bernoulli` -> sigma = 0.5 (conservative upper bound, max std dev for [0,1]-bounded data)
- `Normal { known_variance: Some(v) }` -> sigma = sqrt(v)
- `Normal { known_variance: None }` -> Welford online estimate (existing behavior)

**Files modified:**
- `crates/seq-anytime-valid/src/monitor/anytime.rs` -- DataFamily dispatch in `update()`
- `crates/seq-anytime-valid/tests/anytime_data_family.rs` -- 4 tests
- `tck/seq-anytime-valid/features/anytime_data_family.feature` -- TCK spec

**Tests:** 4 passing (bernoulli_uses_fixed_sigma, bernoulli_ci_width_is_deterministic, normal_known_variance_uses_specified_sigma, normal_unknown_variance_uses_welford)

### A2: Fix SequentialInstrument DataFamily (Tier 0 -- BLOCKING)

**Commit:** `0272512`

**Bug:** SequentialInstrument hardcoded `DataFamily::Normal { known_variance: None }` for all data, including binary MCQ outcomes.

**Fix:** Added `infer_data_family()` function that inspects `TrialRecord` outcomes:
- `Outcome::Binary` -> `DataFamily::Bernoulli`
- All others -> `DataFamily::Normal { known_variance: None }`

**Files modified:**
- `crates/eval-orchestrator/src/instruments/sequential.rs`
- `tck/eval-orchestrator/features/sequential_data_family.feature`

**Tests:** 2 new tests (infer_data_family_selects_bernoulli_for_binary, binary_outcomes_use_bernoulli_family), all 64 eval-orchestrator tests passing

### A3: Gate 4 Monte Carlo test for production path (Tier 0 -- BLOCKING)

**Commit:** `8719092`

**What:** The existing Gate 4 test (`gate4_monte_carlo.rs`) tested `normal_mixture_cs_known_sigma` -- NOT the production `AnytimeMonitor::update()` path. Added a test that feeds Bernoulli(p) data through AnytimeMonitor and verifies coverage.

**Files created:**
- `crates/seq-anytime-valid/tests/gate4_anytime_monitor.rs`

**Tests:** 2 tests passing:
- `anytime_monitor_bernoulli_coverage_gate4` -- 10,000 reps at p in {0.1, 0.3, 0.5, 0.7, 0.9}, N=200, coverage >= 93% at all p values
- `anytime_monitor_bernoulli_ci_shrinks` -- CI width decreases with N

### A4: Data quality gate for n_samples=0 (Tier 0 -- BLOCKING)

**Commit:** `10aaa28`

**What:** 20 cells in WMDP bio had n_samples=0, corrupting variance decomposition. Added validation that rejects cells with n_samples=0 before Sobol estimation.

**Files modified:**
- `crates/mojave-gsa/src/analyze.rs` -- n_samples field + validation gate
- `tck/mojave-gsa/features/data_quality_gate.feature`

**Tests:** 2 tests (test_zero_n_samples_rejected, test_n_samples_absent_passes)

### A5: Sobol convergence diagnostics (Tier 2)

**Commit:** `5dca454`

**What:** Added convergence diagnostics that warn on:
- Negative S1 (insufficient N or model misspecification)
- CI crossing [0,1] boundary (sign uncertainty)
- CI width exceeding 10% of point estimate
- sum(ST) > 1.3 (substantial factor interactions or insufficient N)
- Automatic "recommend doubling N_base" when convergence issues detected

**Files created:**
- `crates/mojave-gsa/src/diagnostics.rs` -- SobolDiagnosticEntry, DiagnosticKind, run_diagnostics()
- `tck/mojave-gsa/features/convergence_diagnostics.feature`

**Files modified:**
- `crates/mojave-gsa/src/analyze.rs` -- wired diagnostics into output JSON and stderr
- `crates/mojave-gsa/src/main.rs` -- module declaration

**Tests:** 5 tests (negative_s1, ci_crossing_zero, sum_st_threshold, ci_width_threshold, clean_results)

## Test Results

| Crate | Tests | Status |
|-------|-------|--------|
| seq-anytime-valid (anytime_data_family) | 4 | All passing |
| seq-anytime-valid (gate4_anytime_monitor) | 2 | All passing |
| eval-orchestrator (all) | 64 | All passing |
| mojave-gsa (all) | 33 | All passing |

**Clippy:** Zero warnings across all three owned crates
**Rustfmt:** Clean

## Task A6: Waudby-Smith Betting CS (NOT STARTED)

A6 requires full literature review of Waudby-Smith & Ramdas 2024 and implementation of the hedged capital confidence sequence (ONS bet sizing, wealth process with LBOW tracking, CI inversion via bisection). This is a substantial effort beyond the scope of the current session. The sigma=0.5 conservative fix from A1 provides correct (if conservative) coverage in the meantime.

## Unblocking Signal

Streams C (QMU) and D (WMDP rerun) are now unblocked:
- CS pipeline produces valid coverage for Bernoulli data
- Data quality gate rejects n_samples=0 cells
- Gate 4 Monte Carlo calibration confirms >= 93% coverage at all tested p values
- Convergence diagnostics automate the "double N" decision
