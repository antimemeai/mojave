# Dissent -- mojave Advisory 2026-05-30

**Role:** 10th Dentist
**Agent:** claude-opus-4-6
**Date:** 2026-05-30

I have read the MANIFEST, all five wave-1 deposits, all six wave-2 deposits. No synthesis existed at the time of writing. The advisory is thorough, technically competent, and internally consistent. That is precisely what makes the unexamined assumptions dangerous. What follows is what the advisory got wrong, overstated, or declined to question.

---

## Dissenting Findings

### 1. The QMU/metrology vocabulary is strategic theater, not engineering necessity

**The claim:** The QMU framework (Confidence Ratio = margin/uncertainty) is a "thin integration layer over existing primitives" that "reframes mojave's output from 'here are your statistics' to 'here is a quantified safety argument with explicit risk'" (wave_2/01, Section 1). The advisory treats this as zero-new-math, enormous-strategic-value.

**The problem:** QMU was developed for certifying nuclear warhead reliability when full-scale testing was prohibited. The analogy is seductive but leaks badly. Nuclear QMU operates on systems with known physics, decades of subcritical test data, validated computational models, and performance thresholds derived from weapons requirements with fixed consequence severity. LLM evaluation has none of these properties. There is no validated computational model of "reasoning." There are no agreed-upon performance thresholds for most capabilities. The "margins" change with every model release, prompt revision, and fine-tuning run. The "uncertainties" are dominated by factors (benchmark contamination, sandbagging, distribution shift) that QMU's aleatory/epistemic separation does not address because they are neither stochastic noise nor reducible-by-measurement -- they are adversarial or structural.

The advisory recommends CR thresholds of 2:1 to 10:1 "calibrated over decades of engineering experience" for nuclear weapons. For AI evaluation, there is no calibration data. The advisory explicitly acknowledges "there is no published guidance" on appropriate CR thresholds for AI (wave_2/01, Section 9, gap 1), then proceeds to recommend implementing the framework anyway. The resulting CR numbers will have no empirical grounding -- they will be precision without accuracy. Defense customers will see a number that looks like a QMU Confidence Ratio and assume it carries the inferential weight of a nuclear QMU CR, when in fact it carries only the weight of whatever ad hoc threshold the mojave team chose.

**The evidence:** The National Academies 2009 QMU review found significant disagreement between LLNL, LANL, and Sandia on how to compute CR even within the nuclear domain, with decades of shared physics. Transporting CR to a domain with no shared physics, no validated models, and no calibration history is not "thin integration" -- it is metaphor dressed as mathematics. The advisory's own wave_2/01 lists five open questions about QMU application to AI that are not resolvable by implementation work alone (multivariate QMU, non-normal measurement distributions, CR threshold selection, SPC interaction with QMU, coverage factor selection). These are not gaps to be closed; they are symptoms that the framework may not transfer.

**What the project should do instead:** Use the GUM uncertainty budget and JCGM 106 guard-band decision rules -- these are genuinely applicable to any measurement and do not require the structural analogy to nuclear weapons. Drop the QMU vocabulary unless and until the project can demonstrate empirical calibration of CR thresholds on real deployment data. Use the defense-procurement language (margins, thresholds, consumer risk) without claiming the inferential apparatus of nuclear stockpile certification.

---

### 2. The "prompt template explains 85% of variance" finding is the project's showcase result, and it is broken

**The claim:** Sobol decomposition reveals that prompt template is the dominant source of variance in WMDP evaluation (S1 ~ 0.85-0.93 across benchmarks), demonstrating mojave's ability to surface measurement-relevant variance structure.

**The problem:** The advisory's own adversary and GSA deep-dive both identify that this finding is driven by a pathological "bare" prompt level that strips all formatting instructions. Wave_2/06 Section 4.2 quantifies it: "roughly 60-70% of prompt_template's apparent variance is driven by the 'bare' level alone." The total output variance drops from 0.058 to 0.006 when bare cells are excluded. This means the headline result is not "prompt engineering matters enormously for WMDP" but "if you remove all instructions from the prompt, the model fails." This is not a measurement insight; it is a truism.

