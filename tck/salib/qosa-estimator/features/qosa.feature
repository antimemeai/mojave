# Quantile-Oriented Sensitivity Analysis (Maume-Deschamps & Niang
# 2018 Prop 3.1) — partition-based estimator on (X, Y) given-data.
#
# The headline claim QOSA makes that variance-based Sobol' cannot:
# different inputs may dominate at different *quantile levels* of Y.
# The tail-vs-median scenario pins this distinguishing property.
#
# Per `decisions/2026-04-29-saltelli-qosa.md`. Mechanized:
# `crates/saltelli-estimators/tests/qosa_tck.rs`.

Feature: estimate_qosa — quantile-oriented sensitivity

  Scenario: Ishigami at α=0.5 orders factors like first-order Sobol'
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096
    When I estimate QOSA at α=0.5
    Then S^α_2 exceeds S^α_1
    And S^α_1 exceeds S^α_3
    And S^α_3 is below 0.2

  Scenario: Independent factor on a Y = X_0 model has QOSA ≈ 0
    Given the Y = X_0 model on Uniform[0, 1]³
    And LHS samples at N=1024
    When I estimate QOSA at α=0.5
    Then S^α_1 is below 0.1
    And S^α_2 is below 0.1
    And S^α_0 exceeds 0.3

  Scenario: Tail-α correctly identifies tail-driver over median-driver
    Given the gated tail model Y = X_0 + 8·X_1·1{X_2 > 0.95}
    And LHS samples at N=4096
    When I estimate QOSA at α=0.5
    And I record S as median_S
    And I estimate QOSA at α=0.95
    And I record S as tail_S
    Then median_S[0] dominates median_S[1] and median_S[2]
    And tail_S[2] exceeds tail_S[0]
    And tail_S[2] exceeds median_S[2]

  Scenario: QOSA is deterministic
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=1024
    When I estimate QOSA at α=0.75 twice
    Then the two index sets are bit-identical
