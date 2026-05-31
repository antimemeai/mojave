Feature: Betting confidence sequence for [0,1]-bounded data (Waudby-Smith & Ramdas 2024)
  Hedged capital confidence sequence using predictable plug-in bet sizing.
  Provides anytime-valid CIs for bounded observations without distributional assumptions.

  # Gate 3: Property tests
  Scenario: betting CS contains true mean
    Given BettingMonitor with alpha=0.05 and grid_size=500
    When I feed 200 observations from Bernoulli(0.3)
    Then the confidence interval contains 0.3

  Scenario: betting CS narrows with more observations
    Given BettingMonitor with alpha=0.05
    When I feed 50 observations then record width_50
    And I feed 150 more observations then record width_200
    Then width_200 < width_50

  Scenario: betting CS is tighter than sigma=0.5 conservative bound
    Given BettingMonitor and AnytimeMonitor(Bernoulli) both with alpha=0.05
    When I feed 200 identical Bernoulli(0.5) observations to both
    Then BettingMonitor CI width < AnytimeMonitor CI width

  # Gate 4: Monte Carlo calibration
  Scenario: betting CS achieves 95% coverage across p values
    Given 10000 replications at each p in {0.1, 0.3, 0.5, 0.7, 0.9}
    When I run BettingMonitor with alpha=0.05 and N=200
    Then coverage >= 93% at every p value

  # Input validation
  Scenario: rejects alpha outside (0,1)
    When I create BettingMonitor with alpha=0.0
    Then I receive an InvalidAlpha error

  Scenario: rejects grid_size < 10
    When I create BettingMonitor with grid_size=5
    Then I receive an InvalidGridSize error

  Scenario: rejects observation outside [0,1]
    Given BettingMonitor with alpha=0.05
    When I feed observation 1.5
    Then I receive an InvalidBettingObservation error
