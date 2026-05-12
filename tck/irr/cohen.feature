Feature: Cohen kappa
  Two-rater agreement beyond chance — unweighted and weighted.
  Reference: Cohen (1960) "A coefficient of agreement for nominal scales"

  # Gate 1: Textbook reproduction — unweighted
  Scenario: Cohen 1960 worked example (unweighted)
    Given the Cohen golden dataset "cohen_1960.json"
    When I compute Cohen kappa
    Then Cohen kappa is approximately 0.6528 with tolerance 0.001

  # Gate 1: Textbook reproduction — linear weighted
  Scenario: Cohen 1960 worked example (linear weighted)
    Given the Cohen golden dataset "cohen_1960.json"
    When I compute Cohen weighted kappa with linear weights
    Then Cohen kappa is approximately 0.6649 with tolerance 0.001

  # Gate 1: Textbook reproduction — quadratic weighted
  Scenario: Cohen 1960 worked example (quadratic weighted)
    Given the Cohen golden dataset "cohen_1960.json"
    When I compute Cohen weighted kappa with quadratic weights
    Then Cohen kappa is approximately 0.6759 with tolerance 0.001

  # Gate 2: Cross-check against irrCAC (Gwet 2014)
  Scenario: Gwet 3x3 abstractors (unweighted) matches irrCAC
    Given the Cohen golden dataset "cohen_gwet2014_3x3.json"
    When I compute Cohen kappa
    Then Cohen kappa is approximately 0.796 with tolerance 0.001

  Scenario: Gwet 3x3 abstractors (linear weighted) matches irrCAC
    Given the Cohen golden dataset "cohen_gwet2014_3x3.json"
    When I compute Cohen weighted kappa with linear weights
    Then Cohen kappa is approximately 0.843 with tolerance 0.001

  Scenario: Gwet 3x3 abstractors (quadratic weighted) matches irrCAC
    Given the Cohen golden dataset "cohen_gwet2014_3x3.json"
    When I compute Cohen weighted kappa with quadratic weights
    Then Cohen kappa is approximately 0.892 with tolerance 0.001

  Scenario: Gwet 4x4 diagnosis (unweighted) matches irrCAC
    Given the Cohen golden dataset "cohen_gwet2014_4x4.json"
    When I compute Cohen kappa
    Then Cohen kappa is approximately 0.432 with tolerance 0.001

  Scenario: Gwet 4x4 diagnosis (linear weighted) matches irrCAC
    Given the Cohen golden dataset "cohen_gwet2014_4x4.json"
    When I compute Cohen weighted kappa with linear weights
    Then Cohen kappa is approximately 0.407 with tolerance 0.001

  Scenario: Gwet 4x4 diagnosis (quadratic weighted) matches irrCAC
    Given the Cohen golden dataset "cohen_gwet2014_4x4.json"
    When I compute Cohen weighted kappa with quadratic weights
    Then Cohen kappa is approximately 0.383 with tolerance 0.001

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

  # Gate 3: Property — weighted symmetry
  Scenario: Weighted kappa is symmetric in raters
    Given two raters on a 5-point scale with 30 items seeded at 77
    When I compute Cohen weighted kappa with linear weights
    And I swap the raters and compute Cohen weighted kappa with linear weights again
    Then both Cohen kappa values are identical

  # Edge cases
  Scenario: Empty data is an error
    Given two empty rater vectors
    When I compute Cohen kappa
    Then I get a Cohen error about empty data

  Scenario: Unequal length is an error
    Given rater1 with 10 items and rater2 with 8 items
    When I compute Cohen kappa
    Then I get a Cohen error about unequal length

  Scenario: All-same-category is degenerate
    Given two raters who both assign category 0 to all 20 items
    When I compute Cohen kappa
    Then I get a Cohen error about degenerate data