The advisory identifies this problem clearly but then treats it as a reporting issue to be fixed by "running both with and without bare" rather than as a foundational problem with the project's first empirical demonstration. If mojave's marquee result needs an asterisk the size of the result itself, the demonstration has failed its purpose. The secondary finding -- that among realistic prompt templates, prompt_template explains 25-40% of variance -- is genuinely interesting but has not been computed with a proper Saltelli design. The "given-data Sobol estimator on a subset" workaround suggested by the advisory is less efficient and not validated against the full-design estimator on mojave's own data.

Additionally, the advisory found 20 zero-sample cells coded as accuracy=0.0 (wave_2/02 Section 3), the confidence sequence pipeline using estimated sigma that produces 46% coverage instead of 95% (wave_2/02 Section 1), the plan's own convergence threshold violated by 4.4x without triggering the planned doubling (wave_2/02 Section 2.4), and negative Sobol indices for minor factors. The entire WMDP Phase 1 analysis is built on statistically invalid confidence intervals applied to corrupted data with insufficient sample size in a design dominated by a pathological perturbation level. Every one of these problems was documented across the advisory. None of them were caught before the advisory.

**The evidence:** Wave_2/02 Monte Carlo table (46% coverage at p=0.5, 17% at p=0.1). Wave_2/06 Section 4.2 (bare-prompt dominance). Wave_2/06 Section 3 (20 corrupted cells). Wave_2/02 Section 2.4 (CI width 4.4x threshold). These are not edge cases; they are the primary results.

**What the project should do instead:** Treat the WMDP Phase 1 results as a pilot study, not a demonstration. Fix the confidence sequence pipeline (sigma=0.5 upper bound as immediate patch). Add the n_samples=0 data quality gate. Rerun with N=1024 and a 4-level prompt_template axis (excluding bare). Only then does mojave have a defensible showcase result.

---

### 3. The 4-layer measurement qualification stack is a framework the advisory invented, not something the project has validated

**The claim:** Wave_2/04 proposes a "measurement system qualification stack" with four layers: MSA gauge qualification, ISO 5725 configuration validation, G-theory reliability assessment, and Sobol sensitivity analysis, "each layer gating the next."

**The problem:** This stack is compelling on paper. It is also entirely aspirational. As of today:

- **Layer 1 (MSA):** Not implemented. The IRR crate has no ndc or P/T computation. No customer has been asked whether gauge qualification is a workflow they want.
- **Layer 2 (ISO 5725):** Not implemented. No Mandel h/k statistics. No repeatability/reproducibility decomposition.
- **Layer 3 (G-theory):** Quarantined. The code exists but was deliberately removed from the active codebase. The advisory recommends activating it without discussing why it was quarantined in the first place.
- **Layer 4 (Sobol):** Production-grade but with the statistical issues documented in Finding 2.

The advisory recommends implementing all four layers without presenting evidence that any customer or potential customer has asked for measurement system qualification, interlaboratory comparison diagnostics, or generalizability coefficients. The defense market positioning is asserted ("ISO 5725 is a recognized standard in defense procurement") but no customer interview, RFP analysis, or competitive analysis demonstrates demand for this specific stack.

The ratio of "frameworks the advisory recommends" to "things the project has validated with real customers" is approximately infinity. The entire advisory recommends acquiring 30+ papers, implementing 6+ new statistical methods, adding 3+ new crates or major crate extensions, building assurance case templates, adding Sigstore integration, and upgrading key management. The project has one customer-facing empirical result (WMDP Phase 1), and that result is compromised (Finding 2).

**The evidence:** Zero customer validation is cited anywhere in the advisory. The CLAUDE.md mentions "defense establishment first" but no specific defense customer engagement is referenced. The FUTURE_WORK.md lists 8 deferred items, several of which are prerequisites for the advisory's recommendations (pre-registration, range orchestration, runner integrations). The project is designing a measurement qualification stack for a market it has not yet entered.

**What the project should do instead:** Pick one layer and validate it with one customer before building the stack. The most natural candidate is Layer 4 (Sobol), which is the closest to production-ready. Get the WMDP results fixed (Finding 2), present them to a real defense customer, and learn whether variance decomposition is what they need before building three additional layers on top of it.

