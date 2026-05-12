# RBD-FAST estimator on Ishigami — load-bearing scientific claim
# for `saltelli_estimators::estimate_rbd_fast` with Plischke 2010
# bias correction.
#
# Per `decisions/2026-04-29-saltelli-rbd-fast.md`. Ishigami at
# canonical `(a=7, b=0.1)` has closed-form first-order Sobol'
# indices (Saltelli Primer 2008 Eq 5.16-5.18):
#   S = [0.314, 0.442, 0.000]
#
# RBD-FAST has a small bias floor on Ishigami that decays with N.
# Tolerance regime sized to ~3× realized max error at N=4096.
#
# Mechanized: `crates/saltelli-estimators/tests/rbd_fast_tck.rs`.

Feature: estimate_rbd_fast — Ishigami at d=3, M=10

  Scenario: estimator recovers analytic first-order within bias floor
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096 with harmonic M=10
    When I estimate RBD-FAST first-order indices
    Then S approximates 0.314 0.442 0.000 within 0.06

  Scenario: indices stay within unit interval (with Plischke slack)
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096 with harmonic M=10
    When I estimate RBD-FAST first-order indices
    Then every S is within negative 0.05 to 1.05

  Scenario: factor ranking by S is exactly correct
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096 with harmonic M=10
    When I estimate RBD-FAST first-order indices
    Then S_2 strictly exceeds S_1
    And S_1 strictly exceeds S_3
