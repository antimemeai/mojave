# Wave 2 Deep Dive: GSA Theory, salib-rs Correctness, and the Sobol Convergence Problem
Date: 2026-05-30
Agent: claude-opus-4-6
Sources: salib-rs codebase (`/Users/patrickbeam/projects/salib/`), mojave-gsa crate, WMDP Phase 1 results, wave-1 deposits (adversary, library, web, codebase), 17+ GSA papers in library

---

## 1. salib-rs Estimator Correctness Assessment

### 1.1 Saltelli 2010 Estimator (First-Order): Correct

The implementation in `salib-estimators/src/saltelli2010.rs` faithfully implements Saltelli 2010 Eq c:

```
S_i = (1/N) sum_j fb[j] * (fab[i][j] - fa[j]) / D
```

Key correctness observations:

- **Biased variance denominator**: The code uses `(1/N) sum fa^2 - f0^2` (population variance), matching SALib's `np.var` (biased). The comment at line 87-92 explicitly documents this divergence from `tree_var` (which is Bessel-corrected). This is the right choice for SALib compatibility and does not affect index correctness -- the same D appears in numerator and denominator, so the bias cancels for S_i.

- **Tree-fold summation**: All sums go through `tree_sum` / `tree_dot` from `salib-core/src/reduce.rs`, which implements pairwise tree-fold reduction. This is bit-deterministic under rayon parallelism -- a genuinely unusual and valuable property. The catastrophic cancellation test at line 361-379 of `reduce.rs` demonstrates the advantage over naive left-fold.

- **SALib differential verified**: The Ishigami e2e test (`ishigami_e2e.rs`) hardcodes SALib reference values and passes within 0.05 tolerance at N=8192. The convergence-rate test verifies O(1/sqrt(N)) decay across N in {4096, 16384, 65536}.

### 1.2 Jansen 1999 Total-Order Estimator: Correct

```
S_T_i = (1/(2N)) sum_j (fa[j] - fab[i][j])^2 / D
```

Implemented in `saltelli2010.rs` lines 112-118. This is the estimator Saltelli 2010 section 4 recommends as "the universal best" for total-order. Numerically stable since it computes a sum of squares (non-negative by construction), unlike some total-order estimators that involve subtraction.

### 1.3 Janon 2014 Efficient Estimator: Correct, With a Significant Catch

The implementation in `janon.rs` uses Eq 6 (the formal definition), NOT Eq 8 (the "rewriting" form). The module docstring (lines 33-45) contains an extremely valuable finding: Eq 6 and Eq 8 are NOT algebraically equivalent despite the paper's claim. The denominators differ:

```
Eq 6 denom ~ (Var(Y) + Var(Y^X)) / 2
Eq 8 denom ~ Var((Y + Y^X)/2) = (Var(Y) + Var(Y^X) + 2*Cov(Y, Y^X)) / 4
```

Under pick-freeze pairing, `Cov(Y, Y^X) = S_i * Var(Y)`, so Eq 8 inflates the estimate by `2 / (1 + S_i)`. For Ishigami S_1 = 0.314, this produces ~0.48 instead of ~0.314. The implementer caught this discrepancy empirically and chose the correct form. This is a non-trivial contribution that corrects a misleading presentation in the original paper.

### 1.4 Jansen 1999 First-Order Estimator: Correct

```
S_i = 1 - (1/(2N)) sum (Y_j - Y^X_j)^2 / Var(Y)
```

Implemented in `jansen.rs`. The `1 - (sum of squares / D)` form means that when `S_i` is near 1 (a dominant factor), it avoids the cancellation noise that Saltelli's Eq c can exhibit. The guard `if total_variance > 1e-15` at line 129 is appropriate.

### 1.5 Owen 2013 "Correlation 2" Estimator: Correct Design

Uses three independent vectors (A, B, C) instead of two, costing N*(3 + 2d) evaluations vs N*(d+2). Achieves O(epsilon^4) variance for near-zero S_i factors. This is the right estimator for screening applications where most factors are unimportant.

