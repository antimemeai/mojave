# Iman-Conover dependent-input correlation transformation
# (Iman & Conover 1982; Mara-Tarantola-Annoni 2015 § 3.2). Converts
# independent marginal samples into correlated samples preserving
# the marginals and inducing the target Spearman rank correlation.
#
# The headline claim: feeding correlated inputs to a sensitivity
# estimator that assumes independence biases the indices; applying
# Iman-Conover before the estimator recovers the correlated-input
# analytic indices.
#
# Per `decisions/2026-04-29-saltelli-iman-conover.md`. Mechanized:
# `crates/saltelli-samplers/tests/iman_conover_tck.rs`.

Feature: iman_conover_transform — dependent-input correlation

  Scenario: marginals are preserved column-wise
    Given N=1024 independent standard-normal samples on d=3
    When I apply Iman-Conover with target ρ_01=0.5, ρ_02=0.3, ρ_12=0.2
    Then each output column is a permutation of the corresponding input column

  Scenario: Pearson correlation matches the target on Gaussian marginals
    Given N=4096 independent standard-normal samples on d=3
    When I apply Iman-Conover with target ρ_01=0.6
    Then the output Pearson correlation between factors 0 and 1 is within 0.05 of 0.6

  Scenario: identity target leaves rank patterns near-independent
    Given N=2000 independent uniform samples on d=3
    When I apply Iman-Conover with the identity correlation matrix
    Then every pairwise output Pearson correlation is below 0.1 in magnitude

  Scenario: dependent-input Sobol' indices recover via IC
    Given N=8192 independent standard-normal samples on d=3
    When I apply Iman-Conover with target ρ_01=0.6
    And I evaluate Y = X_0 + X_1 + X_2 on the transformed samples
    And I estimate first-order Sobol' indices on the (X, Y) data
    Then S_0 approximates 0.610 within 0.10
    And S_1 approximates 0.610 within 0.10
    And S_2 approximates 0.238 within 0.10
    And the sum of first-order Sobol' indices exceeds 1.0

  Scenario: Iman-Conover transformation is deterministic
    Given N=512 independent uniform samples on d=3
    When I apply Iman-Conover twice with the same RngState
    Then the two output matrices are bit-identical
