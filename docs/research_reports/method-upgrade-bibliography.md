# Method-Upgrade Bibliography

Companion bibliography for the critical review of *Longitudinal SPC for AI Agent Evaluation*. Citations are organized by the six method-upgrade recommendations from the prior report (IRT, G-theory → mixed-effects, SPRT → anytime-valid inference, Sobol → Shapley/fANOVA, Krippendorff α → bias-aware latent-class, ICH E9(R1) → NIST RMF + ABC stack), with a final section on the broader LLM-eval state-of-the-art that justifies the moves.

Each entry has a short annotation explaining *what role it plays* in the upgrade. Citations marked **⭐** are the ones I'd put in your repository's `papers/` directory first.

---

## 1. Item Response Theory for LLM / agent evaluation

The recommendation: keep IRT, default to 2PL with Bayesian estimation, *empirically* test unidimensionality, drop 3PL except when guessing is mechanically present (MCQ), and use bifactor / multidimensional IRT when items load on heterogeneous capabilities.

### 1.1 Foundational IRT (textbook layer)

- Lord, F. M. and Novick, M. R. (1968). *Statistical Theories of Mental Test Scores*. Addison-Wesley. — The classical reference for IRT and classical test theory. The Lord 3PL is from here.
- Birnbaum, A. (1968). "Some latent trait models and their use in inferring an examinee's ability." In Lord & Novick (eds.), as above, ch. 17–20. — 2PL and 3PL parameterizations.
- Rasch, G. (1960). *Probabilistic Models for Some Intelligence and Attainment Tests*. Danish Institute for Educational Research. — Rasch / 1PL model.
- Samejima, F. (1969). "Estimation of latent ability using a response pattern of graded scores." *Psychometrika Monograph Supplement* 17. — **Graded response model.** This is what you want for ordinal judge scores; reserve 2PL/3PL for binary success/fail.
- Bradlow, E. T., Wainer, H., and Wang, X. (1999). "A Bayesian random effects model for testlets." *Psychometrika* 64(2): 153–168. https://doi.org/10.1007/BF02294533 — **Testlet response model.** Use this when items cluster (same scaffolding, same generator template) and local independence is violated. Detect violations with Yen's Q3.
- Yen, W. M. (1984). "Effects of local item dependence on the fit and equating performance of the three-parameter logistic model." *Applied Psychological Measurement* 8(2): 125–145. — Yen's Q3 residual statistic for detecting local-independence violations.
- Reckase, M. D. (2009). *Multidimensional Item Response Theory*. Springer. — Reference text for M2PL and bifactor models. Use when unidimensionality fails.
- Holzinger, K. J. and Swineford, F. (1937). "The bi-factor method." *Psychometrika* 2(1): 41–54. — Original bifactor model. Combined with modern estimation (Gibbons & Hedeker 1992), it's the natural choice for agent eval where there's a general capability plus task-cluster-specific abilities.

### 1.2 IRT applied to NLP / LLM evaluation (recent, application layer)

- **⭐ Lalor, J. P., Rodriguez, P., Sedoc, J., and Hernández-Orallo, J. (2024). "Item Response Theory for Natural Language Processing."** *Proceedings of the 18th Conference of the European Chapter of the Association for Computational Linguistics: Tutorial Abstracts*, pp. 9–13. St. Julian's, Malta. ACL Anthology: 2024.eacl-tutorials.2. https://aclanthology.org/2024.eacl-tutorials.2/ — **The canonical handle for "IRT in NLP is a real thing."** This tutorial and its accompanying reading list are your single best citation when defending the framework to a frontier-lab reviewer.
- **⭐ Zhou, H., Huang, H., Zhao, Z., Han, L., Wang, H., Chen, K., Yang, M., Bao, W., Dong, J., Xu, B., Zhu, C., Cao, H., and Zhao, T. (2026). "Lost in Benchmarks? Rethinking Large Language Model Benchmarking with Item Response Theory."** *Proceedings of the AAAI Conference on Artificial Intelligence*. arXiv:2505.15055. https://arxiv.org/abs/2505.15055 — **PSN-IRT (Pseudo-Siamese Network for IRT).** Demonstrates that traditional IRT struggles at the complexity/scale of LLM benchmarks and proposes a richer neural-IRT formulation. 11 benchmarks, 41,871 items. Cite when you need a 2025/2026 reference for IRT-on-LLMs.
- Luo, R. et al. (2025). "Measuring Competency, Not Performance: Item-Aware Evaluation Across Medical Benchmarks." arXiv:2509.24186. — **MEDIRT.** Joint modeling of latent competency and item difficulty/discrimination across 71 LLMs on USMLE-aligned content. Includes *benchmark integrity validation* — empirically tests whether items within a topic measure a single coherent ability. This is the unidimensionality-testing pattern you should adopt.
- Lalor, J. P. and Rodriguez, P. (2022). "py-irt: A Scalable Item Response Theory Library for Python." *INFORMS Journal on Computing*. — Reference Python implementation; you may want it as a baseline for cross-checking your Rust port.
- Lalor, J. P., Wu, H., and Yu, H. (2016). "Building an Evaluation Scale using Item Response Theory." *Proceedings of EMNLP 2016*. — Early application of IRT to NLP evaluation.
- Lalor, J. P., Wu, H., Munkhdalai, T., and Yu, H. (2018). "Understanding Deep Learning Performance through an Examination of Test Set Difficulty: A Psychometric Case Study." *Proceedings of EMNLP 2018*. — Demonstrates IRT-based test-set difficulty estimation for DNNs.
- Sedoc, J. and Ungar, L. (2020). "Item Response Theory for Efficient Human Evaluation of Chatbots." *Proceedings of the First Workshop on Evaluation and Comparison of NLP Systems*, pp. 21–33. — IRT for human-rated NLG / chatbots.
- Rodriguez, P., Htut, P. M., Lalor, J. P., and Sedoc, J. (2022). "Clustering Examples in Multi-Dataset Benchmarks with Item Response Theory." *Proceedings of INSIGHTS at ACL 2022*. — Shows that current IRT models are not as good at identifying differences as the field expects; useful for honest disclosure of limitations.
- "Lifting the Benchmark Iceberg with Item-Response Theory." OpenReview ZyVQqK7mcP. https://openreview.net/forum?id=ZyVQqK7mcP — Shows hidden implementation choices in benchmarks bias rankings; IRT proposed as transparency fix.

