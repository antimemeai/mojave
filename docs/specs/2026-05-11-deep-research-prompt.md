# Deep Research Request: Longitudinal SPC for Agent Evaluation

## Context

I'm building a product that applies rigorous measurement science to the evaluation of AI agents. Not a benchmark. Not a dashboard. A framework that surfaces the measurement-quality questions developers don't know to ask about their evaluations, then runs statistically rigorous experiments (powerset ablations, fractional factorials, sequential testing) on the customer's own infrastructure to answer them.

The core thesis: teams building agents are making deployment decisions based on eval scores they've never validated. They don't know if their judges agree with each other, whether their score movements are real or noise, which of their tasks actually discriminate between agent versions, or what factors drive their scores. This product answers those questions with statistical rigor inherited from classical measurement theory (psychometrics, clinical trials, SPC) and incorporating emerging AI measurement science (Stanford AIMS, NIST CAISI).

## Technical Design

### Architecture

All logic in Rust (compiled, fast). Python is a thin user-facing scripting shell and integration point for torch_measure (Stanford AIMS IRT/factor model toolkit).

Components communicate via binary APIs with efficient serialization (bincode) internally, JSON only at user-facing boundaries. No FFI coupling.

### Math Core (Rust crates, built first)

1. **salib-rs** — Global sensitivity analysis library. Complete Rust implementation of SALib-equivalent functionality: Sobol (Saltelli2010, Jansen, Janon, Owen), Morris, FAST, RBD-FAST, Borgonovo δ, PAWN, DGSM, regression (SRC/SRRC/PCC/PRCC), given-data Sobol, G-theory, ANOVA, PCE, Shapley effects, active subspaces. Deterministic (ChaCha20 + tree-fold), validated against SALib reference outputs. Already implemented — needs repack for publication. No Rust SALib equivalent exists in the wild.

2. **irr** — Inter-rater reliability. Krippendorff α, Fleiss κ, Cohen κ/weighted κ, Gwet AC1/AC2, Bland-Altman. Bootstrap CIs for all. Handles missing data, exposes paradox behavior. Answers: "do your judges agree?"

3. **seq-test** — Sequential testing. Wald SPRT, Pocock, O'Brien-Fleming, Lan-DeMets α-spending. Bias-adjusted estimators at stopping time. Answers: "can you stop evaluating early with controlled error?" Makes expensive ablations affordable.

4. **reliability** — Classical reliability + IRT. McDonald's ω (primary), Cronbach α (documented limitations), IRT 1PL/2PL/3PL with mandatory convergence diagnostics. Answers: "how reliable is your scoring?"

5. **prereg** — Pre-registration contracts. Hash-anchored analysis plans, canonical serialization, deviation detection. ICH E9 R1 estimand structure adapted to eval. Answers: "did you execute what you planned?"

### Orchestration Layer (Rust, built after math core)

- Experiment designer (ablation schedule generation — full/fractional factorial, blocking, randomization)
- Scheduler (dispatch runs to customer infra, concurrency, retries)
- Range manager (repeatable agent execution environments)
- Results collector (outcomes → change×task matrix)
- State manager (longitudinal tracking, baselines, SPC control limits)
- Optional git/VCS integration for change attribution

### Deployment Modes

- **Hosted (AWS)**: multi-tenant, managed, cost attribution
- **On-prem**: single-tenant, customer provides infrastructure, we provide the binary

Same core in both. Multi-tenancy layered on for hosted.

### The Product Surface

The output isn't a score. It's an eval integrity report:

> "Run 47: agent scored 72% after change Y (baseline: 75%). Noise floor: ±3.1%. Regression verdict: NOT SIGNIFICANT. Eval health: judge IRR α=0.71 (acceptable), 6/40 tasks non-discriminating (flagged), perturbation sensitivity dominated by model factor (good). Trust level: MODERATE — judge agreement below 0.8 threshold."

Over time, this builds a change×task matrix — which changes affected which tasks, with what confidence, enabling blast-radius prediction and causal attribution.

### Validation Discipline

Every math primitive passes four gates:
1. Textbook reproductions (golden datasets from canonical papers)
2. Reference implementation cross-checks (R packages, pinned versions, automated in CI)
3. Property-based tests (invariants, identities, boundary conditions)
4. Monte-Carlo calibration cards (coverage, Type-I, power)

## Market Thesis

### Primary market: Defense / IC

AI agents are being deployed in defense and intelligence contexts. T&E (Test & Evaluation) offices require defensible claims about system capability. "We ran it and it scored 85%" doesn't pass muster — they need to know: is that score reliable? Is the eval valid? Does it generalize? Can you reproduce it?

This product provides exactly the evidentiary basis T&E needs: statistical rigor, pre-registered analysis plans, tamper-evident audit trails, reproducible execution on controlled ranges.

