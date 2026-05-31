# Scout: X-Factor -- mojave
Date: 2026-05-30

## Key Findings

### 1. Quantification of Margins and Uncertainties (QMU) -- the nuclear weapons framework mojave is reinventing without knowing it

The US nuclear weapons Stockpile Stewardship Program developed QMU in the late 1990s at LLNL, LANL, and Sandia to solve a problem structurally identical to mojave's: **certifying system reliability when you cannot run the full test.** After the 1992 moratorium on nuclear testing, the weapons labs needed to certify that warheads still worked using only simulation, subcritical experiments, and component-level data. QMU formalizes this as: (1) identify performance thresholds (the cliff edges), (2) quantify the margin between current performance and each threshold, (3) quantify the uncertainty in that margin, (4) compute a Confidence Ratio CR = margin / uncertainty, and require CR >> 1. The 2008 National Academies review (SAND2006-5001, Pilch/Trucano/Helton) is the canonical reference.

This maps directly onto mojave. An LLM eval benchmark is a system you cannot exhaustively test. The Sobol decomposition tells you which factors push performance toward thresholds. The confidence sequences tell you how uncertain the margin estimate is. The SPC charts tell you when the margin is drifting. What mojave lacks is the QMU decision framework that binds these together: **a formal Confidence Ratio that combines the sensitivity analysis (which factors matter), the sequential inference (how certain are we), and the control chart (is the process stable) into a single accept/reject/investigate decision.** Defense customers will recognize this framework immediately. QMU is their native language for "is this system safe to deploy."

Actionable: Implement a `QmuAssessment` struct that takes a performance threshold, a current performance estimate with uncertainty (from confidence sequences), and a sensitivity profile (from Sobol), and emits a Confidence Ratio with a structured decision. This is a thin wrapper over existing mojave primitives, not new math.

Paper downloaded to intake: `Pilch2006_QMU_WhitePaper.pdf` (SAND2006-5001, 15pp). Also recommended: National Academies "Evaluation of QMU Methodology" (2009, ISBN 978-0-309-12094-8, freely available from NAP).

### 2. ISO 5725 and interlaboratory comparison -- the framework that makes "multiple eval labs get different scores" a solved problem

ISO 5725 (Accuracy of measurement methods and results, 6 parts) provides the international standard framework for quantifying repeatability (same lab, same conditions) and reproducibility (different labs, different conditions) of a measurement method. Mandel's h and k statistics detect outlier laboratories: h measures between-lab consistency, k measures within-lab consistency. The framework decomposes total measurement variance into repeatability variance + between-laboratory variance -- which is exactly what mojave's G-theory variance decomposition does, but in a framework that ISO-accredited testing labs worldwide already use.

The connection to mojave: when different organizations evaluate the same LLM using the same benchmark but get different scores, this is an interlaboratory comparison problem. Each "lab" is an eval configuration (infrastructure, prompt template, decoding parameters, random seeds). ISO 5725's repeatability and reproducibility conditions map onto mojave's perturbation families. Mandel's h and k statistics could be implemented as diagnostics that answer: "is this eval configuration an outlier relative to the population of configurations?"

A brand-new paper (Takeshita et al., arXiv 2602.01931, Feb 2026) develops bootstrap-based estimation and inference for the ISO 5725 variance components, replacing the classical ANOVA estimators with resampling methods that work in small-to-moderate designs. This is directly applicable to mojave's bootstrap CI machinery.

Actionable: Frame mojave's multi-configuration eval runs as ISO 5725 interlaboratory studies. Implement Mandel's h and k statistics in the `irr` crate as outlier diagnostics for eval configurations. Cite ISO 5725 in sales materials for defense customers -- they know this standard.

Paper downloaded to intake: `Takeshita2026_BootstrapISO5725.pdf` (arXiv 2602.01931, 28pp).

### 3. JCGM 106 guard-band decision rules -- how metrology decides accept/reject under measurement uncertainty

JCGM 106:2012 ("The role of measurement uncertainty in conformity assessment") formalizes how to make accept/reject decisions when the measurement result has uncertainty. The core concept: when a measured value falls near a specification limit, you cannot simply compare it to the limit -- you must account for the measurement uncertainty. "Guard bands" shrink the acceptance zone to control either the consumer's risk (probability of accepting a bad unit) or the producer's risk (probability of rejecting a good unit), or both via shared-risk rules.