### 1.3 Stable estimation at LLM scale (the small-n problem)

- Castleman, B., Zhuang, S., Xu, Z. et al. (2025). "An Interpretable and Scalable Framework for Evaluating Large Language Models." arXiv:2605.07046. https://arxiv.org/abs/2605.07046 — **Majorization-minimization IRT estimator built for LLM-scale settings.** This is the algorithm you should benchmark your Rust implementation against; addresses the n≈5–50 testees problem directly.
- Plumed, F. M. et al. (2019). "Item Response Theory in AI: Analysing Machine Learning Classifiers at the Instance Level." *Artificial Intelligence* 271: 18–42. — Early systematic application of IRT to ML classifiers; useful for the "this isn't a new idea" framing.

---

## 2. Generalizability Theory → Mixed-Effects Variance Decomposition

The recommendation: rename your G-theory module "Mixed-Effects Variance Decomposition." Treat judge config as a *fixed effect*, treat sampling/seed as random. Cite Brennan §3.4.

### 2.1 Classical G-theory

- Cronbach, L. J., Gleser, G. C., Nanda, H., and Rajaratnam, N. (1972). *The Dependability of Behavioral Measurements: Theory of Generalizability for Scores and Profiles*. Wiley. — Original G-theory monograph. The framework you are correctly identifying as awkward for LLM judges.
- **⭐ Brennan, R. L. (2001). *Generalizability Theory*. Springer-Verlag. ISBN 978-0-387-95282-3.** — **The reference text.** §3.4 ("Mixed Designs") explicitly handles fixed-effect facets within G-theory — the exact reframing you need. Dependability coefficient Φ extends naturally.
- Shavelson, R. J. and Webb, N. M. (1991). *Generalizability Theory: A Primer*. SAGE. — Accessible companion to Brennan.

### 2.2 Mixed-effects models for inter-rater data

- Hoyt, W. T. (2000). "Rater bias in psychological research: When is it a problem and what can we do about it?" *Psychological Methods* 5(1): 64–86. https://doi.org/10.1037/1082-989X.5.1.64 — Treatment of rater bias as a fixed effect within ANOVA frameworks.
- Putka, D. J., Le, H., McCloy, R. A., and Diaz, T. (2008). "Ill-structured measurement designs in organizational research: Implications for estimating interrater reliability." *Journal of Applied Psychology* 93(5): 959–981. — How to handle the realistic case where not all raters rate all items (your "not all judges judge all tasks" scenario).
- Bates, D., Mächler, M., Bolker, B., and Walker, S. (2015). "Fitting Linear Mixed-Effects Models Using lme4." *Journal of Statistical Software* 67(1): 1–48. https://doi.org/10.18637/jss.v067.i01 — The canonical reference for the modern estimation toolchain; your Rust port should produce results matching `lme4`.
- Searle, S. R., Casella, G., and McCulloch, C. E. (1992). *Variance Components*. Wiley. — Reference text for variance-component estimation.

### 2.3 ICC and reliability extensions

- McGraw, K. O. and Wong, S. P. (1996). "Forming inferences about some intraclass correlation coefficients." *Psychological Methods* 1(1): 30–46. — The ICC variants (ICC(2,1), ICC(2,k), etc.) for different judge-design assumptions.
- Koo, T. K. and Li, M. Y. (2016). "A Guideline of Selecting and Reporting Intraclass Correlation Coefficients for Reliability Research." *Journal of Chiropractic Medicine* 15(2): 155–163. — Practical guideline for choosing the right ICC form.

---

## 3. SPRT → Anytime-Valid Inference (the largest single swap)

The recommendation: replace SPRT and α-spending as primary primitives with confidence sequences and e-processes. They handle dependence, peeking, and composite hypotheses correctly. **This is the upgrade that most strengthens defensibility for a 2026 reviewer.**

### 3.1 Foundational

- Wald, A. (1945). "Sequential tests of statistical hypotheses." *Annals of Mathematical Statistics* 16(2): 117–186. — The classical SPRT. The thing you are replacing.
- Robbins, H. (1970). "Statistical methods related to the law of the iterated logarithm." *Annals of Mathematical Statistics* 41(5): 1397–1409. — Confidence sequences via LIL; the proto-anytime-valid construction.
- Robbins, H. and Siegmund, D. (1974). "The expected sample size of some tests of power one." *Annals of Statistics* 2(3): 415–436.

### 3.2 Modern anytime-valid inference (cite all four; ⭐ are must-cites)

