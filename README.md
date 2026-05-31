# mojave

Measurement science for AI evaluation.

---

Most evals tell you a score went up or down. They don't tell you
whether the movement is signal or noise, whether your judges agree
or just share biases, whether your tasks are measuring one thing or
five, or whether you can stop the run early and trust the answer.

84% of LLM benchmarks use no statistical testing. Nearly half target
contested constructs. Nobody checks whether the evaluation itself is
reliable, sensitive, or well-calibrated.

mojave does. It applies the statistical discipline of manufacturing
quality control, psychometric test design, and nuclear stockpile
certification to AI evaluation. The math is old. The application
is new. The standards are non-negotiable.

## What it answers

| Question | Method | Crate |
|----------|--------|-------|
| What's driving your scores? | Sobol/Saltelli variance decomposition | `salib-*` |
| How much is signal vs noise? | G-theory reliability coefficients | `salib-estimators` |
| Do your judges agree? | IRR + latent-class diagnostics | `irr` |
| Can the judges discriminate? | MSA ndc, Mandel h/k | `irr` |
| Can you stop early? | Anytime-valid inference, betting CS | `seq-anytime-valid` |
| Did anything change? | SPC control charts, e-detector | `spc-charts` |
| Does the model pass? | QMU confidence ratio, JCGM 106 guard bands | `eval-orchestrator` |
| Which tasks are doing work? | IRT item analysis | `mojave-calibrate` |
| Is the eval gameable? | Randomized item selection | `eval-design` |
| Is the chain tamper-evident? | SHA-256 hash chain, Ed25519/COSE_Sign1 | `audit-*` |

## Architecture

Two layers, clean boundary. Rust owns correctness and real-time
decisions. Python owns offline model fitting (IRT calibration,
factor analysis). They communicate via subprocess + JSON.

```
              eval runner output (Inspect, HAL, custom)
                              │
                              ▼
                        ┌───────────┐
                        │ eval-ingest│
                        └─────┬─────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
  ┌───────────┐     ┌─────────────────┐    ┌──────────────┐
  │ Rust engine│     │ Python calibrate│    │ audit chain  │
  │            │     │                 │    │              │
  │ salib  GSA │     │ py-irt     IRT  │    │ tamper-      │
  │ irr    IRR │     │ deepirtools DFA │    │ evident      │
  │ seq-*  CS  │     │ semopy     CFA  │    │ provenance   │
  │ spc-*  SPC │     └────────┬────────┘    │ Sigstore     │
  │ qmu   QMU  │              │             └──────────────┘
  └──────┬─────┘              │
         │    JSON boundary   │
         │◀───────────────────┘
         ▼
  ┌──────────────────────────────────────┐
  │ mojave-cli / mojave-gsa              │
  │                                      │
  │ run cards ─ conformity decisions ─   │
  │ stop/continue ─ control charts ─     │
  │ convergence diagnostics              │
  └──────────────────────────────────────┘
```

## Rust crates

### Sensitivity analysis ([salib-rs](https://crates.io/crates/salib))

Global sensitivity analysis in Rust. Strict superset of Python SALib.

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

1. **Textbook reproductions** — golden datasets from the original papers
2. **Reference impl cross-checks** — R packages at pinned versions
3. **Property-based tests** — invariants, identities, boundary conditions
4. **Monte Carlo calibration** — coverage, Type I error, power

## Theory

The core bet is that AI evaluation is a measurement problem. The
field is rediscovering — poorly — ideas that psychometrics, industrial
statistics, and clinical trials solved decades ago.

**Variance decomposition** (Sobol 1993, Saltelli 2010) tells you how
much of your eval score comes from the model vs the prompt vs the
judge vs the temperature vs noise. Most frameworks don't ask the
question.

**G-theory** (Cronbach 1972) produces reliability coefficients and
D-study projections: "you need N=1024 to achieve Phi >= 0.80." Replaces
ad-hoc sample size decisions with principled planning.

**Sequential testing** (Wald 1945, Waudby-Smith & Ramdas 2024) lets
you monitor a running eval and stop early with statistical guarantees.
The betting confidence sequence achieves near-optimal width for bounded
data without distributional assumptions.

**QMU** (Pilch 2006) transforms eval output from "here are statistics"
into "the model does / does not pass under guarded acceptance with
consumer risk < 5%." Defense-native decision framework. JCGM 106 guard
bands formalize accept/reject under measurement uncertainty.

**SPC** (Shewhart 1931, Page 1954) watches a process over time and tells
you when something changed. Every commit either holds the line or
triggers a control chart signal.

**The oracle problem** (Barr et al. 2015) connects black-box testing
theory to eval grading. LLM-as-judge is a derived oracle; metamorphic
relations formalize invariance tests; mutation analysis calibrates eval
sensitivity. mojave's measurement stack is the quantitative treatment
of oracle reliability and validity.

## By the numbers

| | |
|-|-|
| Rust crates | 18 |
| Rust source | ~30,000 lines |
| Gherkin feature files | 77+ |
| Gherkin scenarios | 400+ |
| Pre-commit gates | clippy (zero warnings), rustfmt |

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## License

MIT OR Apache-2.0