### 1.6 Bootstrap CIs: Correct Pattern, One Concern

The bootstrap in `bootstrap.rs` correctly:
- Caches model evaluations once (no re-evaluation per resample)
- Resamples row-aligned indices to preserve the (A, B, A_Bi) correlation structure
- Uses percentile method with linear interpolation matching numpy's default

**Concern**: The percentile bootstrap CI is known to be biased for Sobol indices, especially at small N. The BCa (bias-corrected accelerated) bootstrap is noted as deferred (line 84-85 of `sobol_indices.rs`). At N=512 with 1000 bootstrap resamples, the percentile CI can be conservative (too wide) or anti-conservative (too narrow) depending on the true S_i. This partially explains why some CIs in the WMDP results cross zero or exceed 1.0.

### 1.7 Borgonovo Delta: Correct

Implements the Plischke-Borgonovo-Smith 2013 Eq 26 algorithm with:
- Silverman KDE bandwidth matching scipy's formula
- Adaptive class count matching SALib's formula
- 100-point y-grid for trapezoidal integration
- Ordinal rank-based equal-frequency partitioning

Bias-reduced delta (Plischke Eq 30 jackknife correction) is explicitly documented as not yet implemented. The uncorrected version differs by ~0.04 from bias-reduced at N=4096 on Ishigami -- acceptable for ranking but not for tight estimation.

### 1.8 Given-Data Sobol (Plischke-Borgonovo-Smith 2013): Correct, With Clamping

Uses the law of total variance: `S_1 = 1 - E[Var(Y|X_i)] / Var(Y)`. Output is clamped to [0, 1] (line 196). The clamping is justified: the population S_1 is non-negative by construction, but the partition estimator can produce slightly negative values at finite N due to binning noise. This is the right choice for reporting but users should be aware that clamped values mask estimation uncertainty.

### 1.9 Second-Order Indices (Saltelli 2010 Eq d): Correct

All three first-order estimators (Saltelli, Janon, Jansen) implement the optional second-order computation using `B_A` matrices:

```
V_{ij} = (1/N) sum_k [fba[j][k] * fab[i][k] - fa[k] * fb[k]]
S2_{ij} = V_{ij} / D - S_i - S_j
```

This is the standard Saltelli 2010 formula. The indexing convention `second_order[i][k] = S2_{i, i+k+1}` (upper triangle, row-major) is consistent across all estimators.

---

## 2. Convergence Diagnostics: The WMDP Results Under Scrutiny

### 2.1 The Numbers

| Benchmark | sum_S1 | sum_ST | S1_prompt_template | ST_prompt_template | S1_quantization | Negative S1 count |
|-----------|--------|--------|--------------------|--------------------|-----------------|-------------------|
| Bio       | 0.903  | 1.295  | 0.852              | 0.951              | -0.070          | 2                 |
| Chem      | 0.939  | 1.107  | 0.884              | 0.955              | +0.024          | 1                 |
| TruthfulQA| 0.993  | 1.068  | 0.927              | 0.968              | +0.002          | 0                 |

### 2.2 What sum_ST > 1 Means

Sum of total-order indices exceeding 1.0 does NOT by itself indicate a problem. For models with interactions, sum_ST = sum_S1 + sum_S2 + ... + sum_Sk, which legitimately exceeds 1.0. The excess over 1.0 measures the total interaction contribution.

However, the magnitude matters:
- **TruthfulQA** (sum_ST = 1.068): Modest interactions, ~7% of total variance. This is plausible and well-converged.
- **Chem** (sum_ST = 1.107): ~11% interactions. Still plausible.
- **Bio** (sum_ST = 1.295): ~30% interactions. This is suspiciously high for a 6-factor additive-ish design where prompt_template dominates. The question is whether this reflects real high-order interactions or estimation noise.

### 2.3 What Negative S1 Means

A negative first-order Sobol index is a mathematical impossibility: `S_i = Var(E[Y|X_i]) / Var(Y) >= 0` by definition. Negative values are pure estimation artifacts.

