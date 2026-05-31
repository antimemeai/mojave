# Wave 2: Statistical Foundations -- What's Actually Correct

Date: 2026-05-30
Agent: claude-opus-4-6
Built on: Adversary findings 1, 2, 6; Web finding on Koning2025; Codebase finding on confidence sequences

---

## Thesis

mojave's statistical engine has three load-bearing defects, two of which are independently fatal to the confidence sequence pipeline. (A) The `AnytimeMonitor` CI uses Welford's estimated sigma, reducing anytime-valid coverage from the guaranteed 95% to 46% at p=0.5 and 17% at p=0.1 on Bernoulli data. (B) The production confseq pipeline declares `DataFamily::Bernoulli` but routes binary MCQ data through the Gaussian normal-mixture CS formula, which assumes continuous sub-Gaussian data with known variance. (C) Twenty zero-sample cells in WMDP-Bio inject accuracy=0.0 as real data into the Sobol estimator, corrupting the variance decomposition -- but this is dwarfed by a larger design issue: the "bare" prompt template accounts for ~90% of the total output variance, making the headline finding ("prompt template dominates") technically correct but operationally vacuous. All three defects have clean fixes that use primitives already present in the codebase.

---

## Bibliography

Sources traced during this analysis:

| ID | Source | Role |
|----|--------|------|
| B1 | `crates/seq-anytime-valid/src/monitor/anytime.rs` | Production AnytimeMonitor with Welford sigma |
| B2 | `crates/seq-anytime-valid/src/evidence/confseq.rs` | `normal_mixture_cs` (estimated sigma) vs `normal_mixture_cs_known_sigma` |
| B3 | `crates/seq-anytime-valid/src/monitor/bernoulli.rs` | BernoulliMonitor (correct Beta-mixture mSPRT, but no CI output) |
| B4 | `crates/seq-anytime-valid/src/evidence/msprt.rs` | `bernoulli_msprt_log_lr` (Johari et al. 2022) and `gaussian_msprt_log_lr` |
| B5 | `crates/mojave-gsa/src/confseq.rs` | Production confseq pipeline: uses AnytimeMonitor with `DataFamily::Bernoulli` |
| B6 | `crates/mojave-gsa/src/analyze.rs` | Saltelli 2010 Eq c / Jansen 1999 Eq f estimator |
| B7 | `crates/seq-anytime-valid/tests/gate4_monte_carlo.rs` | Gate 4 test: validates `normal_mixture_cs_known_sigma` only |
| B8 | `crates/eval-orchestrator/src/instruments/sequential.rs` | Eval orchestrator: uses AnytimeMonitor with `DataFamily::Normal { known_variance: None }` |
| B9 | `data/v2/bio_sobol_analysis.json`, `chem_sobol_analysis.json`, `truthfulqa_sobol_analysis.json` | Sobol analysis outputs |
| B10 | `data/v2/bio_results_gsa.json` | Raw results with 20 zero-sample cells |
| B11 | `data/v2/manifest_bio_512.json` | Saltelli design manifest for bio |
| B12 | Howard et al. 2021, "Time-uniform, nonparametric, nonasymptotic confidence sequences" | The CS formula mojave implements |
| B13 | Waudby-Smith & Ramdas 2024, "Estimating means of bounded random variables by betting" | The correct CS for [0,1] data |
| B14 | Koning & van Meer 2025, "Anytime Validity is Free" | Sequentialized z/t-tests from fixed-sample tests |
| B15 | Johari et al. 2022, "Always Valid Inference: Continuous Monitoring of A/B Tests" | Beta-mixture mSPRT for Bernoulli data |
| B16 | Saltelli 2010, "Variance Based Sensitivity Analysis of Model Output" | First-order estimator (Eq c) |
| B17 | Jansen 1999, "Analysis of variance designs for model output" | Total-order estimator (Eq f) |

---

## Analysis

### Issue 1: Estimated sigma voids anytime-valid coverage guarantee

**Code path traced.** The production CI in `AnytimeMonitor::update()` (B1, lines 66-76) computes:

```
variance = welford_m2 / n
sigma = sqrt(variance).max(1e-10)
width = sigma * sqrt(2 * (1 + 1/n) * ln(sqrt(n+1) / alpha) / n)
```

The docstring on `normal_mixture_cs` (B2, line 16) explicitly warns: "using a sample estimate does not preserve the coverage guarantee." The `normal_mixture_cs_known_sigma` function exists for when sigma is known, but `AnytimeMonitor` never uses it.