This is directly applicable to mojave's sequential testing decisions. When mojave's confidence sequence says "model accuracy is 0.82 +/- 0.03" and the threshold is 0.80, what decision should be made? The current implementation makes a binary comparison. JCGM 106 provides the principled framework: define the decision rule (simple acceptance, guard-banded acceptance, shared risk), compute the conformance probability given the measurement result and its uncertainty, and emit a structured decision with explicit risk quantification.

For defense customers, this is powerful: instead of "the model passes with 95% confidence," mojave could say "the model passes with consumer risk < 2% under guard-banded acceptance per JCGM 106." This is the language of conformity assessment, which is what defense procurement actually does.

Actionable: Extend the `Decision` enum in eval-orchestrator to include conformity assessment decisions with explicit consumer/producer risk. Implement JCGM 106 guard-band computation as a thin layer over confidence sequences.

Paper already in library: `lib/statistics_probability_and_uncertainty/JCGM2012_106_RoleMeasurementUncertainty.pdf`. Fresh copy also in intake.

### 4. Deming's analytic vs. enumerative study distinction -- the conceptual error nearly every LLM eval commits

W. Edwards Deming (1950, "Some Theory of Sampling") distinguished two fundamentally different kinds of statistical study: (a) **enumerative studies**, which aim to describe a finite, existing population (e.g., "what fraction of items in this lot are defective?"), and (b) **analytic studies**, which aim to predict the behavior of a process that will continue producing results in the future (e.g., "will this manufacturing process produce acceptable items tomorrow?").

Nearly every LLM benchmark treats evaluation as enumerative: "what is this model's accuracy on this fixed test set?" But the actual question customers ask is analytic: "will this model perform reliably in deployment?" These are different questions requiring different methods. For enumerative studies, standard confidence intervals and hypothesis tests apply. For analytic studies, the standard error does not address the most important source of uncertainty -- the change in conditions in the future. SPC charts, not confidence intervals, are the appropriate tool.

Mojave already has both tools (confidence sequences for the enumerative question, SPC charts for the analytic question), but it does not explicitly frame the distinction. This is a conceptual weapon: **mojave can argue that competitors who report only confidence intervals are committing the enumerative fallacy -- answering the wrong question.** The analytic question (will the model perform well in production?) requires longitudinal monitoring (SPC), sensitivity analysis (what conditions change performance?), and adaptive testing (CAT) -- all of which mojave provides.

Actionable: Frame the distinction in marketing materials and run-card reports. When a customer asks "what is the accuracy?", mojave should respond with both: (1) the enumerative answer (point estimate + CI on the fixed benchmark), and (2) the analytic answer (SPC baseline + sensitivity profile + conditions under which the estimate would change). No competitor does this.

### 5. Measurement System Analysis (MSA) / Gauge R&R -- the framework that treats LLM judges as measurement instruments

In manufacturing, before you trust any measurement, you validate the measurement system itself. Measurement System Analysis (MSA, per AIAG guidelines) decomposes total observed variation into: part variation (the thing you care about), repeatability (same operator, same instrument, same part -- variation from the instrument), and reproducibility (different operators, same instrument, same part -- variation from the human). The combined repeatability + reproducibility is the "Gauge R&R."

Mojave's IRR crate measures judge agreement. But agreement is not the same as measurement system capability. Two judges can agree perfectly and both be wrong. MSA goes further: it asks whether the measurement system (the LLM judge) can actually discriminate between good and bad units (the LLM outputs being scored). The key statistic is the precision-to-tolerance (P/T) ratio or the number of distinct categories (ndc). If ndc < 5, the measurement system cannot distinguish enough levels to be useful, regardless of agreement.

This reframes mojave's LLM-as-judge analysis: instead of just "do judges agree?" (IRR), ask "can judges discriminate?" (MSA). A judge might have high agreement (kappa > 0.8) but low discrimination (ndc < 3), meaning it can tell good from bad but cannot rank intermediates. Conversely, a judge might have moderate agreement but excellent discrimination. These are different failure modes requiring different interventions.