---

### 4. salib-rs is a maintenance liability, not a strategic moat

**The claim:** salib-rs is "NON-NEGOTIABLE" (CLAUDE.md) and positioned as a strategic asset -- the only Rust GSA library, a strict superset of Python SALib, with 77 TCK specs and 4-gate validation.

**The problem:** The advisory confirms salib-rs is correct (wave_2/06 verifies all 9 estimators). But correctness is not the same as strategic soundness. The project is maintaining a full-featured sensitivity analysis library covering Saltelli, Jansen, Janon, Owen, Borgonovo delta, PAWN, DGSM, ANOVA, G-theory, Morris screening, Shapley effects, PCE surrogates, and HDMR -- all of this for a product that currently uses exactly one estimator (Saltelli 2010 Eq c) on exactly one type of data (discrete MCQ accuracy) with exactly one sampling strategy (Saltelli radial). The other estimators exist in the library and are tested against the Ishigami function, but they have never been exercised on real mojave data.

The bus-factor risk is real: one person maintains the entire GSA library, the entire mojave measurement engine, and the Python orchestration layer. The advisory's own adversary finding (wave_1/adversary, Finding 7) flags this but the wave-2 deep dives proceed to recommend expanding salib-rs's scope (OT-GSA, BCa bootstrap, convergence-rate diagnostics, discrete-factor convergence tests, ANOVA alternative evaluation). Each recommendation adds maintenance surface.

The "NON-NEGOTIABLE" stance prevents the project from using Python SALib as a development accelerator during the phase where speed matters most. The advisory found (wave_2/06, Section 9.1) that mojave-gsa duplicates Sobol/bootstrap computation locally rather than calling salib-rs canonical implementations, suggesting the two codebases are already drifting. If the project's own analysis pipeline duplicates the library it insists on owning, the ownership constraint is generating cost without eliminating risk.

Meanwhile, the advisory identifies that for mojave's current 6-factor discrete design, "a full factorial (960 cells) with replication may be more statistically efficient than the 4096-cell Saltelli design" (wave_2/06, Section 6.3, wave_2/04, Section 1.1). If true, this means salib-rs's core Saltelli sampling machinery -- the most complex part of the library -- is overkill for the project's actual use case. The project may be maintaining a continuous-input sensitivity analysis library for a problem that is better solved by classical ANOVA.

**The evidence:** Wave_2/06 Section 6 (discrete factor problem). Wave_2/04 Section 1.1 (full factorial cheaper than Saltelli). Wave_2/06 Section 9.1 (code duplication between mojave-gsa and salib-rs). Wave_1/adversary Finding 7 (bus-factor risk). Wave_1/adversary Finding 7 also notes Gate 2 R cross-checks silently skip when fixtures are missing.

**What the project should do instead:** Proceed with eyes open about the maintenance cost. The library's correctness is a genuine asset and the 4-gate validation is impressive. But stop expanding the library's scope until it is exercised on real customer data beyond the Ishigami function. Consider whether the full-factorial + ANOVA path should be the default for discrete-factor designs, with Saltelli reserved for future continuous-factor or high-dimensional problems. And fix the code duplication in mojave-gsa -- it is actively undermining the rationale for maintaining salib-rs.

---

### 5. The audit chain trust model is weaker than the advisory admits

**The claim:** Wave_2/05 assesses the audit chain and concludes "the path from current state to genuine tamper-evidence is short -- maybe 2-3 weeks of focused work." Binary signing is the primary gap, with Python-Rust parity as secondary.

**The problem:** The advisory is correct that binary signing is critical. But the "2-3 weeks" estimate understates the gap because it counts only implementation effort, not the organizational and procedural changes required for defense deployment:

1. **Key management.** The advisory notes that `KeyRef::Env` is insufficient for NIST 800-171 / CMMC compliance but estimates key management upgrade as a "near-term" task. For defense customers, key management is not a feature -- it is a compliance regime. HSM procurement, key ceremony procedures, key rotation schedules, incident response for key compromise, audit logging of key operations -- these are not 2-week tasks. They are procurement and compliance processes that can take months.

