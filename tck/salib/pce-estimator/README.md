# TCK — saltelli PCE estimator

Layer-1 outer Gherkin gate for `saltelli_surrogate::fit_full_pce` +
`saltelli_surrogate::sobol_indices_from_pce` — full Polynomial Chaos
Expansion (OLS, total-degree truncation) plus Sudret 2008 closed-form
Sobol' indices from PCE coefficients. PR 16b of
`plans/0003-saltelli-phase-d.md`.

## What this directory covers

- **`pce_ishigami.feature`** — five scenarios:
  - First-order indices at Ishigami canonical recover to PCE tolerance.
  - Total-order indices at Ishigami canonical recover to PCE tolerance.
  - Sobol' decomposition identities (`S_i ≤ S_T_i`; `Σ S_i ≤ 1`) hold
    exactly (up to clamp rounding) under PCE.
  - Degree-convergence: error at `p=10, N=4096` strictly below error
    at `p=4, N=256`.
  - Determinism: same input → bit-identical PCE coefficients and
    Sobol' indices.

## What this directory does NOT cover

- **Sparse-LARS adaptive PCE.** PR 16c — separate ADR + TCK.
- **PCE on non-Uniform inputs.** Hermite (Normal), Laguerre (Exponential),
  Jacobi (Beta) bases are exercised in `polynomial::tests` but no
  end-to-end fixture exists yet — bead-eligible.
- **Multi-output PCE.** Tracked in `prior-project-ype`.
- **PCE with importance-sampling weights.** Tracked in `prior-project-e03`.
- **Sparse-grid quadrature alternative.** Tracked in `prior-project-ff9`.

## Step-definition home

`crates/saltelli-surrogate/tests/pce_ishigami_tck.rs`.
