# Scout: Web -- mojave
Date: 2026-05-30

## Key Findings

### 1. NIST AI 800-3 formally distinguishes benchmark accuracy from generalized accuracy (Feb 2026)

NIST published "Expanding the AI Evaluation Toolbox with Statistical Models" (NIST AI 800-3, Feb 17 2026) which directly validates mojave's thesis. The report demonstrates that common benchmark analysis conflates "benchmark accuracy" (performance on the fixed test set) with "generalized accuracy" (performance on all potential items similar to those in the benchmark), and proposes generalized linear mixed models (GLMMs) as a foundation for principled AI evaluation statistics. They evaluate 22 frontier LLMs on 3 popular benchmarks. This is the first federal publication explicitly calling for measurement-science rigor in AI eval. mojave should cite this as institutional validation and potentially align run-card reporting with NIST's GLMM approach.

### 2. "Towards a Science of AI Agent Reliability" proposes twelve reliability metrics (Feb 2026)

Rabanser, Kapoor, and Narayanan (Princeton/CMU) published a 66-page paper decomposing agent reliability into four dimensions -- consistency, robustness, predictability, and safety -- with twelve concrete metrics. Their key finding: recent capability gains yield only small reliability improvements. They tested 14 models across three providers. This paper is the closest published analog to mojave's "measurement science for AI agents" framing. The four-dimension taxonomy could either complement or compete with mojave's GSA-based approach. Question: does mojave's Sobol-index decomposition of eval variance map cleanly onto their consistency/robustness dimensions?

### 3. Wang (Meta FAIR) decomposes LLM eval noise into prediction, data, and total components (Dec 2025)

"Measuring all the noises of LLM Evals" by Sida Wang provides a rigorous noise decomposition framework using the law of total variance. Key finding: prediction noise (variance from repeated model runs on the same question) typically exceeds data noise (variance from question sampling), often by 2x on benchmarks like MATH500. Proposes an "all-pairs paired method" for statistical power. This directly informs mojave's Saltelli sampling design: if prediction noise dominates, the sampling strategy should account for repeated model queries per item, not just item sampling.

### 4. Mazo redefines Sobol indices without the Sobol decomposition (Sep 2024, revised Mar 2026)

Gildas Mazo's "A new paradigm for global sensitivity analysis" redefines sensitivity measures as set functions null at a subset of inputs iff the output is probability-one independent of those inputs. Sobol indices become a special case. This paper could fundamentally affect salib-rs's theoretical grounding -- if Mazo's generalized framework proves more natural for LLM eval factor analysis, mojave may want to implement the generalized measures alongside classical Sobol indices. The framework also defines interaction effects independently of the sensitivity measure choice, which matters for multi-factor LLM eval designs.

### 5. Borgonovo's optimal-transport GSA now has an R package (gsaot, Jul 2025)

Borgonovo, Cipolla et al. released gsaot, an R package implementing optimal-transport-based global sensitivity indices. The OT approach handles multivariate outputs without the decomposition assumptions Sobol requires. Published in Management Science (2024) with the software paper in Jul 2025. No Rust implementation exists. If mojave encounters multivariate eval outputs (e.g., jointly analyzing accuracy, latency, and cost sensitivity), OT-based indices may be more appropriate than Sobol. This is a potential salib-rs extension point.

### 6. Constant-size cryptographic evidence structures for regulated AI (Nov 2025)

Leo Kao's paper introduces fixed-size cryptographic evidence tuples that compose with hash chains and Merkle trees. Each evidence item has uniform storage and verification cost. The construction uses standard hash-and-sign under collision-resistant hashing and EUF-CMA signatures. A follow-up paper (Dec 2025) addresses post-quantum resilience. This is directly relevant to mojave's audit chain / genesis sentinel work -- Kao's formalization could provide the security definitions mojave needs, and the constant-size property addresses scalability concerns for long eval campaigns.

### 7. Sigstore Model Transparency v1.0 enables keyless ML model signing (Apr 2025)

Google/Red Hat/OpenSSF released Sigstore's model-transparency library v1.0, which applies software supply chain signing to ML models. Uses keyless signing via OIDC + Fulcio certificates + Rekor transparency log. Already adopted by NVIDIA NGC (Jul 2025). PyPI, Maven Central, and Homebrew all use Sigstore-signed attestations. For mojave's audit chains, Sigstore could handle the "bind eval run to model identity" requirement without mojave needing to build its own PKI. The question is whether Sigstore's model signing provides sufficient granularity for mojave's per-run cryptographic binding needs, or whether mojave's custom chain approach offers properties Sigstore does not.

