# TCK — saltelli analytic test functions

The Layer-1 outer Gherkin gate for `saltelli-validation`'s analytic
test functions and their closed-form Sobol' indices.

Mirrors the TCK posture in
`decisions/2026-04-28-saltelli-tck-posture.md` § "Layer 1 — Outer
Gherkin TCK." Per-function unit-test coverage is Layer 2 (in-crate
under `crates/saltelli-validation/src/*/tests`).

## What this directory covers

- **`ishigami_analytic.feature`** — closed-form invariants for
  Ishigami: `S_3 = 0` (the canary), `S_T2 = S_2` (X_2 has no
  interactions), published-value agreement at canonical `(a, b) =
  (7, 0.1)`, monotone behavior under parameter sweeps. Pins
  `decisions/2026-04-28-saltelli-validation-pattern.md` § "Ishigami."

- **`sobol_g_analytic.feature`** — closed-form invariants for
  Sobol' G: `V_i = (1/3) / (1 + a_i)²` per factor, total variance
  `D = Π (1 + V_i) - 1` product form, ranking agreement under
  varying `a` parameter (smaller `a` ⇒ larger `S_i`). Pins
  `decisions/2026-04-28-saltelli-validation-pattern.md` § "Sobol' G."

## What this directory does NOT cover

- **Total-order Sobol' G indices.** Deferred. Saltelli-Sobol 1995
  Eqs 22-24 give a recursive product form; lands in a follow-on PR.
- **Morris-test (Morris 1991 §4).** Its analytic ground truth is
  EE-style (`μ`, `μ*`, `σ`), shape-mismatched with
  `SobolIndicesAnalytic`. Bundles with the Morris estimator (PR 8
  of `plans/0002-saltelli-roadmap.md`).
- **Bratley / Borgonovo / Oakley-O'Hagan.** Land as their respective
  estimators do; the pattern is established here.
- **Frozen-CSV `SALib` differential.** Layer 3 of the validation
  strategy; lands in PR 7 (the first Sobol estimator) under
  `crates/saltelli-validation/reference/salib_outputs/`.
- **Convergence-rate tests.** Layer 4; runs against estimator output,
  not against the closed-form analytics here.

## Step-definition home

- `crates/saltelli-validation/tests/ishigami_tck.rs` wires
  `ishigami_analytic.feature`.
- `crates/saltelli-validation/tests/sobol_g_tck.rs` wires
  `sobol_g_analytic.feature`.

## See also

- `decisions/2026-04-28-saltelli-validation-pattern.md` — the ADR
  this directory operationalizes.
- `decisions/2026-04-28-saltelli-tck-posture.md` — four-layer
  validation strategy.
- `tck/saltelli/README.md` — saltelli-wide TCK layout.
- Saltelli et al. 2008, *Global Sensitivity Analysis: The Primer*,
  §5.4 (Ishigami closed forms).
- Saltelli-Sobol 1995 (Sobol' G derivation).
