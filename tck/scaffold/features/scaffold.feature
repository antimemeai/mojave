Feature: TCK harness scaffold
  Proves the Gherkin parser + SyncRunner wire end-to-end.
  No real behavior under test — just accumulator mechanics.

  Scenario: Record two events
    Given a fresh accumulator
    When I record the event "alpha"
    And I record the event "bravo"
    Then the accumulator holds "alpha, bravo"

  Scenario: Empty accumulator
    Given a fresh accumulator
    Then the accumulator holds ""

  Scenario Outline: Parameterized recording
    Given a fresh accumulator
    When I record the event "<first>"
    And I record the event "<second>"
    Then the accumulator holds "<expected>"

    Examples:
      | first | second | expected |
      | foo   | bar    | foo, bar |
      | x     | y      | x, y     |
      | one   | two    | one, two |