- **⭐ Howard, S. R., Ramdas, A., McAuliffe, J., and Sekhon, J. (2021). "Time-uniform, nonparametric, nonasymptotic confidence sequences."** *Annals of Statistics* 49(2): 1055–1080. DOI: 10.1214/20-AOS1991. arXiv:1810.08240. https://projecteuclid.org/journals/annals-of-statistics/volume-49/issue-2/Time-uniform-nonparametric-nonasymptotic-confidence-sequences/10.1214/20-AOS1991.full — **The canonical citation for confidence sequences valid at every stopping time.** Replaces α-spending for monitored experiments. The companion software is at https://github.com/gostevehoward/confseq.
- Howard, S. R., Ramdas, A., McAuliffe, J., and Sekhon, J. (2020). "Time-uniform Chernoff bounds via nonnegative supermartingales." *Probability Surveys* 17: 257–317. — The probability theory foundation. Treat as the "how-it-works" companion to the AoS paper.
- **⭐ Ramdas, A., Grünwald, P., Vovk, V., and Shafer, G. (2023). "Game-Theoretic Statistics and Safe Anytime-Valid Inference."** *Statistical Science* 38(4): 576–601. DOI: 10.1214/23-STS894. arXiv:2210.01948. — **The state-of-the-art unified treatment of SAVI.** Treat as the survey/textbook chapter you'd hand a new engineer joining the team. Introduces test martingales, e-processes, confidence sequences in one framework.
- **⭐ Ramdas, A. and Wang, R. (2025). *Hypothesis Testing with E-Values*.** Manuscript / book, CMU. https://stat.cmu.edu/~aramdas/ebook-final.pdf — **Your operational reference text.** Chapter 7 covers sequential testing with e-processes specifically. Use this as the conceptual basis for `seq-test`.
- Ramdas, A., Ruf, J., Larsson, M., and Koolen, W. M. (2022). "Admissible anytime-valid sequential inference must rely on nonnegative martingales." arXiv:2009.03167. — **Important theoretical result:** any admissible anytime-valid procedure is a nonnegative martingale or test supermartingale. Justifies the architectural choice.

### 3.3 E-values and combination

- Vovk, V. and Wang, R. (2021). "E-values: Calibration, combination and applications." *Annals of Statistics* 49(3): 1736–1754. — **E-value averaging / merging.** Lets you combine evidence from multiple eval streams (agent×task×judge slices) without union bounds. Critical for your multi-metric dashboard case.
- Grünwald, P., de Heide, R., and Koolen, W. M. (2024). "Safe testing." *Journal of the Royal Statistical Society Series B*. arXiv:1906.07801. — Composite-hypothesis testing with optional stopping; the e-process framework applied to common parametric tests.
- Shafer, G. (2021). "Testing by betting: A strategy for statistical and scientific communication." *Journal of the Royal Statistical Society Series A* 184(2): 407–431. — The betting-game intuition that makes SAVI digestible to non-mathematicians. Useful for documentation aimed at customers.
- **Koning, N. W. and van Meer, S. (2026). "Anytime validity is free: inducing sequential tests."** *Journal of the Royal Statistical Society Series B*, advance article. DOI: 10.1093/jrsssb/qkag050. arXiv:2501.03982. — **The "no power cost" result.** Shows that for any valid N-observation test, you can induce an anytime-valid sequential test that matches its power at N. This is the *political* citation: it removes the "anytime-valid procedures sacrifice power" objection. (Note: I attributed this incorrectly to Koolen in the original report; Koolen gave feedback but the authors are Koning & van Meer.)

### 3.4 Engineering / practical SAVI

- Waudby-Smith, I. and Ramdas, A. (2024). "Estimating means of bounded random variables by betting." *Journal of the Royal Statistical Society Series B* 86(1): 1–27. arXiv:2010.09686. — **The "betting" approach to confidence sequences for bounded random variables** (e.g., binary judge outcomes, bounded task scores). Practical and tight; use this for your binary-outcome `seq-test` API.
- Shin, J., Ramdas, A., and Rinaldo, A. (2023). "E-detectors: A Nonparametric Framework for Sequential Change Detection." *New England Journal of Statistics in Data Science*. https://doi.org/10.51387/23-NEJSDS51 — **Sequential change-point detection with e-processes.** This is the right tool for SPC drift detection on agent metrics.
- Fischer, L. and Ramdas, A. (2024). "Improving the (approximate) sequential probability ratio test by avoiding overshoot." arXiv:2410.16076. — Documents the overshoot bias in approximate SPRT, justifying the move to e-processes even for users who think classical SPRT "just works."
- Johari, R., Pekelis, L., and Walsh, D. J. (2022). "Always valid inference: Continuous monitoring of A/B tests." *Operations Research* 70(3): 1806–1821. — The mSPRT framework deployed at Optimizely / Microsoft. **The pragmatic A/B-test framing that maps onto your customers' mental model.** Includes implementation guidance.
- Ter Schure, J., Pérez-Ortiz, M., Ly, A., and Grünwald, P. (2024). "The Anytime-Valid Logrank Test: Error Control Under Continuous Monitoring with Unlimited Horizon." *New England Journal of Statistics in Data Science*. — Survival-style anytime-valid testing; useful if you ever model time-to-failure for agent tasks.

### 3.5 Multi-stream / FWER under sequential monitoring

- Bartroff, J., Lai, T. L., and Shih, M.-C. (2014). "Sequential Tests of Multiple Hypotheses Controlling Type I and II Familywise Error Rates." PMC4118217. *Sequential Experimentation in Clinical Trials*. — Multi-stream SPRT with FWER control. Your fallback for the multi-comparison binary case.

### 3.6 Critique of importing classical sequential testing naively

- "Sequential Test for Practical Significance: Truncated Mixture Sequential Probability Ratio Test." arXiv:2509.07892, September 2025. — Recent critique of practical SPRT use; documents the conditions under which Type I error breaks down.

