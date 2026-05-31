# Papers Acquired -- mojave Advisory 2026-05-30

All papers downloaded to `/Users/patrickbeam/projects/neurotic_library/intake/` during the advisory.

---

## Web Scout acquisitions (12 papers)

| # | Filename | Citation | Agent | Relevance |
|---|----------|----------|-------|-----------|
| 1 | Wang2025_MeasuringNoisesLLMEvals.pdf | Wang, S. (2025). "Measuring all the noises of LLM Evals." arXiv:2512.21326 | Web Scout | Noise decomposition framework (prediction/data/total). Informs Saltelli sampling design -- prediction noise dominates data noise by 2x. |
| 2 | Rabanser2026_ScienceAgentReliability.pdf | Rabanser, S., Kapoor, S. & Narayanan, A. (2026). "Towards a Science of AI Agent Reliability." arXiv:2602.16666 | Web Scout | Closest published analog to mojave. 12 reliability metrics across 4 dimensions (consistency/robustness/predictability/safety). Complementary, not competing. |
| 3 | Kao2025_ConstantSizeCryptoEvidence.pdf | Kao, L. (2025). "Constant-Size Cryptographic Evidence Structures for Regulated AI Workflows." arXiv:2511.17118 | Web Scout | Security definitions for audit chains. Composable with hash chains and Merkle trees. |
| 4 | Kao2025_PostQuantumAuditEvidence.pdf | Kao, L. (2025). "Post-Quantum-Resilient Audit Evidence for Long-Lived Regulated Systems." arXiv:2512.00110 | Web Scout | Post-quantum extensions of evidence structures. Relevant for long-lived defense evaluations. |
| 5 | Mazo2024_NewParadigmGSA.pdf | Mazo, G. (2024/2026). "A new paradigm for global sensitivity analysis." arXiv:2409.06271 | Web Scout | Redefines Sobol indices without decomposition. No impact on current salib-rs but provides theoretical umbrella for future generalized measures. |
| 6 | Koning2025_AnytimeValidityFree.pdf | Koning, N. & van Meer, R. (2025). "Anytime Validity is Free: Inducing Sequential Tests." arXiv:2501.03982 | Web Scout | Any fixed-sample test can be sequentialized into anytime-valid test. Simplifies CS implementation. |
| 7 | Mitra2026_SparkLLMEval.pdf | Mitra, S. (2026). "Spark-LLM-Eval." arXiv:2603.28769 | Web Scout | Competing framework. Distributed eval with bootstrap CIs, paired significance tests, Delta Lake caching. Lacks GSA and audit chains. |
| 8 | Chen2026_EfficientInferenceNoisyJudge.pdf | Chen, Y. (2026). "Efficient Inference for Noisy LLM-as-a-Judge Evaluation." arXiv:2601.05420 | Web Scout | EIF estimators unifying measurement-error and prediction-powered inference. 35-55% narrower CIs than standard PPI. |
| 9 | BHI2026_BenchmarkHealthIndex.pdf | (2026). "Benchmark Health Index." arXiv:2602.11674 | Web Scout | Framework for auditing benchmark saturation/health along 3 axes. Static benchmarks lose signal in < 2 years. |
| 10 | Keller2026_NISTAIEvalToolbox.pdf | Keller, A. et al. (2026). "Expanding the AI Evaluation Toolbox with Statistical Models." NIST AI 800-3 | Web Scout | First federal publication calling for measurement-science rigor in AI eval. GLMMs for variance decomposition. Institutional validation. |
| 11 | SafetyBenchBenchmark2026_HowShouldBenchmarkSafety.pdf | (2026). "How Should AI Safety Benchmarks Benchmark Safety?" arXiv:2601.23112 | Web Scout | Meta-framework for safety benchmark design. |
| 12 | Ndzomga2026_EfficientBenchmarkingAgents.pdf | Ndzomga, J. (2026). "Efficient Benchmarking of AI Agents." arXiv:2603.23749 | Web Scout | Sample-efficient agent eval with 44-70% task reduction. |

## X-Factor Scout acquisitions (3 papers + 1 duplicate)

| # | Filename | Citation | Agent | Relevance |
|---|----------|----------|-------|-----------|
| 13 | Pilch2006_QMU_WhitePaper.pdf | Pilch, M., Trucano, T. & Helton, J. (2006). "Ideas Underlying Quantification of Margins and Uncertainties (QMU): A White Paper." SAND2006-5001. Sandia National Laboratories. | X-Factor | Canonical QMU reference. Defines Confidence Ratio = margin/uncertainty. Structural isomorphism to mojave's 5 pillars. |
| 14 | Takeshita2026_BootstrapISO5725.pdf | Takeshita, J. et al. (2026). "Bootstrap-based estimation and inference for the ISO 5725 variance components." arXiv:2602.01931 | X-Factor | Bootstrap extension of ISO 5725 for small-to-moderate designs. Reference implementation for Mandel h/k with resampling CIs. |
| 15 | Mari2005_FoundationsMeasurement.pdf | Mari, L. (2005). "The problem of foundations of measurement." *Measurement* 38:259-266. | X-Factor | Three positions on measurement (realist/representational/model-dependent). Argues most AI eval is number assignment, not measurement. |
| 16 | JCGM2012_106_ConformityAssessment.pdf | JCGM 106:2012. "Evaluation of measurement data -- The role of measurement uncertainty in conformity assessment." | X-Factor | Duplicate of copy already in lib. Guard-band decision rules for accept/reject under measurement uncertainty. |

---

## Summary

- **Total papers acquired:** 16 unique (1 duplicate of existing library holding)
- **Agents acquiring:** Web Scout (12), X-Factor Scout (4)
- **All confirmed present** in `/Users/patrickbeam/projects/neurotic_library/intake/` as of 2026-05-30
- **No papers acquired by:** Codebase Scout (N/A), Adversary (by design), Library Scout (identified priorities only), Wave-2 agents (built on wave-1 acquisitions)
