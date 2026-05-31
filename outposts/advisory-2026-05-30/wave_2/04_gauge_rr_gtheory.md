# Wave 2 Deep Dive: Gauge R&R, Interlaboratory Comparison, and Generalizability Theory

Date: 2026-05-30
Built on: X-Factor findings 2, 5; Library critical gap (zero G-theory); Codebase IRR crate; Web finding on Rabanser2026

---

## Executive Summary

mojave already has the mathematical primitives to implement three distinct measurement traditions that the project has not yet explicitly connected: (1) generalizability theory (G-theory), which decomposes measurement variance across facets using the same ANOVA-based decomposition that Sobol indices use; (2) ISO 5725 interlaboratory comparison, which treats multi-configuration LLM evaluation as a repeatability/reproducibility problem with formal outlier detection; and (3) MSA/Gauge R&R, which asks whether LLM judges can actually discriminate between performance levels -- a question IRR agreement statistics cannot answer.

The central finding of this deep dive is that **G-theory's D-study is a budget optimizer for mojave's Saltelli sampling**, and that Sobol indices and G-theory variance components are two notations for the same underlying ANOVA decomposition applied to different domains. mojave can implement D-study projections that tell customers "you need N more items but can reduce raters" before running the expensive Saltelli campaign -- saving GPU budget while maintaining statistical power. The quarantined G-theory implementation in salib-rs already does the variance decomposition; what is missing is the bridge that connects D-study projections to Saltelli sample size planning.

---

## 1. G-Theory and Sobol Indices: Two Notations for the Same Decomposition

### 1.1 The structural isomorphism

Sobol indices and G-theory variance components share a common mathematical ancestor: the functional ANOVA decomposition. Both partition total variance into main effects and interactions. The difference is domain and purpose:

| Aspect | G-Theory (Cronbach 1972) | Sobol Indices (Sobol 1993) |
|--------|--------------------------|---------------------------|
| Domain | Behavioral measurement | Engineering sensitivity analysis |
| Input space | Discrete facets (persons, items, raters) | Continuous/discrete factors (parameters) |
| Decomposition | SS_p, SS_i, SS_r, SS_pi, SS_pr, SS_ir, SS_pir | S_1, S_2, ..., S_T for each factor |
| Output | Variance components (sigma^2) | Proportion of output variance explained |
| Decision tool | D-study (reliability projection) | Factor fixing/prioritization |
| Normalization | sigma^2_p / (sigma^2_p + error) | V_i / V_total |

The quarantined `g_theory.rs` in salib-estimators (lines 92-278) implements the crossed p x i x r ANOVA decomposition identically to how classical G-theory textbooks present it. The mean squares (MS_p, MS_i, MS_r, MS_pi, MS_pr, MS_ir, MS_pir) are computed from the balanced grid, then variance components are extracted using the standard EMS equations (e.g., sigma_p = (MS_p - MS_pi - MS_pr + MS_pir) / (n_i * n_r)). This is the exact same computation that, when normalized by total variance, yields Sobol first-order and interaction indices for a fully factorial discrete design.