---

## 4. Sobol Indices → Shapley Effects / Functional ANOVA / Mixed-Effects Variance Components

The recommendation: keep Sobol/Saltelli for continuous-hyperparameter scans (temperature, top-p, retrieval-k); default to **Shapley effects** for categorical factor designs; offer **mixed-effects variance components** as the "principled" report. Rename the module "Influence Attribution."

### 4.1 Classical variance-based GSA

- Sobol', I. M. (1993). "Sensitivity estimates for nonlinear mathematical models." *Mathematical Modelling and Computational Experiments* 1: 407–414. — Original Sobol indices.
- Saltelli, A. (2002). "Making best use of model evaluations to compute sensitivity indices." *Computer Physics Communications* 145(2): 280–297. — Saltelli2002 estimator.
- Saltelli, A., Annoni, P., Azzini, I., Campolongo, F., Ratto, M., and Tarantola, S. (2010). "Variance based sensitivity analysis of model output. Design and estimator for the total sensitivity index." *Computer Physics Communications* 181(2): 259–270. — Saltelli2010, your current default.
- Saltelli, A., Ratto, M., Andres, T., Campolongo, F., Cariboni, J., Gatelli, D., Saisana, M., and Tarantola, S. (2008). *Global Sensitivity Analysis: The Primer*. Wiley. — Textbook reference.
- Jansen, M. J. W. (1999). "Analysis of variance designs for model output." *Computer Physics Communications* 117(1–2): 35–43. — Jansen estimator.
- Janon, A., Klein, T., Lagnoux, A., Nodet, M., and Prieur, C. (2014). "Asymptotic normality and efficiency of two Sobol index estimators." *ESAIM: Probability and Statistics* 18: 342–364. — Janon estimator with efficiency analysis.
- Owen, A. B. (2013). "Better estimation of small Sobol' sensitivity indices." *ACM Transactions on Modeling and Computer Simulation* 23(2): 1–17. — Owen estimator.

### 4.2 Shapley effects — the recommended default for categorical inputs

- **⭐ Owen, A. B. (2014). "Sobol' indices and Shapley value."** *SIAM/ASA Journal on Uncertainty Quantification* 2(1): 245–251. DOI: 10.1137/130936233. — **The originating Shapley-effects paper.** Argues Shapley value gives a coherent attribution that sums to total variance even when first-order + total Sobol fail to.
- **⭐ Song, E., Nelson, B. L., and Staum, J. (2016). "Shapley Effects for Global Sensitivity Analysis: Theory and Computation."** *SIAM/ASA Journal on Uncertainty Quantification* 4(1): 1060–1083. DOI: 10.1137/15M1048070. — **The companion theory/computation paper.** Extends to dependent inputs. Your implementation reference.
- Owen, A. B. and Prieur, C. (2017). "On Shapley value for measuring importance of dependent inputs." *SIAM/ASA Journal on Uncertainty Quantification* 5(1): 986–1002. DOI: 10.1137/16M1097717. — Handling input dependence.
- Iooss, B., Da Veiga, S., Janon, A., and Pujol, G. (2019/2021). "Shapley effects for sensitivity analysis with correlated inputs: Comparisons with Sobol' indices, numerical estimation and applications." *International Journal for Uncertainty Quantification* 9(5): 493–514. — Application-focused treatment with code. (Note: in my original report I conflated this with a 2021 "Iooss et al." paper; the canonical author for the dependent-inputs treatment is Owen-Prieur with Iooss et al. providing the comparative analysis.)
- Plischke, E., Rabitti, G., and Borgonovo, E. (2021). "Computing Shapley effects for sensitivity analysis." *SIAM/ASA Journal on Uncertainty Quantification* 9(4): 1411–1437. — **Major computational speed-ups.** Practical-implementation paper; cite when explaining why Shapley is now affordable at your scale.
- Rabitti, G. and Borgonovo, E. (2019). "A Shapley–Owen Index for Interaction Quantification." *SIAM/ASA Journal on Uncertainty Quantification* 7(3): 1060–1075. — Shapley-Owen for interaction effects (i.e., not just attribution to individual factors but to factor combinations).

### 4.3 Functional ANOVA and density-based GSA

- Hoeffding, W. (1948). "A class of statistics with asymptotically normal distribution." *Annals of Mathematical Statistics* 19(3): 293–325. — The Hoeffding-ANOVA decomposition that everything variance-based rests on.
- Hooker, G. (2007). "Generalized Functional ANOVA Diagnostics for High-Dimensional Functions of Dependent Variables." *Journal of Computational and Graphical Statistics* 16(3): 709–732. — Functional ANOVA for dependent inputs.
- Rahman, S. (2014). "A Generalized ANOVA Dimensional Decomposition for Dependent Probability Measures." *SIAM/ASA Journal on Uncertainty Quantification* 2(1): 670–697. — Theoretical extension you'll need to cite when defending the categorical-input case.
- Borgonovo, E. (2007). "A new uncertainty importance measure." *Reliability Engineering & System Safety* 92(6): 771–784. — Borgonovo δ (moment-independent), already in your stack; cite when arguing why moment-based measures might mislead in tail-risk regimes.
- Pianosi, F. and Wagener, T. (2015). "A simple and efficient method for global sensitivity analysis based on cumulative distribution functions." *Environmental Modelling & Software* 67: 1–11. — PAWN; the right tool when variance is not a sufficient summary.

### 4.4 GSA for stochastic models (your output is stochastic)