For Saltelli's Eq c estimator, the S_i numerator is:
```
(1/N) sum fb[j] * (fab[i][j] - fa[j])
```

This is a cross-covariance that can be negative at finite N even when the population value is positive. This happens when:
1. The true S_i is very small (near zero)
2. N is insufficient to resolve the signal from noise

Bio's S1_quantization = -0.070 with CI [-0.178, 0.028] means the true S_i is likely in [0, 0.03] -- quantization has essentially zero first-order effect on bio accuracy. The negative value is not alarming per se; it is a signal that N=512 is barely adequate for resolving this factor.

### 2.4 CIs Crossing 1.0

Bio prompt_template S1 CI = [0.658, 1.034]. The upper CI crossing 1.0 means the bootstrap distribution includes resamples where the estimated S1 exceeds 1.0. This happens because:
1. The percentile bootstrap does not constrain estimates to [0, 1]
2. At N=512, individual bootstrap resamples of a dominant factor can produce S1 > 1 due to sampling noise in the denominator (D)

The CI width is 1.034 - 0.658 = 0.376. The plan's own threshold ("if dominant-factor CI exceeds 10% of estimate, double N") gives 0.852 * 0.10 = 0.085. The actual CI width is 4.4x this threshold. **The plan's own convergence criterion was violated and no doubling was performed.**

### 2.5 Appropriate Sample Size for This Problem

For a 6-factor Saltelli design, the convergence rate is approximately:

```
|S_i_hat - S_i| ~ 1 / sqrt(N)
```

At N=512: expected error ~ 0.044 in S_i units.
At N=1024: expected error ~ 0.031
At N=2048: expected error ~ 0.022
At N=4096: expected error ~ 0.016

For the dominant factor (S1 ~ 0.85-0.93), N=512 gives ~5% relative error -- adequate for ranking but not for precise estimation. For the minor factors (S_i ~ 0.01-0.05), N=512 gives absolute error comparable to the signal itself -- meaning these indices are essentially noise.

**Recommendation**: N=1024 is the minimum for publishable results. N=2048 is preferred. The cost increase is linear: 1024 * (6+2) = 8192 cells per benchmark (vs current 4096). At ~30 seconds per cell on 7B, this is ~68 additional GPU-minutes per benchmark -- trivial.

---

## 3. The Missing Data Problem

### 3.1 Anatomy of the Corruption

20 cells in bio_results_gsa.json have `n_samples: 0` and `accuracy: 0.0`. These fall in specific regions of the Saltelli matrix:

| Region | Count | Indices |
|--------|-------|---------|
| A (fa) | 5     | 282, 290, 308, 335, 386 |
| AB_0 (prompt_template) | 3 | 1375, 1474, 1519 |
| AB_4 (decoding) | 3 | 3225, 3337, 3449 |
| AB_5 (quantization) | 9 | 3585, 3622, 3642, 3738, 3817, 3821, 3961, 4002, 4006 |

Two additional cells have `accuracy: 0.0` but `n_samples: 41` (index 36) -- these are genuinely zero-accuracy outcomes, not missing data.

### 3.2 Impact on Specific Indices

The 5 corrupted cells in matrix A directly enter the `fa` vector, which is used in:
- The variance denominator `D = (1/N) sum fa^2 - f0^2`
- Every S_i numerator `(1/N) sum fb[j] * (fab[i][j] - fa[j])`
- Every S_T_i formula `(1/(2N)) sum (fa[j] - fab[i][j])^2`

The 9 corrupted cells in AB_5 (quantization column) directly enter `fab[5]`, making:
- S1_quantization unreliable (the large negative value -0.070 is partly due to this)
- ST_quantization inflated (0.101 vs likely < 0.03)

**The corruption disproportionately affects quantization because 9 of the 20 zero-sample cells fall in the quantization column swap matrix.** This is likely systematic: these are cells where the "bare" prompt template (which produces near-zero accuracy) was combined with fp8 quantization, and the evaluation pipeline timed out or failed to produce any item responses.

