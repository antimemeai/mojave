# Advisory Execution Plan — mojave 2026-05-30

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Execute all actionable findings from the 2026-05-30 advisory board across Tiers 0–3, fixing load-bearing statistical defects, hardening the audit chain, building the QMU decision framework, completing the measurement qualification stack, and rerunning WMDP with corrected methodology.

**Architecture:** Five parallel work streams assigned to peer claudes, coordinated through a control channel. Streams are ordered by dependency: Stream A (statistical correctness) unblocks Streams C and D; Stream B (audit chain) is fully independent; Stream E (IRR/measurement qualification) is independent until it feeds into Stream C's QMU struct. Each stream follows JSMNTL: literature review → TCK red → compile/run red → implement → green → code review.

**Tech Stack:** Rust 1.87+ (workspace crates), Python 3.11+ (scripts/v2), salib-rs (../salib), LaTeX (run card templates), Sigstore cosign (binary signing), GitHub Actions (CI/CD)

**Advisory Source:** `outposts/advisory-2026-05-30/` — MANIFEST.md, 5 wave-1 reports, 6 wave-2 deep dives, 6 deliverables

---

## Work Stream Map

| Stream | Peer | Scope | Repos | Tiers | Unblocks |
|--------|------|-------|-------|-------|----------|
| **A: Statistical Correctness** | Peer 1 | CS pipeline fix, data quality gate, Gate 4 production test, betting CS, convergence diagnostics | mojave | 0, 2 | C (QMU needs valid CIs), D (WMDP rerun needs valid pipeline) |
| **B: Audit Chain Trust** | Peer 2 | Retire Python writer, Sigstore signing, canonical encoding spec, CI pipeline, Rekor witnessing | mojave | 0, 1, 2, 3 | None (fully independent) |
| **C: QMU + Defense Framework** | Peer 3 | QmuAssessment struct, JCGM 106 guard bands, NIST 800-3 run card section, GSN template | mojave | 1, 3 | Blocked by A (needs valid CS pipeline) |
| **D: GSA + G-Theory + WMDP** | Peer 4 | Sobol code dedup, G-theory Gate 1 validation + D-study optimizer, S2 indices, WMDP N=1024, bare-prompt analysis | mojave + ../salib | 1, 2, 3 | Blocked by A (WMDP rerun needs valid pipeline) |
| **E: IRR + Measurement Qualification** | Peer 5 | Bootstrap CIs wired to IRR, MSA ndc/P-T, Mandel h/k, ISO 5725 R&R | mojave | 2, 3 | None (independent, feeds into C later) |

```
Stream A ──────┬──► Stream C (QMU)
               │
               └──► Stream D (WMDP rerun)
Stream B ──────────  (independent)
Stream E ──────────  (independent, late feed → C)
```

---

## Stream A: Statistical Correctness (Peer 1)

**Lit review:** Waudby-Smith & Ramdas 2024 (betting CS), Howard et al. 2021 (normal mixture CS), Koning 2025 (sequentialized tests), Johari 2022 (Beta-mixture mSPRT). Check `../evals_papers/` for these.

**Cites:** wave_2/02_statistical_correctness.md, wave_1/adversary.md findings 1, 2, 3, 6

### Task A1: Fix AnytimeMonitor sigma for Bernoulli data (Tier 0 — BLOCKING)

**What:** AnytimeMonitor uses Welford's estimated sigma, voiding the anytime-valid guarantee. For Bernoulli data, sigma=0.5 is the conservative known upper bound. AnytimeMonitor must dispatch on DataFamily.

**Files:**
- Modify: `crates/seq-anytime-valid/src/monitor/anytime.rs`
- Modify: `crates/seq-anytime-valid/src/types.rs` (if DataFamily needs changes)
- Test: `crates/seq-anytime-valid/tests/gate4_monte_carlo.rs`

- [ ] **Step 1: Write TCK scenario for DataFamily dispatch**

Add to `tck/seq-anytime-valid/features/` or directly as a Rust test:

```gherkin
Feature: AnytimeMonitor DataFamily dispatch

  Scenario: Bernoulli family uses sigma=0.5 not estimated sigma
    Given an AnytimeMonitor configured with DataFamily::Bernoulli and alpha=0.05
    When I feed 100 observations drawn from Bernoulli(0.5)
    Then the confidence interval width uses sigma=0.5
    And the width does not depend on the observed sample variance

  Scenario: Normal family with known variance uses that variance
    Given an AnytimeMonitor configured with DataFamily::Normal(known_variance=1.0) and alpha=0.05
    When I feed 100 observations drawn from N(0,1)
    Then the confidence interval width uses sigma=1.0

  Scenario: Normal family without known variance uses Welford estimate
    Given an AnytimeMonitor configured with DataFamily::Normal(known_variance=None) and alpha=0.05
    When I feed 100 observations drawn from N(0,1)
    Then the confidence interval width uses the running Welford estimate
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p seq-anytime-valid -- bernoulli_uses_fixed_sigma`
Expected: FAIL — AnytimeMonitor currently ignores DataFamily

- [ ] **Step 3: Implement DataFamily dispatch in AnytimeMonitor**

In `anytime.rs`, modify `update()` to match on `self.config.data_family`:
- `DataFamily::Bernoulli` → use `sigma = 0.5` (max std dev for Bernoulli)
- `DataFamily::Normal { known_variance: Some(v) }` → use `sigma = v.sqrt()`
- `DataFamily::Normal { known_variance: None }` → use Welford estimate (existing behavior)

The key change is around line 68-74 where sigma is computed. Replace the unconditional Welford path with a match.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p seq-anytime-valid -- bernoulli_uses_fixed_sigma`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/seq-anytime-valid/
git commit -m "fix(seq-anytime-valid): dispatch AnytimeMonitor on DataFamily, use sigma=0.5 for Bernoulli"
```

### Task A2: Fix eval-orchestrator SequentialInstrument (Tier 0 — BLOCKING)

**What:** SequentialInstrument hardcodes `DataFamily::Normal { known_variance: None }` at line 22-29 of `instruments/sequential.rs`. Must use `DataFamily::Bernoulli` for binary MCQ data.

**Files:**
- Modify: `crates/eval-orchestrator/src/instruments/sequential.rs`
- Modify: `crates/eval-orchestrator/src/analyze.rs` (if config needs to pass DataFamily)

- [ ] **Step 1: Write failing test**

Test that SequentialInstrument configured for binary outcomes uses Bernoulli family.

- [ ] **Step 2: Run test to verify it fails**

