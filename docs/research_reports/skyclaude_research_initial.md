# A Critical Review of Longitudinal SPC for AI Agent Evaluation

**Bottom line up front:** The product has a *real* and defensible wedge — Sayash Kapoor and Arvind Narayanan's February 2026 "Towards a Science of AI Agent Reliability" paper, the October 2025 Holistic Agent Leaderboard work, and a year of LLM-IRT papers (PSN-IRT, MEDIRT, Lalor et al.) have created intellectual demand for exactly the measurement-science framing you propose. **But three of your five chosen statistical pillars are imported from settings whose assumptions do not hold for LLM agents, and your defense-first GTM is competing against a *free*, government-backed framework (UK AISI's Inspect) that the U.S. CAISI, METR, and Apollo Research already use.** The product survives if you (a) replace SPRT/G-theory with anytime-valid inference and a mixed-effects reframing, (b) drop the "leaderboard alternative" positioning and reframe as compliance evidence generation for SR 11-7 / EU AI Act high-risk / FDA PCCP regimes, and (c) treat defense as one of three parallel verticals rather than the primary. A pivot is sketched at the end.

---

## TL;DR

- **Methods (deepest critique):** IRT is defensible and has 2024–2026 precedent (Lalor et al. EACL 2024 tutorial; Zhou et al. PSN-IRT, AAAI 2026; MEDIRT, Sept 2025) — keep it, but treat unidimensionality empirically. **Classical SPRT is the wrong primitive** — replace with Howard–Ramdas–McAuliffe–Sekhon confidence sequences / e-processes (anytime-valid inference is "free," per Koolen et al. JRSS B 2025) because your customers *will* peek and your observations are not i.i.d. **G-theory needs a mixed-effects reframing** — judge config is fixed, sampling/seed is random — otherwise the variance-component interpretation is incoherent. **Sobol indices applied to categorical, stochastic-output eval pipelines are not well-defined in the standard form** — use Shapley-effect / functional-ANOVA decompositions instead. **Krippendorff α understates the LLM-judge problem** — it measures agreement, not shared-source-of-error, and the "preference leakage" literature (Li et al. 2025) shows judges from the same model family inflate agreement systematically. **ICH E9(R1) is a forced analogy** — the Kahan et al. 2024 BMJ primer and the arxiv 2406.10366 paper ("Improving Validity of AI/ML Evaluations Using an Estimands Framework") show how to adapt it, but a tighter foundation is the NIST AI RMF + Mitchell-style model cards plus a hashed analysis plan.
- **Competitive scan:** The "rigor and validity" niche is less crowded than the observability niche but it is not empty. The space you are missing is **(a) Inspect (UK AISI, open source, ~200 evals, government-blessed) — a free, well-funded, government-adopted substitute for your orchestration layer**; (b) **METR, Apollo Research, Pattern Labs** — they are the buyers AND the competition for safety-eval contracts; (c) **ValidMind (~$11M, Point72 Ventures), Credo AI, Holistic AI, Monitaur, Fiddler, ModelOp, RelyanceAI** in the bank-MRM/AI-governance lane; (d) **Ketryx in FDA SaMD compliance**; (e) **Braintrust ($80M Series B, $800M valuation, Iconiq) and Galileo ($68M total, 834% revenue growth) are eating the "eval observability" market with positioning you cannot easily out-compete on engineering**. Robust Intelligence sold to Cisco for ~$400M in August 2024 (Sequoia disclosure) — that is the comp for an "AI rigor" exit, not the $10B+ observability multiples.
- **Path forward:** Defense T&E is a viable but slow lane. **Your three highest-probability non-defense paths in order are: (1) Bank/insurer Model Risk Management as the validation evidence layer under SR 11-7 / SS1/23 / OSFI E-23 / EU AI Act Art. 9–15; (2) White-label/OEM into Big-4 AI assurance practices (Deloitte, PwC, KPMG, EY) and into validators like NAMSA in FDA SaMD; (3) Open-source core ("Red Hat for eval rigor") wrapping or extending Inspect, with a paid pre-registration/audit-trail SaaS and a services arm.** Frontier-lab internal eval teams are *not* a viable first ICP — they are NIH, have larger budgets than you, and several (Anthropic, OpenAI, Google DeepMind) already contribute to or fork Inspect.

---

## 1. Method-Level Technical Scrutiny

I will be specific, name papers, and label each finding as **fatal**, **fixable-with-different-method**, or **disclaimer-only**.

### 1.1 Item Response Theory (1PL/2PL/3PL) for agent task completion

**Verdict: Defensible, with caveats — fixable-with-different-method on two specific assumptions.**

There is real, recent literature applying IRT to LLM evaluation. The most relevant references for your defense are:

- **Lalor, Rodriguez, Sedoc, and Hernández-Orallo, "Item Response Theory for Natural Language Processing," EACL 2024 tutorial.** This is the canonical handle — they explicitly establish IRT as a measurement framework for NLP.
- **Zhou et al., "Lost in Benchmarks? Rethinking LLM Benchmarking with Item Response Theory" (PSN-IRT), arXiv 2505.15055, AAAI 2026.** Analyzes 11 LLM benchmarks across 41,871 items, builds a Pseudo-Siamese Network IRT to enrich item parameters, and demonstrates that IRT can shrink benchmarks while preserving rank fidelity to human preference.
- **Luo et al., "Measuring Competency, Not Performance: Item-Aware Evaluation Across Medical Benchmarks" (MEDIRT), arXiv 2509.24186.** Joint modeling of latent competency and item difficulty/discrimination across 71 LLMs on USMLE-aligned content; importantly, it includes **benchmark-integrity validation to test whether items within a topic actually measure a single coherent ability** — i.e., empirically tests unidimensionality, which is the assumption your design most needs to defend.
- **"An Interpretable and Scalable Framework for Evaluating LLMs," arXiv 2605.07046 / Castleman et al. 2025 / Zhuang et al. 2025 / Xu et al. 2025.** Majorization-minimization IRT for stable estimation at LLM scale.
- **OpenReview, "Lifting the benchmark iceberg with item-response theory."** Shows hidden implementation choices in benchmarks bias rankings; IRT proposed as a transparency fix.