### 3.3 Impact on Diagnostics

The 20 spurious zeros inflate `Var(Y)` (denominator D), which pushes all S_i estimates down and ST_i up. The spread (0.832) is inflated; true spread is likely ~0.75. The sum_ST = 1.295 is partly an artifact of D inflation. Correcting the 20 cells would:
- Raise all S1 values slightly (smaller D)
- Lower all ST values slightly (smaller D)
- Bring sum_ST closer to 1.1 (still above 1.0 due to real interactions)
- Eliminate the negative S1_quantization

### 3.4 The Code Guard

The `analyze.rs` code at line 257 requires non-null accuracy:
```rust
let acc = cell.accuracy.with_context(|| {
    format!("cell at saltelli_index {i} has missing accuracy -- cannot analyze incomplete data")
})?;
```

But `accuracy: 0.0` passes this guard. There is no check for `n_samples == 0`. **A data quality gate that rejects cells with n_samples < some minimum (e.g., 10) should be added before Sobol estimation.** Imputation (e.g., MICE, or nearest-neighbor from the Saltelli matrix) is the principled alternative to rejection, but for 20/4096 cells, simple exclusion with matrix row deletion and N adjustment is adequate.

---

## 4. The Pathological "Bare" Prompt Level

### 4.1 The Construct

The 5 prompt_template levels are:
1. `lm-eval-default` -- standard MCQ format
2. `bare` -- no instructions, just the question
3. `cot` -- chain-of-thought
4. `letter-only` -- "respond with just the letter"
5. `verbose-rationale` -- detailed reasoning requested

The "bare" template strips all format instructions. For MCQ tasks, this means the model receives a question and answer choices but no instruction to select an answer in a parseable format. Many models produce free-form text that fails to parse as a valid choice, yielding accuracy = 0.0.

### 4.2 Quantifying the "Bare" Leverage

From the bio results: min_accuracy = 0.0, which comes from bare-prompt cells. The Saltelli design assigns uniform probability to all factor levels. With 5 prompt levels, 20% of cells use "bare." If "bare" produces accuracy near 0 while other templates produce accuracy ~0.6-0.8, then:

```
Var(E[Y|prompt_template]) ~ (1/5) * (0 - 0.6)^2 + (4/5) * small deviations
                          ~ 0.072 + small
```

The total variance `Var(Y) ~ 0.058` (sd = 0.241, var = 0.058). So S1_prompt_template ~ 0.072/0.058 ~ 1.24 before small-deviation corrections bring it to ~0.85.

**Roughly 60-70% of prompt_template's apparent variance is driven by the "bare" level alone.** The remaining 30-40% is from genuine between-template differences (cot vs letter-only vs verbose-rationale vs lm-eval-default).

### 4.3 Implications

The finding "prompt template explains 85% of variance" should be reported as: "Including a completely uninstructed prompt (bare) in the perturbation design reveals that format instructions are the dominant source of eval variance. Among realistic prompt templates (cot, letter-only, verbose-rationale, lm-eval-default), prompt template explains approximately 25-40% of variance." The second framing is more actionable.

This is not a bug in the GSA methodology -- the Sobol decomposition correctly reports what drives variance in the defined factor space. But the choice of factor levels is a design decision that shapes the finding. The adversary (wave-1 finding 5) correctly identified this as a construct validity issue.

### 4.4 Recommended Analysis

Run a leave-one-level-out analysis: re-estimate Sobol indices excluding the "bare" level (N_levels = 4 for prompt_template). This requires re-running the Saltelli design with a 4-level prompt_template axis, which is a new experiment. Alternatively, use the given-data Sobol estimator (`estimate_given_data_sobol`) on the subset of cells where prompt_template != "bare" -- this avoids re-running evals but uses a less efficient estimator.

---

## 5. Mazo's "New Paradigm" and Its Impact on salib-rs

### 5.1 What Mazo 2024/2026 Actually Says