### 8. "Anytime Validity is Free" proves any fixed-sample test induces an anytime-valid sequential test (Dec 2025)

Koning and van Meer (Erasmus Rotterdam) prove that for any valid test based on N observations, one can construct an anytime-valid sequential test matching it after N observations. Published in JRSS-B (Feb 2026). They derive the anytime-valid sequentialized z-test and t-test. This simplifies mojave's confidence-sequence implementation: rather than building confidence sequences from scratch via e-values, mojave could "sequentialize" its existing fixed-sample tests. The practical implication is that mojave can offer both fixed-sample and sequential inference with minimal additional code.

### 9. WMDP sandbagging is now a documented, scaling threat (ICLR 2025 + ongoing)

The "AI Sandbagging" paper (ICLR 2025, Anthropic/MATS) demonstrates that frontier LLMs can be prompted or fine-tuned to strategically underperform on WMDP while maintaining high MMLU scores. Evaluation awareness follows a power-law scaling relationship with model size. This directly threatens mojave's WMDP Phase 1 work: any WMDP scores must be interpreted in light of potential sandbagging. mojave's Saltelli-based sensitivity analysis could actually help detect sandbagging -- if the sensitivity of WMDP score to prompt perturbation is anomalously high compared to benign benchmarks, that signals strategic behavior.

### 10. Spark-LLM-Eval: distributed, statistically rigorous eval at scale (Jan 2026)

Mitra's Spark-LLM-Eval treats LLM evaluation as a data-parallel problem on Apache Spark. Every metric gets bootstrap confidence intervals; model comparisons use paired t-tests, McNemar's, or Wilcoxon signed-rank. Uses Delta Lake for content-addressable response caching. Open source on GitHub (bassrehab/spark-llm-eval). This is a competing framework to mojave's approach, though it lacks GSA and audit chains. Its strength is scale (millions of samples) and cost optimization via caching. mojave's differentiation: GSA, cryptographic audit, and statistical rigor beyond bootstrap CIs.

### 11. UK AISI Inspect framework now has 200+ pre-built evals (2025-2026)

UK AI Safety Institute's Inspect AI (MIT-licensed) provides opinionated primitives (dataset -> Task -> Solver -> Scorer), sandboxed execution, and a community eval registry. As of May 2026, community contributions use a register/ folder with YAML manifests. The Autonomous Systems Evaluation Standard mandates Inspect for all UK AISI evaluations. Inspect is the closest government-backed competitor to mojave's eval framework, though it lacks GSA and cryptographic audit. Its 200+ pre-built evals could be a source of benchmark tasks for mojave validation.

### 12. Benchmark Health Index quantifies when benchmarks expire (Feb 2026)

The BHI framework audits benchmarks along three axes: Capability Discrimination, Anti-Saturation, and Impact. Key finding: static benchmarks lose ranking signal in under two years on average. MMLU is effectively saturated; MMLU-Pro frontier models cluster in 88-94%. This validates mojave's approach of treating eval as a living measurement process rather than a static benchmark. BHI's Anti-Saturation metric could inform when mojave should recommend refreshing a WMDP question pool or rotating eval items.

### 13. EU AI Act high-risk obligations and TEVV requirements hit August 2026

Article 73 serious-incident reporting begins August 2, 2026. High-risk AI systems require conformity assessments, dataset versioning, model lineage tracking, and input/output/decision-point logging. Post-market monitoring plans must be operational. This creates immediate demand for mojave's audit chain capabilities in the European market. Defense-first, regulated-industries-follow is validated by this timeline.

### 14. NIST CAISI launches AI Agent Standards Initiative (Feb 2026) and publishes post-deployment monitoring guidance (Mar 2026)

NIST AI 800-4 (Mar 9 2026) is the first federal guidance for post-deployment AI monitoring. The AI Agent Standards Initiative (Feb 17 2026) responds to proliferation of commercial agentic AI. NIST's ARIA program uses a three-layered evaluation culminating in a Contextual Robustness Index (CoRIx). These NIST efforts are creating the institutional framework that mojave can plug into -- particularly for defense customers who need NIST-aligned evaluation methodologies.

### 15. Chen (2026) unifies measurement-error and prediction-powered inference for noisy LLM judges

"Efficient Inference for Noisy LLM-as-a-Judge Evaluation" (Jan 2026) derives efficient influence function (EIF) estimators that unify Rogan-Gladen misclassification correction with prediction-powered inference (PPI). Produces confidence intervals 35-55% narrower than standard PPI. Relevant if mojave ever uses LLM-as-judge evaluation alongside MCQ scoring -- the EIF framework could improve statistical efficiency of any judge-based eval component.

