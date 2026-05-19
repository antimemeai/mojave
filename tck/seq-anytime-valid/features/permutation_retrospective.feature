Feature: Permutation-based retrospective sequential analysis
  Given already-collected binary outcomes, permute the item order K times
  and run a sequential test on each permutation to estimate a stopping
  time distribution. This is a retrospective counterfactual, not a
  prospective guarantee.

  Scenario: Returns K stopping times
    Given 100 binary outcomes with 50 successes
    And Bernoulli mSPRT with p0 = 0.25 and alpha = 0.05
    When I run 100 permutations with seed 42
    Then I get exactly 100 stopping time results

  Scenario: All permutations stop when signal is overwhelming
    Given 100 binary outcomes with 80 successes
    And Bernoulli mSPRT with p0 = 0.25 and alpha = 0.05
    When I run 100 permutations with seed 42
    Then at least 95 permutations stopped before observing all items

  Scenario: Few permutations stop under null
    Given 100 binary outcomes with 25 successes
    And Bernoulli mSPRT with p0 = 0.25 and alpha = 0.05
    When I run 100 permutations with seed 42
    Then fewer than 10 permutations stopped before observing all items

  Scenario: Deterministic with same seed
    Given 100 binary outcomes with 50 successes
    And Bernoulli mSPRT with p0 = 0.25 and alpha = 0.05
    When I run 50 permutations with seed 42
    And I run 50 permutations with seed 42 again
    Then both runs produce identical stopping times
