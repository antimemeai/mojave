# TCK — saltelli Sobol' estimator (Saltelli2010)

The Layer-1 outer Gherkin gate for `saltelli_estimators::estimate_saltelli2010`
(Saltelli's 2010 first-order + Jansen 1999 total-order).

Mirrors the TCK posture in
`decisions/2026-04-28-saltelli-tck-posture.md` § "Layer 1." This is
the first PR (PR 7 of `plans/0002-saltelli-roadmap.md`) that closes
the **reviewer-affordance contract** end-to-end — every saltelli
estimator PR from here forward inherits the artifact pattern this
PR establishes.

## What this directory covers

- **`saltelli2010_ishigami.feature`** — the headline scenario.
  Saltelli2010 over Ishigami at canonical `(a=7, b=0.1, N=8192)`
  produces `S_1, S_2, S_3, S_T1, S_T2, S_T3` within MC-noise
  tolerance of analytic; `S_3 ≈ 0` (the canary); `S_T2 ≈ S_2`
  (X_2 has no interactions); `Σ S_i ≤ 1`. Pins
  `decisions/2026-04-29-saltelli-saltelli2010-estimator.md` §
  "Reviewer-affordance contract close."

## What this directory does NOT cover

- **The convergence-rate test, identity test, and SALib differential.**
  Those live as inner tests in
  `crates/saltelli-estimators/tests/ishigami_e2e.rs` because their
  shape (parameter sweeps, file reads) doesn't fit Gherkin cleanly.
  The Gherkin scenario here is the *headline* assertion that a
  reviewer reads first; the inner tests are the discharge of the
  full reviewer-affordance contract.

- **Morris elementary-effects estimator.** Lands with PR 8.
- **FAST / eFAST / RBD-FAST / Borgonovo / PAWN / DGSM / regression
  / given-data.** Phase C.
- **Janon 2014, Owen 2013, Jansen alt estimators.** Phase D.

## Step-definition home

- `crates/saltelli-estimators/tests/saltelli2010_tck.rs` wires
  `saltelli2010_ishigami.feature`.

## See also

- `decisions/2026-04-29-saltelli-saltelli2010-estimator.md` — the
  ADR this directory operationalizes.
- `decisions/2026-04-28-saltelli-tck-posture.md` — four-layer
  validation strategy + reviewer-affordance contract.
- `crates/saltelli-estimators/tests/ishigami_e2e.rs` — the
  contract-close in-crate test surface (Layers 2–4).
- `crates/saltelli-validation/reference/salib_outputs/ishigami_saltelli2010_n8192.csv`
  — frozen `SALib` reference data.
- Saltelli, A. et al. (2010). "Variance based sensitivity analysis
  of model output. Design and estimator for the total sensitivity
  index."
- Jansen, M. J. W. (1999). "Analysis of variance designs for model
  output."
