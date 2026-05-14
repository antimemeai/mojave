Feature: Roberts 1959 EWMA chart

  Scenario: EWMA statistic is weighted average
    Given an EWMA chart with mu_0=0 sigma=1 lambda=0.2 L=3
    When I observe value 1.0
    Then Z = 0.2*1.0 + 0.8*0.0 = 0.2

  Scenario: EWMA detects sustained small shift
    Given an EWMA chart with mu_0=0 sigma=1 lambda=0.2 L=3
    When I observe 100 values from N(0.5, 1)
    Then OutOfControl is signaled

  Scenario: EWMA Z is bounded by observation range
    Given an EWMA chart with any parameters
    When I observe values in [a, b]
    Then Z is always in [min_obs, max_obs] range

  Scenario: Time-varying control limits converge to asymptotic
    Given an EWMA chart with lambda=0.2 L=3
    When I compute UCL at i=1 and i=1000
    Then UCL_1000 is within 0.1% of the asymptotic UCL
