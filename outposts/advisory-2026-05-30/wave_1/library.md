# Scout: Library -- mojave
Date: 2026-05-30

## Key Findings

1. **The library has a deep GSA collection that maps precisely to salib-rs's estimator surface.** There are 17+ papers on global sensitivity analysis including Saltelli 2010 (total sensitivity index estimator), Sobol 2001 (original indices), Owen 2014 (Sobol-Shapley bridge), Plischke 2021 (computing Shapley effects), Broto 2020 (variance reduction for Shapley), Song 2016 (Shapley computation), Borgonovo 2007 (delta importance measure), Campolongo 2007 (screening designs), RBD-FAST (Tarantola/Gatelli/Mara 2006), Maume-Deschamps 2018 (quantile-oriented sensitivity), Roustant 2017 (Poincare inequalities for SA), Janon 2014 (Sobol estimator asymptotics), Castellan 2018 (non-parametric Sobol estimation), Mazo 2024 (new paradigm for GSA), and Todorov 2025 (polynomial lattice rules for Sobol). This is excellent coverage for salib-rs's 4-gate validation. The library also has Blatman 2009 and Blatman/Sudret 2011 for PCE-based surrogate sensitivity analysis (relevant to salib-surrogate). Notably missing: Saltelli's 2004 textbook "Sensitivity Analysis in Practice" and the Iooss/Looss 2015 review paper that surveys all GSA methods.

2. **The confidence sequences / anytime-valid inference collection is remarkably strong and directly supports crates/seq-anytime-valid.** The library holds: Ramdas 2022 (admissible SAVI must rely on nonnegative martingales), Ramdas 2023 (game-theoretic statistics and SAVI -- the field-defining survey), Ramdas 2025 (hypothesis testing with e-values -- the new textbook), Vovk/Wang 2021 (e-values calibration and combination), Howard 2020 (time-uniform Chernoff bounds), Howard 2021 (confidence sequences -- the foundational paper), Koning 2026 (anytime validity is free), Koning 2024 (anytime-valid causal ML), Waudby-Smith/Ramdas 2024 (estimating means by betting), Fischer/Ramdas 2026 (improving SPRT overshoot), Shin 2023 (e-detectors for sequential change detection), Henzi/Ziegel 2022 (sequential forecast performance), Gruenwald 2019 (safe testing), Wald 1945 (original sequential tests), and the original sequential significance paper. This is close to complete coverage of the SAVI literature. The Shin 2023 e-detectors paper is particularly relevant to mojave's SPC-charts crate for detecting eval drift.

3. **The psychometrics/IRT collection is deep and directly relevant to mojave-calibrate.** Key holdings: Cronbach/Meehl 1955 (construct validity -- the original), Messick 1995 (validity as scientific inquiry), Borsboom 2003 (latent variables), Borsboom 2004 (concept of validity), Borsboom 2006 (attack of the psychometricians), Cronbach 1951 (coefficient alpha), Sijtsma 2009 (misuse of alpha), Michell 2008 (psychometrics as pathological science), Paun 2018 (Bayesian annotation models), Cohen 1960 (kappa coefficient), Lalor 2022 (py-irt), Burkner 2021 (Bayesian IRT in R), Chalmers 2012 (mirt package), De Boeck 2011 (IRT via lme4), Rizopoulos 2006 (ltm package). The applied-to-LLMs papers: Bean 2025 (construct validity in LLM benchmarks), Freiesleben 2026 (nomological networks for LLM benchmarks), Freiesleben/Zezulka 2025 (benchmarking epistemology), LLMPsychometrics 2025 (systematic review), PsychometricsEvalLLM 2024, DiagnosingReliability 2026 (IRT for LLM-as-judge), Zhou 2026 (PSN-IRT for LLM benchmarking), Chen 2025 (IrtNet compact LLM abilities), ATLAS 2025 (adaptive testing for LLM eval), and Kearns 2026 (quantifying construct validity). This is an exceptionally strong collection for mojave's BEAD-0005 (IRT) and BEAD-0011 (construct validity dossier).

4. **The LLM-as-Judge literature is well-represented and directly relevant to mojave's perturbation/measurement error analysis.** Holdings include: Zheng 2023 (MT-Bench -- foundational), Panickssery 2024 (self-preference bias), Bavaresco 2024 (meta-evaluation across 20 tasks), Haldar 2025 (self-inconsistency/rating roulette), Spiliopoulou 2025 (statistical method for self-bias), JudgeSense 2026 (prompt sensitivity benchmark), Li 2025 (preference leakage as contamination), Shi 2025 (position bias), Feng 2025 (assessing LLM-as-judge), judging-with-many-minds 2025 (multi-agent bias), judge reliability harness (stress testing), and the comprehensive survey (llm-as-a-judge-survey-2024). All of these inform mojave's perturbation engine design (BEAD-0020), since LLM judges introduce their own measurement error atop the construct being measured.

