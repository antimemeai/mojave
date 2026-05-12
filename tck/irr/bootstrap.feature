Feature: Bootstrap confidence intervals
  Percentile bootstrap CIs for inter-rater reliability statistics.
  Item-level resampling with replacement, deterministic via seed.
  Reference: Efron & Tibshirani (1993), Chapter 13.

  # Gate 3: Property — CI endpoints ordered
  Scenario: CI endpoints are ordered
    Given a rating matrix with mixed agreement on 20 items and 3 raters seeded at 42
    When I bootstrap Krippendorff alpha with 500 resamples at 95% confidence seeded at 1
    Then the CI lower bound is at most the upper bound

  # Gate 3: Property — perfect agreement
  Scenario: Perfect agreement produces tight CI at 1.0
    Given a rating matrix where 3 raters agree perfectly on 10 items across 3 categories
    When I bootstrap Krippendorff alpha with 500 resamples at 95% confidence seeded at 1
    Then the CI lower bound is greater than 0.99
    And the CI upper bound is at most 1.0

  # Gate 3: Property — contains point estimate
  Scenario: Point estimate falls within CI
    Given a rating matrix with mixed agreement on 30 items and 4 raters seeded at 55
    When I bootstrap Krippendorff alpha with 1000 resamples at 95% confidence seeded at 1
    Then the Krippendorff alpha point estimate falls within the CI

  # Gate 3: Property — monotone in confidence level
  Scenario: Higher confidence level produces wider CI
    Given a rating matrix with mixed agreement on 20 items and 3 raters seeded at 42
    When I compare bootstrap CIs at 90% and 99% confidence with 500 resamples seeded at 1
    Then the 99% CI width is at least the 90% CI width

  # Gate 3: Property — reproducibility
  Scenario: Same seed produces identical CI
    Given a rating matrix with mixed agreement on 20 items and 3 raters seeded at 42
    When I bootstrap Krippendorff alpha with 500 resamples at 95% confidence seeded at 1 twice
    Then both runs produce identical CIs

  # Edge cases
  Scenario: Empty matrix is an error
    Given an empty rating matrix for bootstrap
    When I attempt bootstrap CI computation
    Then I get a bootstrap error about empty data

  Scenario: Single item is degenerate for Krippendorff alpha
    Given a rating matrix with 1 item rated by 3 raters
    When I attempt bootstrap CI computation
    Then I get a bootstrap error about statistic failure

  Scenario: Invalid confidence level is an error
    Given a rating matrix with mixed agreement on 20 items and 3 raters seeded at 42
    When I attempt bootstrap with confidence 0.0
    Then I get a bootstrap error about invalid confidence

  Scenario: Confidence level above 1.0 is an error
    Given a rating matrix with mixed agreement on 20 items and 3 raters seeded at 42
    When I attempt bootstrap with confidence 1.5
    Then I get a bootstrap error about invalid confidence

  Scenario: Zero resamples is an error
    Given a rating matrix with mixed agreement on 20 items and 3 raters seeded at 42
    When I attempt bootstrap with 0 resamples
    Then I get a bootstrap error about invalid resamples
