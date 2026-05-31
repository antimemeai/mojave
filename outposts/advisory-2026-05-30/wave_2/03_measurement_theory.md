# Wave 2 Deep Dive: Measurement Theory and Construct Validity

**Agent:** claude-opus-4-6
**Date:** 2026-05-30
**Built on:** X-Factor findings 4, 6, 7, 8, 9; Library Cronbach-Borsboom-Freiesleben chain; Library gap on G-theory
**Scope:** Measurement-theoretic foundations for mojave -- what separates "measurement" from "number assignment," how to operationalize construct validity, G-theory's role, Rasch modeling, and the ergodicity problem

---

## 1. Number Assignment vs. Measurement: Mari's Three Positions and What They Demand of mojave

### The philosophical scaffold

Mari 2005 distinguishes three positions on measurement:

- **P1 (Realist):** There exists a true value; measurement approximates it. Requires: the measurand must be definable independently of the measurement procedure. For AI eval, this would mean "reasoning ability" exists as a property of the model independently of how we test it.

- **P2 (Representational):** Measurement is a homomorphism from an empirical relational system (ERS) to a numerical relational system (NRS). Requires: the ERS must be explicitly declared. For AI eval, this means specifying what empirical relations hold (e.g., "model A reasons better than model B on task class T") and proving the numerical assignment preserves those relations.

- **P3 (Model-dependent):** Measurement results are meaningful only within a declared model. The model specifies the relationship between the measurand, the measurement procedure, and the numerical output. Requires: an explicit measurement model, uncertainty quantification within that model, and traceability to reference standards.

### Why current LLM evaluation fails all three

Most benchmarks satisfy none of these positions:

1. **No declared ERS (P2 fails).** MMLU does not specify what empirical relation "accuracy on multiple-choice questions about world knowledge" is supposed to represent. Is it a homomorphism from "knowledge" to [0,1]? If so, what is the empirical ordering on "knowledge" independent of MMLU? There is none.

2. **No reference standard (P1 is vacuous).** There is no calibrated reference model against which MMLU scores are traceable. A model scoring 0.82 on MMLU -- 0.82 relative to what? The VIM (JCGM 200:2012) defines a measurement result as "a set of quantity values being attributed to a measurand together with any other available relevant information." Without traceability, the "quantity values" are arbitrary.

3. **No declared measurement model (P3 is unsatisfied).** The implicit model is: "accuracy = mean(correct_i), where correct_i in {0,1} for each item i drawn from a fixed set." This model says nothing about what generates the responses, what sources of variation exist, or under what conditions the numerical result would change. It is not a measurement model in the metrological sense -- it is a counting procedure.

### What mojave must do to satisfy P3

P3 is the achievable target. P1 requires philosophical commitments about AI capabilities that may be premature. P2 requires proving homomorphism properties that are mathematically demanding. P3 requires:

1. **Declare the measurement model.** For each eval, mojave should emit a structured statement: "We model accuracy as a function of item parameters (difficulty, discrimination), model ability (theta), and perturbation factors (prompt template, system prompt, n-shot, choice order, decoding, quantization). The measurement model is 2PL IRT with crossed random effects."

2. **Quantify uncertainty within the model.** This is what the GUM framework (JCGM 100:2008) provides. The standard uncertainty is the positive square root of the estimated variance. For mojave, this means reporting: (a) the SE from IRT ability estimation, (b) the sensitivity decomposition from Sobol analysis showing which factors contribute to the variance, and (c) the expanded uncertainty from confidence sequences. The GUM's Type A evaluation (statistical analysis of observations) maps to mojave's bootstrap CIs and confidence sequences. The GUM's Type B evaluation (other means -- manufacturer specs, calibration certificates, etc.) maps to mojave's perturbation analysis: prior knowledge about how eval conditions affect results.

3. **Establish traceability.** This is the hardest requirement. In physical metrology, traceability means an unbroken chain of comparisons to the SI. For AI eval, traceability could mean: (a) item parameters calibrated against a reference population of models with known properties (the IRT calibration pool), (b) perturbation levels chosen from a documented, reproducible protocol, (c) results chained via the audit trail to the specific model binary, item set, and configuration that produced them. Mojave's genesis sentinel and audit chain provide the infrastructure for traceability; what's missing is the policy layer that says what constitutes a valid calibration chain.

### The measurement equation

Following GUM Part 6 (JCGM 2020), mojave should formalize its measurement equation. For an MCQ benchmark:

