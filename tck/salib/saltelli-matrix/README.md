# TCK — saltelli `(A, B, A_Bⁱ)` matrix construction

The Layer-1 outer Gherkin gate for `saltelli_samplers::build_saltelli_matrix`
(radial Saltelli design, Saltelli 2010).

Mirrors the TCK posture in
`decisions/2026-04-28-saltelli-tck-posture.md` § "Layer 1." Inner
unit tests in `crates/saltelli-samplers/src/saltelli_matrix.rs::tests`
cover the same properties at finer granularity.

## What this directory covers

- **`saltelli_matrix_structure.feature`** — output shape per
  `(n, sampler.dim() / 2)`; `A_Bⁱ` is `A` with column `i` replaced by
  `B`'s column `i`; `B_Aⁱ` (when `second_order`) is the symmetric
  swap; `A` and `B` are the first-half and second-half columns of
  the 2d-dim base sample. Pins
  `decisions/2026-04-29-saltelli-matrix-construction.md` § "What
  this gates."

- **`saltelli_matrix_determinism.feature`** — same `(sampler,
  RngState)` produces bit-identical output across LHS-base and
  Sobol-base samplers. Distinct streams (LHS) produce different
  matrices.

- **`saltelli_matrix_validation.feature`** — `n == 0` errors;
  `sampler.dim()` odd errors with `OddBaseDim`.

## What this directory does NOT cover

- **Original (Saltelli 2002) design.** Deferred — see ADR § "Scope
  decision: Radial only." Original requires per-sampler-class
  handling for two independent samples (LHS RNG-fork; Sobol' would
  need a `start_index` field to advance past the first N points).
- **Estimator construction over the matrix.** PR 7 — Saltelli2010
  estimator + BCa bootstrap + Ishigami convergence-rate against
  closed-form indices.

## Step-definition home

- `crates/saltelli-samplers/tests/saltelli_matrix_tck.rs` wires all
  three feature files.

## See also

- `decisions/2026-04-29-saltelli-matrix-construction.md` — the ADR
  this directory operationalizes.
- `decisions/2026-04-28-saltelli-lhs-sampler.md` — sibling sampler
  ADR.
- `decisions/2026-04-29-saltelli-sobol-sampler.md` — sibling sampler
  ADR.
- Saltelli, A. (2002). "Making best use of model evaluations to
  compute sensitivity indices."
- Saltelli et al. (2010). "Variance based sensitivity analysis of
  model output. Design and estimator for the total sensitivity
  index."