**On the four classical IRT assumptions for LLM-agent settings:**

1. **Unidimensionality** is the weakest assumption empirically. Agent tasks load on heterogeneous latent capabilities (planning, tool use, code generation, retrieval, reasoning) — exactly what Kapoor et al.'s Holistic Agent Leaderboard (HAL, NeurIPS 2025 Datasets & Benchmarks) was built to surface. **Fix: don't assume — test.** Adopt MEDIRT's "benchmark integrity validation" step: fit a confirmatory factor model, report McDonald's ω total vs. ω hierarchical, run a parallel analysis. Where unidimensionality fails, fit a **bifactor or multidimensional IRT (M2PL)** model and report capability-specific θ estimates. Don't ship 1PL/2PL/3PL as your default; ship a model-selection gate that picks among (1PL, 2PL, 3PL, M2PL, bifactor) and *reports the choice*.
2. **Local independence given θ** is violated when tasks share scaffolding, judges, prompts, or come from the same generator (a real problem in agent benchmarks where multiple tasks descend from the same template). **Fix:** detect with Yen's Q3 statistic on residuals; if violated, use **testlet response models** (Bradlow, Wainer, Wang 1999) which explicitly model intra-cluster dependence.
3. **Monotonic item characteristic curves** — generally fine for binary completion outcomes but breaks for graded judges where "partial credit" is non-monotonic in θ. **Fix:** use Samejima's graded response model for ordinal judge scores; reserve 2PL/3PL for binary success/fail.
4. **Population of testees that discriminates items** — this is the *real* threat. There are usually 5–50 frontier LLMs in your "population," not the n ≈ 1000 that the IRT measurement-error literature assumes. Standard errors on item parameters will be enormous, especially for 3PL (which is notoriously unstable below n ≈ 500). **Fix:** Use 2PL (or Rasch/1PL where defensible) by default, switch to 3PL only when item-level guessing is mechanically present (e.g., MCQ), and use Bayesian estimation with weakly informative priors (Stan / `mirt::mirt(..., method="EM", SE="MHRM")`). Report posterior credibility intervals on item parameters in the integrity report. **The Castleman/Zhuang/Xu majorization-minimization stack (arXiv 2605.07046) was built precisely to address this small-n stability problem — that should be your reference implementation.**

**Failure modes you should disclose:** (i) When all models in your testee population are above the difficulty range of all items (a saturated benchmark — Zhou et al. 2025 document this on MMLU-style benchmarks), θ becomes unidentified; (ii) when judges and items share generator lineage (preference leakage, Li et al., arXiv 2502.01534), item parameters are biased toward the related family; (iii) when agents have access to retrieval/tools, the "trait" being measured is the *agent system*, not the *base LLM* — be explicit about which θ you're estimating.

### 1.2 Generalizability Theory variance decomposition with deterministic judges

**Verdict: Coherence problem in the classical framing — fixable-with-different-method (reframe as mixed-effects ANOVA / linear mixed model). Not fatal, but you cannot ship the textbook G-theory description.**

You correctly identified the issue. Classical Cronbach-Gleser-Nanda-Rajaratnam G-theory (1972) treats facets as random samples from an exchangeable universe. An LLM judge at (model=Claude-3.7-Sonnet, prompt=v3, temp=0, seed=42) is a single point, not a draw. Standard G-coefficient interpretation — "if I sampled a new rater from the same universe, my expected reliability would be X" — has no meaning when there is no universe to sample from.

**The fix is well-known in the educational-measurement and behavioral-genetics literatures:** treat the judge configuration as a **fixed effect** and the residual stochasticity (LLM sampling at T>0, seed variation, prompt order, sentence order from agent) as the random effect. This is a standard mixed-effects ANOVA / linear mixed model. The variance you decompose is then:

- σ²(agent) — the trait variance you want to detect
- σ²(item) — item difficulty
- σ²(judge_config) — fixed, reported as a between-judge bias rather than a variance component
- σ²(seed | judge_config) — pure measurement noise
- interaction terms (agent×item, agent×judge_config) — measurement bias

**The dependability coefficient (Φ) in G-theory naturally extends to this fixed-judge case** — Brennan (2001), *Generalizability Theory*, §3.4 ("Mixed Designs") handles exactly this. **You should rename your G-theory module "Mixed-Effects Variance Decomposition" and cite Brennan §3.4 and the SEM literature on rater bias (Hoyt 2000, Putka et al. 2008) rather than vanilla G-theory.** This is more defensible AND avoids reviewers who will (correctly) say "your judges aren't exchangeable."

**Is there published G-theory for LLM judges specifically?** I found nothing directly applying G-theory to LLM-as-judge as of May 2026. There *is* a substantial inter-rater-reliability literature for LLMs (Krippendorff α specifically — see §1.5) and there are studies using ICCs on LLM grader configs (Rating Roulette, ACL 2025 EMNLP findings, arXiv 2510.27106) — but no full G-study. **This is a publishable gap** and a content-marketing wedge: a paper titled "Generalizability Theory for LLM-as-Judge: A Mixed-Effects Reframing" would be both academically novel and a credible signal of your team's measurement chops.

### 1.3 Sequential testing: SPRT is the wrong choice

**Verdict: Fixable-with-different-method, and the better method is mature, better-publicized, and better-suited to your customers. Replace SPRT with anytime-valid inference.**

Wald's SPRT (1945) requires i.i.d. observations under both hypotheses. Two failure modes for agent eval:

1. **Within-task non-i.i.d.** Multi-turn tasks, shared context windows, tool-state carryover, in-context learning within a session — observations *inside* a task are dependent. The likelihood ratio statistic Λ_t accumulates correlated evidence, inflating Type I error.
2. **Across-task non-i.i.d.** Customers will run the same agent against your eval over weeks, with prompt changes between runs (the "longitudinal" in your tagline). This is precisely the scenario where SPRT's "peeking" guarantee fails. The classical SPRT also requires fixed simple null vs. simple alternative — a strong constraint your customers will not respect.