```
Y_ij = f(theta_j, b_i, a_i, P_k, S_l, N_m, C_n, D_o, Q_p) + epsilon_ij
```

where:
- `Y_ij` is the response of model j to item i
- `theta_j` is the latent ability of model j
- `b_i, a_i` are item difficulty and discrimination (IRT parameters)
- `P_k, S_l, N_m, C_n, D_o, Q_p` are the perturbation factors (prompt template, system prompt, n-shot fraction, choice order, decoding strategy, quantization)
- `epsilon_ij` is the residual (including all unmodeled sources of variation)

The Sobol decomposition decomposes `Var(Y)` across these factors. The IRT model provides the structural relationship between theta and item parameters. The perturbation engine systematically varies the factors to estimate their contributions. This is mojave's measurement model, and it should be stated explicitly in every run card.

**Actionable for mojave:** Add a "Measurement Model" section to the run-card LaTeX template that declares the measurement equation, lists all identified sources of variation, and states the assumptions (local independence, 2PL ICC, crossed design). This transforms the run card from a results report into a metrological document.

---

## 2. Construct Validity: From Cronbach-Meehl to Operationalization

### The intellectual lineage

The construct validity chain in the library traces a 70-year argument:

1. **Cronbach & Meehl 1955.** Construct validity is established by embedding the test in a nomological network -- a system of lawful relations between observable properties and theoretical constructs. A test has construct validity if it behaves as the theory predicts: correlating with measures of related constructs (convergent validity) and not correlating with measures of unrelated constructs (discriminant validity).

2. **Messick 1995.** Expanded validity to six aspects: content, substantive, structural, generalizability, external, and consequential. Validity is not a property of the test but of the inferences drawn from test scores. This matters for mojave: the question is not "is MMLU valid?" but "is the inference 'model X has adequate knowledge for deployment in context Y' valid?"

3. **Borsboom 2004.** Radically simplified: a test is valid if variation in the attribute causes variation in the test score. This is a causal definition. For LLM eval, it means: does variation in the model's actual reasoning ability cause variation in its MMLU score? If MMLU scores vary primarily because of prompt template sensitivity (as mojave's Sobol analysis suggests), then MMLU has low construct validity for "reasoning" -- it is measuring prompt sensitivity, not reasoning.

4. **Borsboom 2006.** "The Attack of the Psychometricians" -- argued that latent variable models (IRT) are the only principled way to connect test scores to constructs, because they make the causal structure explicit: the latent trait causes the responses.

5. **Freiesleben 2026 (nomological networks for LLM benchmarks).** Proposes operationalizing Cronbach-Meehl for LLM evaluation: build a network of expected relationships between benchmark scores and external criteria, then test whether the observed relationships match. If MMLU claims to measure "knowledge," then MMLU scores should correlate with performance on knowledge-intensive downstream tasks and not correlate with performance on tasks that require no knowledge.

6. **Kearns 2026.** Attempts to quantify construct validity via structured measurement models. This is the most directly actionable paper for BEAD-0011.

### What construct validity means for mojave concretely

Mojave's construct validity dossier (BEAD-0011, currently deferred) should provide three kinds of evidence:

**A. Content validity.** Does the item pool adequately cover the construct? For WMDP, this means: do the bio/chem/cyber items cover the knowledge domains relevant to WMD development? Content validity is established by expert review, not statistical analysis. Mojave can automate content coverage reporting (item counts per domain, item difficulty distribution per domain) but cannot automate the validity judgment itself.

**B. Convergent and discriminant validity via MTMM.** Campbell and Fiske's 1959 Multitrait-Multimethod (MTMM) matrix is the key design here. The design:

- **Traits:** LLM capabilities (e.g., bio knowledge, chem knowledge, reasoning, instruction following)
- **Methods:** Evaluation approaches (MCQ, open-ended generation, LLM-as-judge, automated code execution)

The MTMM matrix reveals:
- **Convergent validity:** "Bio knowledge" measured by MCQ should correlate with "bio knowledge" measured by open-ended generation (same trait, different method)
- **Discriminant validity:** "Bio knowledge" and "instruction following" measured by the same MCQ benchmark should NOT correlate highly (different traits, same method)

If method variance dominates trait variance, the benchmark is measuring the method (e.g., MCQ test-taking ability) rather than the construct (e.g., bio knowledge). This is exactly what the adversary report's finding about prompt_template dominance suggests: the WMDP Sobol analysis shows that method variance (how the question is asked) dominates trait variance (what the model knows).

**Implementation path for MTMM in mojave:**

