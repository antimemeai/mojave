# mojave

Measurement science for AI evaluation.

~30,000 lines of Rust across 18 crates. 400+ behavioral test scenarios.
4-gate validation against textbook reproductions, reference implementations,
property-based invariants, and Monte Carlo calibration.

---

Most evals tell you a score went up or down. They don't tell you
whether the movement is signal or noise, whether your judges agree
or just share biases, whether your tasks are measuring one thing or
five, or whether you can stop the run early and trust the answer.

A systematic review of 445 LLM benchmarks found that 84% use no
statistical testing, nearly half target contested constructs, and
only 16% report uncertainty estimates [1]. Nobody checks whether the
evaluation itself is reliable, sensitive, or well-calibrated.

mojave does. It applies the statistical discipline of manufacturing
quality control, psychometric test design, and nuclear stockpile
certification to AI evaluation. The math is old. The application
is new. The standards are non-negotiable.

## What it answers

| Question | Method | Crate |
|----------|--------|-------|
| What's driving your scores? | Sobol/Saltelli variance decomposition [2][3] | `salib-*` |
| How much is signal vs noise? | G-theory reliability coefficients [4] | `salib-estimators` |
| Do your judges agree? | IRR + latent-class diagnostics [5][6][7] | `irr` |
| Can the judges discriminate? | MSA ndc [8], Mandel h/k [9] | `irr` |
| Can you stop early? | Anytime-valid inference, betting CS [10][11] | `seq-anytime-valid` |
| Did anything change? | SPC control charts, e-detector [12][13] | `spc-charts` |
| Does the model pass? | QMU confidence ratio [14], JCGM 106 guard bands [15] | `eval-orchestrator` |
| Which tasks are doing work? | IRT item analysis [16] | `mojave-calibrate` |
| Is the eval gameable? | Randomized item selection | `eval-design` |
| Is the chain tamper-evident? | SHA-256 hash chain, Ed25519/COSE_Sign1 | `audit-*` |

## Architecture

Two layers, clean boundary. Rust owns correctness and real-time
decisions. Python owns offline model fitting (IRT calibration,
factor analysis). They communicate via subprocess + JSON.

```
              eval runner output (Inspect, HAL, custom)
                              |
                              v
                        +-----------+
                        | eval-ingest|
                        +-----+-----+
                              |
        +---------------------+---------------------+
        |                     |                     |
        v                     v                     v
  +-----------+     +-----------------+    +--------------+
  | Rust engine|     | Python calibrate|    | audit chain  |
  |            |     |                 |    |              |
  | salib  GSA |     | py-irt     IRT  |    | tamper-      |
  | irr    IRR |     | deepirtools DFA |    | evident      |
  | seq-*  CS  |     | semopy     CFA  |    | provenance   |
  | spc-*  SPC |     +--------+--------+    | Sigstore     |
  | qmu   QMU  |              |             +--------------+
  +------+-----+              |
         |    JSON boundary   |
         |<-------------------+
         v
  +--------------------------------------+
  | mojave-cli / mojave-gsa              |
  |                                      |
  | run cards - conformity decisions -   |
  | stop/continue - control charts -     |
  | convergence diagnostics              |
  +--------------------------------------+
```

## Rust crates

### Sensitivity analysis ([salib-rs](https://crates.io/crates/salib))

Global sensitivity analysis in Rust. Strict superset of Python SALib [17].

| Crate | What |
|-------|------|
| `salib-core` | RNG, distributions, problem specs |
| `salib-samplers` | LHS, Sobol QMC, Morris, FAST, Saltelli |
| `salib-estimators` | Sobol (S1/S2/ST), Morris, FAST, Borgonovo, G-theory, ANOVA, HDMR |
| `salib-surrogate` | Polynomial chaos expansion |
| `salib-shapley` | Shapley effects |

### Measurement engine

| Crate | What |
|-------|------|
| `irr` | Cohen/Fleiss/Krippendorff/Gwet + bootstrap CIs, Dawid-Skene, MSA ndc/P-T, Mandel h/k |
| `seq-anytime-valid` | SPRT, mSPRT, e-values, confidence sequences, Waudby-Smith betting CS |
| `spc-charts` | Shewhart, CUSUM, EWMA, e-detector, ARL |
| `eval-design` | Computerized adaptive testing, anti-gaming scheduling |
| `perturbation-engine` | Deterministic perturbation primitives |
| `change-attribution` | Git-commit-to-score-change attribution |

### Infrastructure

| Crate | What |
|-------|------|
| `eval-core` | Trial records, score types |
| `eval-ingest` | Pluggable ingestion from eval runner output |
| `eval-orchestrator` | Pipeline orchestration, QMU conformity assessment |
| `mojave-cli` | Unified CLI |
| `mojave-gsa` | Saltelli manifest generation, Sobol/Borgonovo analysis |
| `audit-chain` | Tamper-evident hash chain |
| `audit-sign` | Ed25519 signing, COSE_Sign1 attestation |
| `audit-emit` | Event emitter with blob store |
| `audit-recover` | Crash recovery, chain replay |

## Validation

Every estimator passes a 4-gate validation:

1. **Textbook reproductions** -- golden datasets from the original papers
2. **Reference impl cross-checks** -- R packages at pinned versions
3. **Property-based tests** -- invariants, identities, boundary conditions
4. **Monte Carlo calibration** -- coverage, Type I error, power

## Theory

The core bet is that AI evaluation is a measurement problem, not a
software engineering problem. The field is rediscovering -- poorly --
ideas that psychometrics, industrial statistics, and clinical trials
solved decades ago.