2. **Reproducible builds.** Binary signing proves the binary has not been tampered with after signing. It does not prove the binary was built from the claimed source code. For defense customers who need to audit the supply chain, reproducible builds (deterministic compilation from auditable source) are required alongside signing. The advisory does not mention reproducible builds.

3. **Python-Rust parity is not "broken" -- it is abandoned.** The advisory found that the Python writer produces chains the Rust verifier cannot consume (wave_2/05 Section 3). The cross-language verification test "is unclear whether this test has ever passed against the current codebase (post-genesis-sentinel merge)." This is not a parity bug; it is a dead code path that has been non-functional through at least one major merge. The advisory recommends either fixing it or removing the Python writer. The recommendation should be stronger: remove the Python writer. A "cross-language compatibility contract" that has never been tested is not a contract -- it is a liability.

4. **Model identity binding.** The advisory notes that production chains use `StructuredDescriptor` (hash of metadata), not `WeightFile` (hash of actual weights). It recommends enforcing `WeightFile` for production. But hashing a 7B model's weights takes non-trivial time and requires access to the weight files at chain genesis time. For API-served models (which are the majority of frontier models), weight-file hashing is impossible -- the customer does not have the weights. The advisory does not address how model identity binding works for API-served models.

**The evidence:** Wave_2/05 Section 6 (threat model gaps table lists 11 gaps, of which only 2 are addressed by binary signing). The test `test_rust_verifier_accepts_python_chain` uses `pytest.skip` when no binary is found -- this test is the only thing preventing silent format drift and it is not running.

**What the project should do instead:** Drop the "2-3 weeks" estimate. The audit chain is sound cryptographically but the deployment story requires compliance infrastructure that is measured in months, not weeks. Remove the Python audit writer immediately. Design for API-served model identity (e.g., fingerprinting via API responses, or accepting structured descriptors with documented limitations). And do not promise defense customers "tamper-evident evaluation" until binary signing, key management, and a formal threat model document ship together.

---

### 6. The advisory is solving a supply problem when the actual bottleneck is demand

**The claim:** Implicit throughout the advisory: mojave needs more statistical methods, more frameworks, more theoretical grounding, more library acquisitions, more paper citations. The advisory recommends acquiring 30+ papers and implementing 10+ new capabilities.

**The problem:** The advisory never asks whether anyone wants what mojave is building. The project's positioning is "measurement science for AI agents" targeting the defense establishment. The advisory validates this positioning by citing regulatory deadlines (Aug 2026 DoD NDAA), NIST publications (AI 800-3), and institutional frameworks (QMU, ISO 5725, JCGM 106). But regulatory deadlines create compliance demand, not measurement-science demand. The DoD AI cybersecurity mandate requires "lifecycle security, model tampering protection, data leakage prevention, continuous monitoring" -- these are security controls, not measurement qualification stacks.

The advisory cites Rabanser et al. 2026 as "the closest published analog to mojave's mission." Rabanser's headline finding is "recent capability gains yield only small reliability improvements." This is interesting to researchers. Is it interesting to defense procurement officers? The procurement question is: "does this model pass the evaluation?" not "is the evaluation reliable?" If the customer's question is the former, mojave is answering a question they did not ask. If the customer's question is the latter, mojave needs to demonstrate that unreliable evaluation leads to bad procurement decisions -- a claim the project has not made empirically.

The advisory's competitive analysis (wave_1/web) finds that "no framework combines GSA + audit chains + confidence sequences." This is presented as a strategic moat. An alternative interpretation: nobody has combined these things because nobody has found the combination useful. First-mover advantage is only an advantage if the market exists.

**The evidence:** Zero customer engagement data in the advisory. No competitive win/loss analysis. No pricing model. No pilot customer results. The enter_mojave_plan file is 56KB -- suggesting extensive planning -- but the data/ directory contains results from one benchmark suite (WMDP) on one model (Qwen2.5-7B-Instruct). The ratio of planning to empirical validation is extreme.

**What the project should do instead:** Before building any more measurement infrastructure, run a customer discovery sprint. Talk to 3-5 defense AI evaluation teams. Ask what their current eval workflow is, where it breaks, and what they would pay to fix. If the answer is "I need my confidence sequence to use the correct distributional family for Bernoulli data," then proceed as planned. If the answer is "I need to run MMLU faster on 10 models," then mojave is solving the wrong problem. The advisory's recommendations are all technically sound. Whether they are commercially relevant is entirely unknown.

