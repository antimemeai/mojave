Feature: Bernoulli mSPRT with Beta mixing distribution
  Johari 2022 §3.1: mSPRT for Bernoulli data with Beta(a,b) mixing
  on the alternative hypothesis.

  Scenario: Always-valid p-value is 1.0 with no data
    Given Bernoulli mSPRT with p0 = 0.25 and Beta(1, 1) mixing
    When I have observed 0 data points
    Then the always-valid p-value is 1.0

  Scenario: Strong evidence against H0 gives low p-value
    Given Bernoulli mSPRT with p0 = 0.25 and Beta(1, 1) mixing
    When I observe 50 successes out of 100
    Then the always-valid p-value is less than 0.001

  Scenario: Data consistent with H0 gives high p-value
    Given Bernoulli mSPRT with p0 = 0.25 and Beta(1, 1) mixing
    When I observe 25 successes out of 100
    Then the always-valid p-value is greater than 0.3

  Scenario: All successes gives minimum p-value
    Given Bernoulli mSPRT with p0 = 0.25 and Beta(1, 1) mixing
    When I observe 20 successes out of 20
    Then the always-valid p-value is less than 0.001

  Scenario: All failures gives high p-value when p0 is low
    Given Bernoulli mSPRT with p0 = 0.25 and Beta(1, 1) mixing
    When I observe 0 successes out of 20
    Then the always-valid p-value is greater than 0.5

  Scenario: Agrees with Gaussian approximation for large n
    Given Bernoulli mSPRT with p0 = 0.5 and Beta(1, 1) mixing
    When I observe 550 successes out of 1000
    Then the always-valid p-value is within 0.1 of the Gaussian mSPRT p-value

  Scenario: Invalid p0 rejected
    Given Bernoulli mSPRT with p0 = 0.0 and Beta(1, 1) mixing
    Then construction fails with an error

  Scenario: Invalid Beta parameters rejected
    Given Bernoulli mSPRT with p0 = 0.25 and Beta(0, 1) mixing
    Then construction fails with an error