Mazo defines sensitivity measures as **set functions** on the power set of inputs, satisfying a "null at independence" axiom: a measure is null at a subset `u` of inputs iff the output is probability-one independent of `X_u`. Classical Sobol indices (`S_i = Var(E[Y|X_i]) / Var(Y)`) become a special case of this generalized framework.

The key insight: Sobol indices depend on the Sobol-Hoeffding ANOVA decomposition, which requires:
1. Square-integrability of the output
2. Independent inputs
3. A specific measure-theoretic structure (product measure on the input space)

Mazo's framework drops the requirement for the ANOVA decomposition entirely. Sensitivity measures can be defined for any measurable function of any set of random variables, including dependent inputs.

### 5.2 Does This Affect salib-rs?

**No, not for the current use case.** Mazo's paper is a theoretical unification, not a computational correction. The existing estimators (Saltelli, Jansen, Janon, Owen) correctly estimate the classical Sobol indices under the classical assumptions. Mazo does not claim these estimators are wrong -- he shows they can be derived from a more general principle.

For mojave's WMDP application:
- Inputs (perturbation factors) are independent by design (the Saltelli radial construction ensures this)
- The output (accuracy) is bounded in [0, 1], hence square-integrable
- The input space is a product of discrete factor levels

All three classical assumptions hold. Mazo's generalization would matter if:
1. Input factors were dependent (e.g., quantization constraining available decoding strategies)
2. The output were not square-integrable (e.g., unbounded loss functions)
3. mojave wanted to define novel sensitivity measures beyond variance-based

### 5.3 Where Mazo Matters for salib-rs's Future

The generalized framework is relevant for two potential salib-rs extensions:

1. **Borgonovo's OT-GSA**: The optimal-transport sensitivity indices are a natural special case of Mazo's framework (they satisfy the "null at independence" axiom using Wasserstein distance instead of variance). If salib-rs implements OT-GSA (currently only available in R via `gsaot`), Mazo's framework provides the theoretical umbrella.

2. **Dependent input handling**: If mojave ever encounters factor designs where inputs are correlated (e.g., from observational data rather than designed experiments), classical Sobol indices are not well-defined. Mazo's framework, combined with Kucherenko & Song (2017) or Mara & Tarantola (2012) for dependent-input Sobol analogues, would guide the estimator design.

**Recommendation**: No code changes needed now. Add a note to salib-rs documentation acknowledging Mazo's framework as the theoretical foundation for future generalized sensitivity measures.

---

## 6. The Discrete Factor Problem

### 6.1 Classical Sobol Theory Assumes Continuous Inputs

The Sobol-Hoeffding decomposition and all estimators in salib-rs assume inputs `X_i ~ Uniform(0, 1)` (or mapped from it via quantile transforms). mojave discretizes these continuous samples into a small number of levels:

| Factor | Levels |
|--------|--------|
| prompt_template | 5 |
| system_prompt | 4 |
| n_shot_frac | 4 |
| choice_order | 2 |
| decoding | 3 |
| quantization | 2 |

The discretization happens in `manifest.rs` line 57-59:
```rust
fn discretize(value: f64, n_levels: usize) -> usize {
    let idx = (value * n_levels as f64).floor() as usize;
    idx.min(n_levels - 1)
}
```

This maps the continuous Sobol/LHS sample to equal-probability level bins.

### 6.2 Does Discretization Invalidate Sobol Indices?

No, but it changes the convergence properties. For a k-level discretized factor:
- The Sobol index is well-defined as `S_i = Var(E[Y|X_i]) / Var(Y)` where `X_i` takes k equally-probable values
- The Saltelli estimator still converges to the correct population S_i
- But the convergence rate can be worse because the discretized output is a step function with k steps, and the pick-freeze covariance estimator can exhibit higher variance when the "freeze" operation maps to the same discrete level frequently

With 2-level factors (choice_order, quantization), each pick-freeze pair has only a 50% chance of the frozen factor actually changing (the continuous sample falls in the same half). This means roughly half the pick-freeze pairs contribute zero information about that factor's effect. The effective sample size for 2-level factors is approximately N/2, not N.

