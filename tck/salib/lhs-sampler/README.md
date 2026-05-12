# TCK — saltelli LHS sampler

The Layer-1 outer Gherkin gate for `saltelli_samplers::LhsSampler`
(Latin Hypercube Sampling, classic and centered variants).

Mirrors the TCK posture in
`decisions/2026-04-28-saltelli-tck-posture.md` § "Layer 1 — Outer
Gherkin TCK." Inner unit tests under
`crates/saltelli-samplers/src/lhs.rs::tests` cover the same
properties at finer granularity.

## What this directory covers

- **`lhs_structural.feature`** — output shape, [0, 1) bounds, the
  load-bearing LHS property (each column has exactly one sample per
  stratification cell `[k/n, (k+1)/n)`), centered-cell-center
  property. Pins
  `decisions/2026-04-28-saltelli-lhs-sampler.md` § "What this gates."

- **`lhs_determinism.feature`** — same `RngState` produces
  bit-identical matrix; different streams produce different
  matrices; `RngState` advances `word_pos`; centered uses fewer RNG
  bytes than classic. Pins the same ADR § "Determinism."

## What this directory does NOT cover

- **Maximin / orthogonal / replicated LHS.** Defer to follow-on PRs
  with their own ADRs.
- **Sampler trait conformance for non-LHS samplers.** Each sampler
  gets its own TCK directory (`sobol-sampler/`, `morris-sampler/`,
  …) per `plans/0002-saltelli-roadmap.md`.
- **Saltelli matrix construction on top of LHS.** Lands in PR 6
  (`tck/saltelli/saltelli-sampler/`).
- **Cross-platform-byte-exact `unit_sample` under FMA on/off.** The
  sampler doesn't use FMA-sensitive operations (only u32 → f64
  conversion + `+ /`); the no-FMA reference build at
  `xtask reference-ci` exercises it. Bead `prior-project-i8q` covers
  the GitHub Actions matrix.

## Step-definition home

- `crates/saltelli-samplers/tests/lhs_tck.rs` wires both
  `lhs_structural.feature` and `lhs_determinism.feature`.

## See also

- `decisions/2026-04-28-saltelli-lhs-sampler.md` — the ADR this
  directory operationalizes.
- `decisions/2026-04-28-saltelli-tck-posture.md` — four-layer
  validation strategy.
- `tck/saltelli/README.md` — saltelli-wide TCK layout.
- McKay-Beckman-Conover 1979 — original LHS paper.
