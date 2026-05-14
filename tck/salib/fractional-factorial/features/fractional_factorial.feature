Feature: Fractional Factorial screening — Plackett-Burman

  Scenario: Linear 3-factor model recovers exact main effects
    Given a 3-factor linear model f(x) = 2*x1 + 3*x2 + 0.5*x3
    And x_i in [-1, +1]
    When I run Plackett-Burman screening
    Then main_effect[0] is within 0.01 of 4.0
    And main_effect[1] is within 0.01 of 6.0
    And main_effect[2] is within 0.01 of 1.0

  Scenario: Nonlinear 3-factor model ranks X2 highest via PB screening
    Given a 3-factor model f(x) = x1^2 + 5*x2 + 0.5*x3 on [-1,+1]^3
    When I run Plackett-Burman screening
    Then the factor with the largest absolute main effect is X2

  Scenario: Main effects from a balanced design sum to near zero for zero-mean Y
    Given a 5-factor model f(x) = x1 - x2
    When I run Plackett-Burman screening
    Then factors 3, 4, 5 have main effects within 0.5 of zero