Actionable: Add MSA-inspired diagnostics to the IRR crate: P/T ratio, ndc (number of distinct categories). These are straightforward to compute from the same data IRR already consumes. Frame LLM judges as measurement instruments that must be qualified before use, per AIAG MSA Reference Manual (4th ed, 2010).

### 6. Campbell and Fiske (1959) MTMM matrix -- the forgotten validation design for eval construct validity

Campbell and Fiske's Multitrait-Multimethod (MTMM) matrix is a 1959 framework for validating whether a test actually measures what it claims. The design: measure multiple traits (constructs) using multiple methods, then examine the correlation matrix. Convergent validity: measures of the same trait by different methods should correlate highly. Discriminant validity: measures of different traits by the same method should NOT correlate highly. If method variance dominates trait variance, your test is measuring the method, not the trait.

Applied to mojave: the "traits" are LLM capabilities (reasoning, knowledge, safety). The "methods" are evaluation approaches (MCQ, open-ended, judge-scored, automated). An MTMM matrix would reveal whether a model's "reasoning score" on MCQ correlates with its "reasoning score" on open-ended tasks (convergent validity), and whether "reasoning" and "knowledge" scores from the same MCQ benchmark are improperly correlated (discriminant validity, or lack thereof).

This directly informs BEAD-0011 (construct validity dossier). When mojave builds the construct validity layer, it should include MTMM analysis as a core diagnostic. The existing factor analysis (CFA via semopy) can operationalize MTMM via a correlated-traits/correlated-methods CFA model. This is the standard modern approach to MTMM (Marsh & Grayson 1995).

Actionable: When building the construct validity dossier, implement MTMM analysis as a CFA model with trait and method factors. The library already has Campbell/Fiske 1959 in `lib/psychology/Cronbach1955_ConstructValidity.pdf` (same intellectual lineage). Acquire the original 1959 paper.

Paper already in library (related): `Construct Validity in Psychological Tests - Cronbach et al. 1955.pdf`.

### 7. Rasch's "specific objectivity" -- the 1960 measurement principle that says item-free person measurement is mathematically possible, and why mojave should care

Georg Rasch (1960) proved that under his measurement model, person ability estimates are independent of which items are administered, and item difficulty estimates are independent of which persons take the test. He called this "specific objectivity" -- a measurement property so strong that physicists would recognize it as a calibration invariance.

Mojave's CAT engine (eval-design crate) already uses 2PL IRT for item selection, which is a generalization of Rasch. But the project does not exploit the specific objectivity property. The Rasch model's claim is radical: **if your eval items satisfy the Rasch model, you can compare model abilities across completely different item subsets**, because the person parameter is separable from the item parameters (mathematical sufficiency of the total score).

This has a direct operational implication for mojave: if a customer's eval items fit the Rasch model, mojave can declare that adaptive testing (selecting different items for different models) produces comparable ability estimates. If they don't fit, the comparison is not item-free and mojave must report this as a measurement limitation. This is a testable claim that mojave could automate: compute Rasch model fit statistics (infit, outfit, point-measure correlation) and emit a "measurement comparability" diagnostic.

Actionable: Add Rasch model fit testing to mojave-calibrate as a gating diagnostic for CAT deployment. If items don't fit the Rasch model, warn that adaptive testing may produce non-comparable estimates.

### 8. Mari (2005) "The problem of foundations of measurement" -- the metrological argument that most AI evaluation is not measurement at all

Luca Mari's 2005 paper in Measurement distinguishes three philosophical positions on measurement: P1 (realism -- there is a true value), P2 (representational -- measurement is a homomorphism from empirical to numerical structures), and P3 (model-dependent -- measurement results are meaningful only within a declared model). The paper argues that metrological traceability -- an unbroken chain of comparisons to reference standards -- is the operational solution to the problem of measurement objectivity.

The devastating implication for AI evaluation: **most LLM benchmarks satisfy none of the three positions.** There is no declared empirical relational system (P2 is unsatisfied). There is no reference standard against which scores are traceable (P1 is vacuous). The "model" relating scores to capabilities is implicit and untested (P3 is unsatisfied). What AI evaluation does is *assign numbers to things*, which is not the same as measurement.

