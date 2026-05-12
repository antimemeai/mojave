# TCK — saltelli Morris elementary-effects estimator

The Layer-1 outer Gherkin gate for `saltelli_estimators::estimate_morris_effects`
+ `saltelli_samplers::build_morris_trajectories` (Morris 1991 +
Campolongo 2007 `μ*`).

Mirrors the TCK posture in
`decisions/2026-04-28-saltelli-tck-posture.md` § "Layer 1." This is
the second PR (PR 8 of `plans/0002-saltelli-roadmap.md`) closing the
**reviewer-affordance contract**, after PR 7's Saltelli2010 estimator.

## What this directory covers

- **`morris_additive_linear.feature`** — the headline scenario.
  Morris elementary-effects estimator over the additive-linear test
  function `Y = Σ i·xᵢ` for d=8 produces `μ_i = i` exactly (purely
  linear; no MC noise on EE), `μ*_i = i`, `σ_i = 0`. Plus
  `μ*_i ≥ |μ_i|` identity (Campolongo 2007).
- **`morris_quadratic_additive.feature`** — substantive contract
  close. Morris elementary-effects estimator over the quadratic-
  additive test function `Y = Σ bᵢxᵢ + cᵢxᵢ²` (`bᵢ = cᵢ = i+1`)
  for d=8. `μᵢ = 2(i+1)`, `σᵢ = (i+1)/3` recovered within MC
  tolerance at R=1000. Pins the convergence-rate behavior the
  linear case can't (linear EE has no MC noise). Per
  `decisions/2026-04-29-saltelli-morris-quadratic-contract.md`.

## What this directory does NOT cover

- **The convergence-rate, identity, and SALib differential tests.**
  They live as inner tests in
  `crates/saltelli-estimators/tests/morris_e2e.rs` (matching PR 7's
  pattern: Gherkin for the headline, in-crate for the contract artifacts).
- **Morris 1991 §4 20-factor function.** Deferred. Its analytic μ
  requires fixing the random β coefficients at a specific seed; PR
  8 ships the additive-linear case instead (per
  `decisions/2026-04-29-saltelli-morris-estimator.md` § "Scope
  refinement").
- **Campolongo trajectory optimization.** Deferred to PR 8.5.
- **Trajectory-builder Gherkin scenarios.** The trajectory sampler
  has its own load-bearing claim (OAT property), pinned by inner
  unit tests in `crates/saltelli-samplers/src/morris.rs::tests`.
  Not in Gherkin today.

## Step-definition home

- `crates/saltelli-estimators/tests/morris_tck.rs` wires
  `morris_additive_linear.feature`.
- `crates/saltelli-estimators/tests/morris_quadratic_tck.rs` wires
  `morris_quadratic_additive.feature`.

## See also

- `decisions/2026-04-29-saltelli-morris-estimator.md` — the ADR
  this directory operationalizes.
- `decisions/2026-04-28-saltelli-tck-posture.md` — four-layer
  validation strategy + reviewer-affordance contract.
- `crates/saltelli-estimators/tests/morris_e2e.rs` — the
  contract-close in-crate test surface.
- Morris, M. D. (1991). "Factorial sampling plans for preliminary
  computational experiments."
- Campolongo, F., Cariboni, J., Saltelli, A. (2007). "An effective
  screening design for sensitivity analysis of large models."