The key insight: **mojave is already doing G-theory when it runs a balanced Saltelli design over discrete factor levels.** The Saltelli 2010 estimator computes first-order (S1) and total-order (ST) indices from the f(A), f(B), f(A_Bi) evaluation vectors. These are moment-based sensitivity measures that correspond to specific sums of variance components in the ANOVA table. When the input factors are discrete (as mojave's perturbation axes are -- 5 prompt templates, 4 system prompts, etc.), the Saltelli estimates converge to the same quantities that a balanced factorial ANOVA would produce, divided by total variance.

### 1.2 Where they diverge

The correspondence is not perfect, and the divergences matter:

**Sampling strategy.** G-theory requires observing every cell of the p x i x r grid (balanced crossed design). Sobol/Saltelli uses quasi-random sampling of the factor space with N(k+2) evaluations. For mojave's 6-factor design at N=512, this is 4,096 cells. A full factorial at the given levels (5x4x4x2x3x2 = 960 cells) would cost less than a single Saltelli replicate. This means **for mojave's current discrete-level designs, a full factorial is cheaper than Saltelli and yields exact variance components rather than estimated indices.** The Saltelli design is designed for continuous input spaces where full factorial is impossible; for mojave's 4-6 level discrete factors, it is overkill.

**Negative estimates.** Both frameworks produce negative variance component estimates when sample sizes are small relative to true effect sizes. The adversary (Finding 2) flagged that WMDP-Bio has negative S1 values (S1_quantization = -0.070). In G-theory, negative variance components are a known phenomenon: the standard practice is to set them to zero (Brennan 2001, ch. 3), accept that the estimation is unstable, and increase sample size. The same interpretation applies to negative Sobol indices.

**Error structure.** G-theory distinguishes relative error (for ranking persons/models) from absolute error (for comparing to a fixed standard). This maps directly to mojave's two use cases: (a) "which model is better?" (relative) and (b) "does this model meet the threshold?" (absolute). The G-coefficient captures relative reliability; the Phi-coefficient captures absolute reliability. Sobol indices do not make this distinction -- they quantify sensitivity, not reliability. mojave needs both.

### 1.3 What the existing implementation covers

The quarantined salib-estimators `g_theory.rs` (now in `quarantine/dat/crates/saltelli-estimators/src/g_theory.rs`) implements:

- **G-study**: Crossed p x i x r ANOVA decomposition yielding 7 variance components
- **Reliability coefficients**: G-coefficient (relative) and Phi-coefficient (absolute)
- **Bootstrap CIs**: Balanced-axis resampling with percentile confidence intervals for all components and coefficients
- **D-study projections**: 4-point projection surface (current, doubled items, doubled raters, both doubled)

The spc-charts crate has a `g_theory.rs` feature-gated module that converts G-theory error variance to SPC control limits via the formula: sigma^2 = sigma_pi/n_i + sigma_pr/n_r + sigma_pir/(n_i * n_r). This bridge is already tested (`g_theory_tck.rs`).

What is **not** implemented:

- Unbalanced designs (common in real eval data where some model-item-rater cells are missing)
- Nested designs (raters nested within items, relevant when different judges score different subsets)
- Mixed designs (some facets crossed, some nested)
- Arbitrary D-study projection grids beyond the 4-point surface
- D-study budget optimization (finding the cheapest design that achieves a target reliability)
- Connection between D-study projections and Saltelli N_base planning

---

## 2. D-Study as Budget Optimizer for Saltelli Sampling

### 2.1 The opportunity

mojave's current workflow: (1) define perturbation axes and levels, (2) set N_base (currently 512), (3) run N_base*(k+2) = 4,096 eval cells, (4) compute Sobol indices. The plan says "if dominant-factor CI exceeds 10% of estimate, double N" -- but the adversary found this threshold was already exceeded (44% CI width for bio's prompt_template) without triggering a doubling.

G-theory's D-study offers a principled alternative. Instead of doubling N blindly, a D-study projects reliability under alternative designs. The workflow would become:

1. Run a pilot G-study (small balanced factorial, e.g., N=64 or even the 960-cell full factorial for the current 6-factor design)
2. Estimate variance components from the pilot
3. Project D-study reliability at candidate sample sizes
4. Find the minimum N that achieves target reliability (G >= 0.80 or Phi >= 0.80)
5. Run the full Saltelli campaign at that N

This inverts the current approach: instead of "run big, check if big enough," the D-study says "here is how big you need." For defense customers paying per GPU-hour, this is a concrete cost-saving tool.

### 2.2 The math

For a crossed p x i x r design, the D-study standard error of measurement (SEM) for absolute decisions is:

```
SEM_abs = sqrt(sigma_i/n_i' + sigma_r/n_r' + sigma_pi/n_i' + sigma_pr/n_r' + sigma_ir/(n_i'*n_r') + sigma_pir/(n_i'*n_r'))
```

where n_i' and n_r' are the projected (D-study) sample sizes. The Phi coefficient is:

```
Phi = sigma_p / (sigma_p + SEM_abs^2)
```

Setting a target Phi and solving for n_i' and n_r' subject to a cost constraint (cost = c_i * n_i' + c_r * n_r' + c_ir * n_i' * n_r') yields the optimal design. This is a constrained optimization problem solvable by Lagrange multipliers (see Marcoulides 1993, or the recent paper by He et al. 2024 in PMC: "New roles of Lagrange multiplier method in generalizability theory").

### 2.3 Mapping to Saltelli

The connection to Saltelli N_base: in the Saltelli design, N_base controls the number of quasi-random base samples. The total cells are N_base * (k+2). If the D-study projects that the dominant noise source is item variance (sigma_pi), then increasing N_base (which adds more item-level replications) is the right move. If the dominant noise source is rater variance (sigma_pr), then adding more judges is more efficient than increasing N_base.