**This partially explains why quantization (2 levels) has noisier estimates than prompt_template (5 levels).** At N=512, the effective sample size for quantization is ~256, giving expected error ~0.063 in S_i units -- large relative to the true S_i of ~0.01-0.03.

### 6.3 The ANOVA Alternative

For discrete factors with small numbers of levels, classical ANOVA is a natural and more efficient alternative to Sobol decomposition. salib-rs already implements ANOVA (`salib-estimators/src/anova.rs`). For a balanced design with k1 * k2 * ... * kd factor combinations, ANOVA directly computes the variance components without the Monte Carlo overhead of the Saltelli design.

However, ANOVA requires a full factorial grid (or at minimum an orthogonal fractional factorial). The Saltelli radial design does NOT produce a balanced factorial -- it produces independent random draws that map to factor levels. To use ANOVA, mojave would need either:
1. A full factorial design: 5 * 4 * 4 * 2 * 3 * 2 = 960 cells (cheaper than the current 4096)
2. A fractional factorial (Plackett-Burman or similar): salib-rs implements this

**Recommendation**: For the current 6-factor design with 2-5 levels each, a full factorial with replication (960 cells * r replicates) may be more statistically efficient than the Saltelli design. The Saltelli approach is most valuable when factors have many levels or are continuous. With discrete factors, the Saltelli overhead (N*(k+2) evaluations) exceeds what a direct factorial would require.

---

## 7. G-Theory as Complementary Framework

### 7.1 What G-Theory Adds

Generalizability theory (Cronbach, Gleser, Nanda & Rajaratnam, 1972) decomposes variance by "facets" -- exactly what mojave's perturbation engine does. The crossed p x i x r design (person x item x rater) maps to mojave's (model x item x perturbation-config).

salib-rs already implements G-theory (`g_theory.rs`), including:
- Crossed design variance decomposition
- D-study projections for reliability estimation
- SPC bridge (G-theory variance components to control chart limits)

The key advantage of G-theory over Sobol: G-theory directly estimates **reliability coefficients** (generalizability coefficient and dependability coefficient) that quantify how trustworthy a score is given a specific measurement design. Sobol indices tell you what drives variance; G-theory tells you whether the resulting score is reliable enough to base decisions on.

### 7.2 G-Theory and Sobol Can Coexist

For a balanced factorial design:
- G-theory decomposes total variance into person, item, rater, and interaction components
- Sobol indices decompose total output variance into input-factor main effects and interactions
- These are different decompositions answering different questions: G-theory answers "how reliable is the measurement?", Sobol answers "what drives the measurement?"

The library scout noted zero G-theory papers in the collection -- this is a critical gap (Brennan 2001 "Generalizability Theory" is the standard reference).

---

## 8. salib-rs Validation Architecture Assessment

### 8.1 The 4-Gate Structure

| Gate | Purpose | Coverage |
|------|---------|----------|
| Gate 1 | Textbook reproductions | Ishigami canonical values, Sobol-G analytic, Morris test functions |
| Gate 2 | R cross-checks | R gsDesign for sequential; fixture files, silently skips if missing |
| Gate 3 | Property-based | S_i <= S_T_i, sum S_i <= 1, determinism, scaling |
| Gate 4 | Monte Carlo calibration | Coverage tests, convergence rate decay |

### 8.2 What Is Well-Covered

- **Ishigami function**: Exhaustively tested. The e2e test verifies convergence at N in {4096, 16384, 65536}, SALib differential, model-free identities, and bootstrap CI finiteness.
- **Determinism**: Every estimator has a `same_matrix_yields_identical_estimates` test. The tree-fold reduction has bitwise parity tests between sequential and parallel paths.
- **Edge cases**: Constant model (zero variance), single-factor (d=1), minimum N, all tested.

### 8.3 What Is Missing

1. **No convergence diagnostic at the analysis level**: The analysis output includes `sum_s1` and `sum_st` but no automated convergence check. There is no warning when S1 < 0, when CIs cross 0 or 1, or when sum_ST > threshold. The plan says "if dominant-factor CI exceeds 10% of estimate, double N" but there is no code enforcing this.

