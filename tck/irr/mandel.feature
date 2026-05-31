Feature: Mandel h/k consistency statistics

  ISO 5725-2 Mandel statistics for interlaboratory outlier detection.
  h = between-configuration consistency statistic.
  k = within-configuration consistency statistic.

  Scenario: Mandel h identifies between-configuration outlier
    Given an interlaboratory dataset with 5 labs and one outlier lab
    When I compute Mandel h statistics at alpha 0.01
    Then the outlier lab has absolute h exceeding the critical value
    And non-outlier labs have absolute h below the critical value

  Scenario: Mandel k identifies within-configuration inconsistency
    Given an interlaboratory dataset with 5 labs and one high-variability lab
    When I compute Mandel k statistics at alpha 0.01
    Then the high-variability lab has k exceeding the critical value
    And other labs have k below the critical value

  Scenario: Mandel h and k for consistent dataset
    Given an interlaboratory dataset with 5 consistent labs
    When I compute Mandel h and k statistics at alpha 0.05
    Then no labs are flagged as h outliers
    And no labs are flagged as k outliers