---

### 7. The Construct Validity Index is a novel theoretical contribution that has not been validated

**The claim:** Wave_2/03 proposes CVI = 1 - sum(S1_irrelevant) as "a novel theoretical contribution that mojave can make." The CVI for WMDP-bio is estimated at ~0.12, described as "devastating for WMDP's validity claims."

**The problem:** CVI is not a validity coefficient. It is a variance proportion. Borsboom's causal definition of validity (which the advisory cites as the theoretical foundation) says "a test is valid if variation in the attribute causes variation in the test score." The CVI inverts this: it computes the fraction of test-score variance NOT explained by construct-irrelevant factors. But unexplained variance is not the same as construct-relevant variance. The residual variance after removing prompt_template, system_prompt, decoding, etc., could be driven by item-level noise, model-specific quirks, or unmeasured confounds -- not by the construct (hazardous knowledge).

The specific CVI=0.12 estimate is contaminated by the bare-prompt problem (Finding 2). Without bare, S1_prompt_template drops to ~0.25-0.40, and CVI rises to ~0.50-0.65. The jump from "devastating CVI=0.12" to "decent CVI=0.55" depending on whether you include a pathological perturbation level demonstrates that CVI is sensitive to design decisions, not just construct properties. A validity coefficient that changes by 5x based on factor level selection is not measuring validity -- it is measuring the analysis design.

More fundamentally: the advisory proposes that the customer should declare which factors are "construct-relevant" and which are "construct-irrelevant," and CVI is computed from this declaration. But this means CVI is not a property of the test -- it is a property of the customer's declaration. Two customers looking at the same data with different declarations get different CVIs. This is the opposite of what a validity coefficient should do.

**The evidence:** Wave_2/03 Section 6 (CVI definition and WMDP calculation). Wave_2/06 Section 4.2 (bare-prompt leverage on prompt_template S1). The advisory's own caveat that "CVI rises substantially if bare is excluded" is buried in a parenthetical in Section 6, not treated as the central problem it is.

**What the project should do instead:** Do not publish CVI as a validity coefficient. The Sobol decomposition of eval-pipeline variance is genuinely useful as a diagnostic -- it tells you where the variance is coming from. Reframe it as a "measurement noise budget" or "sensitivity profile" rather than claiming it is a validity measure. The connection between Sobol indices and Borsboom's causal validity is suggestive but not formally established; establishing it would require a proper psychometric study with known-validity and known-invalidity benchmarks.

---

### 8. The advisory recommends too many things and prioritizes none of them

**The claim:** The advisory does not have a single prioritization section; recommendations are scattered across 11 deposit files with inconsistent priority labels.

**The problem:** Across the deposits, I count approximately:
- 10 "immediate" recommendations
- 15 "near-term" recommendations  
- 12 "medium-term" recommendations
- 6 "strategic" recommendations
- 30+ library acquisition recommendations
- 5 "blocking" items

If everything is a priority, nothing is. The advisory recommends fixing the confidence sequence pipeline AND implementing QMU AND building the 4-layer measurement stack AND fixing the audit chain AND implementing CVI AND adding Rasch fit diagnostics AND computing second-order Sobol indices AND adding MSA/Gauge R&R AND implementing ISO 5725 AND upgrading key management AND integrating Sigstore AND activating G-theory from quarantine AND adding ergodicity diagnostics AND building assurance case templates AND running customer discovery.

A solo developer (or even a small team) cannot execute on 40+ recommendations simultaneously. The advisory's value is diminished by its inability to say: "do these 3 things first, ignore the rest until they are done."

**The evidence:** The recommendation counts above, tabulated from all deposits.

**What the project should do instead:** The three things that matter most, in order:

1. **Fix the confidence sequence pipeline** (sigma=0.5 upper bound, n_samples=0 data quality gate). This is a correctness bug in load-bearing code. Nothing else matters until the statistical engine produces valid results. Estimated effort: 1-2 days.

