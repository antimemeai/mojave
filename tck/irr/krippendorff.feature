Feature: Krippendorff alpha
  Krippendorff's alpha for inter-rater reliability.
  Metric level must be specified explicitly — no default.
  Reference: Krippendorff (2011) "Computing Krippendorff's alpha-reliability"

  # Gate 1: Textbook reproduction
  Scenario: Reproduce Krippendorff 2011 nominal example
    Given the Krippendorff 2011 nominal dataset
    When I compute alpha with level nominal
    Then alpha is approximately 0.6753 with tolerance 0.001

  # Gate 3: Property — perfect agreement
  Scenario: Perfect agreement yields alpha = 1.0
    Given a rating matrix where all raters agree perfectly on 3 categories
    When I compute alpha with level nominal
    Then alpha is approximately 1.0 with tolerance 0.001

  # Gate 3: Property — chance agreement
  Scenario: Random ratings yield alpha near 0
    Given a 100-item 5-rater matrix with random labels from 3 categories seeded at 42
    When I compute alpha with level nominal
    Then alpha is between -0.15 and 0.15

  # Gate 3: Property — permutation invariance
  Scenario: Alpha is invariant under rater permutation
    Given the Krippendorff 2011 nominal dataset
    When I compute alpha with level nominal
    And I permute the rater columns and compute again
    Then both alpha values are identical

  # Gate 3: Property — item permutation invariance
  Scenario: Alpha is invariant under item permutation
    Given the Krippendorff 2011 nominal dataset
    When I compute alpha with level nominal
    And I permute the item rows and compute again
    Then both alpha values are identical

  # Gate 3: Property — alpha <= 1.0 always
  Scenario: Alpha never exceeds 1.0
    Given a rating matrix where all raters agree perfectly on 3 categories
    When I compute alpha with level nominal
    Then alpha is at most 1.0

  # Gate 2: Cross-check against Python krippendorff 0.8.2
  Scenario: Interval alpha matches Python reference
    Given the Krippendorff 2011 nominal dataset
    When I compute alpha with level interval
    Then alpha is approximately 0.8621 with tolerance 0.001

  Scenario: Ordinal alpha matches Python reference
    Given the Krippendorff 2011 nominal dataset
    When I compute alpha with level ordinal
    Then alpha is approximately 0.8049 with tolerance 0.001

  Scenario: Ratio alpha matches Python reference
    Given the Krippendorff 2011 nominal dataset
    When I compute alpha with level ratio
    Then alpha is approximately 0.7446 with tolerance 0.001

  # Edge cases
  Scenario: Missing metric level is an error
    Given the Krippendorff 2011 nominal dataset
    When I compute alpha without specifying a level
    Then I get an error requiring metric level

  Scenario: Empty data is an error
    Given an empty rating matrix
    When I compute alpha with level nominal
    Then I get an error about empty data

  Scenario: Single item returns degenerate error
    Given a rating matrix with 1 item and 3 raters all rating 2
    When I compute alpha with level nominal
    Then I get an error about degenerate data

  Scenario: All missing except one rater per item is degenerate
    Given a rating matrix where each item has only 1 rater
    When I compute alpha with level nominal
    Then I get an error about degenerate data