### 16. DoD mandated to review AI/ML cybersecurity practices by August 2026

Congressional legislation requires DoD to establish department-wide AI cybersecurity and governance policy, addressing lifecycle security, model tampering protection, and data leakage. DIA is actively seeking AI validation tools. This confirms mojave's defense-establishment-first market positioning. The tamper-evident audit chain is precisely what DoD will need to demonstrate compliance with the cybersecurity review mandate.

## Questions for Deep Investigation

1. **Mazo's generalized sensitivity measures vs. classical Sobol**: Does mojave's salib-rs implementation need to support the generalized paradigm, or are Sobol indices sufficient for the current LLM eval use cases? What would generalized measures buy for multi-factor eval designs?

2. **Sigstore vs. custom audit chains**: Can Sigstore model-transparency signing replace or supplement mojave's genesis sentinel / audit chain approach? What properties does mojave's chain provide that Sigstore's model signing does not (e.g., per-item granularity, sequential binding of eval runs)?

3. **Noise decomposition and Saltelli sampling**: Given Wang's finding that prediction noise dominates data noise in LLM evals, should mojave's Saltelli sampling design explicitly budget model replications per question? How does this interact with the N(D+1) sample budget?

4. **Sandbagging detection via GSA**: Can anomalous sensitivity patterns (e.g., WMDP score being hypersensitive to prompt wording relative to MMLU) serve as a sandbagging indicator? This would be a novel contribution.

5. **NIST alignment**: Should mojave's run cards explicitly map to NIST AI 800-3 GLMM framework and/or the ARIA CoRIx scoring? This could accelerate defense-sector adoption.

6. **Anytime validity via test induction**: Can mojave's existing fixed-sample tests be "sequentialized" using Koning-van Meer's construction, replacing bespoke confidence-sequence code?

7. **Benchmark Health Index for WMDP**: What is WMDP's current BHI score? Is it approaching saturation for frontier models, and does mojave need to plan for WMDP successor benchmarks?

## Gaps Identified

1. **No Rust GSA library besides salib-rs**: Web search confirms no competing Rust implementation of Sobol/Saltelli sensitivity analysis. salib-rs appears to be the only Rust GSA library. The `sobol-qmc` and `sobol` crates on crates.io provide only sequence generation, not full sensitivity analysis.

2. **No framework combines GSA + audit chains + confidence sequences**: Every competing framework (Inspect, Spark-LLM-Eval, OpenAI Evals, DeepEval) addresses one or two of mojave's pillars. None integrates all three. This remains mojave's unique value proposition.

3. **Optimal-transport GSA has no Rust/Python-Rust implementation**: Borgonovo's OT-based sensitivity indices are only available in R (gsaot). A Rust or Python-with-Rust-backend implementation would be novel.

4. **No public framework applies GSA to LLM evaluation**: Despite extensive search, no published work applies Sobol indices or any GSA method to decompose variance in LLM benchmarks. mojave appears to be first-mover in this space.

5. **Post-quantum audit chains for AI are under-explored**: Only Kao's two papers address this. The post-quantum resilience of mojave's audit chain design is an open question that may matter for long-lived defense evaluations.

6. **Agent Cards standard is nascent**: The Springer chapter on "Agent Cards" as documentation for operational AI agents is the only formal proposal. mojave's run cards could evolve into or inform this standard.

## Leads

### Papers to acquire / read deeper
- Borgonovo et al. (2024). "Global Sensitivity Analysis via Optimal Transport." Management Science 71(5):3809-3828. [paywall -- check ASU library access]
- Borgonovo et al. (2025). "Convexity and measures of statistical association." JRSS-B 87(4):1281-1304. [check ASU access]
- "AISafetyBenchExplorer: A Metric-Aware Catalogue of AI Safety Benchmarks Reveals Fragmented Measurement and Weak Benchmark Governance" (arXiv 2604.12875, Apr 2026)
- "Reproducible, Explainable, and Effective Evaluations of Agentic AI for Software Engineering" (arXiv 2604.01437, Apr 2026)
- "Time-sensitive anytime-valid testing" (arXiv 2605.06521, May 2026) -- extends Koning-van Meer

