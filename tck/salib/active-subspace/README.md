# TCK — saltelli active-subspace estimator

Layer-1 outer Gherkin gate for `saltelli_surrogate::compute_active_subspace`
— Constantine-Dow-Wang 2014 gradient-based dimension reduction. PR
18 of `plans/0003-saltelli-phase-d.md`.

## What this directory covers

- **`active_subspace.feature`** — three scenarios:
  - **Ridge function** `f(x) = aᵀx` produces rank-1 `C̃` with leading
    eigenvector aligned to `a/||a||` (Constantine 2014 § 2.1 special
    case).
  - **Ishigami canonical** spectrum identifies X_2 as the leading
    active direction (per-factor mean-squared gradient dominance via
    the `a=7` coefficient on `sin²(X_2)`).
  - **Determinism** — same gradient input → bit-identical
    eigendecomposition.

## What this directory does NOT cover

- **Active-subspace response-surface fitting** (Constantine § 4-5
  builds a Kriging surrogate on the projected coordinates). PR 18
  ships subspace identification only — caller fits surrogates at the
  downstream layer using the recovered eigenvectors. Bead-tracked in
  `prior-project-6qb`.
- **Bootstrap CIs over eigenvalues / eigenvectors.** Bead-eligible.
- **Adaptive sampling** for the gradient ensemble (Constantine § 7).
  Bead-eligible.

## Step-definition home

`crates/saltelli-surrogate/tests/active_subspace_tck.rs`.
