# Wave 2 Deep Dive: Nuclear QMU and Defense Measurement Framework

**Agent:** claude-opus-4-6
**Date:** 2026-05-30
**Built on:** X-Factor findings 1, 3, 10; Web findings 1, 16; Library GUM/VIM metrology suite; Codebase eval-orchestrator/seq-anytime-valid/spc-charts architecture

---

## Thesis

mojave should adopt the nuclear weapons QMU (Quantification of Margins and Uncertainties) framework as its top-level decision architecture for defense customers. The Confidence Ratio CR = margin / uncertainty composes all five of mojave's existing mathematical pillars into a single accept/reject/investigate decision that defense procurement evaluators already understand. This is not a new mathematical capability -- it is a thin integration layer over existing primitives that reframes mojave's output from "here are your statistics" to "here is a quantified safety argument with explicit risk." Combined with JCGM 106 guard-band decision rules for the accept/reject boundary and GSN assurance cases for the reporting structure, this creates a defense-native output format that no competing framework can produce. The Aug 2026 DoD AI cybersecurity review mandate and NIST AI 800-3's call for measurement-science rigor in AI eval create an immediate market window.

---

## Bibliography

### Primary sources read for this analysis

1. **Pilch, Trucano, Helton (2006).** "Ideas Underlying Quantification of Margins and Uncertainties (QMU): A White Paper." SAND2006-5001. Sandia National Laboratories. -- The canonical QMU reference. Defines QMU as a decision-support methodology with three elements: performance thresholds, margins, and quantified uncertainty. Distinguishes risk-informed decision analysis (RIDA) from risk-based decision making. Introduces Best Estimate Plus Uncertainty (BE+U) as the core information form. Separates aleatory and epistemic uncertainty. [neurotic_library/intake/Pilch2006_QMU_WhitePaper.pdf]

2. **JCGM 106:2012.** "Evaluation of measurement data -- The role of measurement uncertainty in conformity assessment." Joint Committee for Guides in Metrology. -- Formalizes accept/reject decisions under measurement uncertainty. Defines tolerance intervals, acceptance intervals, guard bands (guarded acceptance, guarded rejection), conformance probability, consumer's risk (accepting a non-conforming item), and producer's risk (rejecting a conforming item). Introduces measurement capability index Cm = (TU - TL) / (2 * k * u), where u is standard uncertainty. [neurotic_library/lib/statistics_probability_and_uncertainty/JCGM2012_106_RoleMeasurementUncertainty.pdf]

3. **National Academies (2009).** "Evaluation of Quantification of Margins and Uncertainties Methodology for Assessing and Certifying the Reliability of the Nuclear Stockpile." National Research Council, ISBN 978-0-309-12094-8. -- External review of QMU methodology across LLNL, LANL, and Sandia. Documents differences in how labs compute CR. Acceptable CR ranges cited as 2:1 to 10:1 depending on consequence severity. [Accessed via NAP web; acquisition recommended]

4. **Keller et al. (2026).** "Expanding the AI Evaluation Toolbox with Statistical Models." NIST AI 800-3. -- First federal publication calling for measurement-science rigor in AI eval. Distinguishes benchmark accuracy from generalized accuracy. Proposes GLMMs as principled statistical framework. Performs variance decomposition on 22 LLMs across 3 benchmarks. [neurotic_library/intake/Keller2026_NISTAIEvalToolbox.pdf]

5. **Goodenough, Weinstock, Klein (2012).** "Toward a Theory of Assurance Case Confidence." CMU/SEI-2012-TR-002. -- Introduces eliminative argumentation for assurance cases: confidence comes not from positive evidence alone but from systematically identifying and eliminating defeaters (rebutting, undermining, undercutting). Proposes Assurance Claim Points as nodes where confidence is assessed. [neurotic_library/lib/engineering/Goodenough_2012_Toward_Theory_Assurance_Case_Confidence_SEI.pdf]

6. **Goodenough (2015).** "Eliminative Argumentation." SEI. -- Extension of 2012 framework. Formalizes the three defeater types and their role in building indefeasible confidence. [neurotic_library/lib/engineering/Goodenough_2015_Eliminative_Argumentation_SEI.pdf]