- Castellan, G., Cousien, A., and Tran, V. C. (2018). "Nonparametric adaptive estimation of order 1 Sobol indices in stochastic models, with an application to Epidemiology." arXiv:1611.07230. — Sobol indices when the model output is stochastic.
- Hart, J. L., Alexanderian, A., and Gremaud, P. A. (2017). "Efficient Computation of Sobol' Indices for Stochastic Models." *SIAM Journal on Scientific Computing* 39(4): A1514–A1530. — Practical algorithms. Cite when explaining why you replicate within-config.
- Mazo, G. (2017/2018). "Pick and freeze estimation of sensitivity indices for models with dependent and dynamic input processes." arXiv:1403.5539. — Picking and freezing under dependence.

### 4.5 Active subspaces (you already have this)

- Constantine, P. G. (2015). *Active Subspaces: Emerging Ideas for Dimension Reduction in Parameter Studies*. SIAM. — Reference text for active subspaces.

---

## 5. Krippendorff α → Bias-Aware Latent-Class Agreement

The recommendation: keep α, but stratify by judge family; require a human anchor on a calibration set; report a Dawid-Skene latent-class model jointly estimating latent truth, judge accuracy, and judge confusion patterns. Augment with self-consistency metrics.

### 5.1 Classical agreement statistics

- Krippendorff, K. (2004). *Content Analysis: An Introduction to Its Methodology* (2nd ed.). SAGE. — Reference for α.
- Krippendorff, K. (2011). "Computing Krippendorff's alpha-reliability." Annenberg School for Communication, University of Pennsylvania. http://repository.upenn.edu/asc_papers/43 — Practical computation guide.
- Cohen, J. (1960). "A coefficient of agreement for nominal scales." *Educational and Psychological Measurement* 20(1): 37–46. — Cohen's κ.
- Fleiss, J. L. (1971). "Measuring nominal scale agreement among many raters." *Psychological Bulletin* 76(5): 378–382. — Fleiss κ.
- Gwet, K. L. (2008). "Computing inter-rater reliability and its variance in the presence of high agreement." *British Journal of Mathematical and Statistical Psychology* 61: 29–48. — Gwet AC1/AC2; addresses Krippendorff's prevalence paradox.
- Bland, J. M. and Altman, D. G. (1986). "Statistical methods for assessing agreement between two methods of clinical measurement." *Lancet* 327(8476): 307–310. — Bland-Altman limits of agreement.
- Hayes, A. F. and Krippendorff, K. (2007). "Answering the Call for a Standard Reliability Measure for Coding Data." *Communication Methods and Measures* 1(1): 77–89. — The call-and-response paper establishing α as the standard.

### 5.2 Bayesian latent-class agreement (the recommended replacement / augment)

- **⭐ Dawid, A. P. and Skene, A. M. (1979). "Maximum likelihood estimation of observer error-rates using the EM algorithm."** *Journal of the Royal Statistical Society Series C* 28(1): 20–28. — **The seminal latent-class model** for jointly estimating rater accuracy and latent truth. Use this as the foundation for your "judges share confusion structure" diagnostic.
- **⭐ Paun, S., Carpenter, B., Chamberlain, J., Hovy, D., Kruschwitz, U., and Poesio, M. (2018). "Comparing Bayesian Models of Annotation."** *Transactions of the Association for Computational Linguistics* 6: 571–585. https://doi.org/10.1162/tacl_a_00040 — **The NLP-native modern survey of Bayesian annotation models.** Covers Dawid-Skene, multinomial extensions, hierarchical models. Your reference for what the latent-class judge model should look like.
- Hovy, D., Berg-Kirkpatrick, T., Vaswani, A., and Hovy, E. (2013). "Learning Whom to Trust with MACE." *Proceedings of NAACL-HLT 2013*: 1120–1130. — MACE: trust-weighted aggregation; another reference implementation.
- Raykar, V. C., Yu, S., Zhao, L. H., Valadez, G. H., Florin, C., Bogoni, L., and Moy, L. (2010). "Learning From Crowds." *Journal of Machine Learning Research* 11: 1297–1322. — Bayesian extension of Dawid-Skene with feature-conditioning; useful when you have item features.

### 5.3 LLM-judge specific reliability / bias literature (cite these in your "why α isn't enough" disclosure)

