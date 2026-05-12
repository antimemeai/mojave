# Closed-form invariants for the Sobol' G function and its analytic
# Sobol' indices.
#
# Per Saltelli-Sobol 1995. Each factor `g_i(x_i; a_i) = (|4 x_i - 2|
# + a_i) / (1 + a_i)` has E[g_i] = 1 and Var[g_i] = 1/(3·(1 + a_i)²).
# By independence, the total variance is the product form
# `D = Π (1 + V_i) - 1`. First-order Sobol' indices fall out as
# `S_i = V_i / D`.
#
# Total-order indices have a recursive product form (Saltelli-Sobol
# 1995, Eqs 22-24); not yet derived. PR 4 ships first-order only;
# total-order is sentinel `NaN` per
# `decisions/2026-04-28-saltelli-validation-pattern.md`.
#
# Mechanized: `crates/saltelli-validation/tests/sobol_g_tck.rs`.

Feature: Sobol' G — closed-form first-order Sobol' indices

  Scenario: V_i closed form is (1/3) / (1 + a_i)²
    Given Sobol' G analytic indices with a vector [0, 1, 9]
    Then for every factor i, V_i (recovered from S_i and D) is approximately (1/3) / (1 + a_i)² within 1e-9

  Scenario: total variance is the product form
    Given Sobol' G analytic indices with a vector [0, 1, 9]
    Then D equals (1 + 1/3)(1 + 1/12)(1 + 1/300) - 1 within 1e-12

  Scenario: smaller a_i means larger first-order index
    Given Sobol' G analytic indices with a vector [0, 1, 9, 99]
    Then the factors are ranked S_1 > S_2 > S_3 > S_4

  Scenario: first-order indices are positive and sum to at most 1
    Given Sobol' G analytic indices with a vector [0, 1, 4.5, 9, 99]
    Then every first-order index is positive
    And the sum of first-order indices is at most 1

  Scenario: high a_i contributes negligibly
    Given Sobol' G analytic indices with a vector [0, 99]
    Then S_2 is below 0.01

  Scenario: total-order is NaN sentinel until derived
    Given Sobol' G analytic indices with a vector [1, 2]
    Then every total-order index is NaN

  Scenario: canonical screening case ranks factors as expected
    Given Sobol' G analytic indices with a vector [0, 1, 4.5, 9, 99, 99, 99, 99]
    Then the first four factors strictly dominate the last four
    And every last-four factor first-order index is below 1e-3

  Scenario: input distribution has the requested dimensionality
    Given the Sobol' G input distribution at dim 5
    Then it has 5 factors named x1 through x5
    And every factor is Uniform(0, 1)
