Feature: Page 1954 tabular CUSUM chart

  Scenario: In-control observations keep CUSUM near zero
    Given a CUSUM chart with mu_0=0 sigma=1 k=0.5 h=5
    When I observe values 0.1, -0.2, 0.3, -0.1, 0.0
    Then C+ and C- are both less than 1.0

  Scenario: Sustained positive shift triggers upper CUSUM
    Given a CUSUM chart with mu_0=0 sigma=1 k=0.5 h=5
    When I observe 20 values drawn from N(1, 1)
    Then OutOfControl is signaled before observation 20

  Scenario: CUSUM C+ and C- are always non-negative
    Given a CUSUM chart with any parameters
    When I observe 1000 random values
    Then c_plus >= 0 and c_minus >= 0 after every observation

  Scenario: Reset restores initial state
    Given a CUSUM chart with mu_0=0 sigma=1 k=0.5 h=5
    When I observe 5 values then reset
    Then c_plus == 0 and c_minus == 0 and n_observations == 0