7. **Rushby, Bloomfield, Netkachova (2024).** "Defeaters and Eliminative Argumentation in Assurance 2.0." SRI-CSL-2024-01. -- Most recent treatment of defeaters in structured assurance. Extends GSN with doubt nodes and dialectical assessment. Distinguishes positive assurance (claim is true) from indefeasible confidence (no overlooked doubts remain). Directly applicable to mojave's run card reporting. [neurotic_library/lib/computer_science/Rushby_2024_Defeaters_Eliminative_Argumentation_Assurance2.pdf]

8. **Hawkins (2011).** "Clear Safety Arguments." -- Short guide to writing clear, comprehensible safety arguments using GSN. Practical templates for argument structure. [neurotic_library/lib/engineering/Hawkins_2011_Clear_Safety_Arguments.pdf]

9. **JCGM 100:2008 (GUM).** "Guide to the expression of uncertainty in measurement." -- The foundational metrology document. Defines standard uncertainty, combined uncertainty, expanded uncertainty, coverage factor, Type A and Type B evaluation. All uncertainty reporting in mojave run cards should cite GUM. [neurotic_library/lib/statistics_probability_and_uncertainty/JCGM2008_GUM_100_UncertaintyMeasurement.pdf]

10. **JCGM 200:2012 (VIM).** "International vocabulary of metrology." -- Canonical definitions of measurand, measurement result, measurement uncertainty, metrological traceability. [neurotic_library/lib/statistics_probability_and_uncertainty/JCGM2012_200_VIM_InternationalVocabularyMetrology.pdf]

### Secondary sources consulted

11. Crowell & Moring (2026). "CMMC for AI? Defense Policy Law Imposes AI Security Framework and Requirements on Contractors." -- Analysis of FY2026 NDAA Section on DoD AI/ML cybersecurity framework.
12. Akin Gump (2025). "Congress Moves Forward with AI Measures in Key Defense Legislation." -- Overview of legislative mandates for AI security in defense.
13. Freshfields (2026). "AI Supply Chain and Security: Congress Mandates Strict Controls." -- DoD acquisition requirements for AI lifecycle security.
14. arXiv 2603.08760 (2026). "Clear, Compelling Arguments: Rethinking the Foundations of Frontier AI Safety Cases." -- GSN adaptation for frontier AI, ALARP principle application, through-life controls framework.

---

## Analysis

### 1. The structural isomorphism: QMU maps exactly onto mojave's five pillars

The nuclear QMU framework has three elements: (1) performance thresholds, (2) margins, and (3) quantified uncertainty. mojave's five mathematical pillars map onto these elements with no residual:

