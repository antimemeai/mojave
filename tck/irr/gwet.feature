Feature: Gwet AC1/AC2/AC3
  Chance-corrected agreement coefficient resistant to the kappa paradox.
  AC1 uses identity weights, AC2 uses standard weight schemes,
  AC3 uses arbitrary user-provided weights (do not use without a motivated reason).

  Reference: Gwet (2008, 2014). Kappa paradox: Feinstein & Cicchetti (1990).

  # Gate 1: Textbook — Gwet 2014 Table 4.1
  Scenario: AC1 on Gwet 3-abstractor data
    Given the Gwet 2014 Table 4.1 rating matrix
    When I compute Gwet AC1
    Then the result is 0.84933 within 0.0001

  # Gate 1: Textbook — Gwet 2014 Table 5.7 (same data, quadratic weights)
  Scenario: AC2 quadratic on Gwet 3-abstractor data
    Given the Gwet 2014 Table 4.1 rating matrix
    When I compute Gwet AC2 with quadratic weights
    Then the result is 0.94024 within 0.0001

  # Gate 1: Textbook — Krippendorff 2011 multi-rater data
  Scenario: AC1 on Krippendorff multi-rater data
    Given the Krippendorff 2011 reliability data for Gwet
    When I compute Gwet AC1
    Then the result is 0.77544 within 0.001

  # Gate 3: Property — perfect agreement
  Scenario: Perfect agreement yields AC1 = 1.0
    Given a rating matrix where all raters agree on 20 items across 3 categories
    When I compute Gwet AC1
    Then the result is 1.0 within 0.0001

  # Gate 3: Property — kappa paradox demonstration
  Scenario: AC1 >= Cohen kappa on high-prevalence data
    Given a high-prevalence 2-rater matrix with 90% category 0 seeded at 42
    When I compute Gwet AC1
    And I compute Cohen kappa on the same data
    Then AC1 is greater than or equal to kappa

  # Gate 3: Property — identity weights recover AC1
  Scenario: AC2 with identity weights equals AC1
    Given a mixed-agreement 2-rater matrix seeded at 55
    When I compute Gwet AC1
    And I compute Gwet AC2 with identity weights
    Then AC1 and AC2-identity match within 0.0001

  # Gate 3: Property — category relabeling invariance
  Scenario: Relabeling categories does not change AC1
    Given a mixed-agreement 2-rater matrix seeded at 55
    And the same data relabeled from 0,1,2 to 5,10,15
    When I compute Gwet AC1 on original
    And I compute Gwet AC1 on relabeled
    Then both AC1 values match within 0.0001

  # Edge: empty data
  Scenario: Empty matrix is an error
    Given an empty rating matrix for Gwet
    When I attempt Gwet AC1
    Then I get a Gwet error containing "empty"

  # Edge: single rater
  Scenario: Single rater is an error
    Given a single-rater matrix with 10 items
    When I attempt Gwet AC1
    Then I get a Gwet error containing "2 raters"

  # Edge: degenerate prevalence (pe = 1.0)
  Scenario: All-same-category data is degenerate
    Given a matrix where all 20 items are category 0 by 3 raters
    When I attempt Gwet AC1
    Then I get a Gwet error containing "pe"

  # AC3: custom weights with warning
  Scenario: AC3 with custom weight matrix
    Given a mixed-agreement 2-rater matrix seeded at 55
    And a custom 3x3 weight matrix
    When I compute Gwet AC3 with the custom weights
    Then the result is a finite number between -1 and 1
