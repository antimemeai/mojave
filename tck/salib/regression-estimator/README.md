# TCK — saltelli regression-based estimators

Layer-1 outer Gherkin gate for `saltelli_estimators::estimate_regression_indices`
— SRC / SRRC / PCC / PRCC + R² diagnostics. PR 14 of
`plans/0002-saltelli-roadmap.md`.

## What this directory covers

- **`regression_ishigami.feature`** — four scenarios:
  - Identity bounds (`|SRC|, |PRCC| ≤ 1`; `R² ∈ [0, 1]`).
  - R² correctly flags Ishigami as untrustworthy (`R²_linear < 0.5`).
  - SRRC ≈ 0 for non-monotonic factor 2 (sin² is symmetric).
  - SRC ratio recovers coefficients on a known linear fixture
    (`Y = 2·X_0 + X_1` → SRC ratio ≈ 2).

## What this directory does NOT cover

- **Cross-implementation differential against SALib.** Bead-eligible
  (cross-cutting prior-project-82n).
- **Bootstrap CIs for the indices.** Bead-eligible
  (prior-project-vh9 / prior-project-988 family).
- **Cohort-by-cohort comparison with Sobol' indices on linear models.**
  Inner unit test in `regression::tests`.

## Step-definition home

- `crates/saltelli-estimators/tests/regression_tck.rs` wires
  `regression_ishigami.feature`.

## See also

- `decisions/2026-04-29-saltelli-regression.md` — ADR.
- Saltelli, A., et al. (2008). *Global Sensitivity Analysis: The
  Primer*, § 1.2.4 (regression-based indices).
- Conover, W. J., Iman, R. L. (1981). "Rank transformations as a
  bridge between parametric and nonparametric statistics." *The
  American Statistician* 35(3). (Foundation for SRRC/PRCC.)
- Helton, J. C., Davis, F. J. (2002). "Latin hypercube sampling and
  the propagation of uncertainty in analyses of complex systems."
  *Reliability Engineering & System Safety* 81 (PCC/PRCC pragmatics).
