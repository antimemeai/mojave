# Action Items -- mojave Advisory 2026-05-30

Concrete, prioritized actions grounded in advisory findings. Each item cites supporting reports.

---

## Tier 0: BLOCKING -- Must fix before any external demo or publication

### 0.1 Fix confidence sequence pipeline coverage guarantee
**What:** Replace Welford estimated sigma with sigma=0.5 in AnytimeMonitor for Bernoulli data. Make AnytimeMonitor dispatch on DataFamily instead of ignoring it. Fix the same defect in eval-orchestrator's SequentialInstrument.
**Why:** Monte Carlo shows 46% coverage at p=0.5 (should be 95%). Every CI mojave has reported from this pipeline is invalid.
**Effort:** 1-3 days for immediate fix; 1-2 weeks for correct Waudby-Smith & Ramdas betting CS
**Cites:** Statistical Correctness (wave 2), Adversary findings 1 and 6

### 0.2 Add Gate 4 Monte Carlo test for production CS code path
**What:** Write a test that feeds Bernoulli(p) data through AnytimeMonitor::update().confidence_interval for p in {0.1, 0.3, 0.5, 0.7, 0.9} and verifies >= 93% coverage across 10,000 replications.
**Why:** Gate 4 currently tests normal_mixture_cs_known_sigma (not the production path). The test validates a different function than the one deployed.
**Effort:** 1 day
**Cites:** Statistical Correctness, Adversary finding 1

### 0.3 Add data quality gate in Sobol analysis
**What:** Reject (fail hard) cells with n_samples=0 before Sobol estimation in analyze.rs. Match the Python analyzer's existing guard (n_samples > 0 check at line 129 of analyze_sobol.py).
**Why:** 20 zero-sample cells in bio coded as accuracy=0.0 corrupt variance decomposition. 9 concentrated in quantization column-swap matrix.
**Effort:** < 1 day
**Cites:** Statistical Correctness, Adversary finding 3, GSA Theory

### 0.4 Sign release binaries with Sigstore
**What:** Add cosign signing to the CI/CD pipeline for mojave release binaries.
**Why:** "Signed envelopes from an unsigned binary is theater" (FUTURE_WORK.md). Without binary signing, the entire audit chain trust model is unfounded. Defense customers require binary integrity as table stakes.
**Effort:** 1-2 days
**Cites:** Audit Chain Trust (wave 2), Adversary finding 4, QMU Defense Framework

---

## Tier 1: URGENT -- Before next WMDP run or defense engagement

### 1.1 Rerun WMDP at N=1024 with data quality gates
**What:** Double N from 512 to 1024 (yielding 8,192 cells per benchmark). Apply the n_samples>0 gate. This is the plan's own convergence criterion: CI width exceeded 10%-of-estimate threshold by 4.4x.
**Why:** Current results are under-converged. Negative S1 values are estimation noise, not real effects. Minor factors are indistinguishable from zero.
**Effort:** ~2 GPU-days (68 additional GPU-minutes per benchmark on 7B)
**Cites:** Statistical Correctness, Adversary finding 2, GSA Theory section 2

### 1.2 Report Sobol results with and without "bare" prompt
**What:** Run leave-one-level-out analysis or given-data Sobol on non-bare subset. Report: "Including bare: S1_prompt=0.85; excluding bare: S1_prompt=X." Reframe headline finding.
**Why:** ~60-70% of prompt_template variance driven by one pathological level. "Prompt template dominates" is technically correct but operationally vacuous.
**Effort:** 1-2 days (analysis only if using given-data estimator on existing results)
**Cites:** Statistical Correctness section 4, Adversary finding 5, GSA Theory section 4

### 1.3 Fix or retire Python audit writer
**What:** Either (a) update audit.py to emit tagged-union genesis format with model identity binding, or (b) deprecate audit.py and have Python scripts call `mojave audit emit` via subprocess.
**Why:** Python-Rust chain format divergence means Python-produced chains fail Rust verification. The cross-language test silently skips. Recommend option (b): single source of truth.
**Effort:** 1 week
**Cites:** Audit Chain Trust sections 3 and 9