The D-study can answer: "given the variance components from a pilot, what is the minimum N_base * (k+2) that achieves G >= 0.80?" This replaces the current ad-hoc "double N if CIs are too wide" with a principled budget allocation.

### 2.4 What this means for the current WMDP results

The adversary found negative Sobol indices and wide CIs at N=512. A D-study on the WMDP pilot data would answer: does the design need more base samples (larger N), more perturbation levels, or more replications per cell? The current design has zero within-cell replication (each Saltelli cell is evaluated once). Wang 2025 (Meta FAIR) found that prediction noise typically exceeds data noise by 2x. If this holds for WMDP, then **adding within-cell replications (running each cell 3-5 times) may reduce uncertainty more efficiently than increasing N_base.** A D-study with a "replication" facet would quantify this tradeoff.

---

## 3. ISO 5725 Interlaboratory Comparison for Multi-Configuration Eval

### 3.1 The framework

ISO 5725 defines:

- **Repeatability** (r): variation when the same lab repeats the same measurement under identical conditions. For LLM eval: variation when the same eval configuration is run multiple times on the same model.
- **Reproducibility** (R): variation when different labs measure the same thing. For LLM eval: variation when different eval configurations (different prompts, system prompts, decoding parameters) are used on the same model.

The standard decomposes total variance as:

```
sigma_R^2 = sigma_r^2 + sigma_L^2
```

where sigma_r is repeatability variance (within-lab) and sigma_L is between-lab variance. When sigma_L >> sigma_r, the measurement method has a reproducibility problem -- different configurations yield systematically different results. When sigma_r >> sigma_L, the problem is instrument noise.

### 3.2 Mandel h and k statistics

Mandel's h statistic measures between-lab consistency: for lab j,

```
h_j = (mean_j - grand_mean) / s_between
```

where s_between is the standard deviation of lab means. A large |h_j| indicates lab j is an outlier in terms of its mean result.

Mandel's k statistic measures within-lab consistency: for lab j,

```
k_j = s_j / s_pooled
```

where s_j is lab j's within-lab standard deviation and s_pooled is the pooled within-lab standard deviation. A large k_j indicates lab j has anomalously high internal variation.

### 3.3 Application to mojave

Each "lab" is an eval configuration. For the WMDP 6-factor design:

- **h statistics per configuration**: which eval configurations produce systematically different mean accuracy? The X-Factor finding was that prompt_template dominates -- h statistics would quantify *which* prompt templates are outliers (presumably "bare") and by how much.
- **k statistics per configuration**: which eval configurations produce anomalously variable results? A configuration with high k is unreliable even within itself -- it produces inconsistent scores across items.

Implementation path: Mandel h and k are computationally trivial given the data mojave already collects. For each Saltelli cell, the configuration is fully specified. Group cells by configuration, compute per-group means and standard deviations, compute h and k, and flag outliers using the critical values from Wilrich (2013) or the bootstrap approach from Takeshita et al. (2026, arXiv 2602.01931, already in intake).

### 3.4 What this adds beyond Sobol

Sobol indices answer: "which factors explain the most variance?" Mandel h/k answer: "which specific configurations are outliers?" These are complementary:

- Sobol tells you prompt_template is the dominant factor (S1 ~ 0.85)
- Mandel h tells you the "bare" template is 3.2 standard deviations below the mean (h = -3.2) while the other four templates cluster normally (|h| < 1.5)
- Mandel k tells you the "greedy" decoding configuration has anomalously low within-configuration variance (k = 0.4, everything scores the same) while "T=1.0" has anomalously high variance (k = 2.1)

The h/k diagnostics are per-configuration, not per-factor. They answer the practitioner's question: "should I remove this configuration from the study?" Sobol indices cannot answer this because they aggregate across all levels of a factor.

### 3.5 Defense customer value

ISO 5725 is a recognized standard in defense procurement. If mojave can say "we conducted an interlaboratory comparison of eval configurations per ISO 5725, found repeatability sigma_r = 0.03 and reproducibility sigma_R = 0.15, and identified two outlier configurations via Mandel h statistics," this is language that defense quality engineers already understand. It converts mojave's sensitivity analysis from a research artifact into a metrology-compliant study.