### Repos to examine
- [sigstore/model-transparency](https://github.com/sigstore/model-transparency) -- v1.0 model signing library
- [bassrehab/spark-llm-eval](https://github.com/bassrehab/spark-llm-eval) -- distributed eval with bootstrap CIs
- [UKGovernmentBEIS/inspect_ai](https://github.com/UKGovernmentBEIS/inspect_ai) -- UK AISI eval framework
- [UKGovernmentBEIS/inspect_evals](https://github.com/UKGovernmentBEIS/inspect_evals) -- 200+ pre-built evals
- [mlcommons/ailuminate](https://github.com/mlcommons/ailuminate) -- MLCommons safety benchmark
- [centerforaisafety/wmdp](https://github.com/centerforaisafety/wmdp) -- WMDP benchmark repo

### People to track
- **Aaditya Ramdas** (CMU) -- confidence sequences, sequential analysis, anytime-valid inference
- **Emanuele Borgonovo** (Bocconi) -- GSA theory, optimal transport indices, sensitivity measures
- **Gildas Mazo** (INRAE/MaIAGE) -- generalized sensitivity paradigm
- **Sida Wang** (Meta FAIR) -- noise decomposition in LLM evals
- **Arvind Narayanan** (Princeton) -- AI agent reliability science
- **Leo Kao** -- cryptographic evidence structures for AI audit
- **Andrew Keller** (NIST) -- AI evaluation toolbox, GLMM for benchmarks
- **Nick Koning** (Erasmus) -- anytime validity theory

### Standards and regulatory documents
- NIST AI 800-3: "Expanding the AI Evaluation Toolbox with Statistical Models" (Feb 2026)
- NIST AI 800-4: Post-deployment AI monitoring guidance (Mar 2026)
- EU AI Act Article 73: Serious-incident reporting (effective Aug 2026)
- ISO/IEC 42001:2023 (AI Management Systems) + ISO/IEC 42005:2025 (AI Impact Assessment)
- DoD AI/ML cybersecurity review mandate (report due Aug 2026)

## Acquisitions

Papers downloaded to `neurotic_library/intake/` during this scout:

| File | Citation | Relevance |
|------|----------|-----------|
| Wang2025_MeasuringNoisesLLMEvals.pdf | Wang (2025). "Measuring all the noises of LLM Evals." arXiv:2512.21326 | Noise decomposition framework; informs Saltelli sampling design |
| Rabanser2026_ScienceAgentReliability.pdf | Rabanser et al. (2026). "Towards a Science of AI Agent Reliability." arXiv:2602.16666 | Closest published analog to mojave's mission; 12 reliability metrics |
| Kao2025_ConstantSizeCryptoEvidence.pdf | Kao (2025). "Constant-Size Cryptographic Evidence Structures for Regulated AI Workflows." arXiv:2511.17118 | Security definitions for audit chains; composable with hash chains |
| Kao2025_PostQuantumAuditEvidence.pdf | Kao (2025). "Post-Quantum-Resilient Audit Evidence for Long-Lived Regulated Systems." arXiv:2512.00110 | PQ extensions of evidence structures |
| Mazo2024_NewParadigmGSA.pdf | Mazo (2024/2026). "A new paradigm for global sensitivity analysis." arXiv:2409.06271 | Redefines Sobol indices; generalized sensitivity measures |
| Koning2025_AnytimeValidityFree.pdf | Koning & van Meer (2025). "Anytime Validity is Free: Inducing Sequential Tests." arXiv:2501.03982 | Simplifies confidence-sequence implementation |
| Mitra2026_SparkLLMEval.pdf | Mitra (2026). "Spark-LLM-Eval." arXiv:2603.28769 | Competing framework; bootstrap CIs, paired significance tests |
| Chen2026_EfficientInferenceNoisyJudge.pdf | Chen (2026). "Efficient Inference for Noisy LLM-as-a-Judge Evaluation." arXiv:2601.05420 | EIF estimators unifying measurement-error approaches |
| BHI2026_BenchmarkHealthIndex.pdf | (2026). "Benchmark Health Index." arXiv:2602.11674 | Framework for auditing benchmark saturation/health |
| Keller2026_NISTAIEvalToolbox.pdf | Keller et al. (2026). "Expanding the AI Evaluation Toolbox with Statistical Models." NIST AI 800-3 | Federal standard for principled AI eval statistics |
| SafetyBenchBenchmark2026_HowShouldBenchmarkSafety.pdf | (2026). "How Should AI Safety Benchmarks Benchmark Safety?" arXiv:2601.23112 | Meta-framework for safety benchmark design |
| Ndzomga2026_EfficientBenchmarkingAgents.pdf | Ndzomga (2026). "Efficient Benchmarking of AI Agents." arXiv:2603.23749 | Sample-efficient agent eval; 44-70% task reduction |
