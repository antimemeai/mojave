# DGSM estimator on Ishigami — Sobol-Kucherenko 2009 derivative-
# based sensitivity measure with Poincaré-inequality-bounded total-
# order Sobol' indices.
#
# Per `decisions/2026-04-29-saltelli-dgsm.md`. Ishigami at canonical
# `(a=7, b=0.1)` has hand-derivable closed-form ν values:
#   ν_1 ≈ 7.72,  ν_2 = 24.5 (EXACT, = a²/2),  ν_3 ≈ 10.99
#
# Mechanized: `crates/saltelli-estimators/tests/dgsm_tck.rs`.

Feature: estimate_dgsm — Ishigami at d=3

  Scenario: ν recovers closed-form values within MC tolerance
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096 with analytical gradients
    When I estimate DGSM
    Then ν approximates 7.72 24.5 10.99 within 0.2

  Scenario: Poincaré upper bound holds for every factor
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096 with analytical gradients
    When I estimate DGSM
    Then ST analytic is at most ST upper for every factor

  Scenario: central finite-difference matches analytical gradient
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096
    When I estimate DGSM with analytical gradients
    And I estimate DGSM with central finite-difference at eps 1e-5
    Then the two ν vectors agree within 1e-5

  Scenario: factor ranking by ν is exactly correct
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096 with analytical gradients
    When I estimate DGSM
    Then ν_2 strictly exceeds ν_3
    And ν_3 strictly exceeds ν_1
