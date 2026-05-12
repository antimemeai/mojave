# TCK — saltelli DGSM estimator

Layer-1 outer Gherkin gate for `saltelli_estimators::estimate_dgsm`
— Sobol-Kucherenko 2009 derivative-based sensitivity measure with
Poincaré-inequality-bounded total-order Sobol' indices. PR 13 of
`plans/0002-saltelli-roadmap.md`.

## What this directory covers

- **`dgsm_ishigami.feature`** — four scenarios:
  - `ν` recovers closed-form values (`ν_2 = 24.5` exact;
    `ν_1 ≈ 7.72`, `ν_3 ≈ 10.99`).
  - Poincaré property `ST_analytic ≤ ST_upper` holds per factor.
  - Central finite-difference agrees with analytical gradient
    to `1e-5`.
  - Factor ranking by `ν` correct (`ν_2 > ν_3 > ν_1`).

## What this directory does NOT cover

- **Convergence-rate test + cargo-mutants kill rate.** Live inline
  in `crates/saltelli-estimators/tests/dgsm_e2e.rs`.
- **Forward FD truncation-error scaling.** Inner unit test in
  `dgsm::tests`.
- **Poincaré constants for non-Uniform/Normal distributions.**
  Bead-eligible (prior-project-10w, prior-project-4s0).
- **Bootstrap CIs for `ν` and `ST_upper`.** Bead-eligible
  (prior-project-988).

## Step-definition home

- `crates/saltelli-estimators/tests/dgsm_tck.rs` wires
  `dgsm_ishigami.feature`.

## See also

- `decisions/2026-04-29-saltelli-dgsm.md` — ADR.
- Sobol, I. M., Kucherenko, S. (2009). "Derivative based global
  sensitivity measures and their link with global sensitivity
  indices." *Mathematics and Computers in Simulation* 79.
- Roustant, O., Barthe, F., Iooss, B. (2017). "Poincaré inequalities
  on intervals — application to sensitivity analysis." *Electronic
  Journal of Statistics* 11. (Vendored at
  `~/projects/papers/Poincaré Inequalities on Intervals — Roustant et al. 2017.pdf`.)
