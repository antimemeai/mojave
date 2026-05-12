# TCK — saltelli Iman-Conover correlation transformation

Layer-1 outer Gherkin gate for `saltelli_samplers::iman_conover_transform`
— Iman & Conover 1982 dependent-input correlation transformation,
applied to GSA per Mara-Tarantola-Annoni 2015 § 3.2. PR 19b of
`plans/0003-saltelli-phase-d.md`.

## What this directory covers

- **`iman_conover.feature`** — five scenarios:
  - **Marginal preservation**: each output column is a permutation
    of the corresponding input column.
  - **Pearson correlation recovery** on Gaussian marginals to within
    MC tolerance.
  - **Identity target** leaves pairwise correlation near zero.
  - **Engineering pay-off**: dependent-input Sobol' on the linear-
    additive model recovers `(S_0, S_1) = 0.610, S_2 = 0.238`
    (under correlation `ρ_01 = 0.6`) and `Σ S_i > 1` (the Sobol'
    sum-to-one identity holds only under independence per Song 2016
    Theorem 2).
  - **Determinism**: same `RngState` → bit-identical output.

## What this directory does NOT cover

- **Pearson → Spearman correlation conversion** (Liu-Kiureghian
  1986 / Li 2008). The function induces Spearman correlation
  matching the supplied matrix; for Gaussian-shaped marginals
  Spearman ≈ Pearson modulo a small Gaussian-copula factor.
  Bead-eligible.
- **Full copula library** (Gaussian, t, Clayton, Gumbel, Frank).
  Bead-tracked: `prior-project-bsj`.
- **Rosenblatt transformation** (Mara 2015 § 3.1, Appendix A) —
  the alternative dependent-input strategy. Bead-eligible.

## Step-definition home

`crates/saltelli-samplers/tests/iman_conover_tck.rs`.
