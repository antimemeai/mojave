Feature: E-values and safe testing (Gruenwald 2024)
  E-values: nonnegative random variables with E_P[E] <= 1 under H0.
  Reject when E >= 1/alpha.

  Scenario: Product of independent e-values
    Given two independent e-values 3.0 and 4.0
    When I compute the product e-value
    Then the result is 12.0

  Scenario: E-value to p-value conversion
    Given an e-value of 25.0
    When I convert to a conservative p-value
    Then the p-value is 0.04

  Scenario: E-value threshold test at alpha = 0.05
    Given an e-value of 25.0 and alpha = 0.05
    When I check the threshold 1/alpha = 20.0
    Then the decision is Reject

  Scenario: E-value below threshold continues
    Given an e-value of 15.0 and alpha = 0.05
    When I check the threshold 1/alpha = 20.0
    Then the decision is Continue