- **⭐ Li, D., Sun, R., Huang, Y., Zhong, M., Jiang, B., Han, J., Zhang, X., Wang, W., and Liu, H. (2025/2026). "Preference Leakage: A Contamination Problem in LLM-as-a-judge."** arXiv:2502.01534 (v1 Feb 2025, v3 March 2026). https://arxiv.org/abs/2502.01534 — **The smoking gun.** Defines three relatedness regimes (same model, inheritance, same family) and empirically confirms judge bias toward related student models across multiple LLM baselines and benchmarks. Cite this as the core justification for stratifying α by judge family.
- **⭐ Haldar, R. and Hockenmaier, J. (2025). "Rating Roulette: Self-Inconsistency in LLM-As-A-Judge Frameworks."** *Findings of EMNLP 2025*, pp. 24986–25004. arXiv:2510.27106. https://aclanthology.org/2025.findings-emnlp.1361/ — **Demonstrates low intra-rater reliability** for LLM judges across runs and tasks. The empirical basis for your "within-judge α can be lower than between-judge α" claim.
- Zheng, L., Chiang, W.-L., Sheng, Y., Zhuang, S., Wu, Z., Zhuang, Y., Lin, Z., Li, Z., Li, D., Xing, E. P., Zhang, H., Gonzalez, J. E., and Stoica, I. (2023). "Judging LLM-as-a-Judge with MT-Bench and Chatbot Arena." *NeurIPS 2023*. — Foundational LLM-as-judge paper; documents position bias, verbosity bias, self-preference.
- Panickssery, A., Bowman, S. R., and Feng, S. (2024). "LLM Evaluators Recognize and Favor Their Own Generations." arXiv:2404.13076. — Self-preference / egocentric bias.
- Park, J., Jwa, S., Ren, M., Kim, D., and Choi, S. (2024). "OffsetBias: Leveraging Debiased Data for Tuning Evaluators." arXiv:2407.06551. — Seven distinct bias types catalogued via meta-evaluation.
- Bavaresco, A., Bernardi, R., Bertolazzi, L. et al. (2024). "LLMs instead of Human Judges? A Large Scale Empirical Study across 20 NLP Evaluation Tasks." arXiv:2406.18403. — The meta-evaluation reference for "you need a human anchor."
- "Play Favorites: A Statistical Method to Measure Self-Bias in LLM-as-a-Judge." arXiv:2508.06709. — Quantifies family-bias rigorously across 9 judges and 5000+ pairs; shows GPT-4o and Claude 3.5 Sonnet systematically over-score own outputs.
- "An Empirical Study of LLM-as-a-Judge: How Design Choices Impact Evaluation Reliability." arXiv:2506.13639. — Recent (2025) study of design-choice effects on judge reliability; useful methodological reference for your TCK specs.
- "Diagnosing the Reliability of LLM-as-a-Judge via Item Response Theory." arXiv:2602.00521. — **Directly relevant 2026 work** combining IRT with judge reliability diagnostics — possibly the most aligned recent paper with your product thesis. Worth reading carefully.

### 5.4 Self-consistency (the SAGE-style augmentation)

- "SAGE: Self-Aligned Generalized Evaluation framework for LLM judges." arXiv:2512.16041. — Measures local self-consistency and global transitivity of LLM judges. Even Gemini-2.5-Pro and GPT-5 fail consistency in ~24% of difficult cases. Cite when proposing self-consistency as a complementary diagnostic.

---

## 6. ICH E9(R1) Estimands → NIST AI RMF + Model Cards + ABC Stack

The recommendation: keep estimand vocabulary as *one* layer (mostly for FDA/clinical AI customers); shift the primary frame to NIST AI RMF + Mitchell-style model cards + Kapoor/Stroebl/Narayanan's Agentic Benchmark Checklist + cryptographically anchored analysis plans.

### 6.1 ICH E9(R1) — keep for clinical / FDA segments

- ICH (2019). "E9(R1) Addendum on Estimands and Sensitivity Analysis in Clinical Trials." International Council for Harmonisation of Technical Requirements for Pharmaceuticals for Human Use. https://www.ich.org/page/efficacy-guidelines — The source guideline.
- **⭐ Kahan, B. C., Hindley, J., Edwards, M., Cro, S., and Morris, T. P. (2024). "The estimands framework: a primer on the ICH E9(R1) addendum."** *BMJ* 384: e076316. DOI: 10.1136/bmj-2023-076316. PMC10802140. — **The clean primer**, more readable than the ICH guideline itself.
- **⭐ Binette, O. and Reiter, J. P. (2024). "Improving the Validity and Practical Usefulness of AI/ML Evaluations Using an Estimands Framework."** arXiv:2406.10366. https://arxiv.org/abs/2406.10366 — **The only published attempt to adapt ICH E9(R1) to ML evaluation.** Apply via cross-validation, clustering eval, and LLM benchmarking examples. Use this when defending the estimand vocabulary to a methodologically literate reviewer.
- Heinrich, M., Zagorscak, P., Bohn, J., Knaevelsrud, C., and Schulze, L. (2025). "Using the ICH estimand framework to improve the interpretation of treatment effects in internet interventions." *npj Digital Medicine* 8: 469. DOI: 10.1038/s41746-025-01936-0. PMC12368201. — Recent application of E9(R1) to digital interventions; useful precedent for digital-AI extension.

### 6.2 NIST AI governance — the recommended primary frame

- **⭐ NIST (2023). "Artificial Intelligence Risk Management Framework (AI RMF 1.0)."** NIST AI 100-1. https://doi.org/10.6028/NIST.AI.100-1 — **Cite as the primary governance scaffold.** Map your metrics to the four functions: Govern, Map, Measure, Manage.
- **⭐ NIST (2024). "Artificial Intelligence Risk Management Framework: Generative AI Profile."** NIST AI 600-1. https://doi.org/10.6028/NIST.AI.600-1 — **The GenAI-specific extension.** Maps generative-AI risks (CBRN, confabulation, dangerous content, data privacy, human-AI configuration, information integrity, information security, intellectual property, obscenity, toxicity, value chain) to AI RMF subcategories.
- NIST (2024). "Four Principles of Explainable Artificial Intelligence." NIST IR 8312. — Background for explainability claims.

### 6.3 Documentation artifacts (Mitchell / Gebru pattern)

