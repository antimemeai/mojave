# TCK — saltelli Problem + Distribution

The Layer-1 outer Gherkin gate for `saltelli_core::problem::Problem`
(declarative input-space description) and
`saltelli_core::distribution::Distribution` (closed enum of factor
distributions with `quantile` + `support`).

Mirrors the TCK posture in
`decisions/2026-04-28-saltelli-tck-posture.md` § "Layer 1 — Outer
Gherkin TCK." Inner property + identity tests, frozen-CSV SALib
differential, and convergence-rate tests live inside `saltelli-core`
(not here) per Layers 2-4.

## What this directory covers

- **`content_addressing.feature`** — `Problem::content_hash()` is
  stable across calls; content-equivalent Problems hash equally;
  semantically-distinct Problems (different distribution params,
  different factor names, different factor order, different
  `FactorKind`) hash distinctly. Pins
  `decisions/2026-04-28-saltelli-problem-shape.md` § "Content-
  addressing."

- **`inverse_cdf_round_trip.feature`** — per-distribution
  `quantile(u)` properties: support boundaries (`quantile(0)` and
  `quantile(1)`), monotonicity along `[0, 1]`, known-quantile-points
  (e.g. `Normal::quantile(0.5) == mu`, `Uniform::quantile(0.5) ==
  midpoint`), special-case identities (`Beta(1,1) ≡ Uniform`,
  `Gamma(1, scale) ≡ Exponential(1/scale)`, `Weibull(1, scale) ≡
  Exponential(1/scale)`).

## What this directory does NOT cover

- **CDF (forward direction).** Not in scope for PR 3; lands with the
  `Truncated` distribution variant in a follow-on PR.
- **Empirical / Truncated.** Deferred to follow-on PRs each with
  their own ADR (interpolation policy for `Empirical`; truncation
  discipline for `Truncated`).
- **Groups, correlation, output spec.** Deferred to follow-on PRs;
  `Problem` is `#[non_exhaustive]` so they land non-breaking.
- **Sampler-level determinism.** `RngState` + tree-fold reductions
  covered by `tck/saltelli/rng-determinism/`.

## Step-definition home

- `crates/saltelli-core/tests/distribution_tck.rs` wires
  `inverse_cdf_round_trip.feature`.
- `crates/saltelli-core/tests/problem_tck.rs` wires
  `content_addressing.feature`.

## See also

- `decisions/2026-04-28-saltelli-problem-shape.md` — the ADR this
  directory operationalizes.
- `decisions/2026-04-28-saltelli-tck-posture.md` — four-layer
  validation strategy.
- `tck/saltelli/README.md` — saltelli-wide TCK layout.
- `rust_salib_crate_research.md` § 3.1 — sky-side spec for the
  closed `Distribution` enum.
