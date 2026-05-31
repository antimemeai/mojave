Feature: SequentialInstrument DataFamily selection

  SequentialInstrument must derive DataFamily from the outcome type,
  not hardcode Normal(unknown). Binary MCQ outcomes are Bernoulli.

  Scenario: Binary outcomes use DataFamily::Bernoulli
    Given a SequentialInstrument
    And 50 Binary(true) trial records
    When I run the instrument
    Then the monitor uses DataFamily::Bernoulli internally
    And the confidence interval reflects sigma=0.5

  Scenario: Score outcomes use DataFamily::Normal(unknown)
    Given a SequentialInstrument
    And 50 Score(0.75) trial records
    When I run the instrument
    Then the monitor uses DataFamily::Normal(known_variance=None)
