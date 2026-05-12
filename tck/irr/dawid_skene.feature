Feature: Dawid-Skene latent-class agreement model
  EM algorithm jointly estimating latent truth and per-annotator confusion matrices.
  Reference: Dawid & Skene (1979); Paun et al. (2018) Section 2.2.

  # Gate 1: Textbook — Dawid & Skene 1979 Table 1
  Scenario: Reproduces Dawid-Skene 1979 paper results
    Given the Dawid-Skene 1979 golden dataset
    When I fit Dawid-Skene with max 100 EM iterations
    Then the model converged
    And the estimated labels match the 1979 paper Table 4
    And the class priors match the 1979 paper Table 2

  # Gate 3: Property — perfect annotators
  Scenario: Perfect annotators yield identity confusion matrices
    Given 3 annotators who all agree perfectly on 20 items with 3 classes
    When I fit Dawid-Skene with max 100 EM iterations
    Then the model converged
    And all confusion matrices are approximately identity
    And the estimated labels match the input labels

  # Gate 3: Property — bad annotator detection
  Scenario: One bad annotator is detected
    Given 3 annotators on 50 items with 2 classes
    And annotator 0 and 1 are perfect
    And annotator 2 flips labels 30% of the time seeded at 42
    When I fit Dawid-Skene with max 100 EM iterations
    Then the model converged
    And annotator 2 has off-diagonal mass > 0.2
    And annotator 0 and 1 have off-diagonal mass < 0.05
    And the estimated labels mostly match the true labels

  # Gate 3: Property — handles missing data
  Scenario: Handles missing data gracefully
    Given 3 annotators on 30 items with 3 classes
    And 20% of annotations are missing at random seeded at 7
    When I fit Dawid-Skene with max 100 EM iterations
    Then the model converged
    And the estimated labels have > 80% accuracy vs true labels

  # Gate 3: Property — class priors reflect data
  Scenario: Class priors converge to empirical distribution
    Given 3 perfect annotators on 60 items with 3 classes evenly distributed
    When I fit Dawid-Skene with max 100 EM iterations
    Then the model converged
    And each class prior is approximately 0.333 with tolerance 0.05

  # Edge cases
  Scenario: Empty data is an error
    Given no annotation triples
    When I attempt Dawid-Skene fitting
    Then I get a Dawid-Skene error about empty data

  Scenario: Insufficient iterations reports non-convergence
    Given 3 annotators on 50 items with 2 classes
    And annotator 0 and 1 are perfect
    And annotator 2 flips labels 30% of the time seeded at 42
    When I fit Dawid-Skene with max 1 EM iterations
    Then the model did not converge

  Scenario: Single class collapses gracefully
    Given 2 annotators on 10 items all labeled class 0
    When I fit Dawid-Skene with max 100 EM iterations
    Then all estimated labels are class 0