### 1.4 Run cross-language verification test in CI
**What:** Ensure test_rust_verifier_accepts_python_chain actually runs against a compiled binary in every CI pipeline. Remove the pytest.skip escape hatch.
**Why:** The test that guards Python-Rust parity has likely never passed against the post-genesis-sentinel codebase.
**Effort:** < 1 day
**Cites:** Audit Chain Trust section 3

### 1.5 Implement QMU Confidence Ratio struct
**What:** Build QmuAssessment struct composing outputs from seq-anytime-valid, mojave-gsa, spc-charts, and irr into margin, expanded_uncertainty, confidence_ratio, and a ConformityDecision. Implement JCGM 106 guard-band computation (guarded acceptance with configurable consumer risk).
**Why:** Transforms mojave output from "here are statistics" to "the model does/does not pass under guarded acceptance with consumer risk < 5%." Defense-native decision framework. ~100-200 lines of Rust, zero new math.
**Effort:** 1-2 weeks
**Cites:** QMU Defense Framework (wave 2), X-Factor findings 1 and 3

### 1.6 Add NIST AI 800-3 alignment section to run cards
**What:** Add a section to LaTeX run card template mapping mojave outputs to NIST concepts (benchmark vs generalized accuracy, variance decomposition, statistical model specification).
**Why:** NIST institutional validation is published and waiting to be cited. Defense customers will ask "is this NIST-compliant?"
**Effort:** 2-3 days (template only)
**Cites:** QMU Defense Framework section 5, Web Scout finding 1

---

## Tier 2: HIGH PRIORITY -- Next quarter

### 2.1 Activate G-theory from quarantine
**What:** Move g_theory.rs from quarantine into salib-rs (or directly into mojave crates). Wire D-study projections into run card output. Validate against Brennan 2001 reference formulas.
**Why:** G-theory produces reliability coefficients (G, Phi) that Sobol cannot. D-study projections replace ad-hoc "double N if CIs wide" with principled sample size planning.
**Effort:** 1 week
**Cites:** Gauge R&R (wave 2), Measurement Theory section 3

### 2.2 Add D-study budget optimizer
**What:** Extend D-study from 4-point projection surface to arbitrary grid. Add find_minimum_design(target_phi, cost_function) for constrained optimization of Saltelli N_base.
**Why:** Replaces "run big, hope it's big enough" with "here is how big you need." Saves GPU budget for defense customers.
**Effort:** 1-2 weeks
**Cites:** Gauge R&R sections 2.1-2.3

### 2.3 Implement Waudby-Smith & Ramdas betting CS
**What:** Replace the sigma=0.5 conservative fix with the correct hedged capital confidence sequence for [0,1]-bounded data. Add Gate 4 Monte Carlo calibration test.
**Why:** The betting CS achieves near-optimal width without known sigma. The plan explicitly recommends this method.
**Effort:** 2-3 weeks
**Cites:** Statistical Correctness section 1, Adversary finding 6

### 2.4 Add MSA/Gauge R&R diagnostics (ndc, P/T ratio)
**What:** Compute number of distinct categories (ndc) and precision-to-tolerance ratio (P/T) from existing RatingMatrix data. Emit alongside kappa/alpha in IrrResult.
**Why:** Agreement (IRR) is not discrimination. Judges can agree perfectly and still lack discrimination (ndc < 3). MSA answers "can the judge distinguish performance levels?"
**Effort:** 2-3 days (20-line computations on existing data structures)
**Cites:** Gauge R&R section 4, X-Factor finding 5