The Takeshita et al. (2026) bootstrap ISO 5725 paper (already acquired in wave 1) extends the classical ANOVA-based estimators with resampling methods that work for small-to-moderate designs -- exactly mojave's regime. This paper should be a Tier 1 read for anyone implementing the ISO 5725 bridge.

---

## 4. MSA/Gauge R&R: What ndc Adds Beyond IRR Agreement

### 4.1 The distinction between agreement and discrimination

mojave's IRR crate implements seven agreement statistics: Cohen's kappa, Fleiss' kappa, Krippendorff's alpha, Gwet's AC1, Dawid-Skene, Bland-Altman, and preference leakage. These all answer variants of: "do judges agree with each other?"

MSA/Gauge R&R asks a different question: "can the measurement system discriminate between different levels of the thing being measured?" Two judges can have perfect agreement (kappa = 1.0) and still be useless if they give everything the same score. The key MSA statistic is the **number of distinct categories (ndc)**:

```
ndc = floor(1.41 * (sigma_parts / sigma_gauge_rr))
```

where sigma_parts is the standard deviation of the "true" part values (model quality differences) and sigma_gauge_rr is the combined repeatability + reproducibility standard deviation of the measurement system (judge variation).

AIAG MSA Reference Manual acceptance criteria:
- ndc >= 5: acceptable measurement system
- ndc >= 10: excellent measurement system
- ndc < 3: measurement system cannot distinguish meaningful differences
- ndc = 1: the measurement system is binary at best

### 4.2 The LLM judge interpretation

For LLM-as-judge evaluation, the "parts" are the model outputs being scored, and the "gauge" is the LLM judge. If an LLM judge gives every output a 4 or 5 on a 1-5 scale (common with GPT-4 judging), then sigma_parts is small relative to sigma_gauge_rr, and ndc may be 1 or 2 -- meaning the judge cannot distinguish between good and excellent outputs, regardless of how much judges agree with each other on those undifferentiated scores.

This reframes the family_stratification module in the IRR crate. Currently, `stratified_alpha()` decomposes Krippendorff alpha into within-family and between-family components. The bias_burden statistic (mean_within - between_family) measures whether judges from the same family agree more than judges from different families. But it does not ask: "regardless of agreement, can any of these judges actually rank outputs?"

### 4.3 The P/T ratio for threshold decisions

The precision-to-tolerance ratio extends ndc to threshold-based decisions:

```
P/T = k * sigma_gauge_rr / (USL - LSL)
```

where k is typically 5.15 (99% of measurement spread) or 6 (99.73%), and (USL - LSL) is the tolerance band.

For mojave: if a customer's pass/fail threshold is "accuracy >= 0.80" with tolerance +/- 0.05 (i.e., the interesting zone is 0.75-0.85), then P/T tells you whether the eval system can resolve differences within that zone. If P/T > 0.30, the measurement system consumes more than 30% of the tolerance band -- too noisy for reliable pass/fail decisions. This connects directly to the JCGM 106 guard-band framework from X-Factor Finding 3.

### 4.4 Connecting to G-theory

The MSA/Gauge R&R variance decomposition (sigma_parts^2, sigma_repeatability^2, sigma_reproducibility^2) is a special case of G-theory with two facets: operators (reproducibility) and replications (repeatability). In the LLM eval context:

- sigma_p^2 (G-theory) = sigma_parts^2 (MSA) -- true model quality variation
- sigma_r^2 (G-theory) = sigma_reproducibility^2 (MSA) -- between-judge variation
- sigma_pir^2 (G-theory) = sigma_repeatability^2 (MSA) -- within-judge, within-model variation

The G-coefficient from G-theory is the ratio version of the ndc concept: it measures how much of observed variance is signal (model differences) vs. noise (judge variation). ndc translates this into a count that practitioners find intuitive.

### 4.5 Implementation path

Adding ndc and P/T to the IRR crate requires:

1. Compute sigma_parts from the RatingMatrix (standard deviation of item-level means across all raters)
2. Compute sigma_gauge_rr from within-item, within-rater variation (pooled within-cell standard deviation)
3. ndc = floor(1.41 * sigma_parts / sigma_gauge_rr)
4. P/T = 5.15 * sigma_gauge_rr / tolerance_width (tolerance provided by caller)

These computations consume the same `RatingMatrix` the IRR functions already use. No new data collection is needed. The statistics can be emitted alongside kappa/alpha as additional diagnostics on the judge measurement system.

---