- [ ] **Step 3: Implement — derive DataFamily from outcome type**

In `sequential.rs`, determine DataFamily from the TrialRecord outcome variant:
- `Outcome::Binary` → `DataFamily::Bernoulli`
- `Outcome::Score` → `DataFamily::Normal { known_variance: None }`

Or accept DataFamily as config parameter. The simpler fix: change the hardcoded `DataFamily::Normal { known_variance: None }` to `DataFamily::Bernoulli` and make it configurable.

- [ ] **Step 4: Run test to verify it passes**

- [ ] **Step 5: Commit**

```bash
git commit -m "fix(eval-orchestrator): use DataFamily::Bernoulli for binary outcomes in SequentialInstrument"
```

### Task A3: Gate 4 Monte Carlo test for production CS path (Tier 0 — BLOCKING)

**What:** Existing Gate 4 test (`gate4_monte_carlo.rs`) tests `normal_mixture_cs_known_sigma` — NOT the production `AnytimeMonitor::update()` path. Write a test that feeds Bernoulli(p) data through AnytimeMonitor and verifies ≥93% coverage across 10,000 replications.

**Files:**
- Create: `crates/seq-anytime-valid/tests/gate4_anytime_monitor.rs`

- [ ] **Step 1: Write Gate 4 test**

```rust
/// Gate 4: Monte Carlo coverage test for AnytimeMonitor production path.
///
/// For each p in {0.1, 0.3, 0.5, 0.7, 0.9}, generate 10,000 independent
/// Bernoulli(p) streams of length N=200. Feed each through AnytimeMonitor
/// configured with DataFamily::Bernoulli and alpha=0.05. Check that the
/// final confidence interval contains the true p in ≥93% of replications.
/// (93% not 95% to account for finite-sample simulation noise.)
#[test]
fn anytime_monitor_bernoulli_coverage_gate4() {
    let alpha = 0.05;
    let n_reps = 10_000;
    let n_obs = 200;
    let test_ps = [0.1, 0.3, 0.5, 0.7, 0.9];

    for &true_p in &test_ps {
        let mut covered = 0u32;
        for rep in 0..n_reps {
            let mut rng = /* seeded from rep */;
            let config = MsprtConfig {
                data_family: DataFamily::Bernoulli,
                // ...
            };
            let mut monitor = AnytimeMonitor::new(config, alpha);
            for _ in 0..n_obs {
                let obs = if rng.gen::<f64>() < true_p { 1.0 } else { 0.0 };
                monitor.update(obs);
            }
            if let Some((lo, hi)) = monitor.confidence_interval() {
                if lo <= true_p && true_p <= hi {
                    covered += 1;
                }
            }
        }
        let coverage = covered as f64 / n_reps as f64;
        assert!(
            coverage >= 0.93,
            "Coverage at p={true_p}: {coverage:.3} < 0.93"
        );
    }
}
```

- [ ] **Step 2: Run test — must pass after A1 fix**

Run: `cargo test -p seq-anytime-valid --test gate4_anytime_monitor -- --nocapture`
Expected: PASS (coverage ≥93% at all p values). If sigma=0.5 fix is correct, coverage should be ≥98%.

- [ ] **Step 3: Commit**

```bash
git commit -m "test(seq-anytime-valid): Gate 4 Monte Carlo coverage for AnytimeMonitor production path"
```

### Task A4: Data quality gate in Sobol analysis (Tier 0 — BLOCKING)

**What:** Reject cells with `n_samples=0` before Sobol estimation. 20 such cells in WMDP bio corrupt variance decomposition.

**Files:**
- Modify: `crates/mojave-gsa/src/analyze.rs`
- Test: `crates/mojave-gsa/tests/` (or inline)

- [ ] **Step 1: Write TCK scenario**

```gherkin
Feature: Sobol data quality gate

  Scenario: cells with n_samples=0 are rejected before analysis
    Given a Saltelli results file with 3 cells having n_samples=0
    When I run Sobol analysis
    Then the analysis fails with error "N cells have n_samples=0"
    And the error message lists the affected cell indices
```

- [ ] **Step 2: Write failing test**

Test that `analyze_sobol()` returns an error when any cell has `n_samples == 0` (or `n_samples` field missing and accuracy == 0.0).

- [ ] **Step 3: Implement — add validation after cell loading**

In `analyze.rs`, after line ~262 where cells are validated, add:

```rust
let zero_sample_cells: Vec<usize> = cells.iter()
    .enumerate()
    .filter(|(_, c)| c.n_samples.unwrap_or(0) == 0)
    .map(|(i, _)| i)
    .collect();

if !zero_sample_cells.is_empty() {
    return Err(AnalyzeError::ZeroSampleCells {
        count: zero_sample_cells.len(),
        indices: zero_sample_cells,
    });
}
```

- [ ] **Step 4: Run test to verify it passes**

- [ ] **Step 5: Commit**

```bash
git commit -m "fix(mojave-gsa): reject cells with n_samples=0 before Sobol estimation"
```

### Task A5: Sobol convergence diagnostics (Tier 2)

**What:** Warn when S1 < 0, CI crosses [0,1] boundary, sum_ST > 1.3, or CI width > 10% of point estimate. Automate the "double N" decision from the plan.

**Files:**
- Modify: `crates/mojave-gsa/src/analyze.rs`
- Create: `crates/mojave-gsa/src/diagnostics.rs`

- [ ] **Step 1: Write TCK scenarios for each diagnostic**

```gherkin
Feature: Sobol convergence diagnostics

  Scenario: negative S1 triggers warning
    Given Sobol results with S1_quantization = -0.070
    When I run convergence diagnostics
    Then a warning is emitted for factor "quantization" with reason "negative S1"

  Scenario: CI width exceeding threshold triggers doubling recommendation
    Given Sobol results with S1_prompt_template CI width = 0.44 and point estimate = 0.85
    When I run convergence diagnostics with threshold 0.10
    Then a recommendation is emitted to double N

  Scenario: sum_ST exceeding 1.3 triggers interaction warning
    Given Sobol results with sum of ST = 1.295
    When I run convergence diagnostics
    Then a warning is emitted for substantial factor interactions
```

- [ ] **Step 2: Implement SobolDiagnostics struct**

```rust
pub struct SobolDiagnostic {
    pub factor: String,
    pub kind: DiagnosticKind,
    pub message: String,
}

pub enum DiagnosticKind {
    NegativeS1,
    CiCrossesBound,
    SumStExceedsThreshold,
    CiWidthExceedsThreshold,
    RecommendDoubleN,
}

pub fn run_diagnostics(results: &SobolResults, threshold: f64) -> Vec<SobolDiagnostic> { ... }
```