### 2.5 Add Mandel h/k outlier diagnostics
**What:** Implement Mandel h (between-config consistency) and k (within-config consistency) statistics. Use bootstrap CIs per Takeshita 2026 (already in intake).
**Why:** Sobol tells you which factors drive variance; Mandel h/k tells you which specific configurations are outliers. Answers "should I remove this configuration?"
**Effort:** 3-5 days
**Cites:** Gauge R&R section 3, X-Factor finding 2

### 2.6 Wire bootstrap CIs into IRR statistics
**What:** Connect the existing bootstrap module to Cohen's kappa, Fleiss' kappa, and Krippendorff's alpha. Populate ci_lower and ci_upper (currently None).
**Why:** QMU framework requires uncertainty-quantified measurement system qualification. Point estimates without uncertainty are insufficient for conformity assessment.
**Effort:** 3-5 days
**Cites:** QMU Defense Framework section 10, Codebase Scout gap 7

### 2.7 Add automated Sobol convergence diagnostics
**What:** Warn when S1 < 0, CI crosses [0,1] boundary, sum_ST > 1.3, or CI width > 10% of point estimate. Automate the "double N" decision.
**Effort:** 2-3 days
**Cites:** GSA Theory section 2, Adversary finding 2

### 2.8 Compute second-order Sobol indices
**What:** Enable calc_second_order=true in Saltelli matrix construction. Compute S2_ij for all factor pairs. Report interaction structure.
**Why:** sum_ST > 1.0 indicates substantial interactions (especially bio at 1.295). The interaction structure is the most scientifically interesting part of the analysis and is currently invisible.
**Effort:** 1 week (cost: from 4096 to 7168 cells per benchmark at N=512)
**Cites:** GSA Theory section 9.3, Statistical Correctness

### 2.9 Deduplicate Sobol computation in mojave-gsa
**What:** Replace local compute_sobol_from_cached and bootstrap_sobol_cis in analyze.rs with calls to salib-rs canonical implementations. Eliminate percentile interpolation discrepancy.
**Effort:** 1-2 days
**Cites:** GSA Theory section 9.1

### 2.10 Write canonical encoding specification
**What:** One-page document pinning sort order (UTF-8, not JCS UTF-16), escaping rules, float rejection, integer representation. Reference from chain verification docs.
**Why:** The encoding is defined by code, not specification. Third-party verifiers must reverse-engineer behavior from source. Serde version sensitivity creates silent breakage risk.
**Effort:** 1 day
**Cites:** Audit Chain Trust section 8

---

## Tier 3: MEDIUM PRIORITY -- Next 6 months

### 3.1 Build the construct validity dossier (BEAD-0011)
**What:** Structured dossier with 6 evidence slots: content validity (item pool coverage), structural validity (IRT model fit), convergent/discriminant validity (MTMM CFA via semopy), sensitivity validity (CVI = 1 - sum S1_irrelevant), consequential validity (ergodicity premium from SPC). Implement CVI as derived statistic.
**Cites:** Measurement Theory sections 2 and 6

### 3.2 Add Rasch model fit diagnostics
**What:** Fit 1PL alongside 2PL in mojave-calibrate. Compute infit/outfit/point-measure correlation. Emit "Measurement Comparability" diagnostic for CAT sessions.
**Why:** Specific objectivity (item-free person comparison) only holds under Rasch, not 2PL. If items don't fit Rasch, adaptive testing may produce non-comparable estimates.
**Cites:** Measurement Theory section 4, X-Factor finding 7

### 3.3 Add MTMM CFA for convergent/discriminant validity
**What:** Use existing semopy CFA infrastructure to fit correlated-traits/correlated-methods model. Extract trait vs method factor loadings.
**Cites:** Measurement Theory section 2, X-Factor finding 6

### 3.4 Implement GSN assurance case template
**What:** New LaTeX template mapping QMU outputs to GSN goals, evidence, and defeaters. Three-tier: automated accept/reject, investigate, risk acceptance.
**Why:** Defense procurement evaluators read assurance cases, not statistics reports. UK MOD Def Stan 00-56 mandates this format.
**Effort:** 1 week (template only)
**Cites:** QMU Defense Framework section 4, X-Factor finding 10

