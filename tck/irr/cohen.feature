Feature: Cohen kappa
  Two-rater agreement beyond chance — unweighted and weighted.
  Reference: Cohen (1960) "A coefficient of agreement for nominal scales"

  # Gate 3: Property — perfect agreement
  Scenario: Perfect agreement yields kappa = 1.0
    Given two raters who agree perfectly on 20 items across 3 categories
    When I compute Cohen kappa
    Then Cohen kappa is approximately 1.0 with tolerance 0.001

  # Gate 3: Property — random agreement
  Scenario: Random ratings yield kappa near 0
    Given two random raters on 100 items from 3 categories seeded at 42
    When I compute Cohen kappa
    Then Cohen kappa is between -0.15 and 0.15

  # Gate 3: Property — symmetry
  Scenario: Kappa is symmetric in raters
    Given two raters with mixed agreement on 30 items seeded at 55
    When I compute Cohen kappa
    And I swap the raters and compute Cohen kappa again
    Then both Cohen kappa values are identical

  # Weighted kappa
  Scenario: Linear weighted kappa on ordinal data
    Given two raters on a 5-point scale with 30 items seeded at 77
    When I compute Cohen weighted kappa with linear weights
    Then Cohen kappa is a finite number

  Scenario: Quadratic weighted kappa on ordinal data
    Given two raters on a 5-point scale with 30 items seeded at 77
    When I compute Cohen weighted kappa with quadratic weights
    Then Cohen kappa is a finite number

  Scenario: Weighted kappa >= unweighted kappa for ordinal disagreement
    Given two raters on a 5-point scale with 30 items seeded at 77
    When I compute Cohen kappa
    And I compute Cohen weighted kappa with linear weights
    Then weighted kappa is greater than or equal to unweighted kappa

  # Edge cases
  Scenario: Empty data is an error
    Given two empty rater vectors
    When I compute Cohen kappa
    Then I get a Cohen error about empty data

  Scenario: Unequal length is an error
    Given rater1 with 10 items and rater2 with 8 items
    When I compute Cohen kappa
    Then I get a Cohen error about unequal length
