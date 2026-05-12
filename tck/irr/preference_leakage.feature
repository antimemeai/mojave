Feature: Preference Leakage Score (Li et al. 2025)
  PLS measures judge bias toward related student models in LLM-as-a-judge.
  WR(i,j) = win rate of student i judged by model j.
  PLS(i,j) = [(WR(i,i)-AVG(i,j))/AVG(i,j) + (WR(j,j)-AVG(j,i))/AVG(j,i)] / 2
  AVG(i,j) = [WR(i,i) + WR(i,j)] / 2
  Reference: Li et al. 2025, equations 5-6 (ICLR 2026).

  # Gate 1: Textbook — hand-computed from Li 2025 equations 5-6
  Scenario: PLS for symmetric self-bias matches hand computation
    Given PLS golden example "symmetric_3x3"
    When I compute PLS
    Then each PLS value matches the golden expected value

  Scenario: PLS for asymmetric win rates matches hand computation
    Given PLS golden example "asymmetric_2x2"
    When I compute PLS
    Then each PLS value matches the golden expected value

  Scenario: PLS for negative leakage matches hand computation
    Given PLS golden example "negative_2x2"
    When I compute PLS
    Then each PLS value matches the golden expected value

  Scenario: PLS for same-family pairs matches hand computation
    Given PLS golden example "same_family_3x3"
    When I compute PLS
    Then each PLS value matches the golden expected value

  # Gate 3: Property — zero PLS when no bias
  Scenario: Zero PLS when all win rates are equal
    Given 3 models with uniform win rates of 0.5
    And all models are cross-family
    When I compute PLS
    Then all pairwise PLS values are 0.0

  # Gate 3: Property — regime classification
  Scenario: Pairs are correctly classified by relatedness regime
    Given 4 models in 2 families of 2 with uniform win rates of 0.5
    When I compute PLS
    Then there are 2 SameFamily pairs and 4 CrossFamily pairs

  # Edge cases
  Scenario: Empty win-rate matrix is an error
    Given an empty win-rate matrix
    When I attempt PLS computation
    Then I get a PLS error about empty data

  Scenario: Single model produces no pairs
    Given 1 model with win rate 0.5
    And all models are cross-family
    When I compute PLS
    Then there are 0 pairwise PLS values

  Scenario: Non-square matrix is an error
    Given a non-square win-rate matrix
    When I attempt PLS computation
    Then I get a PLS error about non-square matrix

  Scenario: Invalid win rates are rejected
    Given a win-rate matrix with values outside 0 to 1
    When I attempt PLS computation
    Then I get a PLS error about invalid win rate

  Scenario: Degenerate AVG is an error
    Given a win-rate matrix where AVG equals zero
    When I attempt PLS computation
    Then I get a PLS error about degenerate average