- [ ] **Step 3: Wire into analyze.rs output, run tests green**

- [ ] **Step 4: Commit**

### Task A6: Waudby-Smith & Ramdas betting CS (Tier 2)

**What:** Replace the sigma=0.5 conservative fix with the correct hedged capital confidence sequence for [0,1]-bounded data. Add Gate 4 calibration.

**Lit review required:** Full read of Waudby-Smith & Ramdas 2024 "Estimating means of bounded random variables by betting" for the hedged capital process implementation. Also Koning 2025 for the sequentialized Wilson alternative.

**Files:**
- Create: `crates/seq-anytime-valid/src/monitor/betting.rs`
- Modify: `crates/seq-anytime-valid/src/lib.rs`
- Test: `crates/seq-anytime-valid/tests/gate4_betting_cs.rs`

- [ ] **Step 1: Write TCK scenarios**

```gherkin
Feature: Betting confidence sequence for bounded data

  # Gate 1: Textbook reproduction
  Scenario: hedged capital CS matches Waudby-Smith Example 3.1
    Given Bernoulli(0.5) data with N=100 and alpha=0.05
    When I compute the betting CS
    Then the CI width at N=100 matches the paper's reported width within 5%

  # Gate 3: Property tests
  Scenario: betting CS is always valid
    Given any stopping time T in [10, 1000]
    When I compute the betting CS at time T
    Then the true mean is covered with probability >= 1-alpha

  # Gate 4: Monte Carlo calibration
  Scenario: betting CS achieves 95% coverage across p values
    Given 10,000 replications at each p in {0.1, 0.3, 0.5, 0.7, 0.9}
    When I run BettingMonitor with alpha=0.05 and N=200
    Then coverage >= 93% at every p value
    And CI width is narrower than the sigma=0.5 conservative bound
```

- [ ] **Step 2: Implement BettingMonitor**

Implements the hedged capital process from Waudby-Smith & Ramdas 2024:
- Wealth process with LBOW (Lower Bound on Wealth) tracking
- Adaptive bet sizing via ONS (Online Newton Step) or simpler grid hedging
- CI inversion via bisection on the wealth threshold

- [ ] **Step 3: Gate 4 Monte Carlo calibration**

- [ ] **Step 4: Wire into AnytimeMonitor as preferred Bernoulli backend**

- [ ] **Step 5: Commit**

---

## Stream B: Audit Chain Trust (Peer 2)

**Lit review:** Sigstore cosign documentation, COSE RFC 8152, Rekor API. Check `../evals_papers/` for Kao2025.

**Cites:** wave_2/05_audit_chain_trust.md, wave_1/adversary.md finding 4

### Task B1: Retire Python audit writer (Tier 1)

**What:** Python `scripts/audit.py` is format-incompatible with Rust verifier post-genesis-sentinel. Replace with subprocess calls to `mojave audit emit`. Remove audit.py. Fix cross-language test to actually run.

**Files:**
- Delete: `scripts/audit.py`
- Modify: all Python scripts that import audit.py → use `subprocess.run(["mojave", "audit", "emit", ...])`
- Modify: `scripts/tests/test_audit.py` — remove Python-writer tests, keep Rust binary verification tests
- Modify: `scripts/v2/run_mcq.py` (if it calls audit.py)

- [ ] **Step 1: Find all Python callers of audit.py**

```bash
grep -rn "import audit\|from.*audit import\|audit\.py" scripts/ --include="*.py"
```

- [ ] **Step 2: Replace each caller with subprocess invocation**

Pattern:
```python
# Before:
from audit import AuditChain
chain = AuditChain(path)
chain.append(event)

# After:
import subprocess
subprocess.run(
    ["mojave", "audit", "emit", "--chain", str(path), "--event-kind", event_kind, ...],
    check=True,
)
```

- [ ] **Step 3: Remove audit.py**

- [ ] **Step 4: Fix cross-language test — remove pytest.skip, require binary**

In `scripts/tests/test_audit.py`, the `test_rust_verifier_accepts_python_chain` test uses `pytest.skip` when no binary is found. Since Python no longer writes chains directly, convert this test to: emit a chain via `mojave audit emit`, then verify via `mojave audit verify`. This tests the binary round-trip, not cross-language parity.

- [ ] **Step 5: Run tests green, commit**

```bash
git commit -m "refactor(audit): retire Python audit writer, use mojave CLI subprocess"
```

### Task B2: CI/CD pipeline with clippy + test (Tier 1)

**What:** No `.github/workflows/` exists. Create a basic CI pipeline that runs clippy (zero warnings), rustfmt, and `cargo test` on every push/PR.

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write CI workflow**

```yaml
name: CI
on: [push, pull_request]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace
```

- [ ] **Step 2: Commit and push to verify**

```bash
git commit -m "ci: add GitHub Actions workflow for clippy, fmt, and tests"
```

### Task B3: Sigstore binary signing (Tier 0 — BLOCKING)

**What:** Add cosign signing to CI for release binaries.

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Write release workflow with cosign**

Uses `sigstore/cosign-installer` action. Signs release binaries with keyless (OIDC) signing.

```yaml
name: Release
on:
  push:
    tags: ['v*']
jobs:
  build-and-sign:
    runs-on: ubuntu-latest
    permissions:
      id-token: write
      contents: write
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release -p mojave-cli -p mojave-gsa
      - uses: sigstore/cosign-installer@v3
      - run: |
          cosign sign-blob --yes target/release/mojave --output-signature mojave.sig --output-certificate mojave.crt
          cosign sign-blob --yes target/release/mojave-gsa --output-signature mojave-gsa.sig --output-certificate mojave-gsa.crt
      - uses: softprops/action-gh-release@v2
        with:
          files: |
            target/release/mojave
            target/release/mojave-gsa
            mojave.sig
            mojave.crt
            mojave-gsa.sig
            mojave-gsa.crt
```

- [ ] **Step 2: Commit**

```bash
git commit -m "ci: add Sigstore cosign signing for release binaries"
```

### Task B4: Canonical encoding specification (Tier 2)

**What:** One-page document pinning sort order (UTF-8), escaping rules, float rejection, integer representation. Currently defined only by code.

**Files:**
- Create: `docs/adr/0002-canonical-json-encoding-spec.md`

- [ ] **Step 1: Read current implementation** in `crates/audit-chain/src/` to extract exact encoding rules