**Why this breaks.** The Howard et al. 2021 CS boundary (B12) is a super-martingale under the true data-generating distribution ONLY when sigma is the true population standard deviation. With estimated sigma, the width becomes a random variable that is correlated with the data. At early stopping times (small n), sigma is poorly estimated and the width is too narrow. This is not a subtle asymptotic issue -- it is a fundamental violation of the martingale property that the anytime-valid guarantee depends on.

**Monte Carlo verification.** I ran 10,000 replications of the exact code path with Bernoulli(p) data and n_max=200:

| True p | Coverage (estimated sigma) | Coverage (known sigma) | Coverage (sigma=0.5 upper bound) |
|--------|---------------------------|----------------------|----------------------------------|
| 0.1    | 17.3%                     | --                   | 99.9%                            |
| 0.2    | 30.7%                     | --                   | --                               |
| 0.3    | --                        | --                   | 98.9%                            |
| 0.5    | 46.3%                     | 98.3%                | 98.4%                            |
| 0.7    | --                        | --                   | 98.9%                            |
| 0.8    | 30.3%                     | --                   | --                               |
| 0.9    | 16.8%                     | --                   | 100.0%                           |

Coverage with estimated sigma is catastrophically anti-conservative. At p=0.5 it is 46% instead of 95%. At extreme p values (0.1, 0.9) it drops to 17%. The intervals are far too narrow.

**Root cause.** Welford's estimated sigma converges to sqrt(p(1-p)), which is correct asymptotically. But in the early samples (n < 30), the estimate is noisy, and the CS width is a function of this noisy estimate. The anytime-valid guarantee requires that the width is fixed before seeing the data (or is a function of a known upper bound on sigma). Using a data-dependent estimate destroys the guarantee regardless of sample size.

**Why the Gate 4 test missed it.** The Gate 4 Monte Carlo test (B7, lines 86-128) tests `normal_mixture_cs_known_sigma` with sigma=1.0 on N(0,1) data. This is the correct function with the correct sigma. It achieves coverage in [0.93, 0.99] as expected. But the production code path (`AnytimeMonitor::update()`) is never tested for coverage. The test validates a different function than the one deployed.

**The eval-orchestrator path is also affected.** The `SequentialInstrument` (B8, lines 22-28) constructs an `AnytimeMonitor` with `DataFamily::Normal { known_variance: None }` and feeds it `outcome_to_f64` values from `TrialRecord`. When `Outcome::Binary(true/false)` is converted to 1.0/0.0, this is Bernoulli data going through the Gaussian estimated-sigma path. Same defect, different entry point.

### Issue 2: Gaussian mSPRT on Bernoulli data

**Code path traced.** The confseq pipeline (B5, lines 78-83) constructs:

```rust
MsprtConfig {
    theta_0: 0.5,
    mixing_variance: 1.0,
    family: DataFamily::Bernoulli,
    max_samples: None,
};
```

The `DataFamily::Bernoulli` field is set but never consumed by `AnytimeMonitor`. The `AnytimeMonitor::new()` constructor (B1, lines 18-34) takes `MsprtConfig` but ignores the `family` field entirely. It always uses the Gaussian mSPRT formula for the log-likelihood ratio and the normal-mixture CS formula for the confidence interval.

**What the code declares vs what it does.**

| Aspect | Declared | Actual |
|--------|----------|--------|
| Data family | `DataFamily::Bernoulli` | Gaussian formulas used |
| LR formula | Should be Beta-mixture (Johari 2022) | Gaussian mSPRT: `-0.5*ln(1+n*tau^2) + ...` |
| CI formula | Should be Bernoulli/bounded-mean | Normal-mixture CS with estimated sigma |
| sigma | Should be sqrt(p(1-p)) or bounded by 0.5 | Welford's running estimate |

**The right primitive already exists.** `BernoulliMonitor` (B3) correctly implements the Beta-mixture mSPRT from Johari et al. 2022 for the log-likelihood ratio and always-valid p-value. However, it returns `confidence_interval: None` (B3, line 60). To fix the confseq pipeline, `BernoulliMonitor` needs a confidence interval output, or a separate bounded-mean CS must be computed.

