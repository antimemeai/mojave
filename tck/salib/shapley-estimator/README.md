# TCK — saltelli Shapley estimator

Layer-1 outer Gherkin gate for `saltelli_shapley::estimate_shapley`
— Song-Nelson-Staum 2016 Algorithm 1 (random-permutation sampling +
double-loop Monte Carlo for the conditional-variance cost). PR 17
of `plans/0003-saltelli-phase-d.md`.

## What this directory covers

- **`shapley_ishigami.feature`** — four scenarios:
  - `Σ Sh_i = Var(Y)` exactly via prevC-carry telescoping (Song
    2016 Eq 10).
  - Each `Sh_i` recovers the Ishigami closed-form within MC
    tolerance.
  - Shapley sandwich `V_i ≤ Sh_i ≤ V_T_i` under independence
    (Theorem 2).
  - Determinism: same `RngState` → bit-identical indices.

## What this directory does NOT cover

- **Dependent inputs.** PR 17 ships independent-inputs only. Iman-
  Conover transformation lands in PR 19b; full copula library
  bead-tracked in `prior-project-bsj`.
- **Variance-reduced Shapley** (Broto 2020). Bead-tracked in
  `prior-project-gme`.
- **Given-data k-NN Shapley** (Broto 2020 § 5). Bead-tracked in
  `prior-project-gos`.
- **Cross-implementation R-reference (ShapleyEffects) byte-exact
  differential.** Bead-tracked in `prior-project-snp`.

## Step-definition home

`crates/saltelli-shapley/tests/shapley_tck.rs`.
