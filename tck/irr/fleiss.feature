Feature: Fleiss kappa
  Multi-rater nominal agreement beyond chance.
  Reference: Fleiss (1971) "Measuring nominal scale agreement among many raters"

  # Gate 1: Textbook reproduction
  Scenario: Fleiss 1971 Table 1 (Gwet reproduction)
    Given the Fleiss golden dataset "fleiss_1971.json"
    When I compute Fleiss kappa
    Then kappa is approximately 0.2099 with tolerance 0.001

  # Gate 3: Property — perfect agreement
  Scenario: Perfect agreement yields kappa = 1.0
    Given a Fleiss matrix where all 4 raters agree on each of 20 items across 3 categories
    When I compute Fleiss kappa
    Then kappa is approximately 1.0 with tolerance 0.001

  # Gate 3: Property — random agreement
  Scenario: Random ratings yield kappa near 0
    Given a Fleiss matrix with 50 items 5 raters 3 categories seeded at 42
    When I compute Fleiss kappa
    Then kappa is between -0.15 and 0.15

  # Gate 3: Property — permutation invariance
  Scenario: Kappa is invariant under rater permutation
    Given a Fleiss matrix with 20 items 4 raters 3 categories seeded at 99
    When I compute Fleiss kappa
    And I permute the rater columns and compute Fleiss kappa again
    Then both Fleiss kappa values are identical

  # Edge cases
  Scenario: Empty data is an error
    Given an empty Fleiss matrix
    When I compute Fleiss kappa
    Then I get a Fleiss error about empty data

  Scenario: Missing values are rejected
    Given a Fleiss matrix with missing values
    When I compute Fleiss kappa
    Then I get a Fleiss error about missing data

  Scenario: Single rater is an error
    Given a Fleiss matrix with 1 rater
    When I compute Fleiss kappa
    Then I get a Fleiss error about insufficient raters

  Scenario: All-same-category is degenerate
    Given a Fleiss matrix where all raters assign the same category
    When I compute Fleiss kappa
    Then I get a Fleiss error about degenerate data
