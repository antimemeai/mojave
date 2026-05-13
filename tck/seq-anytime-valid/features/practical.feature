Feature: Practical significance (Shim 2025)
  Truncated mSPRT tests for |theta| >= delta, not just theta != 0.

  Scenario: Large effect detected as practically significant
    Given delta = 0.5 and mixing_variance = 1.0 and alpha = 0.05
    When I observe 20 values all equal to 2.0
    Then the practical significance p-value is less than 0.05

  Scenario: Small effect not practically significant
    Given delta = 1.0 and mixing_variance = 1.0 and alpha = 0.05
    When I observe 20 values all equal to 0.1
    Then the practical significance p-value is greater than 0.05

  Scenario: Invalid delta rejected
    When I try to create with delta = -0.5
    Then I get an InvalidPracticalDelta error