**Variance decomposition.** Sobol indices [2][3] tell you how much of
your eval score comes from the model vs the prompt vs the judge vs the
temperature vs noise. Most frameworks don't ask the question.

**Generalizability theory.** G-theory [4] produces reliability
coefficients and D-study projections: "you need N=1024 to achieve
Phi >= 0.80." Replaces ad-hoc sample size decisions with principled
planning.

**Sequential testing.** The Waudby-Smith & Ramdas [10] betting
confidence sequence lets you monitor a running eval and stop early
with statistical guarantees. Near-optimal width for bounded data
without distributional assumptions.

**QMU.** Quantification of Margins and Uncertainties [14] transforms
eval output from "here are statistics" into "the model does / does not
pass under guarded acceptance with consumer risk < 5%." JCGM 106 [15]
guard bands formalize accept/reject under measurement uncertainty.

**Statistical process control.** Shewhart [12] and Page [13] charts
watch a process over time and tell you when something changed. Every
commit either holds the line or triggers a control chart signal.

**The oracle problem.** Barr et al. [18] formalize the connection
between black-box testing theory and eval grading. LLM-as-judge is a
derived oracle; metamorphic relations [19] formalize invariance tests;
mutation analysis [20] calibrates eval sensitivity. CheckList [21]
is the closest prior operationalization. mojave's measurement stack
is the quantitative treatment of oracle reliability and validity.

**Construct validity.** Bean et al. [1] reviewed 445 benchmarks and
found pervasive validity failures. Jacobs & Wallach [22] argue that
AI benchmarks lack measurement models entirely. Raji et al. [23]
show that "general" benchmarks cannot validly operationalize abstract
capabilities. mojave's Sobol decomposition quantifies how much of a
benchmark's score variance is construct-relevant vs construct-irrelevant,
producing a sensitivity profile that no other tool provides.

## References

[1] Bean, Kearns, Romanou et al. "Measuring what Matters: Construct Validity in Large Language Model Benchmarks." NeurIPS 2025 Datasets & Benchmarks. arXiv:2511.04703.

[2] Sobol, I. M. "Sensitivity estimates for nonlinear mathematical models." *Mathematical Modelling and Computational Experiments* 1(4):407-414, 1993.

[3] Saltelli, A. et al. "Variance based sensitivity analysis of model output." *Computer Physics Communications* 181(2):259-270, 2010.

[4] Cronbach, L. J., Gleser, G. C., Nanda, H. & Rajaratnam, N. *The Dependability of Behavioral Measurements: Theory of Generalizability for Scores and Profiles.* Wiley, 1972.

[5] Cohen, J. "A coefficient of agreement for nominal scales." *Educational and Psychological Measurement* 20(1):37-46, 1960.

[6] Krippendorff, K. "Computing Krippendorff's Alpha-Reliability." *Communication Methods and Measures*, 2011.

[7] Gwet, K. L. "Computing inter-rater reliability and its variance in the presence of high agreement." *British Journal of Mathematical and Statistical Psychology* 61(1):29-48, 2008.

[8] AIAG. *Measurement Systems Analysis Reference Manual.* 4th ed., 2010.

[9] ISO 5725-2:1994. "Accuracy (trueness and precision) of measurement methods and results -- Part 2: Basic method for the determination of repeatability and reproducibility of a standard measurement method."

[10] Waudby-Smith, I. & Ramdas, A. "Estimating means of bounded random variables by betting." *Annals of Statistics*, 2024.

[11] Howard, S. R., Ramdas, A., McAuliffe, J. & Sekhon, J. "Time-uniform, nonparametric, nonasymptotic confidence sequences." *Annals of Statistics* 49(2):1055-1085, 2021.

[12] Shewhart, W. A. *Economic Control of Quality of Manufactured Product.* Van Nostrand, 1931.

[13] Page, E. S. "Continuous inspection schemes." *Biometrika* 41(1-2):100-115, 1954.

[14] Pilch, M., Trucano, T. & Helton, J. "Ideas Underlying Quantification of Margins and Uncertainties (QMU)." SAND2006-5001. Sandia National Laboratories, 2006.

[15] JCGM 106:2012. "Evaluation of measurement data -- The role of measurement uncertainty in conformity assessment."

[16] Lord, F. M. & Novick, M. R. *Statistical Theories of Mental Test Scores.* Addison-Wesley, 1968.

[17] Herman, J. & Usher, W. "SALib: An open-source Python library for Sensitivity Analysis." *Journal of Open Source Software* 2(9):97, 2017.

[18] Barr, E. T., Harman, M., McMinn, P., Shahbaz, M. & Yoo, S. "The Oracle Problem in Software Testing: A Survey." *IEEE TSE* 41(5):507-525, 2015.

[19] Segura, S., Fraser, G., Sanchez, A. B. & Ruiz-Cortes, A. "A Survey on Metamorphic Testing." *IEEE TSE* 42(9):805-824, 2016.

[20] DeMillo, R. A., Lipton, R. J. & Sayward, F. G. "Hints on Test Data Selection: Help for the Practicing Programmer." *IEEE Computer* 11(4):34-41, 1978.

[21] Ribeiro, M. T., Wu, T., Guestrin, C. & Singh, S. "Beyond Accuracy: Behavioral Testing of NLP Models with CheckList." ACL 2020.

[22] Jacobs, A. Z. & Wallach, H. "Measurement and Fairness." FAccT 2021.

[23] Raji, I. D., Bender, E. M., Paullada, A., Denton, E. & Hanna, A. "AI and the Everything in the Whole Wide World Benchmark." NeurIPS 2021 Datasets & Benchmarks.

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## License

MIT OR Apache-2.0