The existing CFA calibrator (`mojave_calibrate/cfa.py`) using semopy can fit the correlated-traits/correlated-methods CFA model that is the modern operationalization of MTMM (Marsh & Grayson 1995). The model specification would be:

```
# Traits
bio_knowledge =~ bio_mcq + bio_openended + bio_judge
chem_knowledge =~ chem_mcq + chem_openended + chem_judge
# Methods
mcq_method =~ bio_mcq + chem_mcq + reasoning_mcq
openended_method =~ bio_openended + chem_openended + reasoning_openended
```

The CFA fit indices (CFI, RMSEA) tell you whether the measurement model fits the data. Factor loadings on trait factors vs. method factors tell you how much variance is trait-driven vs. method-driven. The existing `_extract_loadings` and `_extract_fit_indices` functions in `cfa.py` already handle this output format.

**C. Structural validity via IRT model fit.** If the construct is unidimensional (one latent trait underlies all items), then a unidimensional IRT model should fit the data. If it doesn't fit (high infit/outfit statistics), the items are measuring multiple constructs, and the single-score summary is misleading. Mojave should report IRT model fit diagnostics as evidence for (or against) structural validity.

**Actionable for mojave:** When building BEAD-0011, structure the dossier as:
1. Declared construct (what the eval claims to measure)
2. Content validity evidence (item pool coverage analysis)
3. Structural validity evidence (IRT model fit, factor structure)
4. Convergent/discriminant validity evidence (MTMM CFA)
5. Sensitivity evidence (Sobol decomposition: how much variance is trait vs. method)
6. Consequential validity evidence (do high-scoring models actually perform well on the construct in deployment?)

The existing mojave crates already provide evidence for slots 3, 4, and 5. The dossier is an aggregation and interpretation layer, not new math.

---

## 3. Generalizability Theory: What It Offers Beyond ANOVA

### The gap

The library has zero G-theory coverage -- no Brennan 2001, no Shavelson/Webb 1991, no Cronbach 1972. This is significant because mojave's variance decomposition approach is, in measurement-theoretic terms, a reinvention of G-theory applied to a new domain.

### What G-theory is

Generalizability theory (Cronbach, Gleser, Nanda, and Rajaratnam, 1972) extends classical test theory (CTT) by decomposing the total variance of measurements into components attributable to different "facets" of the measurement design. Where CTT treats all non-person variance as undifferentiated "error," G-theory asks: how much of the "error" is due to items? How much to raters? How much to item-by-rater interactions?

The core concepts:

1. **Universe of admissible observations.** The set of all possible measurement conditions the user is willing to generalize over. For LLM eval, this is: all possible prompt templates, all possible system prompts, all possible n-shot configurations, all possible choice orderings, etc.

2. **G-study (Generalizability study).** A fully crossed design that estimates variance components for each facet and their interactions. This is exactly what mojave's Saltelli study does -- but using Sobol decomposition rather than ANOVA-based expected mean squares.

3. **D-study (Decision study).** A projection from the G-study that asks: "If I change the measurement design (more items, more raters, different balance), how does reliability change?" The quarantined salib G-theory code already implements D-study projections.

4. **Generalizability coefficient (G).** The ratio of universe-score variance to expected observed-score variance. Analogous to Cronbach's alpha but decomposed by facets.

5. **Dependability coefficient (Phi).** Like G, but includes absolute-error variance (relevant when comparing to a fixed standard, not just ranking).

### G-theory vs. Sobol decomposition: the critical comparison

| Dimension | G-theory (ANOVA) | Sobol (variance-based GSA) |
|-----------|-----------------|---------------------------|
| Assumptions | Linear model, normally distributed residuals, balanced design | Model-free; only requires finite variance |
| Interactions | Explicit interaction terms (p*i, p*r, i*r, p*i*r) | Total-order indices capture all interactions involving a factor |
| Interpretation | Variance components have direct reliability interpretation (G, Phi coefficients) | Sensitivity indices partition variance but don't produce reliability coefficients |
| Design requirements | Balanced crossed/nested designs | Arbitrary factor spaces; Saltelli sampling handles unbalanced levels |
| Computational cost | O(N) for balanced ANOVA | O(N*(k+2)) for Saltelli's estimator |
| Non-linear effects | Misses them (assumes additivity unless interactions are explicitly modeled) | Captures all non-linear effects via total-order indices |
| Practical for LLM eval | Requires balanced data matrix -- hard when some perturbation combinations fail | Handles missing cells and unbalanced designs naturally |