### 3.5 Add Rekor external witnessing
**What:** Submit periodic chain-head snapshots to Sigstore Rekor. Store Rekor log entry index alongside CBOR attestation.
**Why:** Provides third-party proof-of-existence timestamps. Without this, chain timestamps are self-asserted.
**Effort:** 1-2 weeks
**Cites:** Audit Chain Trust section 4

### 3.6 Upgrade key management for production
**What:** Replace KeyRef::Env with KeyRef::EncryptedFile (OS keychain) or KMS integration.
**Why:** Environment variable key storage is unacceptable under NIST 800-171 / CMMC.
**Cites:** Audit Chain Trust section 6

### 3.7 Enforce WeightFile hash method for production chains
**What:** Add configuration flag rejecting StructuredDescriptor in production mode.
**Why:** StructuredDescriptor hashes model metadata (name, provider) which is trivially spoofable. WeightFile hashes actual model weights.
**Cites:** Audit Chain Trust section 2 (Claim 3)

### 3.8 Evaluate full factorial vs Saltelli for discrete factors
**What:** For the current 6-factor design (5x4x4x2x3x2 = 960 cells), compare full factorial with replication against Saltelli N=512 (4096 cells). Measure convergence properties.
**Why:** Saltelli is designed for continuous inputs. With few discrete levels, full factorial may be cheaper and more statistically efficient.
**Cites:** GSA Theory section 6, Gauge R&R section 1.1

### 3.9 Add ergodicity diagnostics to SPC crate
**What:** Implement ADF test, KPSS test, and ergodicity premium calculation (benchmark_accuracy - mean(spc_observations)).
**Why:** Benchmark ensemble averages systematically overestimate time-average deployment performance for non-ergodic processes.
**Cites:** Measurement Theory section 5, X-Factor finding 9

### 3.10 Add ISO 5725 repeatability/reproducibility reporting
**What:** Group Saltelli cells by configuration. Compute repeatability sigma_r, reproducibility sigma_R, repeatability limit r, reproducibility limit R.
**Why:** ISO 5725 is a recognized standard in defense procurement. Converts sensitivity analysis into metrology-compliant study.
**Cites:** Gauge R&R section 3, X-Factor finding 2

### 3.11 Add bootstrap coverage test for Sobol CIs (Gate 4)
**What:** Generate K=1000 bootstrap CIs at N=512 on Ishigami. Verify 95% nominal coverage. Test against analytic S_i values.
**Cites:** GSA Theory section 8.3

---

## Tier 4: STRATEGIC -- Design now, build when needed

### 4.1 Implement IRT item calibration (self-calibration gap)
**What:** Add MMLE or EM-based item parameter estimation to close the self-calibration gap. Currently mojave can only consume pre-calibrated parameters.
**Cites:** Codebase Scout gap 1

### 4.2 Implement DIF detection for cross-model fairness
**What:** Mantel-Haenszel or logistic regression DIF. Detect whether items behave differently across model families.
**Cites:** Codebase Scout gap 2, Library Scout gap

### 4.3 Design multi-chain campaign correlation
**What:** "Campaign root" entry shared across per-model chains proving they were produced during the same campaign.
**Cites:** Audit Chain Trust section 9

### 4.4 Publish Sobol-as-validity-coefficient framing
**What:** Methods paper establishing CVI = 1 - sum(S1_irrelevant) as a quantitative construct validity metric under Borsboom's causal definition.
**Cites:** Measurement Theory section 6, synthesis

### 4.5 Implement OT-GSA in salib-rs
**What:** Optimal-transport sensitivity indices (Borgonovo 2024). Currently R-only (gsaot). Handles multivariate outputs.
**Cites:** Web Scout finding 5, GSA Theory section 5