- [ ] **Step 2: Write ADR documenting the spec**

Cover: key sort order, string escaping, no floats (must be integer or string), no trailing commas, UTF-8 normalization, serde version sensitivity risks.

- [ ] **Step 3: Commit**

### Task B5: Rekor external witnessing (Tier 3)

**What:** Submit periodic chain-head snapshots to Sigstore Rekor for third-party proof-of-existence timestamps.

**Files:**
- Create: `crates/audit-sign/src/rekor.rs`
- Modify: `crates/mojave-cli/src/commands/audit.rs` (add `audit witness` subcommand)

- [ ] **Step 1: Write TCK scenario**

```gherkin
Feature: Rekor chain-head witnessing

  Scenario: witness subcommand submits chain head to Rekor
    Given a chain with 10 entries
    When I run "mojave audit witness --chain chain.jsonl"
    Then a Rekor log entry is created
    And the response contains a log index
    And the log entry contains the chain-head hash
```

- [ ] **Step 2: Implement Rekor client** — HTTP POST to Rekor API with HashedRekord type

- [ ] **Step 3: Add `audit witness` CLI subcommand**

- [ ] **Step 4: Tests green, commit**

### Task B6: Key management upgrade (Tier 3)

**What:** Replace `KeyRef::Env` with encrypted file or OS keychain integration for NIST 800-171 compliance.

Deferred — design spec needed first. Document the gap and compliance requirements in an ADR.

---

## Stream C: QMU + Defense Framework (Peer 3)

**Blocked by:** Stream A completion (needs valid confidence intervals)

**Lit review:** Pilch et al. 2006 SAND2006-5001 (QMU white paper), Sharp & Wood-Schultz 2003 (CR definition), National Academies 2009 (QMU evaluation), JCGM 106:2012 (conformity assessment), Keller et al. 2026 NIST AI 800-3. Papers in `../evals_papers/` intake.

**Cites:** wave_2/01_qmu_defense_framework.md, wave_1/xfactor.md findings 1, 3

### Task C1: QmuAssessment struct with JCGM 106 guard bands (Tier 1)

**What:** Build a thin composition layer over existing primitives. QMU Confidence Ratio = margin / expanded_uncertainty. JCGM 106 guard bands formalize accept/reject with configurable consumer risk.

**Files:**
- Create: `crates/eval-orchestrator/src/qmu.rs`
- Modify: `crates/eval-orchestrator/src/lib.rs`
- Test: `crates/eval-orchestrator/tests/qmu_tests.rs`
- TCK: `tck/eval-orchestrator/features/qmu.feature`

- [ ] **Step 1: Write TCK scenarios**

```gherkin
Feature: QMU Confidence Ratio and conformity assessment

  # Gate 1: Textbook reproduction
  Scenario: CR computation matches Pilch 2006 Example 3.2
    Given a measurement with estimate=0.82, expanded_uncertainty=0.04, threshold=0.70
    When I compute the QMU assessment
    Then margin is 0.12
    And confidence_ratio is 3.0
    And the decision is Accept

  Scenario: guarded acceptance with JCGM 106 guard band
    Given a measurement with estimate=0.72, expanded_uncertainty=0.04, threshold=0.70
    And a guard band width of 0.02 (consumer risk < 5%)
    When I compute the conformity decision
    Then the guarded threshold is 0.72
    And the decision is Investigate

  Scenario: clear rejection
    Given a measurement with estimate=0.65, expanded_uncertainty=0.04, threshold=0.70
    When I compute the QMU assessment
    Then margin is -0.05
    And the decision is Reject

  # Gate 3: Property tests
  Scenario: CR increases monotonically with margin at fixed uncertainty
    Given fixed expanded_uncertainty=0.04 and threshold=0.70
    When I compute CR for estimates [0.65, 0.70, 0.75, 0.80, 0.85]
    Then CR values are strictly increasing

  Scenario: CR decreases monotonically with uncertainty at fixed margin
    Given fixed estimate=0.80 and threshold=0.70
    When I compute CR for expanded_uncertainties [0.02, 0.04, 0.06, 0.08]
    Then CR values are strictly decreasing
```

- [ ] **Step 2: Write failing tests from TCK**

- [ ] **Step 3: Implement QmuAssessment**

```rust
pub struct QmuAssessment {
    pub estimate: f64,
    pub expanded_uncertainty: f64,
    pub threshold: f64,
    pub margin: f64,
    pub confidence_ratio: f64,
    pub guard_band: Option<f64>,
    pub decision: ConformityDecision,
}

pub enum ConformityDecision {
    Accept,
    Reject,
    Investigate { reason: String },
}

impl QmuAssessment {
    pub fn evaluate(
        estimate: f64,
        expanded_uncertainty: f64,
        threshold: f64,
        guard_band: Option<f64>,
    ) -> Self {
        let margin = estimate - threshold;
        let confidence_ratio = if expanded_uncertainty > 0.0 {
            margin / expanded_uncertainty
        } else {
            f64::INFINITY
        };
        let effective_threshold = threshold + guard_band.unwrap_or(0.0);
        let decision = if estimate >= effective_threshold + expanded_uncertainty {
            ConformityDecision::Accept
        } else if estimate + expanded_uncertainty < threshold {
            ConformityDecision::Reject
        } else {
            ConformityDecision::Investigate {
                reason: format!("CR={confidence_ratio:.2}, within guard band"),
            }
        };
        Self { estimate, expanded_uncertainty, threshold, margin, confidence_ratio, guard_band, decision }
    }
}
```

- [ ] **Step 4: Add JCGM 106 guard band computation**

```rust
pub fn jcgm106_guard_band(
    expanded_uncertainty: f64,
    consumer_risk: f64,  // e.g. 0.05
) -> f64 {
    // Simple symmetric guard band: g = k * u
    // where k is chosen so P(accept defective) < consumer_risk
    // For normal: k ≈ Φ^{-1}(1 - consumer_risk) - coverage_factor
    // Simplified: g = expanded_uncertainty * guard_factor
    // Default guard_factor from JCGM 106 Table 1
    expanded_uncertainty * guard_factor_for_risk(consumer_risk)
}
```

- [ ] **Step 5: Tests green, commit**

```bash
git commit -m "feat(eval-orchestrator): QmuAssessment struct with JCGM 106 guard bands"
```

### Task C2: Wire QMU to existing pipeline outputs (Tier 1)

**What:** Compose QmuAssessment from SequentialInstrument CI (→ expanded_uncertainty), SobolResults (→ sensitivity profile for explaining what drives the margin), and SpcResult (→ stability confirmation).