Mojave's entire project is to elevate AI evaluation from number-assignment to measurement. Mari's framework provides the philosophical scaffold: mojave should explicitly declare the measurement model (what empirical relation is being represented?), establish traceability (against what reference is the score calibrated?), and quantify uncertainty (per GUM). This is what the construct validity dossier (BEAD-0011) should formalize.

Actionable: Use Mari's three-position framework in mojave's white paper and documentation to explain why mojave exists. The argument is: "current AI eval assigns numbers; mojave performs measurement."

Paper downloaded to intake: `Mari2005_FoundationsMeasurement.pdf` (10pp).

### 9. Ergodicity and the ensemble-vs-time-average problem in LLM evaluation

Ole Peters' ergodicity economics program (2019, Nature Physics) shows that ensemble averages (averaging across many agents at one time) and time averages (averaging one agent across many times) diverge for non-ergodic processes. When an absorbing state exists (ruin, catastrophic failure), the time average is systematically lower than the ensemble average.

Applied to AI evaluation: benchmarking 50 models on one test set at one time (the standard approach) is an ensemble average. Tracking one model's performance across deployment (the actual concern) is a time average. If the process generating model performance is non-ergodic (which it is -- models degrade, drift, encounter distribution shift), then benchmark ensemble averages systematically overestimate the time-average performance that any single deployment will experience.

Mojave's SPC charts already track the time average. But the project doesn't frame the distinction. This framing would strengthen mojave's argument that SPC monitoring is not optional: **the ensemble-average benchmark score is a biased predictor of time-average deployment performance**, and the bias grows as conditions change. Only longitudinal monitoring (SPC) tracks the quantity the customer actually cares about.

Actionable: Frame the SPC monitoring module as the "time-average" complement to the "ensemble-average" benchmark. Use the ergodicity argument in positioning materials for defense customers who understand that point-in-time testing does not predict operational reliability.

### 10. Eliminative argumentation and assurance cases -- the defense-native framework for structured confidence claims

Goal Structuring Notation (GSN) and eliminative argumentation (Goodenough/Mead 2012, SEI) provide a formal notation for structuring safety/assurance claims as a hierarchy of goals, strategies, evidence, and defeaters. Defense and safety-critical industries use assurance cases to demonstrate that systems are acceptably safe. The UK MOD Defence Standard 00-56 and the US FDA 510(k) process both use assurance-case-like structures.

Mojave's run cards currently report statistical findings. But defense customers don't just want numbers -- they want a structured argument that the evaluation supports a specific claim. An assurance case for an AI evaluation would look like: **Goal**: "Model X is safe for deployment in context Y." **Strategy**: "Demonstrate via sensitivity-analyzed evaluation." **Evidence**: "Sobol indices show <5% sensitivity to prompt format; IRR > 0.8; sequential test stopped with e-value > 20; SPC shows no drift over N runs." **Defeaters considered and addressed**: "Sandbagging tested via perturbation analysis; contamination checked via cross-context verification."

This is a reporting layer on top of mojave's existing math, not new analysis. But it transforms the output from "here are your statistics" to "here is a structured argument, with evidence, that supports or refutes a specific claim." This is what defense procurement evaluators actually read.

Actionable: Add an assurance case template to the run-card LaTeX system. The template should map mojave's measurement outputs to GSN elements. The library already has Goodenough 2012, Goodenough 2015, Hawkins 2011, and Rushby 2024 on assurance cases.

## Questions for Deep Investigation

1. **QMU Confidence Ratio for AI eval**: What is the right formulation of the margin-to-uncertainty ratio for an LLM benchmark? The nuclear weapons QMU uses performance margin / combined uncertainty. For LLM eval, this could be: (benchmark accuracy - deployment threshold) / expanded uncertainty from confidence sequence. Has anyone published on this?

2. **ISO 5725 for LLM eval**: Has anyone conducted a formal interlaboratory comparison of LLM benchmarks -- i.e., had multiple labs independently evaluate the same model on the same benchmark and computed repeatability/reproducibility statistics per ISO 5725?

3. **Ergodicity testing**: Is there a formal test for whether an LLM eval time series is ergodic? If mojave could detect non-ergodicity, it could warn customers that their benchmark ensemble averages are biased predictors of deployment performance.

