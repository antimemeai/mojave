# FAST/eFAST estimator on Ishigami — load-bearing scientific
# claim for `saltelli_estimators::estimate_fast`.
#
# Per `decisions/2026-04-29-saltelli-fast-estimator.md`. Ishigami
# at canonical `(a=7, b=0.1)` has closed-form first-order and
# total-order Sobol' indices (Saltelli Primer 2008 Eq 5.16-5.18):
#   S  = [0.314, 0.442, 0.000]
#   ST = [0.558, 0.442, 0.244]
#
# eFAST has a known systematic bias on Ishigami driven by the
# `sin(x_1) · x_3⁴` interaction term aliasing into the spectral
# bins of `ω_1` and `ω_3`. SALib exhibits the same bias. Tolerance
# regime is "FAST-bias-aware analytic recovery" + tight SALib
# differential.
#
# Mechanized: `crates/saltelli-estimators/tests/fast_tck.rs`.

Feature: estimate_fast — Ishigami at d=3, M=4

  Scenario: estimator recovers analytic indices within FAST's bias bound
    Given the Ishigami model on Uniform[-π, π]³
    And a FAST design with N=1025 and harmonic M=4
    When I estimate FAST first-order and total-order indices
    Then S approximates 0.314 0.442 0.000 within 0.05
    And ST approximates 0.558 0.442 0.244 within 0.10

  Scenario: total-order is at least first-order on Ishigami
    Given the Ishigami model on Uniform[-π, π]³
    And a FAST design with N=1025 and harmonic M=4
    When I estimate FAST first-order and total-order indices
    Then ST is at least S for every factor

  Scenario: estimator agrees with SALib within MC noise
    Given the Ishigami model on Uniform[-π, π]³
    And a FAST design with N=1025 and harmonic M=4
    When I estimate FAST first-order and total-order indices
    Then S is within 0.05 of SALib's frozen Ishigami reference
    And ST is within 0.05 of SALib's frozen Ishigami reference
