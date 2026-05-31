Feature: Bootstrap confidence intervals wired to IRR statistics

  All IRR statistics (Cohen kappa, Fleiss kappa, Krippendorff alpha, Gwet AC)
  support optional bootstrap CIs via _with_ci variants that call bootstrap_ci
  with the statistic as the closure.

  # Gate 3: Property tests — CIs bracket point estimate

  Scenario: Cohen kappa with bootstrap CIs brackets point estimate
    Given a two-rater rating matrix with moderate agreement seeded at 42
    When I compute Cohen kappa with bootstrap CIs using 1000 resamples at 95% confidence seeded at 99
    Then ci_lower is not None
    And ci_upper is not None
    And ci_lower <= kappa <= ci_upper

  Scenario: Fleiss kappa with bootstrap CIs brackets point estimate
    Given a multi-rater rating matrix with 3 raters and moderate agreement seeded at 42
    When I compute Fleiss kappa with bootstrap CIs using 1000 resamples at 95% confidence seeded at 99
    Then ci_lower is not None
    And ci_upper is not None
    And ci_lower <= kappa <= ci_upper

  Scenario: Krippendorff alpha with bootstrap CIs brackets point estimate
    Given a multi-rater rating matrix with 3 raters and moderate agreement seeded at 42
    When I compute Krippendorff alpha with bootstrap CIs using 1000 resamples at 95% confidence seeded at 99
    Then ci_lower is not None
    And ci_upper is not None
    And ci_lower <= kappa <= ci_upper

  Scenario: Gwet AC1 with bootstrap CIs brackets point estimate
    Given a multi-rater rating matrix with 3 raters and moderate agreement seeded at 42
    When I compute Gwet AC1 with bootstrap CIs using 1000 resamples at 95% confidence seeded at 99
    Then ci_lower is not None
    And ci_upper is not None
    And ci_lower <= kappa <= ci_upper

  # Wider CI at higher confidence level

  Scenario: bootstrap CI width increases with confidence level for Cohen kappa
    Given a two-rater rating matrix with moderate agreement seeded at 42
    When I compute Cohen kappa CIs at 90% and 99% confidence with 1000 resamples seeded at 99
    Then the 99% CI is at least as wide as the 90% CI