**The fix is to use sequential anytime-valid inference (SAVI) primitives:**

- **Howard, Ramdas, McAuliffe, Sekhon, "Time-uniform Chernoff bounds via nonnegative supermartingales" (2020) and the follow-up "Time-uniform, nonparametric, nonasymptotic confidence sequences"** (Annals of Statistics 2021) — confidence sequences that remain valid at every stopping time, including arbitrary peeking.
- **Ramdas, Grünwald, Vovk, Shafer, "Game-Theoretic Statistics and Safe Anytime-Valid Inference," Statistical Science 2023** (the canonical citation, plus the ICML 2025 tutorial by Ramdas).
- **Ramdas & Wang, "Hypothesis Testing with E-Values" (2025 book, CMU)** — your reference text. Chapter 7 specifically covers sequential testing with e-processes.
- **Koolen et al., "Anytime validity is free: inducing sequential tests," JRSS B 2025** — shows you can convert your existing fixed-sample test plans to anytime-valid ones with essentially no power loss.
- For dependence, the **nonnegative supermartingale construction** handles arbitrary dependence within the filtration. This is *strictly* what you need.

**Concrete recommendation:**
- **Replace SPRT as the default** with e-process / confidence-sequence based stopping rules. Wald's two-sided power-one SPRT under approximate boundaries actually overshoots and gives sub-α Type I (arXiv 2410.16076, 2024), so even practitioners using it are accepting an unknown, conservative bias.
- **Use e-values for the multi-stream / multi-metric case** — your dashboard probably wants to monitor several agent×task×judge slices simultaneously. E-values compose via averaging without needing union bounds (Vovk-Wang merging). Bartroff–Lai–Shih (PMC 4118217) provide the multi-stream SPRT alternative with FWER control even under arbitrary dependence; this is your fallback for the binary case.
- **Cite Aaditya Ramdas' work prominently** in your eval-integrity report template — this is the dominant academic frame in 2026 and it is what NeurIPS/ICML reviewers will expect.

The cost of this swap: about two weeks of engineering and one statistician-week of writing. It is unambiguously a strengthening, not a sidegrade.

### 1.4 Global Sensitivity Analysis on categorical-input, stochastic-output systems

**Verdict: Not fatal but the standard Sobol decomposition is *not* well-defined as you have implemented it. Move to Shapley effects or functional ANOVA on categorical factors.**

Variance-based GSA (Sobol 1993; Saltelli et al. 2008) is built on the Hoeffding–ANOVA decomposition of a square-integrable deterministic function `f(X₁,…,X_p)` where the X_i are independent random variables (typically continuous, with probability distributions encoding input uncertainty). Two issues for your eval-pipeline application:

