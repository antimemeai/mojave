# TCK — saltelli sparse-PCE estimator

Layer-1 outer Gherkin gate for `saltelli_surrogate::fit_sparse_pce` —
Blatman-Sudret 2011 sparse polynomial chaos via OMP forward selection
+ leave-one-out cross-validation, with optional hyperbolic q-norm
truncation. PR 16c of `plans/0003-saltelli-phase-d.md`.

## What this directory covers

- **`sparse_pce.feature`** — five scenarios:
  - Sparse PCE recovers Ishigami first-order indices to PCE tolerance.
  - Sparse PCE keeps far fewer non-zero coefficients than the full
    OLS basis (the engineering pay-off).
  - Sparse PCE picks out the active factors of a sparse-additive
    `d=10` model.
  - Hyperbolic q-norm truncation reduces basis size below total-
    degree truncation.
  - Determinism: same input → bit-identical coefficients.

## What this directory does NOT cover

- **OLS-baseline PCE.** Already covered by `pce-estimator` TCK (PR 16b).
- **Multi-output sparse PCE.** Tracked in `prior-project-ype`.
- **Sparse PCE with importance-sampling weights.** Tracked in
  `prior-project-e03`.
- **R-reference (UQLab) byte-exact differential.** Bead-eligible
  cross-cutting (`prior-project-82n`).

## Step-definition home

`crates/saltelli-surrogate/tests/sparse_pce_tck.rs`.