4. **Rasch model fit for LLM benchmarks**: Do standard LLM benchmarks (MMLU, WMDP, HumanEval) satisfy the Rasch model? If they do, specific objectivity holds and adaptive testing produces comparable scores. If they don't, what does the misfit structure tell us about the benchmark?

5. **Guard-band calibration**: What guard-band width is needed for LLM eval decisions under typical measurement uncertainty? Using JCGM 106's framework, what consumer risk does a typical confidence sequence width imply for a pass/fail threshold?

## Gaps Identified

1. **No QMU or margin-uncertainty framework**: mojave has all the pieces (sensitivity analysis, confidence sequences, SPC) but no decision framework that composes them into a single accept/reject/investigate decision. QMU provides this.

2. **No interlaboratory comparison diagnostics**: mojave treats each eval run independently. It has no framework for comparing eval runs across configurations in the ISO 5725 sense (are different configurations measuring the same thing with the same precision?).

3. **No measurement system qualification for judges**: IRR measures agreement but not discrimination capability. MSA/Gauge R&R would tell you whether the judge can actually distinguish between performance levels, not just whether judges agree with each other.

4. **No guard-band decision rules**: The eval-orchestrator makes binary pass/fail decisions without accounting for measurement uncertainty in the decision rule. JCGM 106 provides the principled framework.

5. **No assurance case output format**: The run card reports statistics but does not structure them into a goal-evidence-defeater hierarchy that defense procurement evaluators can consume.

6. **No explicit enumerative/analytic framing**: mojave has both fixed-sample and longitudinal tools but does not explain to users when each applies and why the distinction matters.

7. **No Rasch model fit diagnostics**: mojave-calibrate fits 2PL IRT but does not test whether the simpler Rasch model fits, which would enable stronger comparability claims for adaptive testing.

## Leads

1. **AIAG MSA Reference Manual, 4th Edition (2010)** -- the canonical reference for Measurement System Analysis. Not freely available but widely held. Directly applicable to qualifying LLM judges as measurement instruments.

2. **Borgonovo & Plischke (2016) "Sensitivity analysis: A review of recent advances"** -- updated GSA survey covering OT-based methods. Complements the Mazo 2024 paper the web scout found.

3. **Hernandez-Orallo (2017) "The Measure of All Minds"** -- a book-length treatment of AI measurement theory. The library scout identified this as a gap. It engages directly with Rasch, IRT, and fundamental measurement theory applied to AI. Critical acquisition.

4. **UK MOD Defence Standard 00-56 (Safety Management Requirements for Defence Systems)** -- the defense standard that mandates safety cases. If mojave wants to sell to UK MOD (via DSTL), run cards should map to 00-56 evidence requirements.

5. **NNSA "Assessment Science" reports (annual)** -- the annual assessments of the nuclear stockpile use QMU methodology. These are publicly available from NNSA and show how QMU is applied in practice by the exact customer community mojave targets.

6. **ISO 17025:2017 (General requirements for the competence of testing and calibration laboratories)** -- the accreditation standard for testing labs worldwide. If mojave positions itself as providing "accreditation-grade" evaluation, the reporting should align with ISO 17025's requirements for uncertainty reporting, traceability, and measurement system validation.

7. **Passing & Bablok (1983) regression** -- a non-parametric method comparison technique from clinical chemistry that handles errors in both variables. More appropriate than Bland-Altman for comparing two LLM judges when both have measurement error.

8. **FairDIF (Springer, AI and Ethics, 2026)** -- a framework using DIF to detect and correct bias in classifiers. The library scout noted that mojave lacks DIF. This paper provides a modern operationalization.

## Acquisitions

| File | Location | Status |
|------|----------|--------|
| `Pilch2006_QMU_WhitePaper.pdf` | neurotic_library/intake/ | Downloaded, valid PDF, 15pp |
| `Takeshita2026_BootstrapISO5725.pdf` | neurotic_library/intake/ | Downloaded, valid PDF, 28pp |
| `Mari2005_FoundationsMeasurement.pdf` | neurotic_library/intake/ | Downloaded, valid PDF, 10pp |
| `JCGM2012_106_ConformityAssessment.pdf` | neurotic_library/intake/ | Downloaded, valid PDF (duplicate of lib copy) |
