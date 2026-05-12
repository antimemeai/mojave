# Borgonovo δ estimator on Ishigami — load-bearing scientific
# claim for `saltelli_estimators::estimate_borgonovo_delta`.
#
# Per `decisions/2026-04-29-saltelli-borgonovo-delta.md`. Ishigami
# at canonical `(a=7, b=0.1)` has approximate analytic δ values
# from Plischke-Borgonovo-Smith 2013:
#   δ ≈ [0.214, 0.371, 0.157]
#
# Mechanized: `crates/saltelli-estimators/tests/borgonovo_tck.rs`.

Feature: estimate_borgonovo_delta — Ishigami at d=3

  Scenario: estimator recovers analytic δ within KDE bias floor
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096
    When I estimate Borgonovo delta
    Then δ approximates 0.214 0.371 0.157 within 0.06

  Scenario: indices stay within unit interval (with KDE slack)
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096
    When I estimate Borgonovo delta
    Then every δ is within negative 0.05 to 1.05

  Scenario: factor ranking by δ is exactly correct
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096
    When I estimate Borgonovo delta
    Then δ_2 strictly exceeds δ_1
    And δ_1 strictly exceeds δ_3