**Files:**
- Modify: `crates/eval-orchestrator/src/qmu.rs`
- Modify: `crates/eval-orchestrator/src/analyze.rs`

- [ ] **Step 1: Add `from_pipeline_outputs()` constructor**

```rust
impl QmuAssessment {
    pub fn from_pipeline(
        sequential_result: &SequentialResult,
        threshold: f64,
        guard_band: Option<f64>,
    ) -> Self {
        let (ci_lo, ci_hi) = sequential_result.confidence_interval;
        let estimate = (ci_lo + ci_hi) / 2.0;
        let expanded_uncertainty = (ci_hi - ci_lo) / 2.0;
        Self::evaluate(estimate, expanded_uncertainty, threshold, guard_band)
    }
}
```

- [ ] **Step 2: Add to analyze output, tests green, commit**

### Task C3: NIST AI 800-3 alignment section in run cards (Tier 1)

**What:** Add a section to LaTeX run card template mapping mojave outputs to NIST concepts.

**Files:**
- Modify: `templates/run-card/single-run-card/runcard-v2.tex`

- [ ] **Step 1: Add NIST alignment section**

LaTeX section mapping:
- "Benchmark accuracy" → mojave's point estimate with CI
- "Statistical model specification" → measurement equation Y_ij = f(θ, items, perturbation) + ε
- "Variance decomposition" → Sobol indices table
- "Conformity assessment" → QMU CR and decision

- [ ] **Step 2: Commit**

```bash
git commit -m "feat(templates): add NIST AI 800-3 alignment section to run cards"
```

### Task C4: GSN assurance case template (Tier 3)

**What:** LaTeX template mapping QMU outputs to Goal Structuring Notation (GSN) for defense procurement.

**Lit review:** UK MOD Def Stan 00-56 Issue 7, ISO 15026-2:2022, Rushby 2024 Assurance 2.0.

**Files:**
- Create: `templates/assurance-case/gsn-template.tex`

- [ ] **Step 1: Design GSN structure**

```
G1: Model meets performance threshold under guarded acceptance
├── S1: Strategy — QMU conformity assessment
│   ├── G1.1: Margin exceeds uncertainty (CR > threshold)
│   │   ├── Sn1: Evidence — confidence sequence CI
│   │   └── Sn2: Evidence — Sobol sensitivity profile
│   ├── G1.2: Measurement system is qualified
│   │   ├── Sn3: Evidence — IRR with bootstrap CIs
│   │   └── Sn4: Evidence — MSA ndc ≥ 5
│   └── G1.3: Performance is stable over time
│       └── Sn5: Evidence — SPC control chart
├── D1: Defeater — benchmark contamination
├── D2: Defeater — sandbagging
└── D3: Defeater — binary integrity (mitigated by Sigstore)
```

- [ ] **Step 2: Implement as LaTeX with tikz GSN shapes**

- [ ] **Step 3: Commit**

### Task C5: Construct validity dossier framework (Tier 3)

**What:** Structured framework with 6 evidence slots. Implement CVI as derived statistic from Sobol results. Note the dissent's caveat: CVI is a variance proportion, not a validity coefficient. Frame as "sensitivity profile" / "measurement noise budget."

**Files:**
- Create: `crates/eval-orchestrator/src/validity.rs`
- Modify: `crates/mojave-gsa/src/analyze.rs`

- [ ] **Step 1: Write TCK for CVI computation**

```gherkin
Feature: Construct Validity Index (sensitivity profile)

  Scenario: CVI from Sobol first-order indices
    Given Sobol results with S1 for factors: prompt_template=0.85, system_prompt=0.03, decoding=0.02
    And factors prompt_template, system_prompt, decoding are declared construct-irrelevant
    When I compute CVI
    Then CVI = 1.0 - (0.85 + 0.03 + 0.02) = 0.10

  Scenario: CVI is sensitive to factor designation
    Given the same Sobol results
    And only prompt_template is declared construct-irrelevant
    When I compute CVI
    Then CVI = 1.0 - 0.85 = 0.15
```

- [ ] **Step 2: Implement CVI struct**

- [ ] **Step 3: Add validity dossier template slots**

- [ ] **Step 4: Commit**

---

## Stream D: GSA + G-Theory + WMDP (Peer 4)

**Blocked by:** Stream A (WMDP rerun needs valid CS pipeline)

**Repos:** mojave + ../salib

**Lit review:** Brennan 2001 (G-theory textbook — acquire from ASU), Saltelli et al. 2008 (GSA primer — acquire from ASU), Shavelson & Webb 1991 (G-theory primer). For WMDP rerun: use corrected pipeline from Stream A.

**Cites:** wave_2/06_gsa_theory_salibrs.md, wave_2/04_gauge_rr_gtheory.md

### Task D1: Deduplicate Sobol code in mojave-gsa (Tier 2)

**What:** `analyze.rs` has local `compute_sobol_from_cached()` and `bootstrap_sobol_cis()` that duplicate salib-rs canonical implementations. Replace with calls to salib-rs.

**Files:**
- Modify: `crates/mojave-gsa/src/analyze.rs` (lines 117-217)
- Modify: `crates/mojave-gsa/Cargo.toml` (ensure salib-estimators dependency)

- [ ] **Step 1: Identify exact API surface needed from salib-rs**

Check what `salib-estimators` exports for Sobol computation. The local functions take `fa`, `fb`, `fab` arrays — find the matching salib-rs entry point.

- [ ] **Step 2: Write test comparing local vs salib-rs output on same data**

Feed identical fa/fb/fab arrays to both implementations. Verify results match within f64 epsilon. This catches the percentile interpolation discrepancy (floor vs linear) noted in wave_2/06.

- [ ] **Step 3: Replace local functions with salib-rs calls**

Delete `compute_sobol_from_cached()` and `bootstrap_sobol_cis()` from analyze.rs. Replace call sites with salib-rs canonical functions.

- [ ] **Step 4: Run all mojave-gsa tests green**

- [ ] **Step 5: Commit**

```bash
git commit -m "refactor(mojave-gsa): replace local Sobol/bootstrap with salib-rs canonical implementations"
```

### Task D2: G-theory Gate 1 validation against Brennan 2001 (Tier 2)

**What:** salib-rs has G-theory implementation (640 lines) with TCK tests, but Gate 1 uses a synthetic grid, not textbook golden data. Add Brennan 2001 textbook reproduction.

**Repo:** ../salib