**The key insight:** G-theory and Sobol decomposition answer the same question (how much variance comes from each source?) but make different assumptions and provide different outputs. G-theory's advantage is the reliability interpretation -- G and Phi coefficients directly answer "is this measurement generalizable?" Sobol's advantage is model-free variance decomposition that handles non-linear effects and unbalanced designs.

### What mojave should do

Mojave already has both tools:
- Sobol decomposition via salib-rs (production, active)
- G-theory via the quarantined `saltelli-estimators` g_theory module (working code, but in quarantine)
- The SPC g_theory bridge (`crates/spc-charts/src/g_theory.rs`) that converts G-theory variance components to control limits

The right architecture is:

1. **Run the Sobol analysis as the primary variance decomposition** (model-free, handles the messy realities of LLM eval).
2. **When data are balanced (or can be made balanced via subsetting), also compute G-theory** to get the reliability coefficients.
3. **Report both:** "The Sobol first-order index for prompt_template is 0.85, meaning it explains 85% of variance. The G-theory generalizability coefficient is 0.62 when generalizing over the universe of prompt templates, system prompts, and n-shot configurations." This tells the customer both what drives variance and whether their measurement is reliable.
4. **Use D-study projections for planning:** "If you double the number of prompt templates sampled, the dependability coefficient increases from 0.62 to 0.78. If you double the number of items, it increases from 0.62 to 0.71." This is actionable operational guidance.

**Critical connection:** The formula in `spc-charts/src/g_theory.rs`:
```
sigma_sq = sigma_pi/n_i + sigma_pr/n_r + sigma_pir/(n_i * n_r)
```
is the G-theory absolute error variance formula for a crossed p * i * r design. This formula converts G-theory variance components into control chart limits, which means SPC monitoring is calibrated to the measurement's expected variability. This is a novel and sound bridge -- when the control limits are set from G-theory, the SPC chart fires when performance deviates beyond what measurement error can explain. This is precisely the "analytic study" tool Deming says is required.

### The library gap is real but not blocking

The lack of Brennan 2001 and Shavelson/Webb 1991 means mojave cannot cite the canonical references when talking to psychometricians. But the quarantined implementation follows the correct ANOVA decomposition formulas (verified by inspection of the SS calculations against standard EMS formulas). The gap is in the library holdings and the marketing narrative, not in the code.

**Acquisition priority:** Brennan (2001) "Generalizability Theory" remains HIGH. Also: Webb, Shavelson, & Harding (2006) "Reliability coefficients and generalizability theory" in Handbook of Statistics, which is shorter and more accessible.

---

## 4. Rasch Modeling and Specific Objectivity: What mojave's CAT Crate Is Missing

### The Rasch model vs. 2PL

Mojave's eval-design crate implements 2PL IRT:
```
P(theta) = 1 / (1 + exp(-a * (theta - b)))
```

The Rasch model is the special case where `a = 1` for all items:
```
P(theta) = 1 / (1 + exp(-(theta - b)))
```

This looks like a simplification, but the constraint `a = 1` buys a property no other IRT model has: **specific objectivity**.

### What specific objectivity means

Rasch (1960) proved that under the Rasch model, the total score (number of items answered correctly) is a **sufficient statistic** for the person parameter theta. This means:

1. **Person ability estimates are independent of which items are administered.** If two models take completely different subsets of items from a Rasch-fitting item pool, their ability estimates are directly comparable -- no equating, no linking, no common items required.

2. **Item difficulty estimates are independent of which persons take the test.** If you calibrate item difficulty on one population of models and then use those items to evaluate a different population, the difficulty estimates remain valid.

3. **The comparison between two persons depends only on the items they both took, and the comparison between two items depends only on the persons who took both items.** This is the separability property.

For mojave's CAT engine, this matters enormously: if the items fit the Rasch model, then the adaptive testing procedure (which selects different items for different models) produces **provably comparable** ability estimates. If the items don't fit the Rasch model (and the 2PL is needed because discrimination varies across items), then adaptive testing produces estimates that are technically not directly comparable across models that received different item subsets. The comparison requires equating, which the current eval-design crate does not implement.

### How to test Rasch fit

The standard diagnostics are:

1. **Infit (information-weighted mean square).** Expected value 1.0 under the Rasch model. Values between 0.7 and 1.3 are conventionally acceptable. Infit > 1.3 means the item is less predictable than the model expects (noise). Infit < 0.7 means the item is too predictable (redundancy or response dependence).

