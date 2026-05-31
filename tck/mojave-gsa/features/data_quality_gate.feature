Feature: Sobol data quality gate

  Reject cells with n_samples=0 before Sobol estimation to prevent
  corrupted variance decomposition from empty evaluation cells.

  Scenario: cells with n_samples=0 are rejected before analysis
    Given a results file with 3 cells having n_samples=0
    When I run Sobol analysis
    Then the analysis fails with error mentioning "n_samples=0"
    And the error message lists the affected cell indices

  Scenario: cells with n_samples>0 pass the gate
    Given a results file where all cells have n_samples>=1
    When I run Sobol analysis
    Then the analysis succeeds

  Scenario: cells without n_samples field pass the gate
    Given a results file where n_samples is absent from all cells
    When I run Sobol analysis
    Then the analysis succeeds (backward compatibility)