- **⭐ Mitchell, M., Wu, S., Zaldivar, A., Barnes, P., Vasserman, L., Hutchinson, B., Spitzer, E., Raji, I. D., and Gebru, T. (2019). "Model Cards for Model Reporting."** *Proceedings of the Conference on Fairness, Accountability, and Transparency (FAT\* '19)*: 220–229. DOI: 10.1145/3287560.3287596. arXiv:1810.03993. — **The canonical model-card paper.** Nine categories, ~30 disclosures; the standard format your eval-integrity report should extend.
- **⭐ Gebru, T., Morgenstern, J., Vecchione, B., Vaughan, J. W., Wallach, H., Daumé, H. III, and Crawford, K. (2021). "Datasheets for Datasets."** *Communications of the ACM* 64(12): 86–92. DOI: 10.1145/3458723. arXiv:1803.09010. — **Companion dataset-documentation standard.** Seven categories, 57 questions.
- Bender, E. M. and Friedman, B. (2018). "Data Statements for Natural Language Processing: Toward Mitigating System Bias and Enabling Better Science." *Transactions of the ACL* 6: 587–604. — NLP-specific data documentation.
- Pushkarna, M., Zaldivar, A., and Kjartansson, O. (2022). "Data Cards: Purposeful and Transparent Dataset Documentation for Responsible AI." *FAccT 2022*: 1776–1826. — Modern Data Cards extending Datasheets.
- Arnold, M., Bellamy, R. K. E., Hind, M. et al. (2019). "FactSheets: Increasing Trust in AI Services through Supplier's Declarations of Conformity." *IBM Journal of Research and Development* 63(4/5): 6:1–6:13. — IBM's FactSheets framework; alternative documentation lineage worth knowing.

### 6.4 Reproducibility and the agentic-eval framing

- Pineau, J., Vincent-Lamarre, P., Sinha, K., Larivière, V., Beygelzimer, A., d'Alché-Buc, F., Fox, E., and Larochelle, H. (2021). "Improving Reproducibility in Machine Learning Research (A Report from the NeurIPS 2019 Reproducibility Program)." *Journal of Machine Learning Research* 22(164): 1–20. — The NeurIPS Reproducibility Checklist as procedural standard.
- **⭐ Kapoor, S., Stroebl, B., Siegel, Z. S., Nadgir, N., and Narayanan, A. (2025). "AI Agents That Matter."** *Transactions on Machine Learning Research* (TMLR). arXiv:2407.01502. https://openreview.net/forum?id=Zy4uFzMviZ — **The current canonical critique of agent benchmark practice.** Includes the **Agentic Benchmark Checklist (ABC)** as an agent-specific evaluation pre-registration framework. Cite alongside NIST AI RMF.
- Siegel, Z. S., Kapoor, S., Nagdir, N., Stroebl, B., and Narayanan, A. (2024). "CORE-Bench: Fostering the Credibility of Published Research Through a Computational Reproducibility Agent Benchmark." arXiv:2409.11363. — Companion paper applying ABC principles.
- Kapoor, S., Stroebl, B., Kirgis, P., Nadgir, N., Siegel, Z. S. et al. (2025). "Holistic Agent Leaderboard: The Missing Infrastructure for AI Agent Evaluation." Preprint (HAL). https://github.com/princeton-pli/hal-harness/ — **HAL: the open-source agent eval infrastructure that's becoming the reference standard.** Your orchestration layer should interoperate with this.
- Singh, S., Nan, Y., Wang, A., D'Souza, D., Kapoor, S., Üstün, A., Koyejo, S., Deng, Y., Longpre, S., Smith, N., Ermis, B., Fadaee, M., and Hooker, S. (2025). "The Leaderboard Illusion." *NeurIPS Datasets & Benchmarks 2025*. — Recent companion critique on leaderboard reliability.

### 6.5 Foundational measurement-validity critiques

- Liang, P., Bommasani, R. et al. (2022/2023). "Holistic Evaluation of Language Models." arXiv:2211.09110. *Transactions on Machine Learning Research*. — **HELM.** The frame that "evaluation should be multi-dimensional, not a single score." Cite as motivation for your multi-faceted integrity report.
- Bowman, S. R. and Dahl, G. E. (2021). "What Will it Take to Fix Benchmarking in Natural Language Understanding?" *NAACL 2021*: 4843–4855. — Influential critique of NLU benchmarking; precursor to the current LLM-eval skepticism.
- Raji, I. D., Bender, E. M., Paullada, A., Denton, E., and Hanna, A. (2021). "AI and the Everything in the Whole Wide World Benchmark." *NeurIPS Datasets & Benchmarks*. — On construct validity in AI benchmarking; the right starting reference for the "are we measuring what we think we're measuring" frame.
- Bommasani, R., Klyman, K., Longpre, S., Kapoor, S. et al. (2023). "The Foundation Model Transparency Index." arXiv:2310.12941. — Stanford CRFM transparency work.
- Hutchinson, B., Rostamzadeh, N., Greer, C., Heller, K., and Prabhakaran, V. (2022). "Evaluation Gaps in Machine Learning Practice." *FAccT 2022*: 1859–1876. — Companion critique cited by Binette & Reiter.

### 6.6 Cryptographic anchoring (your `prereg` crate)

- Pasquini, C., Boato, G., and De Natale, F. G. B. (2022). "Image content provenance: A survey of authentication techniques." Journal-specific — Background for content-addressable storage of analysis plans.
- W3C (2022). "Verifiable Credentials Data Model v1.1." W3C Recommendation. https://www.w3.org/TR/vc-data-model/ — **The portable standard for cryptographically anchored claims.** Your `prereg` artifacts should be expressible as VCs.
- Sigstore project (2021–present). https://www.sigstore.dev/ — Practical infrastructure for code/artifact signing; the natural tooling backbone for hash-anchored pre-registration.
- Mehrabi, N., Morstatter, F., Saxena, N., Lerman, K., and Galstyan, A. (2021). "A Survey on Bias and Fairness in Machine Learning." *ACM Computing Surveys* 54(6): 1–35. — Background for the fairness-audit content of your eval-integrity report.

---

## 7. Supporting / Context Literature

These don't tie to a specific method-upgrade but justify the broader move from 1970s-classical to 2020s-contemporary methods.

### 7.1 The "science of agent reliability" frame

- Kapoor, S. and Narayanan, A. (2024). *AI Snake Oil: What Artificial Intelligence Can Do, What It Can't, and How to Tell the Difference*. Princeton University Press. — The intellectual frame your product extends. Featured in Nature's 10 best books of 2024.
- Bengio, Y., Kapoor, S. et al. (2025). "International AI Safety Report." UK AISI / international consortium. — The state-of-the-field report; cite when situating your work in the broader safety/eval landscape.
- Kapoor, S., Kolt, N., and Lazar, S. (2025). "Resist Platform-Controlled AI Agents and Champion User-Centric Agent Advocates." *ICML 2025 Position Paper Track*. — Governance / position paper context.

### 7.2 Eval infrastructure (the field you compete and compose with)

- UK AISI (2024–2026). Inspect AI evaluation framework. https://inspect.ai-safety-institute.org.uk/ — Open-source eval framework; ~200 evals; the de facto safety-eval standard. See also Inspect Sandboxing Toolkit (2025) for agent isolation.
- UK AISI (2025). "Announcing Inspect Evals." https://www.aisi.gov.uk/blog/inspect-evals — Companion eval library.
- Liang, P. et al. (2022–present). HELM and MedHELM. https://github.com/stanford-crfm/helm — Stanford CRFM evaluation framework.
- EleutherAI (2021–present). `lm-evaluation-harness`. https://github.com/EleutherAI/lm-evaluation-harness — Widely used baseline harness; reference point for benchmark integrity claims.
- OpenAI (2023–present). OpenAI Evals. https://github.com/openai/evals — Reference implementation lineage.
- BIG-Bench. Srivastava, A. et al. (2023). "Beyond the Imitation Game: Quantifying and Extrapolating the Capabilities of Language Models." *Transactions on Machine Learning Research*. arXiv:2206.04615. — The big collaborative benchmark whose limitations partly motivate measurement-rigor work.

### 7.3 Construct validity / measurement theory critiques

- Sjøberg, D. I. K. and Bergersen, G. R. (2022). "Construct Validity in Software Engineering." *IEEE Transactions on Software Engineering*. — General construct-validity treatment.
- Jacobs, A. Z. and Wallach, H. (2021). "Measurement and Fairness." *FAccT 2021*: 375–385. — The standard reference for measurement-theoretic critique of ML fairness work.
- Biderman, S. et al. (2024). "Lessons from the Trenches on Reproducible Evaluation of Language Models." arXiv:2405.14782. — Practical reproducibility lessons from the EleutherAI team.

### 7.4 Sequential / experimental design for LLM A/B testing

- Howard, S. R. and Ramdas, A. (2022). "Sequential estimation of quantiles with applications to A/B-testing and best-arm identification." *Bernoulli* 28(3): 1704–1728. — Quantile-CS for A/B testing; the closest analogue to "always-monitorable agent eval" in the existing literature.
- Lattimore, T. and Szepesvári, C. (2020). *Bandit Algorithms*. Cambridge University Press. — Reference text if you ever want to add adaptive task selection (CAT-style) using bandit machinery rather than just IRT.

---

## Quick "first-reads" list

If you can only read ten papers from this bibliography, read these in order:

1. Ramdas, Grünwald, Vovk & Shafer (2023, *Statistical Science*) — SAVI survey.
2. Ramdas & Wang (2025, manuscript) — E-values textbook, ch. 7.
3. Howard, Ramdas, McAuliffe & Sekhon (2021, *Annals of Statistics*) — Confidence sequences.
4. Lalor, Rodriguez, Sedoc & Hernández-Orallo (2024, EACL tutorial) — IRT in NLP.
5. Zhou et al. (2026, AAAI) — PSN-IRT, the modern LLM-IRT reference.
6. Owen (2014, SIAM/ASA UQ) — Shapley effects, the originating paper.
7. Song, Nelson & Staum (2016, SIAM/ASA UQ) — Shapley-effects theory & computation.
8. Li et al. (2025, arXiv:2502.01534) — Preference leakage in LLM-as-judge.
9. Dawid & Skene (1979, *JRSS C*) — Latent-class agreement.
10. Kapoor, Stroebl, Siegel, Nadgir & Narayanan (2025, TMLR) — Agents That Matter; ABC checklist.

After those, Brennan (2001) §3.4 for the G-theory reframing; Binette & Reiter (2024, arXiv:2406.10366) for the ICH-E9 adaptation; and the NIST AI RMF 1.0 + GenAI Profile (NIST AI 100-1 and 600-1) for governance.

---

## Caveat

A few entries from the original report I want to correct here:

- The "anytime validity is free" paper is by **Koning & van Meer**, not Koolen. Koolen provided feedback on early drafts but is not an author. JRSS B 2026 advance article, DOI 10.1093/jrsssb/qkag050.
- "Iooss et al. 2021" on Shapley with correlated inputs as I cited it earlier is best attributed to the Owen-Prieur (2017) line plus the Iooss-Da Veiga-Janon-Pujol (2019/2021) IJUQ paper. The "Shapley effects for sensitivity analysis with correlated inputs" citation I gave was a paraphrase, not a precise title.
- Several "Lalor et al." stability-at-scale references coalesce around the **arXiv:2605.07046** majorization-minimization line of work; if you need a single citation there, use that one.
- A small number of papers cited (the 2602.00521 IRT-LLM-judge work, the 2512.16041 SAGE paper) are very recent (late-2025 / early-2026 preprints) — verify they haven't been superseded before quoting their specific empirical numbers in a public-facing document.
