# TCK — saltelli FAST/eFAST estimator

The Layer-1 outer Gherkin gate for `saltelli_estimators::estimate_fast`
— spectral decomposition recovering `Sᵢ` and `Sᵀᵢ` per Saltelli-
Tarantola-Chan 1999. PR 9b of `plans/0002-saltelli-roadmap.md`,
companion to PR 9a's [`saltelli_samplers::FastDesign`] sampler.

## What this directory covers

- **`fast_ishigami.feature`** — the headline scenario.
  - Estimator recovers Ishigami's analytic `(S, ST)` within FAST's
    known systematic-bias tolerance (`S` ≤ 0.05; `ST` ≤ 0.10).
  - `ST ≥ S` identity for every factor (universal Sobol').
  - Estimator agrees with `SALib`'s frozen reference values within
    MC noise (`S, ST` ≤ 0.05).

## What this directory does NOT cover

- **The reviewer-affordance contract's convergence-rate test +
  cargo-mutants kill rate.** Convergence-rate lives as an inner
  test in `crates/saltelli-estimators/tests/fast_e2e.rs`;
  cargo-mutants is bead-eligible.
- **Other test functions.** Sobol' G, Borgonovo bimodal, etc. land
  with later estimator PRs.

## Step-definition home

- `crates/saltelli-estimators/tests/fast_tck.rs` wires
  `fast_ishigami.feature`.

## See also

- `decisions/2026-04-29-saltelli-fast-estimator.md` — the ADR
  this directory operationalizes.
- `decisions/2026-04-29-saltelli-fast-sampler.md` — sibling sampler ADR.
- `decisions/2026-04-28-saltelli-tck-posture.md` — four-layer
  validation strategy + reviewer-affordance contract.
- Saltelli, A., Tarantola, S., Chan, K. P-S. (1999). "A quantitative
  model-independent method for global sensitivity analysis of model
  output." *Technometrics* 41(1).
- Ishigami, T., Homma, T. (1990). "An importance quantification
  technique in uncertainty analysis for computer models." *ISUMA '90*.