## 5. Rabanser 2026 Framework Comparison

### 5.1 The twelve metrics

Rabanser, Kapoor, and Narayanan (Princeton, arXiv 2602.16666) decompose agent reliability into four dimensions with twelve metrics:

**Consistency** (4 metrics):
- C_out: Outcome consistency -- normalizes per-task success variance by max Bernoulli variance p(1-p)
- C_traj^d: Trajectory consistency (distributional) -- Jensen-Shannon divergence of action distributions
- C_traj^s: Trajectory consistency (sequential) -- normalized Levenshtein distance between action sequences
- C_res: Resource consistency -- exponential of negative average CV across resource types

**Robustness** (3 metrics):
- R_fault: Fault robustness -- clamped accuracy ratio under fault injection vs baseline
- R_env: Environment robustness -- clamped accuracy ratio under environment perturbation vs baseline
- R_prompt: Prompt robustness -- clamped accuracy ratio under paraphrased instructions vs baseline

**Predictability** (3 metrics):
- P_cal: Calibration (Expected Calibration Error)
- P_AUROC: Discrimination (AUROC of success vs failure by confidence)
- P_brier: Brier score (1 - MSE of confidence vs outcome)

**Safety** (2 metrics):
- S_comp: Compliance -- fraction of tasks without constraint violations
- S_harm: Harm severity -- expected severity of violations

### 5.2 What maps to mojave, what does not

| Rabanser Metric | mojave Primitive | Status |
|----------------|-----------------|--------|
| C_out (outcome consistency) | IRR + SPC charts | Partially covered: IRR measures inter-judge consistency; SPC monitors outcome drift. mojave does not compute per-task success variance across runs. |
| C_traj (trajectory consistency) | Not implemented | mojave evaluates outcomes, not trajectories. No action sequence analysis. |
| C_res (resource consistency) | Not implemented | No resource tracking (latency, tokens, API calls). |
| R_fault (fault robustness) | Perturbation engine | Partially covered: perturbation families test format/paraphrase/multi-turn but not fault injection. |
| R_env (environment robustness) | Perturbation engine (stub) | The multi-turn and paraphrase perturbation families are stubs. |
| R_prompt (prompt robustness) | Sobol/Saltelli | **Directly covered**: prompt_template is a Saltelli axis. R_prompt is equivalent to 1 - (accuracy_perturbed / accuracy_baseline), while Sobol S1_prompt_template quantifies the variance contribution. |
| P_cal (calibration) | Not implemented | No confidence calibration. Would require logprob access. |
| P_AUROC (discrimination) | Not implemented; related to ndc | Rabanser's discrimination is about model self-assessment; MSA ndc is about judge discrimination. Different concepts, same word. |
| P_brier (Brier score) | Not implemented | No probabilistic scoring. |
| S_comp (compliance) | Audit chain (partial) | Audit chain tracks eval integrity, not task-level constraint compliance. |
| S_harm (harm severity) | Not implemented | No harm taxonomy or severity scoring. |

### 5.3 Strategic assessment

Rabanser's framework is **complementary, not competing**. Their metrics describe agent behavior during deployment (consistency across runs, robustness to perturbations, predictability of failures, severity of harms). mojave's primitives describe the measurement system used to evaluate agents (sensitivity of scores to eval configuration, reliability of judges, sequential testing efficiency, tamper-evident provenance).

The key difference: Rabanser asks "is this agent reliable?" while mojave asks "is this evaluation reliable?" These are sequential questions -- you must answer the second before you can trust the answer to the first.

Where they overlap is robustness. Rabanser's R_prompt (accuracy ratio under prompt perturbation) is a scalar reduction of what mojave's Sobol decomposition provides in full detail. mojave's value-add: instead of a single robustness ratio, mojave decomposes variance across all perturbation axes simultaneously, identifies interaction effects, and quantifies confidence intervals. Rabanser's R_prompt cannot tell you whether prompt sensitivity interacts with decoding strategy; mojave's Sobol ST can.

### 5.4 What mojave should adopt from Rabanser

1. **C_out normalization**: Rabanser normalizes outcome variance by p(1-p) to disentangle consistency from difficulty. mojave should adopt this normalization when computing SPC control limits for binary outcomes -- otherwise a task with p=0.5 will always appear more variable than a task with p=0.9, even if both are equally consistent.

