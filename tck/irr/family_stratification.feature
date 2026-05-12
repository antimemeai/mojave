Feature: Family-stratified Krippendorff alpha
  Judge-family stratified reliability analysis.
  Decomposes overall Krippendorff alpha into within-family and
  between-family components to detect shared-source bias among
  LLM judges from the same model family.

  Bias-burden = mean(within-family alpha) - between-family alpha.
  High bias-burden means judges from the same family agree with each
  other much more than they agree with judges from other families,
  signaling shared bias rather than genuine agreement.

  Between-family alpha is computed by averaging pairwise Krippendorff
  alpha across all cross-family rater pairs.

  # Gate 3: Property — within-family alpha >= between-family alpha when bias exists
  Scenario: Biased families produce positive bias-burden
    Given a 30-item rating matrix with 6 raters
    And raters r0,r1,r2 belong to family "anthropic"
    And raters r3,r4,r5 belong to family "openai"
    And within-family agreement is 0.9 and between-family agreement is 0.4
    And the data is seeded at 42
    When I compute family-stratified alpha with level nominal
    Then within-family alpha for "anthropic" is greater than 0.5
    And within-family alpha for "openai" is greater than 0.5
    And between-family alpha is less than 0.3
    And bias-burden is greater than 0.2

  # Gate 3: Property — no bias when all raters behave identically
  Scenario: Unbiased raters produce near-zero bias-burden
    Given a 100-item rating matrix with 6 raters
    And raters r0,r1,r2 belong to family "anthropic"
    And raters r3,r4,r5 belong to family "openai"
    And all raters agree with probability 0.7 regardless of family
    And the data is seeded at 99
    When I compute family-stratified alpha with level nominal
    Then bias-burden is between -0.15 and 0.15

  # Gate 3: Property — overall alpha is returned
  Scenario: Overall alpha matches unstratified computation
    Given a 20-item rating matrix with 4 raters
    And raters r0,r1 belong to family "A"
    And raters r2,r3 belong to family "B"
    And all raters agree with probability 0.7 regardless of family
    And the data is seeded at 55
    When I compute family-stratified alpha with level nominal
    Then overall alpha matches a direct Krippendorff alpha computation within 0.001

  # Gate 3: Property — 3+ families
  Scenario: Three families with bias detected
    Given a 50-item rating matrix with 6 raters
    And raters r0,r1 belong to family "anthropic"
    And raters r2,r3 belong to family "openai"
    And raters r4,r5 belong to family "google"
    And within-family agreement is 0.9 and between-family agreement is 0.3
    And the data is seeded at 123
    When I compute family-stratified alpha with level nominal
    Then bias-burden is greater than 0.2
    And within-family alpha for "anthropic" is defined
    And within-family alpha for "openai" is defined
    And within-family alpha for "google" is defined

  # Edge case: fewer than 2 families
  Scenario: Single family is an error
    Given a 10-item rating matrix with 3 raters
    And all raters belong to family "anthropic"
    And all raters agree with probability 0.7 regardless of family
    And the data is seeded at 88
    When I attempt family-stratified alpha with level nominal
    Then I get a stratification error about too few families

  # Edge case: family with only 1 rater (but another family has 2+)
  Scenario: Single-rater family is excluded from within-family computation
    Given a 20-item rating matrix with 4 raters
    And raters r0,r1,r2 belong to family "anthropic"
    And rater r3 belongs to family "openai"
    And all raters agree with probability 0.7 regardless of family
    And the data is seeded at 77
    When I compute family-stratified alpha with level nominal
    Then within-family alpha for "anthropic" is defined
    And within-family alpha for "openai" is not defined
    And between-family alpha is defined

  # Edge case: unmapped rater is an error
  Scenario: Rater not in family map is an error
    Given a 10-item rating matrix with 3 raters
    And raters r0,r1 belong to family "anthropic"
    And all raters agree with probability 0.7 regardless of family
    And the data is seeded at 66
    When I attempt family-stratified alpha with level nominal
    Then I get a stratification error about unmapped rater

  # Edge case: empty data
  Scenario: Empty matrix is an error
    Given an empty rating matrix for stratification
    When I attempt family-stratified alpha with level nominal
    Then I get a stratification error about empty data
