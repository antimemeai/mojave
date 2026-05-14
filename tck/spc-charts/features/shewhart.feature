Feature: Shewhart individuals control chart

  Scenario: In-control observations produce no signal
    Given a Shewhart chart with mu_0=50 sigma=2 k=3
    When I observe values 49, 50, 51, 48, 52, 50
    Then all signals are InControl

  Scenario: 3-sigma violation signals OutOfControl
    Given a Shewhart chart with mu_0=50 sigma=2 k=3
    When I observe value 57
    Then the signal is OutOfControl

  Scenario: WE-2 rule detects 2 of 3 beyond 2-sigma
    Given a Shewhart chart with mu_0=50 sigma=2 k=3 rules=[WE1,WE2]
    When I observe values 55, 49, 55
    Then the third observation signals OutOfControl

  Scenario: WE-4 rule detects 8 consecutive on one side
    Given a Shewhart chart with mu_0=50 sigma=2 k=3 rules=[WE1,WE4]
    When I observe values 51, 51, 51, 51, 51, 51, 51, 51
    Then the eighth observation signals OutOfControl

  Scenario: Shewhart ARL₀ is approximately 370.4 at k=3
    Given a Shewhart chart with mu_0=0 sigma=1 k=3 rules=[WE1]
    When I simulate 10000 in-control sequences of length 2000
    Then the empirical ARL₀ is within 10% of 370.4
