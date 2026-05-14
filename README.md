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

| Question | Method |
|----------|--------|
| How reliable is your scoring? | G-theory variance decomposition |
| Do your judges agree? | IRR + latent-class diagnostics |
| Which tasks are doing work? | IRT item analysis |
| What's driving your scores? | Sobol/Shapley sensitivity analysis |
| Can you stop early? | Anytime-valid inference, e-processes |
| Did anything change? | SPC control charts, e-detector change-point detection |
| Are some tasks redundant? | Factor models |

## Architecture

Rust math primitives with a Python orchestration layer.
The Rust side owns correctness. The Python side owns
workflow integration (Inspect, HAL, lm-eval-harness,
customer-built runners). Clean FFI boundary, no coupling.

```
eval runner output
       │
       ▼
  ┌─────────┐     ┌──────────────────────────────────────┐
  │  ingest  │────▶│  mojave math engine (Rust)            │
  └─────────┘     │                                        │
                  │  salib-*    sensitivity analysis (GSA)  │
                  │  irr        inter-rater reliability     │
                  │  seq-*      sequential testing          │
                  │  spc-charts control chart monitors      │
                  │  eval-core  orchestration primitives    │
                  └──────────────────────────────────────┘
                         │
                         ▼
                  diagnostic reports
                  control chart signals
                  stop/continue decisions
```

## Crate inventory

| Crate | What |
|-------|------|
| `salib-core` | RNG, distributions, problem specs |
| `salib-samplers` | LHS, Sobol, Morris, FAST, Plackett-Burman |
| `salib-estimators` | Sobol indices (S1/S2/ST), Morris, FAST, DGSM, PAWN, Borgonovo, G-theory, ANOVA, HDMR |
| `salib-surrogate` | Polynomial chaos expansion (full + sparse LARS) |
| `salib-shapley` | Shapley effects for categorical inputs |
| `salib-validation` | Reference functions (Ishigami, Sobol G), frozen SALib CSV data |
| `salib-cli` | Command-line interface |
| `irr` | Cohen's/Fleiss' kappa, ICC, Krippendorff's alpha, Gwet's AC |
| `seq-anytime-valid` | SPRT, group-sequential, mSPRT, e-values, confidence sequences |
| `spc-charts` | Shewhart, CUSUM, FIR CUSUM, EWMA, combined, e-detector, ARL |

The `salib-*` family is a strict superset of Python SALib's method coverage.

## Validation

Every estimator passes a 4-gate validation:

1. **Textbook reproductions** — golden datasets from the original papers
2. **Reference impl cross-checks** — R packages at pinned versions
3. **Property-based tests** — invariants, identities, boundary conditions
4. **Monte Carlo calibration** — coverage, Type I error, power

937 tests. Zero clippy warnings. Pre-commit hooks enforce both.

## Theory notes

The core bet is that agent evaluation is a measurement problem,
not a software engineering problem. The field is rediscovering
— poorly — ideas that psychometrics, industrial statistics, and
clinical trials solved decades ago.

Generalizability theory (Cronbach et al. 1972) decomposes score
variance into person, item, rater, and interaction effects. This
tells you how much of your eval signal is the agent vs. the judge
vs. the prompt vs. noise. Most eval frameworks don't ask the question.

Sequential testing (Wald 1945, modernized by Ramdas et al. 2020+)
lets you monitor a running eval and stop early with statistical
guarantees. The e-value framework provides anytime-valid inference:
you can peek at results continuously without inflating your error rate.
This matters when GPU hours cost real money.

SPC (Shewhart 1931, Page 1954) watches a process over time and
tells you when something changed. Applied to agent development:
establish a baseline, then every commit either holds the line or
triggers a control chart signal. The combined Shewhart-CUSUM catches
both sudden failures and slow drift.

The instruments are classical. The application — watching an AI
agent's quality trajectory across a development cycle, with the
statistical rigor of a clinical trial — is not.

## Status

Early. The math foundation is built and validated.
The orchestration layer, runner integration, and
reporting surface are ahead.

## License

MIT OR Apache-2.0