5. **The data contamination collection is strong and directly relevant to WMDP work.** Holdings: WMDP original (Li et al. 2024), Sainz 2023 (contamination per benchmark), Magar 2022 (memorization to exploitation), Dong 2024 (generalization vs memorization), Li 2024 (task contamination), Ponnuru 2024 (comprehensive contamination analysis), Xu 2024 (MMLU-CF contamination-free benchmark), contamination-survey-2025, Zhu 2024 (inference-time decontamination), cross-context-verification 2026, search-time contamination 2025, and Li 2025 (preference leakage). This collection is critical for mojave's integrity story: contamination is a confound that Sobol decomposition should detect but cannot address after the fact.

6. **The audit chain / tamper-evident logging holdings are thin but high-quality.** Two foundational papers: Schneier/Kelsey 1999 (cryptographic support for secure logs -- the ur-text), and Crosby/Wallach 2009 (efficient data structures for tamper-evident logging). Also RFC 9162 (Certificate Transparency v2). The tamper-resistant LLM safeguards papers (tamiminga 2024, filtering-pretraining-tamper-resistant, sycophancy-to-subterfuge) are thematically related but not directly applicable to audit chain design. Missing: the Trillian / Verifiable Data Structures literature from Google, and the NIST Cybersecurity Framework documentation that defense customers would reference.

7. **The metrology/uncertainty quantification collection is surprisingly deep and directly relevant to mojave's "measurement science" framing.** Holdings include: the complete GUM suite (JCGM 2008 GUM 100, Supplement 1 Monte Carlo, Supplement 2, Introduction, Part 6 measurement models, 2023 2nd edition Part 1), VIM (International Vocabulary of Metrology), NIST SP260-202 (uncertainty propagation), Possolo 2006/2013/2015 (Bayesian uncertainty alternatives), Taylor/Kuyatt 1994 (NIST uncertainty guidelines), and the SPC tutorial. These are load-bearing for mojave's run card templates and reporting surface. Mojave's 4-gate validation methodology can cite the GUM framework directly.

8. **The benchmark critique literature forms a coherent collection that supports mojave's thesis.** Key papers: Bowman 2021 (what will it take to fix benchmarking), Benchmark^2 2026 (systematic eval of LLM benchmarks), BenchmarkIceberg_IRT (benchmarks as icebergs), Jo/Wilson (AI evals should be grounded on a theory of capability), Hidden Measurement Error 2026 (measurement error in LLM pipelines distorts everything), Madaan 2024 (quantifying variance in evaluation benchmarks), Variance-Aware LLM Annotation 2026 (diagnostics protocol), Beyond Reproducibility 2026 (token probability nondeterminism), CORE-Bench 2024 (reproducibility benchmark), and Nosek 2018 (preregistration revolution). Collectively, these papers argue that current LLM evaluation is statistically sloppy -- exactly mojave's market thesis.

