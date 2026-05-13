Feature: Wald SPRT boundaries and decisions
  Validates Wald 1945 sequential probability ratio test boundaries
  for both approximate and conservative variants.

  # Gate 1: Wald 1945 textbook values
  # Binomial: p0=0.1, p1=0.2, alpha=0.05, beta=0.10
  # Approximate: A = (1-beta)/alpha = 18.0, B = beta/(1-alpha) = 0.1053
  # Conservative: A = 1/alpha = 20.0, B = beta = 0.10
  # Log-space: ln(A_approx) = 2.8904, ln(B_approx) = -2.2513

  Scenario: Approximate SPRT boundaries for binomial
    Given alpha = 0.05 and beta = 0.10
    When I compute approximate SPRT boundaries
    Then the upper boundary A is approximately 18.0
    And the lower boundary B is approximately 0.10526

  Scenario: Conservative SPRT boundaries for binomial
    Given alpha = 0.05 and beta = 0.10
    When I compute conservative SPRT boundaries
    Then the upper boundary A is approximately 20.0
    And the lower boundary B is approximately 0.10

  Scenario: Log-space approximate boundaries
    Given alpha = 0.05 and beta = 0.10
    When I compute log-space approximate boundaries
    Then ln(A) is approximately 2.8904
    And ln(B) is approximately -2.2513

  Scenario: Degenerate hypotheses rejected
    Given alpha = 0.05 and beta = 0.10
    When I try to create SPRT with theta_0 = 0.5 and theta_1 = 0.5
    Then I get a DegenerateHypotheses error

  Scenario: Invalid alpha rejected
    When I try to create SPRT with alpha = 0.0
    Then I get an InvalidAlpha error

  Scenario: Invalid beta rejected
    When I try to create SPRT with beta = 1.0
    Then I get an InvalidBeta error

  Scenario: Alpha plus beta too large rejected
    When I try to create SPRT with alpha = 0.6 and beta = 0.5
    Then I get an AlphaBetaSum error

  Scenario Outline: Approximate boundaries at various error levels
    Given alpha = <alpha> and beta = <beta>
    When I compute approximate SPRT boundaries
    Then the upper boundary A is approximately <A>
    And the lower boundary B is approximately <B>

    Examples:
      | alpha | beta | A      | B       |
      | 0.01  | 0.01 | 99.0   | 0.01010 |
      | 0.05  | 0.05 | 19.0   | 0.05263 |
      | 0.10  | 0.10 | 9.0    | 0.11111 |
      | 0.05  | 0.20 | 16.0   | 0.21053 |

  Scenario: SPRT rejects with strong Bernoulli evidence for H1
    Given SPRT config with p0 = 0.3 and p1 = 0.7 and alpha = 0.05 and beta = 0.10
    When I feed 20 observations all equal to 1.0
    Then the decision is Reject

  Scenario: SPRT accepts with strong Bernoulli evidence for H0
    Given SPRT config with p0 = 0.3 and p1 = 0.7 and alpha = 0.05 and beta = 0.10
    When I feed 20 observations all equal to 0.0
    Then the decision is Accept