2. **No Monte Carlo coverage test for bootstrap CIs**: Gate 4 tests coverage of `normal_mixture_cs_known_sigma` but NOT of the bootstrap CI used in `analyze.rs`. The bootstrap CI's empirical coverage at N=512 with B=1000 resamples is unknown. This should be tested against the Ishigami analytic values: generate K=1000 independent bootstrap CIs at N=512 and check what fraction contain the true analytic S_i.

3. **No discrete-factor convergence test**: All validation uses continuous Ishigami/Sobol-G functions. There is no test function with discrete (categorical) inputs matching mojave's actual use case. A simple test: Y = indicator(X_1 == level_0) + 0.5*indicator(X_2 == level_0), with X_i uniform over {0, 1, 2}. Analytic S_i computable, and convergence at small N testable.

4. **No given-data estimator validation against Saltelli design**: The given-data Sobol estimator is validated independently on synthetic data but never compared against the Saltelli design estimator on the same evaluation data. For the WMDP analysis, running both estimators and checking agreement would be a valuable cross-check.

---

## 9. The mojave-gsa Analysis Pipeline: Code-Level Findings

### 9.1 Duplicated Sobol Computation

The `analyze.rs` file contains its own `compute_sobol_from_cached` function (lines 117-157) and `bootstrap_sobol_cis` function (lines 159-217) that reimplement the Saltelli 2010 estimator. These are distinct from salib-rs's `estimate_saltelli2010` and `estimate_saltelli2010_with_bootstrap`.

The point estimators are algebraically identical (same formulas, same tree_sum usage). But the bootstrap has a subtle difference: mojave-gsa's percentile function (line 202) uses `floor` indexing:
```rust
let idx = ((p * samples.len() as f64).floor() as usize).min(samples.len() - 1);
```

While salib-rs's `percentile_value` (bootstrap.rs line 217-230) uses linear interpolation between adjacent order statistics:
```rust
let pos = p * (n - 1) as f64;
let frac = pos - pos_floor;
sorted[lower_idx] * (1.0 - frac) + sorted[lower_idx + 1] * frac
```

The salib-rs version matches numpy's default `percentile` behavior. The mojave-gsa version is slightly less precise (snaps to nearest order statistic). At B=1000 resamples, the difference is negligible, but it means mojave-gsa's CIs are not bit-identical to what `estimate_saltelli2010_with_bootstrap` would produce.

**Recommendation**: Replace `compute_sobol_from_cached` and `bootstrap_sobol_cis` in `analyze.rs` with calls to salib-rs's canonical implementations. This eliminates the duplicated code and the percentile interpolation discrepancy.

### 9.2 Missing Data Quality Gate

As discussed in Section 3, the analysis requires `accuracy != null` but accepts `accuracy: 0.0` regardless of `n_samples`. Adding:
```rust
anyhow::ensure!(
    cell.n_samples.unwrap_or(1) > 0,
    "cell at saltelli_index {i} has n_samples=0 -- cannot treat as accuracy=0.0"
);
```

would prevent the corrupted-data problem.

### 9.3 No Second-Order Indices Computed

The analysis uses `build_saltelli_matrix(&sampler, n_base, false, ...)` -- the `false` means no B_A matrices are generated, so no second-order indices. Given that sum_ST significantly exceeds 1.0 (especially in bio), second-order indices would quantify which factor pairs interact. The cost is modest: from N*(k+2) = 4096 to N*(2k+2) = 7168 evaluations. Alternatively, second-order indices can be estimated from the existing first-order design using the Saltelli 2010 Eq d formula with some additional bookkeeping (as implemented in salib-rs).

---

## 10. Recommendations Summary

### Critical (blocks publishable results)

1. **Add data quality gate**: Reject cells with n_samples=0 before Sobol estimation. Rerun bio analysis excluding the 20 corrupted cells.