1. **Categorical inputs.** When `model` has 5 levels, `judge` has 3, `prompt_variant` has 4, etc., the "variance of conditional expectation" V[E[Y|X_i]] is well-defined *only after you specify a distribution over the levels*. Treating each level as equiprobable is an arbitrary choice that determines the result; users will not have intuition for this. The standard literature (e.g., Saltelli's `SALib`) handles this awkwardly via group-wise indices, and the interpretation degenerates when levels are not exchangeable (which they aren't — Claude-3.7-Sonnet ≠ Llama-3.3-70B in any sense that justifies treating them as draws from a common distribution).
2. **Stochastic output.** Sobol indices for stochastic models exist (Castellan, Cousien, Tran 2018, arXiv 1611.07230; Hart, Alexanderian, Gremaud 2017 on Sobol' for stochastic models) but require either (a) metamodels of the conditional mean/variance or (b) repeated evaluation with sufficient replication. Your design's S1/ST estimators are most likely *biased low* (residual stochastic variance gets absorbed into "interaction effects" or "noise") unless you explicitly separate the within-config variance from the between-config variance.

**Better-suited methods, in order of preference for your case:**

- **Shapley effects (Owen 2014; Song, Nelson, Staum 2016).** Specifically built for dependent or categorical inputs and unify first-order + total Sobol indices into a coherent attribution that sums to total variance even under dependence. Strong literature, computationally feasible at your scale, and the "Shapley value" framing connects well to ML audiences already familiar with SHAP.
- **Mixed-effects variance components (the same model you should use for §1.2 G-theory).** A linear mixed model `Y ~ (1|model) + (1|item) + (1|judge_config) + interactions + residual` recovers exactly the variance partition you want, handles categorical factors natively, and the random-effects estimates *are* the variance contributions. This is more honest than dressing it up in GSA terminology.
- **Functional ANOVA decomposition (Hooker 2007; Roosen & Hennig's PAWN, 2017)** if your customers want a distribution-free / moment-free decomposition.
- **PAWN (density-based) and Borgonovo δ (moment-independent)** are appropriate when you cannot assume variance is the right summary — useful for tail-risk in agent eval.

**Concrete recommendation:**
- Keep `salib-rs` as one backend but **rename the module "Influence Attribution"** and offer (i) Shapley effects as the default for categorical factor designs, (ii) Sobol/Saltelli for continuous-hyperparameter scans (temperature, top-p, retrieval-k), (iii) mixed-effects variance components as the "principled" report. Be honest that "Sobol on `model={Claude, GPT, Llama}`" requires the user to specify a prior over those models for the result to mean anything.
- Cite **Iooss, Da Veiga, Saltelli, Lemaitre, "Shapley effects for sensitivity analysis with correlated inputs" (2021)** as your primary methodological reference.

### 1.5 Krippendorff α for LLM-judge agreement

**Verdict: α measures the wrong thing. Disclaimer-only if you keep it; fixable-with-different-method if you want correctness.**

Krippendorff α is designed to measure inter-coder agreement *beyond chance*, where the chance baseline is constructed from the marginal distribution of category use. It implicitly assumes **independent error sources** — which is the entire point of using multiple human coders. For LLM judges:

- **Shared-source-of-error is the dominant failure mode, not random disagreement.** Position bias, verbosity bias, self-preference, and family bias are *correlated across judges*. "EmergentMind"'s analysis of 150,000+ MTBench/DevBench evaluations with 15 LLM judges found >80% of cases yield agreement from ≥2/3 of judges, full unanimity in ~23%, and **models sharing architecture and training lineage (e.g., GPT-4/Turbo, Claude-3 groupings) exhibit higher internal agreement — indicating shared patterns of systematic bias, not better judgment.** High α here is a *bad* signal, not a good one.
- **Preference leakage (Li et al., arXiv 2502.01534, Feb 2025)** is the smoking gun: when a judge LLM shares lineage with the student model it is evaluating, agreement with "ground truth" is inflated systematically.
- **Rating Roulette (Findings of EMNLP 2025, arXiv 2510.27106)** documents that LLM raters often have α far below 0.8 against themselves under repeated sampling — within-judge α can be lower than between-judge α, which is incoherent in classical IRR theory.

**What to do instead:**
- **Report α, but also report it stratified by judge family** (OpenAI-family, Anthropic-family, Google-family, open-weights) — within-family α minus between-family α is a *bias-burden indicator*. If they differ by >0.1, you have a preference-leakage problem.
- **Add a "human anchor" requirement.** Krippendorff α between judges×humans is the metric that survives shared error; without human ground truth on a calibration set, you cannot distinguish "judges agree because they're right" from "judges agree because they share biases." This is the standard recommendation from the Bavaresco et al. 2024 meta-evaluation work.
- **Use a Bayesian latent-class agreement model** (e.g., Dawid-Skene 1979, extended by Paun et al. 2018 for NLP annotation) to *jointly* estimate latent truth, judge accuracy, and judge confusion patterns. The "shared confusion matrix structure across judges" is exactly the parameter you want to recover and α cannot give you.
- **For LLM-as-judge specifically: cite the SAGE framework (arXiv 2512.16041, December 2025)** which measures local self-consistency and global transitivity of LLM judges — even Gemini-2.5-Pro and GPT-5 fail consistency in nearly a quarter of difficult cases.

This is **the easiest section to strengthen as a content-marketing piece.** "Why Krippendorff α Lies About LLM Judges" is a 1500-word blog post that would land with the exact audience that would buy your product.

### 1.6 ICH E9(R1) estimand framework for pre-registration

**Verdict: Forced analogy. Disclaimer-only if you keep it, but a better foundation exists. The right move is to keep estimand vocabulary as one layer of a multi-layer pre-registration stack.**

ICH E9(R1) (final 2019, primer: Kahan et al. *BMJ* 2024, PMC10802140) defines five estimand attributes — **treatment, population, endpoint, intercurrent events, population-level summary** — to clarify what treatment effect is being estimated. The framework's main innovation is the *intercurrent event* concept: events occurring after randomization that affect either the existence or the interpretation of the measurement (e.g., treatment discontinuation, rescue medication, death).

There *is* a published attempt to apply this to ML/AI: **"Improving the Validity and Practical Usefulness of AI/ML Evaluations Using an Estimands Framework" (arXiv 2406.10366, 2024).** It exists, it is citable, and it explicitly aims at your use case. Use it.

**But the mapping is forced:**
- "Population" maps poorly. Are you generalizing to a population of users, of tasks, of inputs, of model checkpoints? Each has different statistical implications.
- "Intercurrent events" — the natural analogs (tool failure, agent retry, mid-eval model API deprecation, judge model version bump, dataset contamination disclosed mid-run) are real but the strategy taxonomy (treatment policy, hypothetical, composite, while-on-treatment, principal stratum) maps awkwardly. "Hypothetical" strategies for AI eval mean very different things in different contexts.
- Clinical-trial reviewers will read "estimand" and expect drug-development semantics; ML reviewers will not know what it means.

**The better stack is a layered pre-registration framework:**
1. **NIST AI RMF 1.0 (January 2023) and AI RMF Generative AI Profile (NIST AI 600-1, July 2024)** as the governance scaffold — measurable, regulator-recognized, and explicitly designed for AI.
2. **Mitchell et al. "Model Cards for Model Reporting" (2019) and Gebru et al. "Datasheets for Datasets" (2021)** for artifact description.
3. **The Reproducibility Checklist (Pineau et al., NeurIPS 2019)** and the ML Reproducibility Challenge norms for procedural reproducibility.
4. **The Agentic Benchmark Checklist (ABC) from Kapoor/Stroebl/Narayanan, NeurIPS 2025 D&B** specifically for agent-eval pre-registration.
5. **The estimand framework as one *option* within the analysis-plan layer** for users who want clinical-trial-grade specification — useful for FDA SaMD or insurance-actuarial customers, weird-looking for everyone else.
6. **Hash-anchored plans as the cryptographic substrate** — this part of your design is unambiguously good; tie it to W3C verifiable credentials or sigstore for portability.

**This is also a positioning win:** "ICH E9(R1) for AI" sounds like cargo-cult medicalization to a frontier-lab evals engineer. "NIST AI RMF + agentic-benchmark checklist + cryptographically anchored analysis plan" sounds like a defensible governance tool that maps to actual compliance regimes. Same primitives, much more sellable.

### 1.7 Method-level critique summary table

| Method | Verdict | Most important fix |
|---|---|---|
| IRT (1PL/2PL/3PL) | Fixable | Test unidimensionality empirically; default to 2PL with Bayesian estimation; offer bifactor/M2PL |
| G-theory | Fixable | Reframe as mixed-effects with fixed judge effects; cite Brennan §3.4 |
| SPRT / α-spending | Replace | Use confidence sequences / e-processes (Howard-Ramdas; Ramdas-Wang 2025) |
| Sobol indices | Fixable | Default to Shapley effects for categorical inputs; rename module "Influence Attribution" |
| Krippendorff α | Disclaimer + augment | Stratify by judge family; require human anchor; report Dawid-Skene latent-class model |
| ICH E9(R1) estimands | Reposition | Make it one layer in a NIST AI RMF + ABC + Mitchell-card stack |

**Net judgment on the math core: defensible and differentiated, but mis-marketed and over-classical.** You are importing 1970s/1980s biostatistical and educational-measurement frameworks when 2020–2025 mathematical statistics has produced strictly better tools (anytime-valid inference, Shapley effects, latent-class agreement, mixed-effects judge models). **A six-month methods refactor to land on Ramdas/Howard for sequential, Shapley/Iooss for attribution, Dawid-Skene/Paun for judges, NIST RMF for governance, and IRT + mixed-effects ANOVA as the latent-variable backbone would be a much stronger, much more publishable, and much more 2026-coherent foundation.**

---

## 2. Competitive Scan (Commercial Evaluation Space)

I'll cover three sub-landscapes: (A) eval observability platforms (your closest competitors), (B) the AI risk/MRM/governance lane, and (C) the AI-safety eval contracting world.

### 2.1 LLM/Agent eval observability — the "usual suspects" plus what they don't do

| Company | Funding (latest) | What they do | What they *don't* do that your product would |
|---|---|---|---|
| **Braintrust** | $80M Series B (Iconiq, with a16z, Greylock, basecase, Elad Gil), ~$800M valuation. Customers: Notion, Replit, Cloudflare, Ramp, Dropbox, Stripe, Vercel | CI/CD-native eval gating, prompt management, trace-to-test pipeline, "Loop" auto-optimization | No psychometrics, no formal sequential testing, no inter-rater quantification beyond percent-agreement; no global sensitivity analysis; no audit-trail/pre-registration; no formal measurement-validity reporting |
| **Galileo** | $45M Series B Oct 2024 (Scale VP led; Databricks, ServiceNow, Amex, Citi, SentinelOne Ventures), $68M total; 834% revenue growth 2024; six Fortune 50 customers incl. Comcast, Twilio, HP, Cisco | "Evaluation Intelligence Platform"; Luna proprietary eval foundation models; hallucination detection, agentic evaluations (Jan 2025) | Their evaluator models are proprietary judges — they have the *judge-bias* problem you would diagnose. No measurement-theory layer; no formal sequential design; no estimand/pre-registration |
| **Arize AI** (AX + Phoenix) | Series C $70M led by Adams Street, 2024 (~$200M raised); broad ML+LLM observability | OpenTelemetry-native tracing, drift detection, agent path/convergence/session evals, open-source Phoenix | Observability-first; psychometric depth is shallow; no IRT, no G-theory; no statistically formal early-stopping |
| **LangSmith (LangChain)** | LangChain raised $25M Series A from Sequoia (2024); reported Series B at ~$1B valuation, 2025 | Tightly LangChain-integrated tracing/eval; March 2026 added Sandboxes + NVIDIA partnership for agent deployment | Tied to one framework; eval methodology is bring-your-own; no rigor-tier features |
| **Langfuse** | Acquired by ClickHouse Jan 2026; previously YC + ~$8M | Open-source observability and eval; broad self-hosting | Now Clickhouse-owned with uncertain roadmap; eval methods are basic; no measurement science |
| **Patronus AI** | $40.1M total ($17M Series A Notable Capital + Datadog, May 2024; Dec 2025: Generative Simulators launch + "ORSI" RL training) | Hallucination/PII/copyright detection; Percival agent eval copilot; recently pivoted to RL training environments | Sells *judge models* as a product — they ARE a source of the bias you'd diagnose; no measurement-validity layer |
| **Humanloop, Vellum, Confident AI, Helicone, Latitude** | Mostly seed/Series A, ranging $5–30M | Prompt management, eval-driven dev, observability | Product-development-loop tools, not validity tools |
| **TruEra** | Acquired by Snowflake (May 2024) | LLM observability + traditional ML monitoring | Now bundled with Snowflake's "AI Observability" |
| **Robust Intelligence** | **Acquired by Cisco August 2024 for ~$400M (Sequoia disclosed figure)** | AI firewall, model validation, vulnerability testing | Pivoted to runtime security; their pre-acquisition validation pitch is closer to your space than anything else on this list — the comp you should be benchmarking against |
| **W&B Weave** | W&B was acquired by CoreWeave for $1.7B (2024) | ML experiment tracking → LLM evals as feature | Eval is a feature, not the product |
| **Fiddler AI** | ~$60M Series B, 2022, then 2024 GenAI pivot | Explainability + LLM observability | Broad and shallow on LLM rigor |

**Players outside the "usual suspects" list you should know:**

- **Latitude (latitude.so)** — agent-session-as-unit-of-analysis observability with issue lifecycle tracking and GEPA auto-generated evals. Closer to your "longitudinal" frame than most.
- **Vijil** — Series A late 2025; a Patronus competitor with focus on agent reliability (mentioned by Tracxn).
- **Credo AI** — AI governance platform, raised ~$40M Series B (2023, Sands Capital), MRM-adjacent.
- **Holistic AI** — UK-based AI governance/audit platform, ~$8M seed.
- **Monitaur** — MRM tooling for insurance/finance, ~$5M seed, narrower than ValidMind but in your lane.
- **ModelOp** — long-standing MRM, ~$30M raised, oriented at banks.
- **Sphera, RiskSpan, Crowe MRM Advisory** — incumbent risk-management software/consulting; legacy MRM tooling that doesn't yet handle LLMs well.

### 2.2 Bank/Finance Model Risk Management — the actual non-defense beachhead

The regulatory hooks are: **U.S. SR 11-7 (Federal Reserve 2011), OCC 2011-12, UK PRA SS1/23, OSFI E-23 (Canada), and now EU AI Act (high-risk financial services use cases under Articles 9, 15, 17, and Annex III).** These regimes *require* documented, independent model validation. LLMs and agentic systems are explicitly in scope.

| Company | Funding | What they do | Gap vs. your product |
|---|---|---|---|
| **ValidMind** | $11.1M total ($8.1M seed led by Point72 Ventures March 2024, Third Prime, NY Life Ventures, AI Fund, FJ Labs); customers via "ValidMind Advantage" partner program; positioned as "the certifying authority" for AI in financial services | Automated model documentation, validation, governance; SR 11-7 / EU AI Act / SS1/23 / OSFI E-23 audit-ready evidence; recently extended to agentic AI governance | **Documentation and workflow tooling, not measurement science.** Their advantage is bank-procurement friendliness and SR 11-7 templates; their gap is exactly your "is this measurement valid?" layer. **You should consider them a partnership target, not a competitor — their templates need your statistical guts.** |
| **Credo AI** | ~$40M total | AI risk + compliance platform, EU AI Act mapping | Policy/governance front-end; thin on quantitative validation |
| **Holistic AI** | ~$8M seed | UK AI governance and audit | Similar profile to Credo, smaller |
| **Monitaur** | ~$5M | Insurance/banking MRM ML focus | Narrower scope, MRM workflow |
| **ModelOp** | ~$30M | Enterprise MRM, ServiceNow ecosystem | Legacy MRM, slow on LLM-specific rigor |
| **Solas AI, Stradigi, RelyanceAI** | Various seed | Audit/compliance with AI focus | Compliance-first, not validity-first |
| **Big-4 AI assurance practices** (Deloitte, PwC, KPMG, EY) | Internal, multi-billion-dollar practices | Bespoke AI audit and validation services; PwC has a "Responsible AI Toolkit"; Deloitte has "Trustworthy AI"; KPMG "Trusted AI"; EY "Trusted AI Platform" | **They sell hours, not tools. They are the ideal white-label / OEM customer for your product.** Their validators are PowerPoint-and-Excel today. |
| **Bloomberg, S&P Global, Moody's** | Public/large | Quant model validation tooling; some moving into AI | Adjacent — would partner, not compete |

**Critical observation:** ValidMind raised only $11.1M and is treated as the de facto leader in this lane. **The market is small but not crowded.** A statistically-rigorous, math-deep entrant could be a sharp wedge into either the ValidMind partnership channel or directly into the Tier-1 bank MRM teams. The procurement path: model risk management is a Second Line of Defense function reporting to Chief Risk Officer; they have annual budgets in the $50–200M range at G-SIBs; they buy tools that produce *audit-defensible* evidence. **Your "eval integrity report" with hash-anchored analysis plans, formal Type I/II error control, and measurement-validity diagnostics is exactly the artifact they want.** The pitch writes itself: "When the Fed examiner asks you to prove your agentic AI assistant is reliable, what do you hand them?"

### 2.3 AI safety eval / alignment evaluation lane

| Org | Status | What they do | Buyer or competitor? |
|---|---|---|---|
| **METR (formerly ARC Evals)** | 501(c)(3); ~$5M funding; <50 staff; contracts with Anthropic, OpenAI | Frontier-model autonomy/agentic evals; HCAST, RE-Bench, Task Standard; time-horizon paper | **Both buyer and competitor.** Their tooling is internal; they could use yours; they would not pay much. They have brand and government access. |
| **Apollo Research** | Spinning out to PBC; ~15 FTE; 7 philanthropic funders + 2 commercial contracts | Deception/scheming evals; "Watcher" oversight product; partner of UK AISI and US AISIC | Same as METR — a possible partner more than a customer |
| **Pattern Labs, Redwood Research, Far AI, MATS-affiliated groups** | Various nonprofits / labs | Niche evals (control, interpretability, deception) | Small total addressable market |
| **UK AISI (AI Security Institute)** | UK government | **Inspect framework (open-source, ~200 evals, MIT-licensed); Inspect Sandboxing Toolkit; collaborates with US CAISI, Apollo, METR, Vector Institute, Arcadia Impact** | **This is the biggest blind spot in your competitive thesis.** Inspect is free, well-documented, government-blessed, has 50+ contributors, and is the *default* tooling for safety-eval contracting work. Your orchestration layer competes with it directly. **You must decide whether to extend Inspect or to displace it.** Extending is strictly easier. |
| **US CAISI (formerly US AISI)** | US government (under NIST) | Standards, evaluations, consortium | Procurement-friendly; participates in same federal pipeline as your defense thesis |
| **EU AI Office** | EU government | High-risk AI registration and standards | Regulatory rule-maker; potential standards adopter |
| **Stanford CRFM/HAI/AIMS, MILA, Berkeley CHAI, CMU, Princeton (Kapoor/Narayanan), Oxford GovAI, MIT FutureTech** | Academic | Frame the discourse | Source of intellectual demand for your product; not customers |
| **OpenAI Evals, Anthropic internal evals, DeepMind internal evals, Meta evals teams** | Frontier labs | Internal tooling, often open-sourced (OpenAI Evals, EleutherAI lm-eval-harness, BIG-bench) | **NIH — do not sell to these as a first ICP.** They have larger teams than you, larger budgets, and often contribute to Inspect. |
| **Hugging Face `evaluate` library, EleutherAI `lm-evaluation-harness`, Google BIG-bench, Stanford HELM, MedHELM, AudioHELM** | Open-source | Eval implementations and leaderboards | Substrates, not competitors |

### 2.4 Healthcare AI / FDA SaMD

- **FDA approved 1,000+ AI/ML-enabled devices by March 2025**, 97% via 510(k); January 2025 draft guidance on lifecycle management and PCCPs; IMDRF GMLP final January 2025.
- **Ketryx** (mentioned in search results) — compliance-engineering tooling that automates traceability for PCCPs, integrates with Jira/GitHub.
- **NAMSA, ICON plc, IQVIA** — clinical research / regulatory consultancies expanding AI SaMD practices.
- **GE HealthCare, Philips, Siemens Healthineers, Aidoc, Tempus** — large vertical players with internal validation teams; they buy services, not tools, today.
- **RSNA, ACR DSI** — radiology professional bodies running their own validation programs (ACR-AI-LAB).
- **No dominant FDA-validation-tooling startup exists** — this is a real gap, partly because the regulatory regime is still settling (PCCP final guidance came late 2024).

### 2.5 Who is conspicuously *missing* from your original list?

In rough order of importance:
1. **Inspect (UK AISI) — this is the single most important omission.** Free, government-blessed, broadly adopted across the safety eval community.
2. **Patronus AI's Percival, Galileo's Luna, Braintrust Loop** — three commercial "agentic eval" products launched in 2024–2025 that compete with the agent-focused parts of your pitch.
3. **Ketryx** for FDA SaMD compliance tooling.
4. **ValidMind, Monitaur, ModelOp** in MRM.
5. **Credo AI, Holistic AI** in AI governance.
6. **The "AI red team" lane**: HiddenLayer, Lakera, Prompt Security, Lasso Security, CalypsoAI — adjacent (security, not validity) but they will show up in the same RFPs.
7. **The procurement-tooling lane**: Calypso, Anch.AI, Trustible — pure governance/workflow.
8. **Big-4 AI assurance practices** as your most plausible white-label channel partners.

---

## 3. Plausible Non-Defense Paths Forward

I'll be opinionated. Below are seven paths, ranked by my estimate of viability for your specific stack and posture.

### Path 1 — Bank/Insurer Model Risk Management evidence layer (BEST FIT)

**Viability: High. ICP definition is clean. Sales cycle is 6–12 months but the buyer has discretion.**

- **ICP:** Tier-1 and Tier-2 banks' Second Line of Defense (Model Risk Management groups); top-25 U.S. insurers; top-10 EU banks; large asset managers; specialty finance lenders deploying LLM-based underwriting, KYC, fraud, customer-service agents.
- **Buyer titles:** Head of Model Risk Management, Chief Model Risk Officer, Head of AI Risk, Director of Model Validation.
- **Trigger event:** New SR 11-7-aligned policy refresh for "AI/ML models" (most G-SIBs have a 2024–2026 refresh underway); EU AI Act high-risk system registration; OCC examiner ask; internal audit finding; a public LLM mishap (Air Canada, Cruise, Gemini).
- **Product changes needed:**
  - **Drop "longitudinal SPC" branding** for this segment — it doesn't map to anything in their world. **Lead with "Model Validation Evidence for Agentic AI."**
  - Add SR 11-7 / EU AI Act / SS1/23 / OSFI E-23 mappings of every metric your system produces ("this Krippendorff α stratified by judge family satisfies SR 11-7's 'conceptual soundness' requirement for the judge component").
  - Output **PDF reports with regulator-friendly language**, not interactive dashboards.
  - SOC 2 Type II is table stakes; FedRAMP not required (commercial banks don't care).
- **Composition with defense:** Strong. The exact same evidence artifacts (hash-anchored pre-registration, sequential test reports, IRT item-validity reports) sell into both DoD T&E and Fed examination. You can position as "the audit-grade AI evaluation infrastructure" with vertical packaging.
- **Competitive friction:** ValidMind is the main competitor; they are workflow-shaped, you would be math-shaped. **Best move may be a ValidMind partnership** (their workflow → your statistical engine) rather than a head-on go-to-market.
- **Comp expected:** Robust Intelligence at $400M to Cisco is your North Star; ValidMind on its current funding trajectory is the closer comp.

### Path 2 — White-label / OEM into Big-4 AI assurance practices (HIGH LEVERAGE)

**Viability: High if you can endure ~9-month sales cycle into a partner and accept partner economics.**

- Deloitte's Trustworthy AI, PwC's Responsible AI Toolkit, KPMG Trusted AI, EY Trusted AI Platform are each ~$50–500M/yr practices that **sell hours and PowerPoint, not tools**. Their validators today are using Excel + Python notebooks + manual sampling. A turnkey tool that produces audit-grade artifacts they can put their logo on is something they have asked for and not received.
- **Product changes needed:** White-labeled UI; multi-tenant data isolation with SOC 2 + ISO 27001; report templates that accept partner branding; pricing model that gives partner 30–60% margin.
- **Composition with defense:** Big-4 do defense work too (Deloitte Federal, KPMG Federal); your defense-cleared engineers are an asset for FedCiv partner sales.

### Path 3 — Open-source core + paid SaaS / services (DURABLE)

**Viability: Medium-high. Slow to monetize but creates the credibility flywheel.**

- **Open-source the math core** (`salib-rs`, `irr`, `seq-test`, `reliability`, `prereg`) under Apache-2.0. **Either integrate with Inspect as a "rigor extension"** (so customers don't have to choose) **or position as Inspect's missing statistical layer.** The 50+ Inspect contributors are your distribution.
- **Monetize via** (i) hosted SaaS for the audit-trail / pre-registration layer, (ii) enterprise support contracts, (iii) compliance-package add-ons (SR 11-7 templates, EU AI Act mapping, FDA PCCP templates).
- **The "Red Hat for AI eval" pitch is real but slow.** Plan a 3-year arc; Year-1 is GitHub stars, conference talks (NeurIPS Datasets & Benchmarks, ICML, AISTATS), and one foundational publication (the G-theory-for-LLM-judges paper recommended above).
- **Composition with defense:** Strong on credibility, neutral on revenue.

### Path 4 — Healthcare AI / FDA SaMD (HIGH MARGIN, SLOW)

**Viability: Medium. Big opportunity, distant timing.**

- FDA's January 2025 draft guidance + PCCP final guidance creates a documented validation workload for every AI-enabled device. Ketryx is the main compliance-tooling startup; there is room for a *statistics-first* complement.
- **Product changes needed:** ISO 13485 quality management, 21 CFR Part 11 compliant audit trails, IEC 62304 software lifecycle alignment. This is a 12-month compliance investment before first dollar.
- **Buyer titles:** Head of Regulatory Affairs, VP Quality, Head of Clinical Affairs at digital-health and medical-device companies.
- **Best entry route:** Partner with NAMSA or ICON's digital-health regulatory practice as a tool vendor rather than direct.
- **Composition with defense:** Weak — different sales motion, different compliance stack. Probably a separate vertical to spin up at Series B, not Series A.

### Path 5 — AI safety / alignment evaluation contracts (LOW VOLUME, HIGH SIGNAL)

**Viability: Low for revenue, high for brand. Treat as marketing, not GTM.**

- METR, Apollo, Pattern Labs, US CAISI, UK AISI, EU AI Office — these orgs have small budgets, mostly philanthropic or government. **They are not where your revenue comes from.** They are where your *credibility* comes from.
- **Move:** Open-source the math core (Path 3), get adopted in Inspect, present at the AI safety evals community workshops (NeurIPS, ICLR safety workshops, the AISI evals workshops), publish the G-theory-for-LLM-judges paper. **Within 18 months you can be the "the rigor people" in a community of ~500 evals practitioners worldwide.**
- **Composition with defense:** Strong on technical credibility; weak on direct revenue.

### Path 6 — Standards/certification body ("TÜV / UL of AI eval") (FUTURE)

**Viability: Aspirational. Pursue as a 3–5 year option, not a Year-1 strategy.**

- TÜV SÜD, UL, BSI, Bureau Veritas are all positioning for AI conformity assessment under the EU AI Act. **These are the natural future customers/partners for a "produce the underlying evidence" tool.** None has built a credible technical evidence layer yet.
- **Move:** Engage CEN-CENELEC JTC21 working groups, ISO/IEC JTC1/SC42 standards work, NIST AI Safety Institute Consortium. Become a technical reviewer / contributor before pitching as a vendor.
- **Composition with defense:** Strong. The same audit-grade artifacts serve both.

### Path 7 — Frontier-lab internal eval teams (DO NOT PURSUE FIRST)

**Viability: Low.**

- Anthropic, OpenAI, Google DeepMind, Meta evals teams are 20–200 people each, well-funded, NIH, and several already contribute to Inspect. They will not pay an outside vendor for a Rust-based math core they could build in three weeks of FTE time, and they will be skeptical of any vendor pushing "psychometrics" without an internal champion who has read Lalor et al.
- **Possible exception:** Anthropic's "Frontier Red Team" or Google's "Frontier Safety Evaluations" might engage on a consulting basis for a specific capability (e.g., a deception eval with rigorous power analysis). But they will not buy a platform.
- **Composition with defense:** Negative — frontier labs will treat your DoD work as a reason for caution.

### A possible pivot: "Audit-Grade AI Evaluation Infrastructure"

If you take this critique seriously, here is the sharper product / GTM thesis I would build:

> **"Audit-Grade AI Evaluation Infrastructure."** Open-source statistical core (anytime-valid inference, IRT, Shapley effects, latent-class judge models, hash-anchored pre-registration) with a paid SaaS for compliance evidence: SR 11-7, EU AI Act high-risk, FDA PCCP, NIST AI RMF, DoD T&E. Sold direct to MRM teams at G-SIBs and white-labeled into Big-4 assurance practices. Defense T&E is one vertical of three, not the primary.

This pivot:
- **Keeps your technical bet** (rigorous measurement science is the moat).
- **Replaces 1970s methods with 2020s methods** (Ramdas, Shapley, Dawid-Skene, NIST AI RMF), making you publishable AND defensible to a frontier-lab reviewer.
- **Targets a regulatory-driven buyer** (MRM, FDA RA, EU AI Act Notified Bodies) where willingness-to-pay is higher and the buyer maps to your artifact, not your dashboard.
- **Composes with defense** rather than depending on it. Defense has 3–5 year procurement cycles; a $50K annual MRM contract closes in 90 days.
- **Has a believable exit comp** (Robust Intelligence at ~$400M to Cisco) and a believable scale comp (ValidMind growing into the certifying-authority role).

---

## Caveats

- **Funding figures and valuations** are reported as of search results retrieved May 2026; private-company valuations and revenue figures (especially Galileo's "834% revenue growth") are company-disclosed and self-serving — discount accordingly. Robust Intelligence's $400M acquisition price is Sequoia-disclosed (sequoiacap.com), not officially announced by Cisco. ValidMind's $11.1M total is self-disclosed.
- **The "no published G-theory for LLM judges" claim** is based on my literature search; absence of evidence is not evidence of absence. I would do one final targeted search before publishing a paper.
- **The Inspect adoption claim** (METR, Apollo, US CAISI use it) comes from AISI's blog posts and substack reviews; I have not independently verified contractual relationships.
- **Frontier-lab NIH-ness** is based on observed behavior (internal evals teams shipping their own tools — OpenAI Evals, Google's evals frameworks, Meta evals) and is a generalization, not a universal law. There are individual researchers inside these labs (e.g., people who have published on LLM-IRT) who *would* be intellectually receptive — that is a recruiting and content-marketing channel, not a sales channel.
- **The "ICH E9(R1) is forced" judgment** is a positioning critique, not a technical one — the arXiv 2406.10366 paper shows it *can* be made to work; my view is that it is not the most sellable framing.
- **Where I have genuinely low confidence:** the size of the FDA SaMD AI-validation-tooling opportunity (could be $100M ARR market or could be $5M); the actual margin economics of Big-4 partnerships; the long-term trajectory of Inspect if UK AISI's funding shifts.
- **2026 statistical methods stack recommendations** (Ramdas, Shapley effects, Dawid-Skene, NIST AI RMF) reflect my read of the active literature; reasonable methodologists could prefer different choices (e.g., always-valid p-values via Johari et al.'s mSPRT, used at Optimizely/Microsoft, instead of confidence sequences). The directional claim — *use 2020s sequential methods, not 1945 SPRT* — is robust; the specific replacement is a design decision.

The critique is the gift. The math core can be strengthened in six months, the GTM can be pivoted in three, and the result is a company that competes on rigor where rigor is regulated — which is a much more defensible moat than competing on rigor where engineers self-select rigor for taste.