**Files:**
- Create: `../salib/tck/salib/g-theory-estimator/features/g_theory_brennan2001.feature`
- Create: `../salib/crates/salib-estimators/tests/g_theory_brennan2001.rs`
- Create: `../salib/crates/salib-estimators/tests/fixtures/brennan2001_table3_1.json`

**Lit review required:** Read Brennan 2001 Chapter 3 for the worked example with known variance components. Extract the grid data and expected sigma values.

- [ ] **Step 1: Write TCK scenario**

```gherkin
Feature: G-theory Brennan 2001 reproduction

  # Gate 1: Textbook reproduction
  Scenario: Brennan 2001 Table 3.1 crossed p x i x r
    Given the Brennan 2001 Table 3.1 dataset
    When I estimate G-theory p x i x r components
    Then sigma_p matches Brennan within tolerance 0.001
    And sigma_i matches Brennan within tolerance 0.001
    And sigma_r matches Brennan within tolerance 0.001
    And sigma_pi matches Brennan within tolerance 0.001
    And sigma_pr matches Brennan within tolerance 0.001
    And sigma_ir matches Brennan within tolerance 0.001
    And sigma_pir matches Brennan within tolerance 0.001
    And G matches Brennan within tolerance 0.001
    And Phi matches Brennan within tolerance 0.001

  Scenario: Brennan 2001 D-study projection matches Table 3.3
    Given the Brennan 2001 variance components from Table 3.1
    When I project D-study at n_items=5 and n_raters=3
    Then projected G matches Brennan Table 3.3 within tolerance 0.001
    And projected Phi matches Brennan Table 3.3 within tolerance 0.001
```

- [ ] **Step 2: Create golden fixture from textbook data**

- [ ] **Step 3: Write failing test**

- [ ] **Step 4: Verify implementation passes (should pass — implementation is complete)**

If it fails, fix the implementation. If it passes, Gate 1 is satisfied.

- [ ] **Step 5: Commit in ../salib**

```bash
cd ../salib && git commit -m "test(salib-estimators): Gate 1 Brennan 2001 reproduction for G-theory"
```

### Task D3: D-study budget optimizer (Tier 2)

**What:** Extend D-study from 4-point projection to arbitrary grid. Add `find_minimum_design(target_phi, cost_function)` for constrained optimization of Saltelli N_base.

**Repo:** ../salib

**Files:**
- Modify: `../salib/crates/salib-estimators/src/g_theory.rs`
- Test: `../salib/crates/salib-estimators/tests/g_theory_d_study_tck.rs`
- TCK: `../salib/tck/salib/g-theory-estimator/features/g_theory_d_study.feature`

- [ ] **Step 1: Write TCK scenario**

```gherkin
Feature: D-study budget optimizer

  Scenario: find minimum design for target Phi
    Given G-theory variance components from a pilot study
    And a target Phi >= 0.80
    And a cost function cost(n_items, n_raters) = n_items * n_raters * 10
    When I find the minimum design
    Then the result has Phi >= 0.80
    And no cheaper design with Phi >= 0.80 exists in the search grid

  Scenario: D-study surface over item/rater grid
    Given G-theory variance components from a pilot study
    When I compute D-study surface for n_items in [2,4,8,16] and n_raters in [1,2,3,5]
    Then the surface has 16 points
    And Phi increases monotonically with both n_items and n_raters
```

- [ ] **Step 2: Implement**

```rust
pub struct DStudySurface {
    pub points: Vec<DStudyPoint>,
}

pub fn d_study_surface(
    result: &GTheoryResult,
    item_counts: &[usize],
    rater_counts: &[usize],
) -> Result<DStudySurface, GTheoryError> {
    let mut points = Vec::with_capacity(item_counts.len() * rater_counts.len());
    for &ni in item_counts {
        for &nr in rater_counts {
            points.push(project_g_theory_d_study(result, ni, nr)?);
        }
    }
    Ok(DStudySurface { points })
}

pub fn find_minimum_design<F>(
    result: &GTheoryResult,
    target_phi: f64,
    max_items: usize,
    max_raters: usize,
    cost_fn: F,
) -> Result<Option<DStudyPoint>, GTheoryError>
where
    F: Fn(usize, usize) -> f64,
{
    let mut best: Option<(DStudyPoint, f64)> = None;
    for ni in 1..=max_items {
        for nr in 1..=max_raters {
            let point = project_g_theory_d_study(result, ni, nr)?;
            if point.phi_coefficient >= target_phi {
                let cost = cost_fn(ni, nr);
                if best.as_ref().map_or(true, |(_, bc)| cost < *bc) {
                    best = Some((point, cost));
                }
            }
        }
    }
    Ok(best.map(|(p, _)| p))
}
```

- [ ] **Step 3: Tests green, commit in ../salib**

### Task D4: Second-order Sobol indices (Tier 2)

**What:** Enable `calc_second_order=true` in Saltelli matrix construction. Compute S2_ij for all factor pairs. Report interaction structure.

**Files:**
- Modify: `crates/mojave-gsa/src/manifest.rs` (add calc_second_order option)
- Modify: `crates/mojave-gsa/src/analyze.rs` (compute S2 from AB matrices)

- [ ] **Step 1: Write TCK scenario**

```gherkin
Feature: Second-order Sobol indices

  Scenario: S2 indices computed when second_order=true
    Given a Saltelli manifest with k=4 factors and second_order=true
    When I run Sobol analysis
    Then S2 indices are computed for all 6 factor pairs
    And sum(S1) + sum(S2) + residual approximates sum(ST)
```

- [ ] **Step 2: Implement S2 computation using salib-rs**

- [ ] **Step 3: Add to analysis output, commit**

### Task D5: WMDP rerun at N=1024 with data quality gates (Tier 1)

**Blocked by:** Stream A completion (valid CS pipeline)

**What:** Double N from 512 to 1024 (8,192 cells per benchmark). Apply n_samples>0 gate. This satisfies the plan's own convergence criterion that was violated 4.4x.

**Files:**
- Modify: `scripts/v2/run_mcq.py` (or generate new manifests)
- Run: `scripts/v2/analyze_sobol.py` on new results

- [ ] **Step 1: Generate N=1024 Saltelli manifests**

```bash
mojave-gsa manifest --n-base 1024 --problem wmdp_bio_problem.json --output manifest_bio_1024.json
mojave-gsa manifest --n-base 1024 --problem wmdp_chem_problem.json --output manifest_chem_1024.json
```

- [ ] **Step 2: Run eval on RunPod fleet** (~68 additional GPU-minutes per benchmark)

