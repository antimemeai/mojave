Feature: Mixture SPRT and always-valid p-values
  Johari 2022: mSPRT with Gaussian mixing distribution.

  Scenario: Always-valid p-value starts at 1 with no data
    Given mSPRT config with theta_0 = 0.0 and mixing_variance = 1.0
    When I have observed 0 data points
    Then the always-valid p-value is 1.0

  Scenario: p-value decreases with evidence against H0
    Given mSPRT config with theta_0 = 0.0 and mixing_variance = 1.0
    When I observe 10 values all equal to 2.0
    Then the always-valid p-value is less than 0.05

  Scenario: p-value stays high under H0
    Given mSPRT config with theta_0 = 0.0 and mixing_variance = 1.0
    When I observe 10 values all equal to 0.0
    Then the always-valid p-value is greater than 0.5