2. **Outfit (unweighted mean square).** More sensitive to unexpected responses far from the item difficulty. Same conventions.

3. **Point-measure correlation.** Should be positive for all items. Negative correlation means higher-ability models are more likely to get the item wrong -- a strong indicator of item dysfunction.

4. **Andersen's likelihood ratio test.** A global test of Rasch model fit that splits the sample at the median ability and tests whether item parameters differ between the two subgroups. Significant difference means the Rasch model does not hold.

The library has MairHatzinger2007 (eRm: Extended Rasch Modeling in R), which provides the reference implementation for all of these diagnostics.

### What this means for mojave's CAT crate

The current eval-design crate (`cat.rs`, `ability.rs`) implements 2PL but never tests whether the Rasch model fits. This creates a gap:

- If items fit Rasch: mojave can claim "adaptive testing produces comparable estimates" with a testable mathematical guarantee. This is a strong claim for defense customers who need to compare models evaluated at different times with different item subsets.

- If items don't fit Rasch: mojave should warn that "adaptive testing may produce non-comparable estimates" and either (a) restrict comparisons to models that received the same items, or (b) implement test equating (Battauz 2015 `equateIRT`, already in library).

**Actionable for mojave:**

1. Add Rasch model fit testing to `mojave-calibrate`. The IrtCalibrator currently fits 2PL via py-irt. Add a mode that fits 1PL (Rasch) and computes infit/outfit/point-measure statistics. This is a thin wrapper -- py-irt supports `model_type="1pl"`.

2. In the run card, report a "Measurement Comparability" diagnostic:
   - If Rasch fits: "Items satisfy the Rasch model (all infit/outfit within [0.7, 1.3]). Adaptive testing produces comparable ability estimates across item subsets."
   - If Rasch doesn't fit: "Items require the 2PL model (discrimination varies: range [a_min, a_max]). Ability estimates from different item subsets may not be directly comparable without equating."

3. When the Rasch model does fit, exploit specific objectivity: the total score is a sufficient statistic, so the CAT engine could report a simpler diagnostic (total score + item count) alongside the full theta estimate.

### The 2PL is not wrong -- it's a different tool

The Rasch model is not "simpler 2PL." The philosophical positions are opposite:

- **Rasch approach:** The model is the criterion. Items that don't fit the Rasch model are defective and should be removed or revised. The goal is to build an item pool that satisfies specific objectivity.

- **2PL approach:** The model is flexible. Items with different discriminations are accommodated by estimating the discrimination parameter. The goal is to model the data, not constrain the items.

For mojave, the practical recommendation is: **fit both, report the fit comparison, and let the customer decide.** If specific objectivity matters (e.g., adaptive testing must produce comparable estimates), use the Rasch approach and discard misfitting items. If maximum measurement efficiency matters (e.g., minimizing the number of items needed to estimate ability), use the 2PL approach and accept the comparability caveat.

---

## 5. The Ergodicity Problem: Why Benchmark Ensemble Averages Lie

### The core argument

Ole Peters (2019, Nature Physics) demonstrated that for non-ergodic processes, the ensemble average (averaging across many agents at one time point) and the time average (averaging one agent across many time points) diverge. When the process has absorbing states or multiplicative dynamics, the ensemble average systematically exceeds the time average.

Applied to LLM evaluation:

- **Ensemble average:** Evaluate 50 models on MMLU today. Report the distribution of scores. This is what benchmarks do.
- **Time average:** Deploy one model and track its performance over 6 months. This is what customers experience.

If the process generating model performance is non-ergodic (and it is), these averages diverge:

1. **Distribution shift.** The queries a model receives in deployment evolve over time. Early queries may come from the training distribution; later queries drift. The model's effective accuracy degrades along trajectories that the ensemble snapshot doesn't capture.

2. **Capability degradation.** API-served models change silently (quantization updates, RLHF iterations, safety patches). Each change is small, but the composition is a random walk that the one-time benchmark doesn't track.

3. **Adversarial adaptation.** Users learn to exploit model weaknesses. The effective difficulty of the deployment environment increases over time. This is an absorbing-state dynamic: once a jailbreak is found, it propagates.

4. **Selection bias.** Models that score well on benchmarks get deployed more. But "scores well on benchmarks" and "performs well over time" are different properties if the process is non-ergodic. This is survivor bias in model selection.

### The mathematical structure

For an ergodic process: `E[f(X)] = lim_{T->inf} (1/T) integral_0^T f(X_t) dt`