2. **Rerun WMDP with N=1024 and bare-prompt-excluded secondary analysis.** This gives the project a defensible showcase result. Estimated effort: 1 week including compute time.

3. **Talk to a defense customer.** Before building anything else, validate that what exists is what they need. Estimated effort: 2-4 weeks to get a meeting and present results.

Everything else -- QMU, ISO 5725, MSA, G-theory, Sigstore, CVI, assurance cases, ergodicity diagnostics -- is inventory until customer demand justifies it.

---

## Unresolved Risks

1. **Single-developer dependency.** Patrick owns the Rust codebase, the Python pipeline, the salib-rs library, the paper library, and the customer relationships. The advisory exacerbates this by recommending 40+ tasks that only Patrick can execute.

2. **Benchmark validity circular dependency.** Mojave measures the reliability of benchmarks. But if benchmarks are fundamentally unreliable (the advisory's own thesis), then mojave's measurements of unreliable benchmarks are measurements of noise. The value proposition requires that benchmarks are partially valid -- enough that measuring them is useful, but not so valid that measuring them is unnecessary. This Goldilocks zone has not been empirically characterized.

3. **Defense market timing.** The Aug 2026 NDAA deadline is real, but the resulting DoD requirements may specify tools that look nothing like mojave. If DoD mandates NIST AI 800-3's GLMM approach (not Sobol), or Inspect (not mojave's orchestrator), or CMMC-certified audit tools (not mojave's chain), then mojave is building to a standard that does not exist yet and may never align.

4. **Open-source exposure.** The project is dual-licensed Apache/MIT. The 4-gate validation methodology, salib-rs, and the audit chain construction are all public. A well-funded competitor (RAND, MITRE, a large defense contractor) could fork the codebase, add the compliance wrapper, and sell it to DoD without mojave in the loop. The strategic moat depends on execution speed and customer relationships, not IP.

5. **The advisory itself is an artifact of the methodology problem.** Eleven agents spent substantial compute producing 11 reports that largely agree with each other, which is not how a diverse advisory board works. The agents share the same training data, the same biases toward comprehensive coverage, and the same inability to talk to customers. The advisory's consensus on "build more statistical infrastructure" may reflect model priors about what rigorous research looks like, not independent judgment about what the market needs.

## The Strongest Argument Against This Project

Mojave is a measurement tool for benchmarks. Benchmarks are broken. Benchmarks are being replaced by agentic evaluations, human-in-the-loop assessments, and deployment monitoring. The measurement-science framing assumes that the object being measured (the benchmark score) is worth measuring precisely. If the field moves to evaluations that look nothing like MCQ benchmarks -- as the agent reliability literature (Rabanser 2026) and the agentic eval literature strongly suggest -- then mojave's Sobol-over-MCQ-accuracy capability becomes obsolete before it is commercialized.

The defense establishment buys solutions to current problems, not anticipated problems. The current problem is "how do I evaluate an LLM for my use case?" not "how do I decompose the variance of my LLM evaluation into perturbation-factor contributions with anytime-valid confidence intervals." Mojave is building a microscope when the customer needs a thermometer.

## What Would Change My Mind

1. **A defense customer paying for a pilot.** Not a meeting, not interest -- a contract. Money is the only reliable signal that the problem is real and the solution is valued.

2. **The WMDP Phase 1 results surviving a rerun** with corrected confidence sequences, cleaned data, N=1024, and bare-excluded secondary analysis. If the Sobol decomposition still surfaces actionable variance structure among realistic prompt templates, the methodology is validated.

3. **A published comparison** showing that mojave's Sobol decomposition catches a real measurement problem that standard methods (bootstrap CIs, fixed-sample tests, simple repeatability checks) miss. The advisory assumes this is true but has not demonstrated it.

4. **Evidence that the 4-layer measurement stack reduces decision errors** in a controlled setting. Run a simulated procurement decision with and without mojave's measurement qualification pipeline and show that the qualification stack prevents bad procurement decisions (accepting models that fail in deployment, rejecting models that would succeed).

5. **A second developer contributing to salib-rs.** The bus-factor risk is existential. A single external contribution that passes the 4-gate validation would prove the library can survive its creator.