**Why this matters operationally.** The confseq pipeline (B5) monitors CI width for early stopping: it stops when `hw < half_width_threshold`. With the wrong distributional family, the CI width converges at the wrong rate and the stopping decision is based on an invalid interval. The stopping times reported in the confseq analysis are meaningful only as exploratory diagnostics, not as statistically valid stopping rules.

**The Waudby-Smith & Ramdas 2024 solution (B13).** The plan explicitly recommends "betting-based confidence sequence for every cell where the score is a bounded mean." The betting CS (also called the hedged capital confidence sequence) works for any [0,1]-bounded data without requiring known sigma or a distributional assumption. It achieves tighter intervals than the normal-mixture CS with sigma=0.5, especially near p=0 and p=1. This is the correct solution for MCQ accuracy data.

**Koning & van Meer 2025 (B14) provides an alternative.** Their theorem shows any valid fixed-sample test can be "sequentialized" into an anytime-valid test. For the Bernoulli case, this means the standard Wilson score interval can be sequentialized. The practical advantage: mojave already computes Wilson CIs in `analyze_sobol.py` (line 44-56). A sequentialized Wilson would reuse this infrastructure.

### Issue 3: Sobol convergence diagnostics and zero-sample cells

**Zero-sample cells: confirmed but less impactful than expected.** Twenty cells in bio_results_gsa.json have `n_samples=0` and `accuracy=0` (B10). These are distributed across the Saltelli matrix segments:

| Segment | Zero-sample cells | Description |
|---------|-------------------|-------------|
| A (0-511) | 5 | Base matrix A |
| A_B[0] (1024-1535) | 3 | Radial matrix for prompt_template |
| A_B[4] (3072-3583) | 3 | Radial matrix for decoding |
| A_B[5] (3584-4095) | 9 | Radial matrix for quantization |

The zero-sample cells are NOT concentrated in one prompt template: 7 use verbose-rationale, 5 use bare, 4 use cot, 3 use lm-eval-default, 1 uses letter-only. This suggests infrastructure failures (RunPod pod failures, vLLM timeouts) rather than a systematic data collection issue.

**Impact on Sobol indices.** 20/4096 = 0.49% of cells. These inject accuracy=0.0 (coded as `"accuracy": 0` in JSON, parsed as `Some(0.0)` in Rust since the field is `Option<f64>`) into the output vector y. The Rust analyzer (B6, line 257) requires non-null accuracy but treats 0.0 as valid: `cell.accuracy.with_context(...)`. Twenty spurious zeros slightly inflate the total variance (the denominator of S1 and ST) and corrupt specific rows in the fa, fb, fab vectors. Given N=512, 20 corrupted rows affect ~3.9% of the A matrix and ~4.7% of A_B[5].

**But this is not the main problem.** The "bare" prompt template is the dominant source of variance by a factor that dwarfs the zero-cell corruption.

**Bare prompt dominance: quantified.** From the manifest and results data:

| Metric | Bare cells (n=816) | Non-bare cells (n=3280) |
|--------|-------------------|------------------------|
| Mean accuracy | 0.140 | 0.710 |
| Min accuracy | 0.000 | 0.000 |
| Max accuracy | 0.590 | 0.832 |
| Variance | ~0.018 | ~0.006 |

Total output variance with all cells: 0.0582. Without bare cells: 0.0061. **The bare prompt accounts for ~89.5% of the total variance.** This means the headline finding "prompt_template explains S1=0.85 of variance" is almost entirely driven by one pathological level (bare) that produces near-zero accuracy because it provides no instructions.

**Sobol diagnostics: what the numbers mean.**

| Benchmark | sum_S1 | sum_ST | Interpretation |
|-----------|--------|--------|---------------|
| WMDP-Bio  | 0.903  | 1.295  | Significant interactions; convergence concerns |
| WMDP-Chem | 0.939  | 1.107  | Moderate interactions; marginal convergence |
| TruthfulQA| 0.993  | 1.068  | Near-additive; reasonable convergence |

sum_ST > 1.0 is expected when interaction effects exist (total-order indices double-count interactions). But sum_ST = 1.295 for bio is high. The CI widths tell the convergence story:

For bio prompt_template: S1 = 0.852, CI = [0.658, 1.034]. The CI width is 0.376, which is 44% of the point estimate. The plan says "if dominant-factor CI exceeds 10% of estimate, double N." This threshold is exceeded by 4.4x and was not acted on.