2. **The four-dimension taxonomy**: "Consistency, robustness, predictability, safety" is a clean customer-facing framework. mojave's run cards could organize their output along these dimensions, mapping Sobol indices to robustness, IRR to consistency, confidence sequences to predictability, and audit chains to safety.

3. **The capability-reliability gap framing**: Rabanser's headline finding -- "recent capability gains yield only small reliability improvements" -- is the single best pitch for mojave's existence. mojave should cite this finding in every customer conversation.

---

## 6. Synthesis: The Measurement System Qualification Stack

The three traditions -- G-theory, ISO 5725, and MSA/Gauge R&R -- compose into a coherent measurement system qualification stack for LLM evaluation:

### Layer 1: Gauge Qualification (MSA)
Before trusting any eval results, qualify the measurement system:
- **ndc >= 5**: The judge can distinguish at least 5 levels of model quality
- **P/T <= 0.30**: The judge's measurement noise is less than 30% of the tolerance band
- **Gauge R&R < 10% of total variation**: The measurement system is adequate

If the judge fails qualification, fix the judge before running the eval. No amount of statistical sophistication downstream can compensate for a measurement system that cannot discriminate.

### Layer 2: Interlaboratory Comparison (ISO 5725)
Once the judge is qualified, validate that different eval configurations measure the same thing:
- **Repeatability sigma_r**: baseline noise of the eval system
- **Reproducibility sigma_R**: total variation across configurations
- **Mandel h/k**: identify and remove outlier configurations
- **Repeatability limit r = 2.8 * sigma_r**: two results from the same configuration should differ by less than r

### Layer 3: Reliability Assessment (G-Theory)
Once configurations are validated, assess overall measurement reliability:
- **G-study**: estimate variance components across models, items, and judges
- **D-study**: project reliability at alternative sample sizes
- **Budget optimization**: find cheapest design achieving target reliability
- **Control limits**: convert G-theory error variance to SPC parameters

### Layer 4: Sensitivity Analysis (Sobol/Saltelli)
Once reliability is established, decompose the sources of variation:
- **S1 indices**: which factors drive variance independently
- **ST indices**: which factors drive variance including interactions
- **Borgonovo delta**: moment-independent sensitivity
- **Sequential testing**: when to stop sampling

This stack is ordered by dependency: each layer assumes the previous layer's conditions are met. Running Sobol analysis (Layer 4) without gauge qualification (Layer 1) is like running a precision experiment with an uncalibrated instrument.

---

## 7. Concrete Recommendations

### 7.1 Near-term (next sprint)

1. **Add ndc and P/T to the IRR crate.** These are 20-line computations on existing data structures. Emit them alongside kappa/alpha in `IrrResult` or as a separate `MsaResult` struct. Gate behind a feature flag if desired.

2. **Add Mandel h and k to the IRR crate or as a standalone diagnostic.** Input: RatingMatrix grouped by configuration. Output: per-configuration h and k statistics with critical values. The ILS R package (Flores et al. 2018) and the Takeshita bootstrap extension (already in intake) are reference implementations.

3. **Wire the quarantined G-theory implementation back into salib-rs.** The code in `quarantine/dat/crates/saltelli-estimators/src/g_theory.rs` is complete with tests and TCK specs. The spc-charts bridge (`g_theory.rs`) already compiles against it. The quarantine decisions documents indicate this was a deliberate design choice, not a rejection.

### 7.2 Medium-term (next 2-3 sprints)

4. **Implement D-study budget optimizer.** Extend `project_g_theory_d_study()` from the current 4-point surface to an arbitrary grid, and add a `find_minimum_design(target_phi, cost_function)` that solves the constrained optimization. This is the bridge between G-theory and Saltelli sample size planning.

5. **Add a pilot-study workflow to mojave-gsa.** Before running a full Saltelli campaign, run a small balanced factorial (the 960-cell full factorial for the current 6-factor design is cheaper than N=128 Saltelli), estimate G-theory variance components, project the D-study, and recommend N_base. This replaces the current "run big, hope it's big enough" approach.

6. **Implement ISO 5725 repeatability/reproducibility reporting.** Group Saltelli cells by configuration, compute repeatability and reproducibility standard deviations, emit Mandel h/k diagnostics, and report r (repeatability limit) and R (reproducibility limit) in the run card.

### 7.3 Longer-term (future beads)

