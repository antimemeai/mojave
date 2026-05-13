Feature: Confidence sequences (Howard 2021)
  Time-uniform confidence intervals valid at any stopping time.

  Scenario: CS width decreases with more observations
    Given a normal-mixture CS at alpha = 0.05
    When I compute CS at n = 100 with mean 0.0 and variance 1.0
    And I compute CS at n = 1000 with mean 0.0 and variance 1.0
    Then the width at n = 1000 is less than the width at n = 100

  Scenario: CS contains true mean for centered data
    Given a normal-mixture CS at alpha = 0.05
    When I compute CS for 50 observations from N(0, 1)
    Then the interval contains 0.0