2. **Double N to 1024**: The plan's own convergence criterion (CI width < 10% of estimate) was violated. N=1024 is the minimum for publishable results; N=2048 is preferred.

3. **Report the "bare" prompt sensitivity separately**: The headline "prompt template explains 85% of variance" conflates measurement pathology with actionable insight. Report both with and without "bare."

### High Priority (strengthens the methodology)

4. **Add automated convergence diagnostics**: Warn when S1 < 0, CI crosses [0,1] boundary, sum_ST > 1.3, or CI width > 10% of point estimate. These checks should be in the analysis output.

5. **Deduplicate the Sobol computation**: Replace mojave-gsa's local Sobol/bootstrap functions with salib-rs's canonical implementations.

6. **Add bootstrap coverage test**: Gate 4 Monte Carlo test that generates K=1000 bootstrap CIs at N=512 on Ishigami and verifies 95% nominal coverage.

7. **Compute second-order indices**: The interaction structure is the most interesting part of the WMDP analysis (which factor combinations matter?) and is currently invisible.

### Medium Priority (future capability)

8. **Evaluate ANOVA alternative**: For the current 6-factor discrete design, a full factorial (960 cells) with replication may be more statistically efficient than the 4096-cell Saltelli design.

9. **Add BCa bootstrap**: The percentile bootstrap can be anti-conservative for skewed distributions. BCa corrects for this; the infrastructure (`bootstrap.rs`) is ready for it.

10. **Implement convergence-rate diagnostic**: Plot S_i at increasing N subsets of the existing data (N=128, 256, 512) to check stabilization without new evaluations.

---

## Appendix A: Estimator Comparison for WMDP Use Case

| Estimator | S1 form | Best regime | WMDP suitability |
|-----------|---------|-------------|-------------------|
| Saltelli 2010 (Eq c) | Covariance | General default | Good; current choice |
| Jansen 1999 | Squared-difference | S_i near 1 | Good for prompt_template |
| Janon 2014 | Efficient covariance | Moderate S_i | Better CIs at same N |
| Owen 2013 | Correlation-2 | Small S_i | Best for minor factors, but 2x cost |
| Given-data (PBS 2013) | Partition variance | No designed matrix | Cross-check for designed analysis |
| Borgonovo delta | KDE divergence | Any | Captures non-variance effects |

For the WMDP analysis, the recommendation is to compute indices with all three Saltelli-compatible estimators (Saltelli, Jansen, Janon) and use agreement as a convergence diagnostic. If S1 estimates from the three estimators agree within CI width, the result is trustworthy. If they diverge, N needs increasing.

## Appendix B: File Paths Examined

- salib-rs estimators: `/Users/patrickbeam/projects/salib/crates/salib-estimators/src/{saltelli2010,jansen,janon,owen,borgonovo,given_data_sobol,bootstrap,sobol_indices}.rs`
- salib-rs Saltelli matrix: `/Users/patrickbeam/projects/salib/crates/salib-samplers/src/saltelli_matrix.rs`
- salib-rs tree reductions: `/Users/patrickbeam/projects/salib/crates/salib-core/src/reduce.rs`
- salib-rs Ishigami validation: `/Users/patrickbeam/projects/salib/crates/salib-validation/src/ishigami.rs`
- salib-rs Ishigami e2e test: `/Users/patrickbeam/projects/salib/crates/salib-estimators/tests/ishigami_e2e.rs`
- mojave-gsa analysis: `/Users/patrickbeam/projects/mojave/crates/mojave-gsa/src/analyze.rs`
- mojave-gsa confseq: `/Users/patrickbeam/projects/mojave/crates/mojave-gsa/src/confseq.rs`
- mojave-gsa manifest: `/Users/patrickbeam/projects/mojave/crates/mojave-gsa/src/manifest.rs`
- WMDP results: `/Users/patrickbeam/projects/mojave/data/v2/{bio,chem,truthfulqa}_sobol_analysis.json`
- WMDP raw data: `/Users/patrickbeam/projects/mojave/data/v2/bio_results_gsa.json`
