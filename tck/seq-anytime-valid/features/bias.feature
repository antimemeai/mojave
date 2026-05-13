Feature: Bias-adjusted estimation at stopping time (Siegmund 1985)
  The MLE at stopping time overestimates effect size.

  Scenario: Bias correction reduces estimate magnitude
    Given a normal-mean SPRT that stopped at n = 20
    And the MLE at stopping is 0.8
    And the SPRT config is mu0 = 0.0, mu1 = 0.5, alpha = 0.05
    When I compute the bias-corrected estimate
    Then the corrected estimate is less than 0.8 in absolute value

  Scenario: Median-unbiased estimate is less extreme than MLE
    Given a normal-mean SPRT that stopped at n = 20
    And the MLE at stopping is 0.8
    When I compute the median-unbiased estimate
    Then the estimate is between 0.0 and 0.8