7. **Support unbalanced G-theory designs.** Real eval data is rarely balanced (some model-item-rater cells are missing due to timeouts, API failures, or adaptive skipping). The GeneralizIT Python package (Martinkova et al. 2024) supports unbalanced designs and could serve as a reference. In Rust, this requires either Type III sum of squares or a mixed-model formulation (lme4-style).

8. **Rabanser C_out normalization for SPC.** When monitoring binary accuracy via SPC charts, normalize by p(1-p) to make control limits independent of baseline difficulty. This is a one-line change in the SPC sigma computation but requires careful documentation.

9. **Build the measurement qualification stack into the eval-orchestrator.** The four-layer stack (MSA -> ISO 5725 -> G-theory -> Sobol) should be the orchestrator's default pipeline for any multi-judge, multi-configuration eval. Each layer emits diagnostics that gate the next layer: if ndc < 3, warn before proceeding to sensitivity analysis.

---

## 8. Acquisition Priorities

| Priority | Item | Rationale |
|----------|------|-----------|
| HIGH | Brennan 2001 "Generalizability Theory" textbook | The canonical G-theory reference. Zero library coverage. Needed for unbalanced designs, nested/mixed layouts, multivariate G-theory. |
| HIGH | Shavelson & Webb 1991 "Generalizability Theory: A Primer" | Accessible intro for implementation reference. Covers D-study budget optimization. |
| HIGH | AIAG MSA Reference Manual 4th ed (2010) | Canonical MSA reference. ndc formula, P/T ratio, gauge qualification criteria. Not freely available but widely held by defense customers. |
| MEDIUM | Takeshita et al. 2026 (arXiv 2602.01931) -- already in intake | Bootstrap ISO 5725. Reference implementation for Mandel h/k with resampling CIs. |
| MEDIUM | GeneralizIT paper (arXiv 2411.17880) | Python G-theory reference implementation. Covers unbalanced designs, nested layouts. Useful as validation target. |
| MEDIUM | Wilrich 2013 "Critical values of Mandel's h and k" | Exact formulae for Mandel statistic critical values at alpha=0.05 and alpha=0.01. Needed for ISO 5725 outlier detection. |
| MEDIUM | He et al. 2024 (PMC11486427) "Lagrange multiplier method in G-theory" | D-study budget optimization via constrained optimization. Directly relevant to Saltelli N_base planning. |
| LOW | Flores et al. 2018 "ILS R package" | R reference implementation for interlaboratory studies. Mandel h/k, Cochran test, Grubbs test. Useful for Gate 2 cross-checks. |
| LOW | Rabanser et al. 2026 (arXiv 2602.16666) -- already in intake | Already acquired in wave 1. Tier 1 read for framework comparison. |

---

## 9. Open Questions

1. **Full factorial vs. Saltelli for discrete factors**: For mojave's current 6-factor design with 5x4x4x2x3x2 = 960 cells, a full factorial is cheaper than N=128 Saltelli (which costs 128*8 = 1,024 cells). Is there a principled reason to prefer Saltelli over full factorial when all factors are discrete with few levels? The answer may be "no for the G-study pilot, yes for the subsequent targeted analysis" -- but this needs formal justification.

2. **G-theory with non-crossed facets in the Saltelli design**: The Saltelli design is not a crossed design -- it uses quasi-random sampling that creates a partially-crossed, partially-nested structure. Can G-theory variance components be estimated from a Saltelli design, or does it require the data to be resampled into a balanced grid first?

3. **ndc for categorical judges**: The ndc formula assumes continuous measurements. When LLM judges assign categorical labels (correct/incorrect, or 1-5 Likert), the ndc computation needs adaptation. For binary judges, ndc is at most 2 by definition. This limits ndc's usefulness for MCQ scoring but makes it highly relevant for open-ended scoring with fine-grained rubrics.

4. **Bootstrap vs. analytical CIs for Mandel statistics**: The classical Mandel h/k critical values assume normality. Takeshita et al. (2026) propose bootstrap-based inference. For LLM eval data, which is typically non-normal (heavy-tailed, bounded, zero-inflated), the bootstrap approach is likely more appropriate. But this needs validation on mojave's actual data.

5. **G-theory for the Wang noise decomposition**: Wang (2025) decomposes eval noise into prediction noise and data noise. G-theory's facet structure could formalize this: prediction noise is within-cell replication variance (sigma_pir), data noise is item-level variance (sigma_i + sigma_pi). Has anyone formalized the Wang decomposition as a G-study? This could be a novel contribution.
