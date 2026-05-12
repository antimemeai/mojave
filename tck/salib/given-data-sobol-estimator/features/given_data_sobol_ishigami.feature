# Given-data Sobol' (Plischke-Borgonovo-Smith 2013) on Ishigami.
#
# Per `decisions/2026-04-29-saltelli-given-data-sobol.md`. Direct
# variance-decomposition first-order Sobol' from generic (X, Y).
# Ishigami at canonical (a=7, b=0.1) has closed-form S_1 =
#   [0.314, 0.442, 0.000]
#
# Mechanized: `crates/saltelli-estimators/tests/given_data_sobol_tck.rs`.

Feature: estimate_given_data_sobol — Ishigami at d=3

  Scenario: estimator recovers analytic first-order indices
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096
    When I estimate given-data Sobol indices
    Then S_1 approximates 0.314 0.442 0.000 within 0.03

  Scenario: indices stay within unit interval
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096
    When I estimate given-data Sobol indices
    Then every S_1 is in 0 to 1

  Scenario: factor ranking is exactly correct
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096
    When I estimate given-data Sobol indices
    Then S_1 for factor 1 strictly exceeds S_1 for factor 0
    And S_1 for factor 0 strictly exceeds S_1 for factor 2
