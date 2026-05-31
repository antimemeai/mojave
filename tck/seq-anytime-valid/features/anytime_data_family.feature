Feature: AnytimeMonitor DataFamily dispatch

  AnytimeMonitor must dispatch on DataFamily to select the correct sigma
  for the confidence sequence. Using Welford's estimated sigma for
  Bernoulli data voids the anytime-valid guarantee.

  Scenario: Bernoulli family uses sigma=0.5 not estimated sigma
    Given an AnytimeMonitor configured with DataFamily::Bernoulli and alpha=0.05
    When I feed 100 observations drawn from Bernoulli(0.5)
    Then the confidence interval width uses sigma=0.5
    And the width does not depend on the observed sample variance

  Scenario: Normal family with known variance uses that variance
    Given an AnytimeMonitor configured with DataFamily::Normal(known_variance=1.0) and alpha=0.05
    When I feed 100 observations drawn from N(0,1)
    Then the confidence interval width uses sigma=1.0

  Scenario: Normal family without known variance uses Welford estimate
    Given an AnytimeMonitor configured with DataFamily::Normal(known_variance=None) and alpha=0.05
    When I feed 100 observations drawn from N(0,1)
    Then the confidence interval width uses the running Welford estimate