**Market access:** I believe I have (with a potential partner) a substantially lowered barrier to entry into the defense market. Not assured, but the path is real and the relationships exist. This is not a cold-start problem.

### Secondary markets (follow-on)

- Regulated industries (healthcare, finance) as AI evaluation requirements tighten
- Enterprise AI teams as agent deployment becomes safety-critical
- Broad market arrives 12-18 months after defense/regulated adoption creates the standard

### Competitive landscape

- **Braintrust, Arize, LangSmith, Humanloop**: Observability/dashboard plays. Sell "see what your agent is doing." Don't measure eval quality.
- **Stanford AIMS / torch_measure**: Academic tooling (MIT-licensed). IRT, adaptive testing, benchmark bug detection. Excellent science, no product, no deployment story. Complementary — we integrate their Python toolkit.
- **NIST CAISI**: Setting standards, not building products. Institutional validation of the approach.
- **RAND Judge Reliability Harness**: One-off tool for judge stress-testing. Not a platform.
- **Nobody**: occupies the position of "run rigorous experiments on your eval to tell you if you can trust it."

### Moat

1. **salib-rs**: Only Rust GSA library. No equivalent exists.
2. **Sensitivity analysis applied to eval pipelines**: Nobody else does systematic perturbation-based variance decomposition on eval factors.
3. **4-gate validation**: Every statistical claim has provenance — textbook, reference-impl, property-test, and calibration backing. Defensible under scrutiny.
4. **Defense market access**: Relationships + clearances + domain understanding.
5. **Integrated methodology**: Not just statistics (AIMS) or just infrastructure (Braintrust) — both, composed correctly.

## Research Questions

Please investigate the following with depth and rigor. Disconfirmation is welcome — a clean "this doesn't work because X" is more valuable than hedged encouragement.

### 1. Market validation

- Is the defense T&E need real and urgent, or aspirational? What specific programs, directives, or acquisition requirements create demand for rigorous AI evaluation tooling right now (2026)?
- What's the actual procurement mechanism? Is this a SBIR/STTR play, a direct contract opportunity, an OTA, or something else? What's the typical timeline from "they want this" to "they're paying for it"?
- Are there competing efforts inside the defense establishment (internal tools, FFRDC work, other contractors) that would preempt a commercial offering?
- Size the addressable market: how many programs are deploying AI agents that need T&E rigor, and what would they plausibly pay for evaluation tooling?

### 2. Technical risk assessment

- Is the IRT/psychometric framework valid for task-completion evaluation of agents (binary/partial-credit outcomes), or does it break down outside traditional multiple-choice benchmarking? What are the specific failure modes?
- G-theory assumes facets are crossed and random. LLM judges are deterministic conditional on input — is the (model × item × judge × seed) facet structure defensible, or does the non-i.i.d. nature of LLM judges invalidate the variance decomposition?
- Sequential testing (SPRT) assumes i.i.d. observations. Agent task completions may have serial dependencies (shared context, tool state carryover). Does this invalidate the early-stopping guarantees?
- The pre-registration concept (ICH E9 R1 estimands) was designed for clinical trials with clear populations, treatments, and outcomes. Does the mapping to AI evaluation hold, or is it a forced analogy? Specifically: what are "intercurrent events" in agent evaluation?

### 3. Competitive dynamics

- Is there any funded effort (startup, FFRDC, government lab) building something similar that I might not be aware of? Search broadly.
- What's the risk that a major eval platform (Braintrust, Arize) adds statistical rigor as a feature? How defensible is the methodology moat vs. the distribution moat they already have?
- Could Stanford AIMS or NIST CAISI productize their work and compete directly? What would that look like?
- Is there any risk from the open-source angle — if salib-rs is published, could someone build the product layer on top?

### 4. Business model

- What's the right pricing model for defense? Per-seat? Per-evaluation-run? Platform license? What do comparable tools in the defense evaluation space charge?
- For the hosted offering: what margins are realistic given that the customer pays their own inference costs?
- Is there a land-and-expand path, or is this a big-contract-upfront business?
- What's the minimum viable offering that would close a first defense contract?

### 5. Strategic questions

- Should salib-rs be published as open-source (builds credibility, community, no Rust competitor) or kept proprietary (protects the moat)?
- Is the "opinionated about questions, unopinionated about answers" positioning (provide default IRR but allow BYO) the right call for defense, or do they want fully opinionated systems they can point to as authoritative?
- What's the right company structure for defense work (traditional defense contractor structures, clearance requirements, CMMC, etc.)?
- Timing: is 2026 the right moment, or is the market still 2-3 years from buying this?

### 6. Blind spots

- What am I not seeing? What kills this idea that isn't obvious from inside the builder's perspective?
- Are there regulatory, legal, or compliance barriers I'm not accounting for?
- What's the most likely failure mode — technical, market, or execution?
