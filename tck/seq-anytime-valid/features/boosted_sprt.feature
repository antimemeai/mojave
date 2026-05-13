Feature: Fischer 2024 boosted SPRT
  Validates the sequential boosting method that avoids overshoot.
  The boosted SPRT truncates likelihood ratio factors via T_alpha
  and applies boost factors b_t >= 1, provably stopping no later
  than the base test.

  Scenario: Truncation function clamps at 1/alpha
    Given alpha = 0.05
    When I apply truncation T_alpha to factor 25.0 with prior mass 0.8
    Then the truncated value is 25.0
    # Because M * x = 0.8 * 25.0 = 20.0 = 1/0.05 = 1/alpha, exactly at boundary

  Scenario: Truncation function caps overshoot
    Given alpha = 0.05
    When I apply truncation T_alpha to factor 30.0 with prior mass 0.8
    Then the truncated value is 25.0
    # Because M * x = 0.8 * 30.0 = 24.0 > 20.0, so T = 1/(M*alpha) = 1/(0.8*0.05) = 25.0

  Scenario: Truncation is identity below threshold
    Given alpha = 0.05
    When I apply truncation T_alpha to factor 10.0 with prior mass 0.8
    Then the truncated value is 10.0
    # Because M * x = 0.8 * 10.0 = 8.0 < 20.0

  Scenario: Boosted SPRT boundaries are at least as tight as conservative
    Given alpha = 0.05 and beta = 0.10
    When I compute boosted boundaries for 10 Bernoulli observations all 1.0
    Then the boosted process value is >= the conservative process value
