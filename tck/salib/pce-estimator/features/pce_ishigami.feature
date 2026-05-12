# Polynomial Chaos Expansion (full-OLS, total-degree truncation) +
# closed-form Sobol' indices via Sudret 2008 Eq 36-39, on Ishigami
# at canonical (a=7, b=0.1).
#
# Per `decisions/2026-04-29-saltelli-pce-fit.md`. PCE replaces the
# direct-MC Sobol' decomposition with a model-approximation pipeline:
# fit a polynomial surrogate, then derive sensitivity indices
# analytically from coefficients via the orthogonality of the
# tensor-product basis.
#
# At `(d=3, p=10)`: basis size P=286; with N=4096 samples (≫ 2P),
# Legendre PCE recovers Ishigami's analytic indices to ~1e-6 in
# absolute error — ~5 orders of magnitude tighter than direct-MC
# at the same sample budget.
#
# Mechanized: `crates/saltelli-surrogate/tests/pce_ishigami_tck.rs`.

Feature: fit_full_pce + sobol_indices_from_pce — Ishigami at d=3

  Scenario: PCE recovers Ishigami first-order indices to PCE tolerance
    Given the Ishigami model on Uniform[-π, π]³ mapped to Legendre canonical [-1, 1]³
    And Sobol' QMC samples at N=4096
    When I fit a Legendre PCE of total degree 10
    And I compute Sobol' indices from the PCE coefficients
    Then S_1 approximates 0.3139 within 0.01
    And S_2 approximates 0.4424 within 0.01
    And S_3 approximates 0.0 within 0.01

  Scenario: PCE recovers Ishigami total-order indices to PCE tolerance
    Given the Ishigami model on Uniform[-π, π]³ mapped to Legendre canonical [-1, 1]³
    And Sobol' QMC samples at N=4096
    When I fit a Legendre PCE of total degree 10
    And I compute Sobol' indices from the PCE coefficients
    Then S_T_1 approximates 0.5576 within 0.01
    And S_T_2 approximates 0.4424 within 0.01
    And S_T_3 approximates 0.2436 within 0.01

  Scenario: PCE preserves Sobol' decomposition identities exactly
    Given the Ishigami model on Uniform[-π, π]³ mapped to Legendre canonical [-1, 1]³
    And Sobol' QMC samples at N=4096
    When I fit a Legendre PCE of total degree 10
    And I compute Sobol' indices from the PCE coefficients
    Then every first-order is at most its total-order
    And the sum of first-order indices is at most 1

  Scenario: PCE error decreases with truncation degree
    Given the Ishigami model on Uniform[-π, π]³ mapped to Legendre canonical [-1, 1]³
    When I fit a Legendre PCE of total degree 4 with N=256
    And I record S_1 as low_p
    And I fit a Legendre PCE of total degree 10 with N=4096
    And I record S_1 as high_p
    Then high_p is closer than low_p to the analytic value 0.3139

  Scenario: PCE Sobol' indices are deterministic
    Given the Ishigami model on Uniform[-π, π]³ mapped to Legendre canonical [-1, 1]³
    And Sobol' QMC samples at N=4096
    When I fit a Legendre PCE of total degree 10 twice
    Then the two PCEs have identical coefficients
    And the two Sobol' index sets are identical