9. **The unlearning collection is relevant to WMDP but the library lacks the core Sobol-for-LLM-evals bridge papers.** The unlearning holdings (OpenUnlearning, Unlearning Microscope, CIR, feature-selective-rmu, dual-space, prompt-attacks-superficial, relearning-attacks, Various2025 unlearning isn't deletion) are strong. But there appears to be no paper that applies Sobol' indices directly to LLM evaluation variance decomposition -- which is mojave's core innovation. This is a genuine gap in the published literature, not just the library. The closest is Fel et al. 2021 (Sobol-based black-box explanations) and Sadeghi 2024 (GSA review with digit classification), both of which apply Sobol to ML models but not to eval pipeline variance.

10. **Safety engineering literature (Leveson/STAMP) is present and creates a bridge to defense customers.** Leveson 2004 (STAMP accident model) and the STPA primer connect systems-theoretic safety to mojave's audit chain and integrity model. The safety_risk_reliability_and_quality collection also has Birnbaum importance measures (which have a mathematical relationship to Sobol indices) and Swiss cheese model papers (Reason 2000/2006). This bridge is underexploited: STAMP's notion of "hazardous states" maps directly to mojave's notion of "eval configurations that produce misleading scores."

## Questions for Deep Investigation

1. **Is there a formal proof that Sobol decomposition of eval pipeline variance is identifiable under the discrete, finite-level factor spaces mojave uses?** Sobol theory assumes continuous inputs; mojave discretizes to 4-5 levels. Janon 2014 and Castellan 2018 address asymptotics but not this exact regime. What are the convergence properties of Saltelli 2010 estimates with N=1024 and d=6 at 4-5 levels each?

2. **Can the e-detectors framework (Shin 2023) be applied to detect model capability drift between eval runs?** Mojave has both spc-charts and seq-anytime-valid crates. The e-detectors paper provides a principled sequential change detection framework that could unify these: each run emits an e-value, and the product of e-values across runs forms an e-process that signals drift with anytime-valid guarantees.

3. **What is the relationship between Borgonovo delta (moment-independent) and Shapley effects (game-theoretic) for non-additive eval pipelines?** Owen 2014 and Plischke 2021 establish the Sobol-Shapley bridge, but Borgonovo delta captures a different aspect of sensitivity. When mojave runs both Saltelli2010 and Borgonovo delta on the same WMDP data, under what conditions will they disagree, and what does disagreement mean?

4. **How should mojave integrate G-theory (generalizability theory) with its Sobol decomposition?** G-theory decomposes variance by "facets" (raters, items, occasions) -- exactly what mojave's perturbation engine does with its "axes." The library has no G-theory papers, which is a significant gap. Cronbach's 1972 "Dependability of Behavioral Measurements" (currently in intake, on disk: NO) would be foundational here.

5. **Can the construct validity framework (Cronbach/Meehl 1955, Borsboom 2004, Freiesleben 2026 nomological networks) be operationalized as a quantitative diagnostic within mojave?** Kearns 2026 appears to attempt this. The BEAD-0011 construct validity dossier is deferred but the library has the theoretical foundation. What would a computable construct validity score look like?

## Gaps Identified

### Library gaps (papers to acquire)
- **Generalizability theory**: Zero coverage. Need Brennan 2001 "Generalizability Theory" textbook, Shavelson/Webb 1991, Cronbach 1972 (in intake but not on disk). This is a critical theoretical gap -- G-theory is the direct precursor to mojave's variance decomposition approach.
- **PAWN sensitivity index**: Pianosi et al. 2015 "A simple and efficient method for global sensitivity analysis based on cumulative distribution functions." salib-rs lists PAWN as an estimator but the library has no paper on it.
- **DGSM (Derivative-based Global Sensitivity Measures)**: Sobol/Kucherenko 2009. Listed in salib-rs estimators, not in library.
- **Morris elementary effects method**: Morris 1991 original paper. Critical for the Morris screening salib-rs implements.
- **Inter-rater reliability**: No Fleiss kappa, no Krippendorff alpha papers. Relevant to mojave's irr crate.
- **Quasi-random / low-discrepancy sequences**: No Halton, Sobol (the sequence, not the index), Niederreiter papers. Relevant to salib-rs's QMC samplers.
- **Saltelli textbook**: "Sensitivity Analysis in Practice" (2004) or "Global Sensitivity Analysis: The Primer" (2008) -- the standard reference. Not in library.
- **Iooss/Looss 2015 review**: "A review of global sensitivity analysis methods" -- the most-cited survey of all GSA methods. Missing.
- **Merkle tree / verifiable data structures**: Beyond CT, need Google Trillian, Crosby/Wallach's history tree extension, or the authenticated data structures survey. Thin coverage for audit chain design.
- **Hernandez-Orallo**: "The Measure of All Minds" (2017) -- AI measurement theory book. Zero coverage for a project called "measurement science for AI agents."
- **Campbell & Fiske 1959**: Multitrait-multimethod matrix -- referenced in mojave's own perturbation lit review but not in library.
- **Mechanism design for evaluation**: No Myerson, no incentive-compatible testing literature. Relevant to BEAD-0010 game-theoretic eval design.
- **Clinical trial design / ICH E9**: Referenced in FUTURE_WORK.md for pre-registration but no papers in library.

### Field gaps (where the literature is thin)
- No published work applies full Sobol variance decomposition to LLM evaluation pipelines. Mojave is the first.
- No framework combines tamper-evident audit chains with IRT-calibrated evaluation. This is unique.
- The intersection of sequential testing (e-values) and LLM benchmarking is unexplored. The Balkir 2026 "Confident Rankings" paper (in library) is the closest.
- No paper addresses the measurement uncertainty of WMDP-style MCQ evaluations using GUM-compliant methodology.

## Leads

### Tier 1 -- Read now, directly load-bearing for current work
- `lib/statistics_probability_and_uncertainty/Variance Based Sensitivity Analysis of Model Output - Saltelli et al. 2010.pdf` -- The estimator mojave-gsa uses. Re-read for edge cases in the discrete-factor regime.
- `lib/statistics_probability_and_uncertainty/Borgonovo2007_A_new_uncertainty_importance_measure.pdf` -- Delta measure implemented in salib-rs. Validate against the WMDP decomposition.
- `lib/statistics_and_probability/Ramdas2023_GameTheoreticStatistics.pdf` -- Master reference for seq-anytime-valid crate.
- `lib/statistics_probability_and_uncertainty/Shin2023_EDetectors.pdf` -- Potential unification of spc-charts and seq-anytime-valid.
- `lib/engineering/Hidden Measurement Error in LLM Pipelines - TEE - Anonymous 2026.pdf` -- Directly addresses mojave's thesis. Check if their diagnostics overlap with perturbation engine.
- `lib/management_science_and_operations_research/Madaan2024_Quantifying_Variance_in_Evaluation_Benchmarks.pdf` -- Benchmark variance quantification. Compare methodology to Sobol approach.
- `lib/statistics_probability_and_uncertainty/JCGM2008_GUM_100_UncertaintyMeasurement.pdf` -- Run cards should cite GUM for uncertainty reporting framework.

### Tier 2 -- Read for upcoming work
- `lib/artificial_intelligence/ConstructValidityLLM2026_NomologicalNetworks.pdf` -- For BEAD-0011 construct validity dossier.
- `lib/psychology/Kearns2026_QuantifyingConstructValidity.pdf` -- Operationalizing construct validity quantitatively.
- `lib/artificial_intelligence/Robertson2025_WhatDoesBenchmarkMeasure.pdf` (Jo/Wilson, theory of capability) -- Theoretical grounding for mojave.
- `lib/management_science_and_operations_research/ATLAS2025_AdaptiveTesting.pdf` -- For eval-design CAT engine.
- `lib/information_systems/schneier-kelsey-1999-secure-audit-logs-to-support-computer-forensics.pdf` -- Foundational for audit-chain design validation.
- `lib/artificial_intelligence/crosby-wallach-2009-efficient-data-structures-for-tamper-evident-logging.pdf` -- History trees for audit-chain.
- `lib/management_science_and_operations_research/RFC9162_Certificate_Transparency_v2.pdf` -- Reference architecture for transparency logs.
- `lib/statistics_probability_and_uncertainty/Fischer2024_SPRTOvershoot.pdf` -- SPRT improvement relevant to sequential eval stopping rules.
- `lib/statistics_and_probability/Koning2026_AnytimeValidity.pdf` -- "Anytime validity is free" -- may simplify seq-anytime-valid design.
- `lib/safety_risk_reliability_and_quality/Leveson2004_NewAccidentModel_STAMP.pdf` -- Systems-theoretic framing for defense customer narrative.

### Tier 3 -- Bridges between domains mojave treats as separate
- `lib/statistics_and_probability/Look at the Variance - Efficient Black-box Explanations with Sobol-based Sensitivity Analysis - Fel et al. 2021.pdf` -- Sobol for ML model explanations. Bridge: GSA <-> interpretability.
- `lib/computer_vision_and_pattern_recognition/Sadeghi2024_GSA_Review_DigitClassification.pdf` -- GSA applied to neural networks. Bridge: GSA <-> ML evaluation.
- `lib/statistics_probability_and_uncertainty/Paun2018_BayesianAnnotationModels.pdf` -- Bayesian annotator models. Bridge: IRT <-> inter-rater reliability <-> LLM-as-judge.
- `lib/artificial_intelligence/ConformalPredictionLLMMCQ2023.pdf` -- Conformal prediction for MCQ. Bridge: sequential testing <-> MCQ evaluation.
- `lib/social_sciences/Variance-Aware LLM Annotation for Strategy Research - Camuffo et al. 2026.pdf` -- Variance-aware LLM measurement. Bridge: measurement error <-> annotation reliability.
- `lib/statistical_and_nonlinear_physics/Various2025_SA_NeuralNetworks.pdf` -- SA methods for neural networks. Bridge: classical GSA <-> deep learning evaluation.

## Acquisitions

None downloaded during this scout. Acquisition priorities for the LIBRARIAN:

1. Brennan 2001 "Generalizability Theory" (G-theory textbook) -- HIGH
2. Saltelli et al. 2008 "Global Sensitivity Analysis: The Primer" -- HIGH
3. Iooss & Looss 2015 "A review and perspective on global sensitivity analysis methods" -- HIGH
4. Morris 1991 "Factorial Sampling Plans for Preliminary Computational Experiments" -- HIGH
5. Pianosi et al. 2015 "A simple and efficient method for GSA based on CDFs" (PAWN) -- MEDIUM
6. Sobol/Kucherenko 2009 "Derivative based global sensitivity measures" (DGSM) -- MEDIUM
7. Hernandez-Orallo 2017 "The Measure of All Minds" -- MEDIUM
8. Campbell & Fiske 1959 "Convergent and discriminant validation by MTMM" -- MEDIUM
9. Fleiss 1971 "Measuring nominal scale agreement among many raters" -- MEDIUM
10. Krippendorff 2011 "Computing Krippendorff's Alpha-Reliability" -- MEDIUM