- [ ] **Step 3: Analyze with data quality gate and convergence diagnostics**

- [ ] **Step 4: Verify convergence criteria met (CI width < 10% of estimate)**

- [ ] **Step 5: Commit results and updated analysis**

### Task D6: Bare-prompt separated reporting (Tier 1)

**What:** Run Sobol analysis with and without "bare" prompt template. Report both: "Including bare: S1_prompt=0.85; excluding bare: S1_prompt=X."

**Files:**
- Modify: `crates/mojave-gsa/src/analyze.rs` (add exclude_levels option)
- Alternatively: use given-data Sobol estimator on non-bare subset

- [ ] **Step 1: Write TCK scenario**

```gherkin
Feature: Leave-one-level-out Sobol analysis

  Scenario: excluding a factor level changes S1
    Given Saltelli results with factor prompt_template having levels [bare, cot, standard, detailed]
    When I run Sobol analysis excluding prompt_template level "bare"
    Then S1_prompt_template is lower than the full-design S1
    And a comparison table is emitted showing both values
```

- [ ] **Step 2: Implement level exclusion filter**

- [ ] **Step 3: Run on WMDP data, generate comparison report**

- [ ] **Step 4: Commit**

### Task D7: Full factorial vs Saltelli evaluation (Tier 3)

**What:** For the 6-factor discrete design (5×4×4×2×3×2 = 960 cells), compare full factorial with replication against Saltelli N=512. Measure convergence properties.

Deferred — requires WMDP rerun data for meaningful comparison.

---

## Stream E: IRR + Measurement Qualification (Peer 5)

**Lit review:** AIAG MSA Manual 4th ed (acquire from ASU), Fleiss 1971, Krippendorff 2011, Takeshita 2026 (in intake), ISO 5725 Parts 1-6.

**Cites:** wave_2/04_gauge_rr_gtheory.md, wave_1/xfactor.md finding 5

### Task E1: Wire bootstrap CIs to IRR statistics (Tier 2)

**What:** Connect existing `bootstrap_ci()` in `crates/irr/src/bootstrap.rs` to Cohen's κ, Fleiss' κ, Krippendorff's α, and Gwet's AC. Currently all return `ci_lower: None, ci_upper: None`.

**Files:**
- Modify: `crates/irr/src/cohen.rs` (lines 67-68, 82-83, 125-126, 153-154)
- Modify: `crates/irr/src/fleiss.rs` (lines 86-87, 103-104)
- Modify: `crates/irr/src/krippendorff.rs` (lines 105-106)
- Modify: `crates/irr/src/gwet.rs` (lines 182-183)
- Test: `crates/irr/tests/`

- [ ] **Step 1: Write TCK scenario**

```gherkin
Feature: Bootstrap confidence intervals for IRR statistics

  # Gate 3: Property test
  Scenario: bootstrap CIs bracket point estimate
    Given a rating matrix with moderate agreement
    When I compute Cohen kappa with bootstrap CIs (n_resamples=1000, alpha=0.05)
    Then ci_lower <= kappa <= ci_upper
    And ci_lower is not None
    And ci_upper is not None

  # Gate 4: Monte Carlo calibration
  Scenario: bootstrap CI coverage is approximately nominal
    Given 1000 simulated rating matrices with true kappa=0.60
    When I compute bootstrap CIs at alpha=0.05 for each
    Then the coverage rate is between 0.90 and 0.99
```

- [ ] **Step 2: Write failing tests (ci_lower/ci_upper are None)**

- [ ] **Step 3: Implement — add optional bootstrap to each IRR function**

Pattern for Cohen (same for others):

```rust
pub fn cohen_kappa_with_ci(
    matrix: &ConfusionMatrix,
    n_resamples: usize,
    alpha: f64,
    rng: &mut impl Rng,
) -> IrrResult {
    let point = cohen_kappa(matrix);
    let ci = bootstrap_ci(
        matrix.raw_data(),
        |sample| cohen_kappa_from_raw(sample).value,
        n_resamples,
        alpha,
        rng,
    );
    IrrResult {
        ci_lower: Some(ci.ci_lower),
        ci_upper: Some(ci.ci_upper),
        ..point
    }
}
```

- [ ] **Step 4: Tests green, commit**

```bash
git commit -m "feat(irr): wire bootstrap CIs to Cohen, Fleiss, Krippendorff, Gwet"
```

### Task E2: MSA ndc and P/T ratio diagnostics (Tier 2)

**What:** Compute number of distinct categories (ndc) and precision-to-tolerance ratio (P/T) from existing RatingMatrix data. These answer "can the judge distinguish performance levels?" — a question IRR agreement statistics cannot answer.

**Files:**
- Create: `crates/irr/src/msa.rs`
- Modify: `crates/irr/src/lib.rs`
- Test: `crates/irr/tests/msa_tests.rs`
- TCK: `tck/irr/features/msa.feature`

- [ ] **Step 1: Write TCK scenarios**

```gherkin
Feature: MSA gauge discrimination diagnostics

  Scenario: ndc from AIAG formula
    Given a rating study with sigma_parts=0.30 and sigma_gauge_rr=0.10
    When I compute ndc
    Then ndc = floor(1.41 * 0.30 / 0.10) = 4
    And an AIAG warning is emitted because ndc < 5

  Scenario: P/T ratio for threshold decision
    Given a rating study with gauge_rr=0.10 and tolerance=0.50
    When I compute P/T ratio
    Then P_T = 6 * 0.10 / 0.50 = 1.20
    And the gauge is adequate (P/T < 1.0 is inadequate... wait)
```

Note: need to read AIAG MSA Manual for exact formulas and thresholds. Lit review required.

- [ ] **Step 2: Implement ndc and P/T**

```rust
pub struct MsaDiagnostics {
    pub ndc: usize,
    pub p_t_ratio: f64,
    pub ndc_adequate: bool,  // AIAG: ndc >= 5
    pub pt_adequate: bool,   // typically P/T < 0.30
}

pub fn msa_diagnostics(sigma_parts: f64, sigma_gauge_rr: f64, tolerance: f64) -> MsaDiagnostics {
    let ndc = (1.41 * sigma_parts / sigma_gauge_rr).floor() as usize;
    let p_t_ratio = 6.0 * sigma_gauge_rr / tolerance;
    MsaDiagnostics {
        ndc,
        p_t_ratio,
        ndc_adequate: ndc >= 5,
        pt_adequate: p_t_ratio < 0.30,
    }
}
```

- [ ] **Step 3: Tests green, commit**

