# mojave

Measurement science for AI agents. Part of [antimeme](https://antimeme.ai).

---

Most eval frameworks tell you a score went up or down.
They don't tell you whether the movement is signal or noise,
whether your judges agree or just share biases,
whether your tasks are measuring one thing or five,
or whether you can stop the run early and trust the answer.

Mojave does. It takes the output of any eval runner and
subjects it to the same statistical discipline used in
manufacturing quality control, psychometric test design,
and clinical trial monitoring. The math is old. The
application is new. The standards are non-negotiable.

## What it answers

| Question | Method | Crate / Package |
|----------|--------|-----------------|
| How reliable is your scoring? | G-theory variance decomposition | `salib-estimators` |
| Do your judges agree? | IRR + latent-class diagnostics | `irr` |
| Which tasks are doing work? | IRT item analysis | `mojave-calibrate` (py-irt) |
| What's driving your scores? | Sobol/Shapley sensitivity analysis | `salib-estimators`, `salib-shapley` |
| Can you stop early? | Anytime-valid inference, e-processes | `seq-anytime-valid` |
| Did anything change? | SPC control charts, e-detector | `spc-charts` |
| Are some tasks redundant? | Factor models, CFA | `mojave-calibrate` (deepirtools, semopy) |
| Is the eval gameable? | Randomized item selection, anti-gaming | `eval-design` |
| What caused the score change? | Git-commit attribution | `change-attribution` |

## Architecture

```
                         ┌─────────────────────────────────┐
                         │       eval runner output         │
                         │  (Inspect, HAL, lm-eval, custom) │
                         └────────────────┬────────────────┘
                                          │
                                          ▼
                                   ┌─────────────┐
                                   │  eval-ingest │
                                   └──────┬──────┘
                                          │
              ┌───────────────────────────┼───────────────────────────┐
              │                           │                           │
              ▼                           ▼                           ▼
   ┌─────────────────┐       ┌──────────────────────┐     ┌──────────────────┐
   │  Rust engine     │       │  Python calibration   │     │  audit-chain     │
   │                  │       │  (mojave-calibrate)   │     │  audit-sign      │
   │  salib-*   GSA   │       │                       │     │                  │
   │  irr       IRR   │       │  py-irt      IRT      │     │  tamper-evident  │
   │  seq-*     seq   │       │  deepirtools factors  │     │  provenance      │
   │  spc-*     SPC   │       │  semopy      CFA/SEM  │     │  Ed25519 signing │
   │  eval-design CAT │       └───────────┬───────────┘     └──────────────────┘
   └────────┬─────────┘                   │
            │                             │
            │         JSON boundary       │
            │◀────────────────────────────┘
            │
            ▼
   ┌──────────────────────────────────────────────────────┐
   │  mojave-cli                                          │
   │                                                      │
   │  diagnostic reports    control chart signals          │
   │  stop/continue         item pool + factor structure   │
   │  decisions             calibration artifacts          │
   └──────────────────────────────────────────────────────┘
```

Two layers, clean boundary. Rust owns correctness and
real-time decisions. Python owns offline model fitting
(IRT calibration, factor analysis, CFA/SEM). They
communicate via subprocess + JSON — no PyO3, no FFI,
no coupling nightmares.

## Rust crates

### Sensitivity analysis (salib-*)

A strict superset of Python SALib's method coverage, in Rust.

| Crate | What |
|-------|------|
| `salib-core` | RNG, distributions, problem specs |
| `salib-samplers` | LHS, Sobol, Morris, FAST, Plackett-Burman, fractional-factorial |
| `salib-estimators` | Sobol (S1/S2/ST), Morris, FAST, RBD-FAST, DGSM, PAWN, Borgonovo, G-theory, ANOVA, HDMR |
| `salib-surrogate` | Polynomial chaos expansion (full + sparse LARS) |
| `salib-shapley` | Shapley effects for categorical inputs |
| `salib-validation` | Reference functions (Ishigami, Sobol G), frozen SALib CSV data |
| `salib-cli` | Command-line interface |

### Measurement engine

| Crate | What |
|-------|------|
| `irr` | Cohen's/Fleiss' kappa, ICC, Krippendorff's alpha, Gwet's AC, Dawid-Skene, preference-leakage diagnostics |
| `seq-anytime-valid` | SPRT, group-sequential, mSPRT, e-values, confidence sequences |
| `spc-charts` | Shewhart, CUSUM, FIR CUSUM, EWMA, combined Shewhart-CUSUM, e-detector, ARL computation |
| `eval-design` | Computerized adaptive testing (CAT), randomized item selection, anti-gaming scheduling |
| `perturbation-engine` | Deterministic perturbation primitives for eval sensitivity analysis |
| `change-attribution` | Git-commit-to-score-change attribution for longitudinal tracking |

### Infrastructure

| Crate | What |
|-------|------|
| `eval-core` | Foundational types — trial records, item metadata, score types |
| `eval-ingest` | Pluggable ingestion from eval runner output formats |
| `eval-orchestrator` | Pipeline orchestration — ingest through analysis through reporting |
| `mojave-cli` | Unified CLI entry point |
| `audit-chain` | Tamper-evident hash chain for audit provenance |
| `audit-sign` | Ed25519 signing and COSE_Sign1 attestation |
| `metric-tck-harness` | Gherkin TCK runner for behavioral specs (dev-only) |

## Python package: mojave-calibrate

Offline calibration pipeline. Fits IRT models, factor models,
and CFA/SEM, then emits mojave-compatible JSON consumed by the
Rust engine.

```bash
# Fit 2PL IRT model, emit item pool JSON
mojave-calibrate irt --input responses.jsonl --output pool.json \
    --model-type 2pl --content-domain reasoning --device cuda

# Fit 3-factor GRM via deepirtools
mojave-calibrate factors --input responses.csv --output factors.json \
    --latent-size 3 --model-type grm

# Fit CFA model via semopy
mojave-calibrate cfa --input data.csv --output cfa.json \
    --model "f1 =~ x1 + x2 + x3"
```

| Module | Wraps | What |
|--------|-------|------|
| `irt.py` | py-irt | GPU Bayesian IRT (1PL, 2PL, 4PL) via Pyro |
| `factors.py` | deepirtools | Multidimensional IRT + factor models via IWAVE |
| `cfa.py` | semopy | Confirmatory factor analysis / structural equation modeling |
| `schema.py` | — | Validation + JSON serialization for item pools and factor structures |
| `protocol.py` | — | `CalibrationResult` dataclass + `Calibrator` protocol |

See [`python/README.md`](python/README.md) for setup and usage.

## Behavioral specs (TCK)

71 Gherkin feature files. 398 scenarios. Every crate has a
corresponding `tck/` directory with `.feature` specs that
define expected behavior before implementation begins.

```
tck/
  irr/                    # 10 feature files — kappa, ICC, Dawid-Skene, ...
  seq-anytime-valid/      # SPRT, mSPRT, e-values, confidence sequences
  spc-charts/             # Shewhart, CUSUM, EWMA, e-detector, ARL
  salib/                  # 30+ feature files — one per estimator/sampler
  eval-ingest/            # ingestion format specs
  eval-orchestrator/      # pipeline behavior
  mojave-cli/             # CLI interface contracts
  audit-chain/            # hash chain integrity
```

## Validation

Every estimator passes a 4-gate validation:

1. **Textbook reproductions** — golden datasets from the original papers
2. **Reference impl cross-checks** — R packages at pinned versions
3. **Property-based tests** — invariants, identities, boundary conditions
4. **Monte Carlo calibration** — coverage, Type I error, power

See [`docs/reference/validation-4-gate.md`](docs/reference/validation-4-gate.md)
for the full validation methodology.

## By the numbers

| | |
|-|-|
| Rust crates | 13 |
| Rust source | ~25,000 lines |
| Python source | ~1,400 lines |
| Gherkin feature files | 71 |
| Gherkin scenarios | 398 |
| Python tests | 31 |
| Pre-commit gates | clippy (zero warnings), rustfmt, ruff, mypy |

## Theory

The core bet is that agent evaluation is a measurement problem,
not a software engineering problem. The field is rediscovering
— poorly — ideas that psychometrics, industrial statistics, and
clinical trials solved decades ago.

**Generalizability theory** (Cronbach et al. 1972) decomposes
score variance into person, item, rater, and interaction effects.
This tells you how much of your eval signal is the agent vs. the
judge vs. the prompt vs. noise. Most eval frameworks don't ask
the question.

**Sequential testing** (Wald 1945, modernized by Ramdas et al.
2020+) lets you monitor a running eval and stop early with
statistical guarantees. The e-value framework provides anytime-valid
inference: you can peek at results continuously without inflating
your error rate. This matters when GPU hours cost real money.

**SPC** (Shewhart 1931, Page 1954) watches a process over time
and tells you when something changed. Applied to agent development:
establish a baseline, then every commit either holds the line or
triggers a control chart signal. The combined Shewhart-CUSUM
catches both sudden failures and slow drift.

**Item Response Theory** (Lord & Novick 1968, modernized via
GPU-accelerated Bayesian estimation) identifies which eval tasks
carry information and which are dead weight. Adaptive testing
then selects the most informative next item, cutting eval cost
without sacrificing precision.

**Factor analysis** (Thurstone 1947, via deep generative models)
reveals whether your 50-item eval is really measuring 3 things.
CFA confirms the structure; exploratory analysis discovers it.

The instruments are classical. The application — watching an AI
agent's quality trajectory across a development cycle, with the
statistical rigor of a clinical trial — is not.

## Repo structure

```
crates/            Rust workspace (13 crates)
python/            mojave-calibrate package (uv + lockfile)
tck/               Behavioral specs (71 Gherkin .feature files)
docs/
  adr/             Architectural Decision Records
  specs/           Design specifications
  reference/       Validation methodology, bibliographies
.context/          LLM working memory (beads, decisions, lit-reviews)
```

## Development

Rust:

```bash
cargo test              # run all Rust tests
cargo clippy --all-targets -- -D warnings
```

Python:

```bash
cd python
uv sync --group dev     # install dependencies
uv run pytest -v        # run all Python tests
```

## License

MIT OR Apache-2.0