**Negative S1 values.** Bio shows S1_quantization = -0.070 and S1_decoding = -0.006. Negative first-order Sobol indices are estimation artifacts from the Saltelli 2010 Eq (c) estimator when: (a) the true S1 is near zero, (b) N is too small relative to the number of levels, or (c) the output is dominated by one factor (making the variance decomposition numerically fragile for the remaining factors). All three conditions hold here. The negatives do not indicate a mathematical error in the implementation -- they indicate insufficient N for factors whose true sensitivity is small relative to the dominant factor.

**N=512 is marginal for k=6 factors with 5 levels.** The Saltelli 2010 estimator needs O(1/sqrt(N)) convergence for S1 CIs. With k=6 and the dominant factor consuming ~85% of variance, the residual variance available for estimating the other 5 factors is very small. An N that is adequate for the dominant factor is inadequate for the minor factors. This is a structural limitation of the one-at-a-time radial design when factors have highly unequal influence.

### Interaction effects: what sum_ST > 1 tells us

The gap between sum_S1 (0.903) and sum_ST (1.295) for bio is 0.392. This means interaction effects contribute approximately 39% of total variance. The dominant interaction is almost certainly prompt_template x quantization (since quantization has the second-largest ST despite negative S1, suggesting it matters only in combination with other factors).

The implemented code does not compute second-order Sobol indices (S2_ij), though BEAD-0016 mentions S2 indices are implemented in salib-rs. Computing S2 would confirm the interaction structure and clarify whether the sum_ST excess is due to pairwise interactions or higher-order effects.

---

## What This Changes

### 1. Confidence sequence pipeline: broken, needs immediate fix

The confseq pipeline's CI output is statistically invalid. The CIs are not anytime-valid -- they are not even valid at fixed n for Bernoulli data (the Welford sigma estimate makes them too narrow). Any stopping decision based on these CIs is unreliable.

**Immediate fix (conservative but correct):** Replace estimated sigma with sigma=0.5 (the maximum standard deviation for Bernoulli data, achieved at p=0.5). This produces a valid CS because sigma=0.5 is a known upper bound, making the CS conservative (wider than necessary at extreme p). Monte Carlo verification shows coverage >= 98.4% at all tested p values.

**Correct fix (tighter intervals):** Implement the Waudby-Smith & Ramdas 2024 betting CS for [0,1]-bounded data. This achieves near-optimal width without requiring known sigma. It is the method the plan explicitly recommends.

**Alternative fix (simpler, via Koning 2025):** Sequentialize the Wilson score interval. Since mojave already computes Wilson CIs in Python, a sequentialized version would provide anytime-valid CIs with minimal new code. The Koning construction guarantees that the sequentialized test matches the fixed-sample Wilson test at the design sample size.

### 2. AnytimeMonitor must dispatch on DataFamily

The `AnytimeMonitor` accepts `DataFamily` in its config but ignores it. The fix: when `family == DataFamily::Bernoulli`, use either (a) sigma=0.5 for the normal-mixture CS, or (b) a bounded-mean CS, or (c) add a CI to `BernoulliMonitor`. The current architecture where `DataFamily` is declared but ignored is actively misleading.

### 3. Gate 4 test gap: add coverage test for production code path

A Gate 4 Monte Carlo test must be added that tests `AnytimeMonitor::update().confidence_interval` with Bernoulli(p) data for multiple p values. This test should fail with the current estimated-sigma code and pass after the fix.

### 4. Sobol: data quality gate before analysis

The `analyze()` function in `mojave-gsa/src/analyze.rs` should reject or flag cells with `n_samples=0`. Options:
- Fail hard: refuse to analyze if any cell has zero samples (current behavior for `accuracy: null`, but `accuracy: 0` passes through)
- Warn and impute: replace zero-sample cells with the mean of cells sharing the same factor levels (requires manifest lookup)
- Warn and continue: flag zero-sample cells in the diagnostics output so downstream consumers can assess impact

Recommendation: fail hard. The Sobol estimator requires a complete output vector. Missing data cannot be imputed without introducing bias, and the bias direction depends on which factor levels the missing cells correspond to.

### 5. Sobol: "bare" prompt needs separate treatment

The finding "prompt_template dominates" is driven by one pathological level. Options:
- Remove "bare" and rerun: this tests sensitivity among realistic configurations
- Report both: "with bare: S1_prompt=0.85; without bare: S1_prompt=X" (where X will be much smaller)
- Leave-one-level-out analysis: systematically drop each level and report robustness of S1