### Task E3: Mandel h/k outlier diagnostics (Tier 2)

**What:** Implement Mandel h (between-configuration consistency) and k (within-configuration consistency) statistics for ISO 5725 outlier detection.

**Lit review:** Takeshita 2026 (in intake) for bootstrap extension. Wilrich 2013 for critical values.

**Files:**
- Create: `crates/irr/src/mandel.rs`
- TCK: `tck/irr/features/mandel.feature`

- [ ] **Step 1: Write TCK scenarios**

```gherkin
Feature: Mandel h/k consistency statistics

  # Gate 1: reproduce ISO 5725-2 Example
  Scenario: Mandel h identifies between-lab outlier
    Given an interlaboratory dataset with one outlier configuration
    When I compute Mandel h statistics
    Then the outlier configuration has |h| > critical value at p=0.01

  Scenario: Mandel k identifies within-lab inconsistency
    Given an interlaboratory dataset with one high-variability configuration
    When I compute Mandel k statistics
    Then the inconsistent configuration has k > critical value at p=0.01
```

- [ ] **Step 2: Implement**

```rust
pub struct MandelStatistics {
    pub h: Vec<f64>,       // per-configuration between-consistency
    pub k: Vec<f64>,       // per-configuration within-consistency
    pub h_critical: f64,   // at specified alpha
    pub k_critical: f64,
    pub h_outliers: Vec<usize>,
    pub k_outliers: Vec<usize>,
}

pub fn mandel_hk(
    configs: &[ConfigResults],  // per-configuration: Vec of replicate values
    alpha: f64,
) -> MandelStatistics { ... }
```

- [ ] **Step 3: Add bootstrap CIs per Takeshita 2026**

- [ ] **Step 4: Tests green, commit**

### Task E4: ISO 5725 repeatability/reproducibility reporting (Tier 3)

**What:** Group Saltelli cells by configuration. Compute repeatability σ_r, reproducibility σ_R, repeatability limit r, reproducibility limit R.

**Files:**
- Create: `crates/irr/src/iso5725.rs`

- [ ] **Step 1: Write TCK from ISO 5725 worked example**

- [ ] **Step 2: Implement r and R computation**

- [ ] **Step 3: Commit**

### Task E5: Rasch fit diagnostics (Tier 3)

**What:** Fit 1PL alongside 2PL in mojave-calibrate Python package. Compute infit/outfit/point-measure correlation.

**Files:**
- Modify: `python/src/mojave_calibrate/` (IRT fitting code)

- [ ] **Step 1: Write test for Rasch fit statistics**

- [ ] **Step 2: Implement infit/outfit computation**

- [ ] **Step 3: Commit**

---

## Tier 4: Strategic (Deferred — Design Only)

These items are documented for future planning. No implementation in this cycle.

| # | Item | What | When |
|---|------|------|------|
| 4.1 | IRT item calibration | MMLE/EM-based item parameter estimation | When CAT is deployed |
| 4.2 | DIF detection | Mantel-Haenszel or logistic regression DIF | When cross-model fairness is needed |
| 4.3 | Multi-chain campaigns | Campaign root entry across per-model chains | When multi-model audits ship |
| 4.4 | CVI paper | Methods paper: Sobol-as-validity-coefficient | After WMDP rerun validates methodology |
| 4.5 | OT-GSA in salib-rs | Optimal-transport sensitivity indices (Borgonovo 2024) | When multivariate outputs needed |

---

## Library Acquisitions (Patrick — ASU access)

Before streams start, acquire the load-bearing papers. Peers will need these for lit review phases.

**Tier 1 (blocking):**
1. Brennan 2001 *Generalizability Theory* — Stream D Gate 1
2. Saltelli et al. 2008 *Global Sensitivity Analysis: The Primer* — Stream D reference
3. Waudby-Smith & Ramdas 2024 (full text) — Stream A Task A6
4. National Academies 2009 QMU report — Stream C (free from NAP)
5. Sharp & Wood-Schultz 2003 Los Alamos Science 28 — Stream C (free from LANL)
6. JASON QMU Report JSR-04-330 — Stream C (free from OSTI)

**Tier 2 (needed within first week):**
7. AIAG MSA Manual 4th ed — Stream E Task E2
8. Campbell & Fiske 1959 MTMM — Stream C Task C5
9. Iooss & Lemaitre 2015 GSA review — Stream D reference

---

## Peer Claude Infrastructure Setup

After this plan is approved, set up the engineering team:

### What each peer needs
1. **MCP server** providing access to the mojave repo (read/write) and salib repo (Peer 4)
2. **Developer channel** for coordination (review requests, dependency signals, blocking notices)
3. **Shared conventions:** commit message format, branch naming (`stream-X/task-name`), PR template
4. **Review protocol:** each peer's work reviewed by controller before merge to master

### Branch strategy
- Each stream works on its own branch: `stream-a/statistical-correctness`, `stream-b/audit-chain`, etc.
- Peers commit frequently to their stream branch
- Controller reviews and merges to master
- Stream A merges first (unblocks C and D)
- Streams B and E can merge independently

### Coordination signals
- Stream A → C: "CS pipeline fixed, Gate 4 passing" → C can build QMU on valid CIs
- Stream A → D: "pipeline fixed, data quality gate in place" → D can start WMDP rerun
- Stream E → C: "IRR bootstrap CIs done, MSA ndc available" → C can wire into QMU assessment

---

## Dissent Responses

The plan incorporates the dissent's strongest critiques:

| Dissent point | Response in plan |
|---------------|-----------------|
| QMU is theater without calibration | C1 implements the math; thresholds are explicitly left uncalibrated with documentation. JCGM 106 guard bands (framework-agnostic) are the primary decision mechanism. |
| WMDP showcase is broken | D5/D6 rerun with corrected pipeline, N=1024, bare-separated reporting |
| 4-layer stack has zero customer validation | Implemented as independent modules (E1-E4), not as a gated stack. Each is useful standalone. |
| salib-rs maintenance liability | D1 deduplicates mojave-gsa code. No new estimators added to salib-rs in this plan. |
| Audit chain "2-3 weeks" understates gap | B3/B4/B5 are scoped realistically. Key management (B6) is deferred with documented gap. |
| Advisory solves supply not demand | Plan does not include customer discovery (that's Patrick's job, not code). |
| CVI is not a validity coefficient | C5 reframes as "sensitivity profile" / "measurement noise budget" per dissent |
| Too many items, no prioritization | Stream ordering + dependency graph = clear priority. Tier 0 first, always. |