For a non-ergodic process, this equality breaks. The ensemble expectation E[f(X)] can be much higher than the time average because the ensemble includes trajectories that, in practice, would have reached absorbing states.

For LLM eval: let X_t be the model's accuracy at time t. The benchmark provides E[X_0] (ensemble average at time 0). The deployment concern is the time average of X_t for a specific deployment. If performance degrades (due to drift, adversarial adaptation, or silent model changes), the time average is lower than E[X_0].

How much lower? Peters shows that for multiplicative processes, the difference grows logarithmically in time. For additive processes with bounded degradation, the difference is bounded. The structure of LLM performance degradation is probably between these cases -- multiplicative for adversarial adaptation (each exploit enables more exploits) and additive for distribution drift.

### What this implies for mojave

Mojave already has the right tool: SPC charts. The argument is:

1. **The benchmark score is an ensemble average.** It answers: "what is this model's expected performance across a population of possible queries at this point in time?"

2. **The SPC chart tracks the time average.** It answers: "is this model's performance stable over time, or is it drifting?"

3. **The gap between the two is the ergodicity premium.** This is the amount by which the benchmark overestimates deployment performance. It is not a fixed number -- it depends on the deployment conditions, the rate of drift, and the adversarial pressure.

**Actionable for mojave:**

1. **Frame SPC monitoring as the time-average complement to the ensemble-average benchmark.** In run cards and sales materials: "The benchmark score tells you what to expect on average across deployments. The SPC chart tells you what to expect over time in your specific deployment. These numbers diverge. The SPC chart is the one that matters."

2. **Quantify the ergodicity premium.** When mojave has both benchmark data (cross-sectional) and monitoring data (longitudinal), compute the gap: `ergodicity_premium = benchmark_accuracy - mean(spc_observations)`. If this is consistently positive, the benchmark is an optimistic predictor. Report this as a calibration diagnostic.

3. **Test for ergodicity.** A formal test of ergodicity (whether the time average converges to the ensemble average) requires comparing cross-sectional and longitudinal distributions. For an eval time series, the Augmented Dickey-Fuller test checks stationarity (a necessary condition for ergodicity). KPSS tests the null of stationarity. If the series is non-stationary, the ensemble average is not a reliable predictor. These are cheap diagnostics that mojave's SPC crate could emit alongside control chart signals.

4. **Connect to Deming's analytic/enumerative distinction (X-Factor finding 4).** The enumerative study (benchmark) answers a question about a fixed population. The analytic study (SPC) answers a question about a process. The ergodicity argument is the mathematical formalization of Deming's intuition: the enumerative answer is the wrong answer when the customer's question is analytic.

---

## 6. The Borsboom Thesis and Its Implications for mojave's Architecture

### The causal definition of validity

Borsboom (2004) defines validity simply: "A test is valid for measuring an attribute if (a) the attribute exists, and (b) variations in the attribute causally produce variations in the measurement outcomes."

This is deceptively simple but has sharp implications:

1. **If prompt template explains 85% of variance in WMDP scores (mojave's Sobol finding), then WMDP is 85% a measure of prompt sensitivity and 15% a measure of bio/chem/cyber knowledge.** Under Borsboom's definition, WMDP's validity for measuring hazardous knowledge is proportional to the fraction of variance caused by the construct. The Sobol decomposition is, in this framing, a validity coefficient.

2. **The perturbation engine is a validity testing engine.** Every perturbation factor that drives significant variance is a source of construct-irrelevant variance -- variance caused by something other than the attribute being measured. Mojave's GSA doesn't just decompose variance; it quantifies invalidity.

3. **The first-order Sobol index for the construct-relevant factor is the upper bound on validity.** If theta (true ability) isn't directly measured but correlates with item responses, and prompt_template explains 85% of variance, then at most 15% of variance can be attributed to theta. The validity ceiling is 1 - S1(prompt_template).

### Connecting Sobol decomposition to validity coefficients

This is a novel theoretical contribution that mojave can make:

Define: **Construct Validity Index (CVI)** = 1 - sum(S1_irrelevant)

where S1_irrelevant is the first-order Sobol index for each perturbation factor that is not part of the construct being measured.

For WMDP-bio (from mojave's data):
- S1_prompt_template = 0.85 (construct-irrelevant: how the question is phrased shouldn't matter for knowledge)
- S1_system_prompt = small (construct-irrelevant)
- S1_n_shot = small (borderline -- in-context learning is arguably related to knowledge)
- S1_choice_order = small (construct-irrelevant)
- S1_decoding = small (construct-irrelevant)
- S1_quantization = small (construct-irrelevant)

CVI approximately equals 1 - 0.85 - (small terms) approximately equals 0.12

This is a quantitative statement: "WMDP-bio as currently administered has a construct validity index of approximately 0.12." This is devastating for WMDP's validity claims but exactly the kind of finding that makes mojave valuable.

**Caveat (from adversary finding 5):** The prompt_template dominance may be driven by the pathological "bare" template. If bare is excluded as an unrealistic perturbation, S1_prompt_template drops substantially, and CVI rises. This is why leave-one-level-out analysis is critical: it separates "prompt wording matters a lot" from "including a broken prompt breaks everything."

**Actionable for mojave:** Implement CVI as a derived statistic in the run card. CVI = 1 - sum of first-order Sobol indices for factors the customer designates as construct-irrelevant. This requires the customer to declare which factors are part of the construct (a content validity decision) and which are noise (a construct validity decision). Mojave provides the math; the customer provides the semantics.

---

## 7. The Michell Critique and Why It Matters

Joel Michell (2008, "Is psychometrics pathological science?") argues that psychometrics has systematically avoided testing whether its constructs satisfy the axioms of quantity (Holder's axioms of additive conjunction). If a construct does not satisfy these axioms, it is not a quantity, and assigning numbers to it is not measurement -- it is, at best, ordinal ranking.

For LLM evaluation, the question is: **is "accuracy" a quantity?** Accuracy = proportion correct is a well-defined ratio-scale number. But "reasoning ability" -- the construct MMLU claims to measure -- may not be. If "reasoning" is not a single additive quantity (because it has multiple dimensions that don't combine additively), then the single-score IRT theta is a convenient fiction, not a measurement.

Mojave's factor analysis (BEAD-0006) addresses this directly: if the factor structure of a benchmark reveals multiple latent dimensions, then the single-score summary is invalid (it conflates multiple constructs). The CFA fit indices tell you whether the assumed structure matches the data.

**Connection to Rasch:** The Rasch model tests a specific form of the additivity axiom. If items fit the Rasch model, then item difficulty and person ability are on a shared interval scale, and the difference (theta - b) is meaningful. This is the closest thing to Michell's quantitative structure that psychometrics provides. Rasch model fit is, in this sense, a test of whether the construct is measurable.

---

## 8. The Missing Bridge: G-Theory, Sobol, and the Measurement-Decision Loop

### The integrated picture

The measurement-theoretic foundations described above assemble into a coherent architecture for mojave. The flow is:

```
Item Pool
    |
    v
[Rasch/2PL Fit Testing] --> Measurement Comparability Diagnostic
    |
    v
[Calibrated Items] --> CAT Engine (eval-design)
    |                       |
    |                       v
    |               [Adaptive Testing with specific objectivity check]
    |
    v
[Full Perturbation Study] --> Saltelli Sampling Design
    |
    v
[Sobol Decomposition] --> First-order and total-order indices
    |                         |
    |                         v
    |                   [CVI: Construct Validity Index]
    |
    v
[G-Theory Decomposition] --> Variance components + G/Phi coefficients
    |                              |
    |                              v
    |                        [D-Study: How to improve reliability]
    |                              |
    |                              v
    |                        [SPC Control Limits from G-theory]
    |
    v
[GUM Uncertainty Budget] --> Expanded uncertainty
    |
    v
[JCGM 106 Guard-Band Decision] --> Accept/Reject with risk quantification
    |
    v
[QMU Confidence Ratio] --> Margin/Uncertainty binding
    |
    v
[Run Card + Construct Validity Dossier]
```

Each layer depends on the one above it. The architecture is:

1. **Item analysis** (Rasch fit, IRT calibration) establishes whether measurement is possible.
2. **Variance decomposition** (Sobol + G-theory) establishes what the measurement is actually measuring.
3. **Uncertainty quantification** (GUM + confidence sequences) establishes how precise the measurement is.
4. **Decision rules** (JCGM 106 + QMU) establish how to act on the measurement.
5. **Longitudinal monitoring** (SPC) establishes whether the measurement remains valid over time.
6. **Validity documentation** (construct validity dossier) establishes the warrant for trusting the measurement.

### Current gaps in the architecture

| Layer | Status | Gap |
|-------|--------|-----|
| Item analysis | Partial -- 2PL only, no Rasch fit testing, no infit/outfit | Need Rasch fit diagnostics, DIF testing |
| Variance decomposition (Sobol) | Production-grade | Adversary findings (negative indices, zero-cell corruption) need fixes |
| Variance decomposition (G-theory) | Quarantined, working code | Need to activate from quarantine into salib-rs, validate against reference |
| Uncertainty quantification | Partial -- CS exists but uses wrong distributional family (adversary finding 6) | Need Bernoulli-specific CS for binary accuracy data |
| Decision rules (JCGM 106) | Not implemented | Need guard-band computation on top of CS |
| Decision rules (QMU) | Not implemented | Need Confidence Ratio struct binding Sobol + CS + SPC |
| Longitudinal monitoring (SPC) | Production-grade | G-theory bridge exists but depends on quarantined G-theory code |
| Validity documentation | Deferred (BEAD-0011) | Need CVI from Sobol, MTMM from CFA, Rasch fit |

---

## 9. Recommendations

### Immediate (before next WMDP run)

1. **Fix the Bernoulli CS path.** The adversary correctly identified that using Gaussian mSPRT for binary data violates the distributional assumptions. The `BernoulliMonitor` exists. Wire it into the confseq pipeline for MCQ accuracy data.

2. **Add data quality gates before Sobol analysis.** Cells with `n_samples=0` must be excluded or flagged, not coded as accuracy=0.0. This is a correctness bug.

3. **Run leave-one-level-out sensitivity analysis** for prompt_template to determine how much of the S1=0.85 finding is driven by the "bare" template.

### Near-term (next quarter)

4. **Activate G-theory from quarantine** into salib-rs (or directly into mojave crates). The implementation is sound. Wire D-study projections into the run card as operational planning guidance.

5. **Add Rasch fit diagnostics** to mojave-calibrate. Fit 1PL alongside 2PL. Report infit/outfit. Emit a "Measurement Comparability" diagnostic for CAT sessions.

6. **Implement CVI** (Construct Validity Index) as 1 - sum(S1_irrelevant) in the run card, with the customer declaring which factors are construct-relevant.

7. **Add the measurement equation** to the run-card template.

### Medium-term (next 6 months)

8. **Build the construct validity dossier** (BEAD-0011) using the architecture above: content validity, structural validity (IRT/CFA), convergent/discriminant validity (MTMM CFA), sensitivity validity (Sobol CVI), consequential validity.

9. **Implement JCGM 106 guard-band decisions** on top of confidence sequences. This transforms the eval-orchestrator's binary pass/fail into metrologically sound conformity assessment.

10. **Implement the QMU Confidence Ratio** as the capstone decision metric binding Sobol + CS + SPC.

11. **Add ergodicity diagnostics** (ADF test, KPSS test, ergodicity premium calculation) to the SPC crate.

### Library acquisitions (priority ordered)

1. **Brennan (2001), "Generalizability Theory"** -- canonical reference, HIGH priority
2. **Shavelson & Webb (1991), "Generalizability Theory: A Primer"** -- accessible introduction, HIGH
3. **Campbell & Fiske (1959), "Convergent and discriminant validation by the MTMM matrix"** -- MEDIUM, needed for BEAD-0011
4. **Hernandez-Orallo (2017), "The Measure of All Minds"** -- MEDIUM, the only book on AI measurement theory
5. **Rasch (1960/1980), "Probabilistic Models for Some Intelligence and Attainment Tests"** -- MEDIUM, foundational for specific objectivity claims
6. **Peters (2019), "The ergodicity problem in economics"** -- MEDIUM, needed for ergodicity framing in marketing materials

---

## 10. The One-Paragraph Thesis

Mojave's thesis is that LLM evaluation should be measurement, not number assignment. This report shows what that thesis demands: a declared measurement model (GUM measurement equation), tested construct validity (Sobol CVI + MTMM CFA + IRT fit), quantified uncertainty (confidence sequences + GUM uncertainty budget), metrologically sound decisions (JCGM 106 guard bands + QMU Confidence Ratios), and longitudinal monitoring that accounts for non-ergodicity (SPC as time-average tracking). Mojave already has most of the mathematical primitives. What it lacks is the interpretive layer that connects the math to the measurement science -- the layer that transforms Sobol indices from "interesting statistics" into "validity coefficients," G-theory variance components from "ANOVA output" into "reliability diagnostics," and SPC charts from "monitoring tools" into "time-average estimators that correct for the ergodicity premium." Building that interpretive layer is the construct validity dossier (BEAD-0011). It is the capstone, and the primitives are now mature enough to support it.
