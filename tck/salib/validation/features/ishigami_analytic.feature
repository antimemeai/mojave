# Closed-form invariants for the Ishigami function and its analytic
# Sobol' indices.
#
# The Ishigami canary: X_3 has zero first-order Sobol' index but
# nonzero total-order. Any estimator that returns a nonzero S_3 (in
# the limit of N → ∞, modulo MC noise) has a bug in its first-order
# computation. Any estimator that returns S_T3 close to S_3 has a bug
# in its total-order computation. Ishigami's whole reason for being
# in the canonical battery is to catch these two bug classes.
#
# Provenance: Ishigami-Homma 1990; Saltelli et al. 2008 *Global
# Sensitivity Analysis: The Primer*, §5.4 Eq 5.16-5.18. The closed
# forms are the values estimator PRs converge to under the
# reviewer-affordance contract (per
# `decisions/2026-04-28-saltelli-tck-posture.md`).
#
# Mechanized: `crates/saltelli-validation/tests/ishigami_tck.rs`.

Feature: Ishigami — closed-form Sobol' indices

  Scenario: X_3 has zero first-order Sobol' index — the canary
    Given Ishigami analytic indices at canonical (a=7, b=0.1)
    Then the X_3 first-order index is exactly 0

  Scenario: X_2 has no interactions — total order equals first order
    Given Ishigami analytic indices at canonical (a=7, b=0.1)
    Then the X_2 total-order index equals the X_2 first-order index

  Scenario: total bounds first per factor
    Given Ishigami analytic indices at canonical (a=7, b=0.1)
    Then for every factor the total-order index is at least the first-order index

  Scenario: first-order indices are non-negative and sum to at most 1
    Given Ishigami analytic indices at canonical (a=7, b=0.1)
    Then every first-order index is non-negative
    And the sum of first-order indices is at most 1

  Scenario: canonical first-order values match Saltelli Primer 2008
    Given Ishigami analytic indices at canonical (a=7, b=0.1)
    Then S_1 is approximately 0.3139 within 5e-4
    And S_2 is approximately 0.4424 within 5e-4
    And S_3 is exactly 0

  Scenario: canonical total-order values match Saltelli Primer 2008
    Given Ishigami analytic indices at canonical (a=7, b=0.1)
    Then S_T1 is approximately 0.5576 within 5e-4
    And S_T2 is approximately 0.4424 within 5e-4
    And S_T3 is approximately 0.2436 within 5e-4

  Scenario: zero a collapses X_2 contribution
    Given Ishigami analytic indices at (a=0, b=0.1)
    Then the X_2 first-order index is exactly 0

  Scenario: zero b removes the X_1 X_3 interaction
    Given Ishigami analytic indices at (a=7, b=0)
    Then the X_3 total-order index is exactly 0

  Scenario: zero a and zero b leave only the X_1 sin term
    Given Ishigami analytic indices at (a=0, b=0)
    Then S_1 is approximately 1 within 1e-12
    And S_T1 is approximately 1 within 1e-12

  Scenario: total variance is positive at canonical params
    Given Ishigami analytic indices at canonical (a=7, b=0.1)
    Then the total variance is positive

  Scenario: input distribution has three Uniform(-π, π) factors
    Given the Ishigami input distribution
    Then it has three factors named x1 x2 x3
    And every factor is Uniform(-π, π)
