# Regression-based estimators (SRC/SRRC/PCC/PRCC + R²) on Ishigami.
#
# Per `decisions/2026-04-29-saltelli-regression.md`. Regression-based
# indices are valid only under linearity (SRC) or monotonicity
# (SRRC/PRCC). Ishigami is neither, so the load-bearing claim is
# that **R² correctly flags the indices as untrustworthy**.
#
# Mechanized: `crates/saltelli-estimators/tests/regression_tck.rs`.

Feature: estimate_regression_indices — Ishigami at d=3

  Scenario: indices stay within correlation bounds
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096
    When I estimate regression indices
    Then every SRC has magnitude at most 1
    And every PRCC has magnitude at most 1
    And R² linear is in 0 to 1
    And R² rank is in 0 to 1

  Scenario: R² correctly flags Ishigami as untrustworthy for linear regression
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096
    When I estimate regression indices
    Then R² linear is below 0.5

  Scenario: SRRC near zero for non-monotonic factor 2
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096
    When I estimate regression indices
    Then SRRC for factor 1 has magnitude below 0.1

  Scenario: SRC ratio recovers coefficients on a known linear model
    Given the linear model Y = 2 X_0 + X_1
    And LHS samples at N=1024
    When I estimate regression indices
    Then R² linear exceeds 0.99
    And the SRC ratio of factor 0 to factor 1 approximates 2 within 0.2
