Feature: Bland-Altman limits of agreement
  Assesses agreement between two measurement methods via mean difference
  and 95% limits of agreement (mean ± 1.96 * SD).

  Reference: Bland & Altman (1986), The Lancet.

  # Gate 1: Textbook — Bland & Altman 1986 PEFR data
  Scenario: PEFR Wright vs mini Wright meter
    Given the Bland-Altman 1986 PEFR data
    When I compute Bland-Altman agreement
    Then mean difference is -2.12 within 0.5
    And SD of differences is 38.77 within 0.5
    And lower LoA is approximately -78.10 within 1.5
    And upper LoA is approximately 73.87 within 1.5

  # Gate 3: Property — constant offset is zero-variance error
  Scenario: Constant offset yields zero-variance error
    Given measurements x = [1.0, 2.0, 3.0, 4.0, 5.0]
    And measurements y = [2.0, 3.0, 4.0, 5.0, 6.0]
    When I attempt Bland-Altman agreement
    Then I get a Bland-Altman error containing "zero variance"

  # Gate 3: Property — sign reversal
  Scenario: Swapping x and y negates mean diff
    Given measurements x = [10.0, 20.0, 30.0, 40.0, 50.0]
    And measurements y = [12.0, 18.0, 33.0, 37.0, 55.0]
    When I compute Bland-Altman agreement for x and y
    And I compute Bland-Altman agreement for y and x
    Then the mean differences are negations within 0.0001
    And the SD values are equal within 0.0001

  # Edge: length mismatch
  Scenario: Mismatched input lengths
    Given measurements x with 5 values and y with 3 values
    When I attempt Bland-Altman agreement
    Then I get a Bland-Altman error containing "equal length"

  # Edge: too few observations
  Scenario: Single observation pair
    Given measurements x = [1.0] and y = [2.0]
    When I attempt Bland-Altman agreement
    Then I get a Bland-Altman error containing "2 paired"

  # Edge: identical inputs
  Scenario: Identical measurements yield zero-variance error
    Given measurements x = [1.0, 2.0, 3.0]
    And measurements y = [1.0, 2.0, 3.0]
    When I attempt Bland-Altman agreement
    Then I get a Bland-Altman error containing "zero variance"