| QMU Element | mojave Primitive | Crate | Role |
|-------------|-----------------|-------|------|
| **Performance threshold** | Deployment acceptance criterion | eval-orchestrator config | The "cliff edge" -- below this, the model is unsafe to deploy |
| **Margin (best estimate)** | Point estimate of model performance | seq-anytime-valid (confidence sequences) | Current best estimate minus threshold = margin |
| **Aleatory uncertainty** | Sobol variance decomposition | mojave-gsa (salib-rs) | Which factors drive performance variance (the "known unknowns") |
| **Epistemic uncertainty** | Confidence sequence width | seq-anytime-valid | How uncertain we are about the margin estimate |
| **Process stability** | SPC control charts | spc-charts | Is the margin drifting? (Deming's analytic question) |
| **Measurement system quality** | IRR / judge agreement | irr | Is the measurement instrument (the eval) itself reliable? |

The Confidence Ratio is then:

```
CR = M / U

where:
  M = (best_estimate - threshold)     -- the margin
  U = expanded_uncertainty             -- from confidence sequence, coverage factor k=2
```

When CR >> 1, the model performance is well above the threshold relative to our uncertainty. When CR < 1, the model may be below threshold. When CR is near 1, we are in the guard-band zone and need either more data or a risk-based decision.

**What mojave adds that nuclear QMU does not have:** The Sobol decomposition tells you *why* the margin is what it is -- which factors (prompt template, system prompt, decoding, quantization) contribute most to performance variance. This is actionable intelligence that defense customers can use to harden their deployment configuration. Nuclear QMU tells you "the margin is adequate." mojave's QMU tells you "the margin is adequate, and here is what would erode it."

### 2. JCGM 106 guard-band decision rules formalize the accept/reject boundary

The current eval-orchestrator `Decision` enum has four variants: `StopEarly`, `ContinueRunning`, `Regression`, and `MeasurementWarning`. None of these express conformity assessment. The missing decision type is:

```
ConformityAssessment {
    series: SeriesKey,
    threshold: f64,               // performance threshold (tolerance limit)
    best_estimate: f64,           // measured performance
    expanded_uncertainty: f64,    // U = k * u(y), typically k=2
    confidence_ratio: f64,        // CR = (best_estimate - threshold) / U
    guard_band_width: f64,        // g, per JCGM 106 Section 8.3
    decision: ConformityDecision, // Accept, Reject, or Indeterminate
    consumer_risk: f64,           // P(accept | truly non-conforming)
    producer_risk: f64,           // P(reject | truly conforming)
    decision_rule: DecisionRule,  // SimpleAcceptance, GuardedAcceptance, GuardedRejection, SharedRisk
}
```

JCGM 106 defines three decision rule families:

**Simple acceptance (shared risk):** Accept if measured value falls within tolerance interval. Both consumer and producer bear risk from measurement uncertainty. This is what most benchmarks do today (implicit shared risk with zero guard band).

**Guarded acceptance:** Shrink the acceptance interval inward from the tolerance limits by guard band width g. Controls consumer risk (probability of accepting a non-conforming model). For defense customers who cannot tolerate deploying a bad model, this is the appropriate rule.

**Guarded rejection:** Expand the acceptance interval outward from tolerance limits. Controls producer risk (probability of rejecting a conforming model). For model vendors who want to minimize false rejects.

The guard band width g depends on the measurement uncertainty u and the desired risk level. For a normal measurement PDF and a one-sided lower tolerance limit TL, JCGM 106 Section 8.3.2 gives:

```
Acceptance limit AL = TL + g

where g = k_p * u(y) for a desired consumer risk p_c

For 95% conformance probability: g = 1.64 * u(y)
For 99% conformance probability: g = 2.33 * u(y)
```

**Application to mojave:** When a confidence sequence produces an estimate y with expanded uncertainty U = 2u, and the customer sets a threshold TL = 0.80, the conformity assessment becomes:

- Simple acceptance: Accept if y >= 0.80
- Guarded acceptance (95%): Accept if y >= 0.80 + 1.64 * u
- Guarded acceptance (99%): Accept if y >= 0.80 + 2.33 * u

For a typical WMDP uncertainty of u = 0.03 (from the wave-1 adversary's analysis of confidence sequence widths), the 95% guarded acceptance limit becomes 0.849 -- substantially higher than the naive threshold. **This is the quantitative argument that defense customers need:** not "the model scored 0.82 on WMDP" but "the model scored 0.82 +/- 0.06 (k=2), and under guarded acceptance with consumer risk < 5%, the acceptance limit is 0.849, so the model does NOT pass."

### 3. QMU's aleatory/epistemic separation maps onto mojave's uncertainty stack

Pilch et al. (2006) emphasize that QMU requires separating two fundamentally different types of uncertainty:

**Aleatory uncertainty** (irreducible, stochastic variability): In mojave, this is the variance captured by Sobol decomposition -- the spread in performance caused by prompt template choice, decoding parameters, quantization, etc. This variance is real and irreducible for a given deployment configuration space. Sobol first-order indices tell you how much of the aleatory variance each factor contributes.

**Epistemic uncertainty** (reducible, due to incomplete knowledge): In mojave, this is the confidence sequence width -- the uncertainty in the point estimate caused by finite sampling. More eval runs reduce epistemic uncertainty. The confidence sequence quantifies exactly how much epistemic uncertainty remains at any given sample size.

The QMU framework requires both types to be quantified and reported separately because they have different implications:

- High aleatory uncertainty + low epistemic uncertainty = "We know the model is variable; the deployment configuration matters enormously." Actionable: lock down the configuration.
- Low aleatory uncertainty + high epistemic uncertainty = "We don't have enough data to know how the model performs." Actionable: run more evals.
- High both = "We don't know enough, and what we do know suggests high variability." Actionable: run more evals AND narrow the deployment configuration.

mojave already computes both quantities. What it lacks is the explicit separation in its reporting output and the QMU framing that makes the separation actionable.

### 4. GSN assurance cases should augment run cards, not replace them

The wave-1 X-Factor finding proposed adding assurance case templates to run cards. After reading Goodenough (2012), Goodenough (2015), Rushby et al. (2024), and the 2026 frontier AI safety case literature, the recommendation is:

**Augment, do not replace.** Run cards are measurement reports -- they present data. Assurance cases are argument structures -- they present reasoning. These serve different audiences and purposes. The measurement data in run cards provides evidence for claims in assurance cases, but the argument structure is separable from the data.

The proposed architecture:

```
Run Card (measurement layer)
  |-- Point estimates, confidence intervals, Sobol indices, SPC charts
  |-- GUM-compliant uncertainty budget
  |-- Raw data + audit chain
  |
Assurance Case (argument layer)
  |-- Top Goal: "Model X is safe for deployment in context Y"
  |-- Strategy: "Demonstrate via QMU-assessed evaluation"
  |-- Sub-goals:
  |     |-- G1: "Measurement system is qualified" (IRR > threshold, MSA ndc >= 5)
  |     |-- G2: "Performance margin is adequate" (CR >= CR_threshold)
  |     |-- G3: "Process is stable" (SPC in-control, no drift)
  |     |-- G4: "Sensitivity profile is acceptable" (no single factor dominates beyond tolerance)
  |     |-- G5: "Known threats are addressed" (sandbagging tested, contamination checked)
  |-- Evidence: pointers to specific run card sections
  |-- Defeaters considered:
  |     |-- D1: "Eval items may be contaminated" -> evidence from cross-context verification
  |     |-- D2: "Model may be sandbagging" -> evidence from perturbation sensitivity analysis
  |     |-- D3: "Judge may be biased" -> evidence from IRR + preference leakage score
  |     |-- D4: "Bare prompt may inflate variance" -> evidence from leave-one-level-out analysis
  |     |-- D5: "Binary signing not implemented" -> explicit residual risk acceptance
  |-- Confidence Ratio summary: CR per sub-goal, composite CR
```

This structure directly maps to what Rushby (2024) calls "Assurance 2.0" -- the argument is not just positive evidence but includes systematic defeater identification and elimination. The key insight from Goodenough (2012): confidence in an assurance case comes not from the strength of the positive argument but from the comprehensiveness of the defeater analysis. An assurance case with strong evidence but no defeaters considered is weaker than one with moderate evidence but thorough defeater elimination.

**For defense customers:** This is the reporting format that procurement evaluators, safety review boards, and accreditation authorities actually read. UK MOD Defence Standard 00-56 mandates safety cases in this structure. The US DoD's emerging AI governance framework (FY2026 NDAA) will require comparable documentation for AI systems. mojave can be the tool that generates both the measurement data and the argument structure.

### 5. NIST AI 800-3 alignment creates institutional validation

NIST AI 800-3 (Keller et al., Feb 2026) makes three moves that directly validate mojave:

**(a) Benchmark accuracy vs. generalized accuracy.** NIST formalizes the distinction between performance on a fixed test set (benchmark accuracy) and performance on the superpopulation of similar items (generalized accuracy). This is precisely Deming's enumerative vs. analytic distinction that the X-Factor scout identified. mojave's confidence sequences estimate generalized accuracy (with uncertainty); mojave's SPC charts track it over time. NIST's framework validates mojave's approach.

**(b) Variance decomposition via GLMMs.** NIST proposes GLMMs for decomposing evaluation variance into model, item, and model-by-item components. This is conceptually identical to mojave's Sobol decomposition, but using a different statistical framework (random effects models vs. ANOVA-based variance decomposition). The two approaches are complementary: GLMMs decompose variance by random factors (model, item), while Sobol decomposition works over designed experimental factors (prompt template, system prompt, decoding). mojave should cite NIST AI 800-3 and show that its Sobol approach generalizes NIST's GLMM variance decomposition to the experimental design setting.

**(c) Explicit statistical models.** NIST argues that evaluators must explicitly specify a statistical model -- the days of "just average the scores" are over. mojave already does this: its confidence sequences specify a data-generating model, its Sobol analysis specifies a variance decomposition model, its IRT specifies a measurement model. mojave is NIST-aligned by construction.

**Strategic implication:** mojave run cards should include a "NIST AI 800-3 alignment" section that maps each output to the corresponding NIST concept. When a defense customer's procurement officer asks "is this evaluation methodology NIST-compliant?", the answer should be self-evident from the run card.

### 6. The Aug 2026 DoD deadline creates an immediate market window

The FY2026 NDAA mandates:

1. DoD must establish department-wide AI/ML cybersecurity and governance policy within 180 days of enactment (~Jun 2026).
2. Comprehensive review of AI/ML cybersecurity practices due Aug 31, 2026.
3. The framework must be incorporated into DFARS and CMMC for contractor compliance.
4. Requirements include lifecycle security, model tampering protection, data leakage prevention, continuous monitoring, and incident reporting.

mojave's capabilities map directly onto several of these requirements:

| NDAA Requirement | mojave Capability |
|-----------------|-------------------|
| Lifecycle security | Audit chain with genesis sentinel binding eval to model identity |
| Model tampering protection | Tamper-evident hash chain; Ed25519 attestation |
| Continuous monitoring | SPC charts with e-detector change detection |
| Incident reporting | Run cards with structured measurement outputs |
| Risk-based framework | QMU Confidence Ratio with JCGM 106 guard bands |

The critical gap: **binary signing** (identified in wave-1 adversary finding 4). Without signed binaries, the audit chain proves integrity conditional on trusting the tool, but the tool itself is unverifiable. For DoD compliance, this must ship before first defense deployment. The FUTURE_WORK.md already flags this as "REQUIRED BEFORE PRODUCTION."

### 7. Mapping the Confidence Ratio to mojave's existing primitives -- implementation sketch

The QMU assessment is a thin integration layer. Here is how it composes from existing crates:

```
Input:
  - performance_threshold: f64          -- from customer requirement
  - coverage_factor: f64 = 2.0          -- GUM standard, gives ~95% coverage for normal
  - guard_band_rule: DecisionRule        -- from customer risk tolerance
  - cr_threshold: f64 = 2.0             -- minimum acceptable confidence ratio

From seq-anytime-valid:
  - best_estimate: f64                  -- current mean from confidence sequence
  - standard_uncertainty: f64           -- half-width of CS at current n, divided by coverage_factor

From mojave-gsa (salib-rs):
  - sobol_first_order: Vec<(String, f64)>  -- S1 indices per factor
  - sobol_total_order: Vec<(String, f64)>  -- ST indices per factor
  - dominant_factor: String             -- factor with highest ST
  - interaction_strength: f64           -- sum(ST) - sum(S1), measures interaction effects

From spc-charts:
  - in_control: bool                    -- is the process stable?
  - drift_detected: bool                -- has the e-detector signaled?
  - drift_magnitude: Option<f64>        -- estimated shift size if detected

From irr:
  - measurement_quality: f64            -- Krippendorff alpha or Gwet AC1
  - judge_agreement_adequate: bool      -- above threshold?

Computation:
  margin = best_estimate - performance_threshold
  expanded_uncertainty = coverage_factor * standard_uncertainty
  confidence_ratio = margin / expanded_uncertainty

  // JCGM 106 guard band
  guard_band_width = match guard_band_rule {
      SimpleAcceptance => 0.0,
      GuardedAcceptance { target_consumer_risk } => {
          quantile_normal(1.0 - target_consumer_risk) * standard_uncertainty
      }
      GuardedRejection { target_producer_risk } => {
          -quantile_normal(1.0 - target_producer_risk) * standard_uncertainty
      }
  };
  acceptance_limit = performance_threshold + guard_band_width;

  decision = if !in_control {
      ConformityDecision::Indeterminate("Process unstable -- SPC out of control")
  } else if !judge_agreement_adequate {
      ConformityDecision::Indeterminate("Measurement system unqualified -- IRR below threshold")
  } else if best_estimate >= acceptance_limit && confidence_ratio >= cr_threshold {
      ConformityDecision::Accept
  } else if best_estimate < performance_threshold {
      ConformityDecision::Reject
  } else {
      ConformityDecision::Indeterminate("Margin insufficient relative to uncertainty")
  };

Output:
  QmuAssessment {
      margin, expanded_uncertainty, confidence_ratio,
      guard_band_width, acceptance_limit, decision,
      consumer_risk, producer_risk,
      sensitivity_profile: sobol_first_order,
      process_stability: in_control,
      measurement_quality,
      aleatory_uncertainty: sobol_total_variance,
      epistemic_uncertainty: expanded_uncertainty,
  }
```

This is approximately 100-200 lines of Rust. It consumes outputs from four existing crates and produces a single structured decision. The mathematical novelty is zero -- it is pure composition. The strategic value is enormous.

### 8. The three-tier decision hierarchy

QMU provides a natural three-tier decision hierarchy for defense customers:

**Tier 1: Accept/Reject (automated).** When CR >> cr_threshold (e.g., CR > 3) and the process is in control and the measurement system is qualified, the model clearly passes or fails. This is the routine case that can be fully automated.

**Tier 2: Investigate (requires human judgment).** When 1 < CR < cr_threshold, or when the process is marginally in control, or when the sensitivity profile reveals a dominant factor that the customer cannot lock down. This is where Pilch et al.'s RIDA (Risk-Informed Decision Analysis) applies -- the QMU numbers inform but do not dictate the decision. "Other factors" (deployment context, consequence severity, alternative models) enter.

**Tier 3: Risk acceptance (requires authority).** When CR < 1 but the model must be deployed anyway (operational necessity, no better alternative). The assurance case must explicitly document this as a residual risk with mitigation measures. This is where Rushby's (2024) defeater analysis is critical: the assurance case must show that all known threats have been considered and that the risk is being accepted with full knowledge, not ignorance.

This hierarchy maps directly onto defense procurement workflows. The procurement officer sees a green/yellow/red decision. The technical evaluator sees the QMU numbers and sensitivity profile. The safety review board sees the full assurance case with defeaters.

---

## What This Changes

### For the product

1. **New crate: `qmu-assessment`** (or extend eval-orchestrator). A thin integration layer that takes outputs from seq-anytime-valid, mojave-gsa, spc-charts, and irr, and produces a `QmuAssessment` struct with Confidence Ratio, guard-band decision, and JCGM 106-compliant risk quantification. Implementation effort: ~1-2 weeks. No new math.

2. **Extended `Decision` enum.** Add `ConformityAssessment` variant to eval-orchestrator's `Decision` type. This is the defense-facing decision that replaces the current `StopEarly`/`ContinueRunning` binary.

3. **Assurance case template.** New LaTeX template in `templates/` that generates a GSN-structured argument from run card data. Maps QMU outputs to goals, evidence, and defeaters. Implementation effort: ~1 week (template only, no new computation).

4. **NIST AI 800-3 alignment section in run cards.** Add a section to the existing run card templates that explicitly maps mojave outputs to NIST concepts (benchmark vs. generalized accuracy, variance decomposition, statistical model specification).

5. **Aleatory/epistemic separation in reporting.** Run cards should explicitly separate Sobol-quantified aleatory uncertainty from confidence-sequence-quantified epistemic uncertainty, per Pilch et al.'s BE+U framework.

### For positioning

6. **Language shift.** Defense sales materials should speak QMU, not statistics. "Confidence Ratio" not "p-value." "Guard-banded acceptance" not "confidence interval." "Measurement capability index" not "standard error." This is not dumbing down -- it is translating into the customer's native framework.

7. **Competitive moat.** No competing eval framework (Inspect, Spark-LLM-Eval, DeepEval, OpenAI Evals) has QMU, JCGM 106, or GSN assurance cases. This is structural differentiation for defense procurement, not feature comparison.

8. **Regulatory alignment.** mojave can claim alignment with NIST AI 800-3 (measurement science), JCGM 106 (conformity assessment), GUM (uncertainty quantification), and the FY2026 NDAA AI governance requirements. This is a compliance story, not just a capability story.

### For the codebase

9. **Priority elevation for binary signing.** The adversary scout correctly identified this as the weakest link. For defense customers, the QMU assurance case is only as strong as its weakest defeater. "Binary is unsigned" is an unresolved defeater that blocks the entire argument. This should move from FUTURE_WORK to active development before first defense engagement.

10. **IRR confidence intervals.** The adversary scout noted that IRR statistics return `ci_lower: None, ci_upper: None`. For the QMU framework, measurement system qualification requires uncertainty-quantified IRR -- you cannot claim the measurement system is adequate without quantifying how uncertain that claim is. Wire the existing bootstrap module into IRR statistics.

---

## Gaps and Open Questions

### Gaps in the current analysis

1. **CR threshold selection for AI eval.** Nuclear weapons QMU uses CR thresholds of 2:1 to 10:1, calibrated over decades of engineering experience. What is the appropriate CR threshold for AI evaluation? This depends on consequence severity (a misclassified WMDP hazardous-knowledge question has different consequences than a nuclear weapon failure) and measurement system maturity. There is no published guidance. mojave will need to propose defaults and let customers adjust.

2. **Multivariate QMU.** The Pilch framework considers one threshold at a time. AI models have multiple performance dimensions (accuracy, safety, latency, cost). How do you compose CRs across dimensions? The naive approach (require CR > threshold for each dimension independently) is conservative. A joint QMU assessment that accounts for correlations between dimensions would be more efficient but requires multivariate uncertainty quantification (GUM Supplement 2, JCGM 102:2011).

3. **Non-normal measurement distributions.** JCGM 106's guard-band formulas assume normal measurement PDFs. For binary accuracy data (Bernoulli), the measurement distribution is not normal, especially at extreme accuracies. The confidence sequence provides a distribution-free interval, but the JCGM 106 risk calculations assume normality. How to reconcile? Options: (a) use the normal approximation (adequate for accuracy 0.2-0.8 with n > 100), (b) use the exact Bernoulli conformance probability (analytically tractable), (c) use Monte Carlo (GUM Supplement 1). The adversary scout's finding that the production path uses Gaussian mSPRT for Bernoulli data is the same underlying issue.

4. **Assurance case automation.** The proposed GSN template is static -- it maps run card fields to argument nodes. A fully automated assurance case would dynamically generate defeaters based on the data (e.g., "Sobol index for prompt_template > 0.5 -- generate defeater for configuration sensitivity"). This is a more ambitious capability that should be designed now but built later.

5. **NIST AI 800-3 GLMM vs. mojave Sobol.** The relationship between NIST's GLMM variance decomposition and mojave's Sobol variance decomposition needs formal characterization. Under what conditions do they agree? When do they diverge? A technical note or methods paper establishing the connection would strengthen mojave's NIST alignment claim.

### Open questions for deeper investigation

6. **Has anyone applied QMU to software or AI systems?** The search found no published work applying QMU outside nuclear weapons, nuclear power, and waste repository assessment. If mojave publishes on QMU for AI evaluation, it would be a novel contribution -- but also means there is no precedent to build on.

7. **Can the Sobol decomposition drive guard-band width?** If a factor has high Sobol total-order index, the margin is sensitive to that factor. Should the guard band be wider for high-sensitivity evaluations? This would connect the GSA output to the JCGM 106 decision rule in a principled way (wider uncertainty from sensitivity analysis -> wider guard band -> more conservative decision).

8. **How should SPC out-of-control signals interact with QMU decisions?** The current sketch treats SPC signals as a hard gate (process unstable -> decision is Indeterminate). But some SPC signals are transient (single out-of-control point vs. sustained drift). Should the QMU assessment distinguish signal types and severity?

9. **What is the right coverage factor for AI evaluation?** GUM uses k=2 for approximately 95% coverage under normality. But AI evaluation uncertainties may have heavier tails than normal (the adversary scout's finding about pathological "bare" prompt levels suggests fat-tailed distributions). Should mojave use a larger coverage factor (k=3 for ~99.7%) or a data-driven coverage factor based on the empirical distribution?

---

## Acquisitions

### Papers to acquire for neurotic_library

| Priority | Title | Why |
|----------|-------|-----|
| HIGH | National Academies (2009). "Evaluation of QMU Methodology for Nuclear Stockpile." ISBN 978-0-309-12094-8 | Canonical external review of QMU across all three weapons labs. Free PDF from NAP. Contains the CR threshold guidance (2:1 to 10:1) and comparison of lab-specific QMU implementations. |
| HIGH | Sharp & Wood-Schultz (2003). "QMU and the Nuclear Weapons Stockpile." Los Alamos Science 28. | Defines the "confidence ratio" term. Original accessible introduction to QMU for non-specialists. |
| HIGH | Eardley et al. (2005). "Quantification of Margins and Uncertainties." JASON Report JSR-04-330. | JASON advisory panel review of QMU. Independent assessment from outside the weapons labs. |
| HIGH | UK MOD Defence Standard 00-56 Issue 7. "Safety Management Requirements for Defence Systems." | The defense standard that mandates safety cases. If mojave targets UK MOD (via DSTL), run cards must map to 00-56 evidence requirements. |
| MEDIUM | ISO 15026-2:2022. "Systems and software engineering -- Systems and software assurance -- Part 2: Assurance case." | The international standard for assurance case structure. Needed if mojave claims ISO alignment for its assurance case output. |
| MEDIUM | Helton, Johnson, Oberkampf (2004). "An exploration of alternative approaches to the representation of uncertainty in model predictions." Reliability Engineering & System Safety 85(1-3). | Pilch et al. cite this as the key reference for aleatory/epistemic separation in QMU. Directly relevant to mojave's uncertainty stack. |
| MEDIUM | Oberkampf & Roy (2010). "Verification and Validation in Scientific Computing." Cambridge University Press. | Comprehensive V&V framework that QMU relies on. Relevant to mojave's 4-gate validation methodology. |
| LOW | Garrick & Christie (2002). "Probabilistic Risk Assessment Practices in the USA for Nuclear Power Plants." Safety Science 40(1-4). | Historical context for RIDA/QRA methodology that QMU builds on. |

### Papers already in library that are load-bearing for this analysis

- JCGM 106:2012 (conformity assessment) -- in lib and intake
- JCGM 100:2008 GUM (uncertainty) -- in lib
- JCGM 200:2012 VIM (vocabulary) -- in lib
- Goodenough 2012 (assurance case confidence) -- in lib
- Goodenough 2015 (eliminative argumentation) -- in lib
- Rushby 2024 (defeaters in Assurance 2.0) -- in lib
- Hawkins 2011 (clear safety arguments) -- in lib
- Pilch 2006 QMU white paper -- in intake
- Keller 2026 NIST AI 800-3 -- in intake

---

## ASU Shopping List

| Resource | Where to look | Why |
|----------|--------------|-----|
| National Academies QMU report (2009) | Free from NAP: nap.nationalacademies.org/catalog/12531 | Free PDF download with NAP account. Contains the CR threshold guidance mojave needs. |
| Sharp & Wood-Schultz (2003) "QMU and Nuclear Weapons Stockpile" | Los Alamos Science No. 28 -- free from LANL website | Original confidence ratio definition, accessible introduction |
| JASON QMU Report (2005) JSR-04-330 | OSTI.gov or FAS.org | Independent external review of QMU methodology |
| UK MOD Def Stan 00-56 Issue 7 | UK MOD publications (may require UK defense access) | Not freely available; check if ASU has access via defense research network |
| ISO 15026-2:2022 | ASU library ISO standards access | Assurance case standard; check ASU's ISO subscription |
| Helton et al. (2004) RESS 85(1-3) | ASU library Elsevier access | Aleatory/epistemic separation in uncertainty quantification |
| Oberkampf & Roy (2010) CUP | ASU library | V&V framework underlying QMU credibility assessment |
| NNSA Annual Stockpile Assessment reports | NNSA public website | Real-world examples of QMU methodology in practice |