Recommendation: report both. The "bare" finding is scientifically valuable (it quantifies how much a completely uninstructed prompt degrades accuracy) but operationally misleading if presented as "prompt engineering is the key lever." The actionable finding is the non-bare Sobol decomposition.

### 6. Sobol: N needs doubling for bio

The plan's own threshold ("if dominant-factor CI exceeds 10% of estimate, double N") was exceeded at 44%. N should be doubled from 512 to 1024, yielding 8192 cells for bio. This is computationally feasible (the codebase already handles 4096 cells per benchmark).

---

## Gaps and Open Questions

1. **BernoulliMonitor has no CI output.** The Beta-mixture mSPRT (Johari 2022) produces valid e-values and p-values but not a confidence interval. To use BernoulliMonitor in the confseq pipeline, either: (a) invert the e-process to construct a CI (Ramdas et al. 2023 Proposition 2), or (b) pair the BernoulliMonitor e-value with a separate bounded-mean CS.

2. **The eval-orchestrator sequential path has the same defect.** `SequentialInstrument` (B8) uses AnytimeMonitor with `DataFamily::Normal { known_variance: None }` and feeds it binary outcomes. This path is used for live eval monitoring, not just retrospective analysis. Fixing the confseq pipeline without fixing the orchestrator leaves the live path broken.

3. **What is the correct sigma for Score outcomes?** Binary outcomes should use the bounded-mean CS (sigma <= 0.5). But `Outcome::Score(f64)` can be any real number. For Score data, the user must supply a known sigma or range bound. The current architecture has no mechanism for this. This is an API design question, not just a bug fix.

4. **Coverage simulation for the Waudby-Smith & Ramdas betting CS has not been done.** Before implementing, a Monte Carlo calibration card should verify that the betting CS achieves the claimed coverage on Bernoulli data at multiple p values. This is the Gate 4 test for the replacement code.

5. **Sobol second-order indices (S2) are not computed in production.** The interaction structure hinted at by sum_ST > 1 cannot be confirmed without S2. salib-rs has the estimator (BEAD-0016), but mojave-gsa does not call it. Adding `--compute-second-order` would require the AB_i matrix in the Saltelli design (calc_second_order=true), which doubles the number of cells from N(k+2) to N(2k+2). At N=512, k=6: 7168 cells instead of 4096.

6. **No data quality diagnostic in the analysis output.** The analysis JSON reports sobol_diagnostics with sum_s1 and sum_st, but does not report convergence quality metrics (e.g., bootstrap coefficient of variation, effective sample size, or the plan's own "CI width / estimate" diagnostic). Adding these would make the "double N" decision automated.

7. **The Python Wilson CI in analyze_sobol.py guards on n_samples > 0** (line 129: `cell.get("n_samples", 0) > 0`), but the Rust analyzer does not. There is an inconsistency in data quality handling between the Python wrapper and the Rust core.

---

## Acquisitions

No papers downloaded. All referenced papers are already in the library (Howard 2021, Waudby-Smith & Ramdas 2024, Koning 2025, Johari 2022, Saltelli 2010, Jansen 1999) or in intake (Koning2025_AnytimeValidityFree.pdf).

---

## ASU Shopping List

| Paper | Why needed | Priority |
|-------|-----------|----------|
| Waudby-Smith & Ramdas 2024, "Estimating means of bounded random variables by betting" (Annals of Statistics) | The correct CS for mojave's MCQ data. Need full text for implementation details of the hedged capital process | HIGH |
| Howard, Ramdas, McAuliffe & Sekhon 2021, "Time-uniform, nonparametric, nonasymptotic confidence sequences" (Annals of Statistics) | Already in library. Re-read Section 4 (sub-Gaussian CS) and Section 5 (empirical Bernstein CS) for the sub-Gaussian variant that accommodates bounded data | MEDIUM |
| Ramdas, Grunwald, Vovk & Shafer 2023, "Game-theoretic statistics and safe anytime-valid inference" (Statistical Science) | Already in library. Proposition 2 (inverting e-processes to CIs) is the key to getting CIs from BernoulliMonitor | MEDIUM |
| Zhang et al. 2015, "Sobol sensitivity analysis: a tool to guide the development and evaluation of systems pharmacology models" (CPT: Pharmacometrics & Systems Pharmacology) | Cited in the plan for the "N in [10^2, 10^4]" guideline. Need to verify the convergence rate claim for discrete factor spaces | LOW |
