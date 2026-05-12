Feature: TrialRecord canonical schema
  The TrialRecord is the foundational data contract.
  Every downstream crate consumes it.

  Scenario: JSON roundtrip with binary outcome
    Given a TrialRecord with binary outcome true
    And agent_id "agent-001" and task_id "task-042"
    And judge_config with model "claude-sonnet-4-6" and family "anthropic"
    When I serialize to JSON
    And deserialize back
    Then the round-tripped record equals the original

  Scenario: JSON roundtrip with score outcome
    Given a TrialRecord with score outcome 0.85
    When I serialize to JSON and deserialize back
    Then the round-tripped record equals the original

  Scenario: JSON roundtrip with graded outcome
    Given a TrialRecord with graded outcome 4
    When I serialize to JSON
    And deserialize back
    Then the round-tripped record equals the original

  Scenario: Outcome variants are distinct
    Given a TrialRecord with binary outcome true
    And a TrialRecord with score outcome 1.0
    Then the two outcomes are not equal

  Scenario: JudgeConfig family is required
    Given a JudgeConfig with model "gpt-5" and family "openai"
    Then the family field is "openai"

  Scenario: TrialRecord without judge config
    Given a TrialRecord with no judge_config
    When I serialize to JSON
    Then the judge_config field is null

  Scenario: MultiCriterion outcome preserves all criteria
    Given a TrialRecord with multi-criterion outcome "accuracy=0.92,helpfulness=0.78,safety=1.0"
    When I serialize to JSON and deserialize back
    Then all three criteria are preserved with exact values

  Scenario: Metadata roundtrips through JSON
    Given a TrialRecord with metadata key "source" value "inspect-v0.3"
    When I serialize to JSON and deserialize back
    Then the metadata key "source" has value "inspect-v0.3"

  Scenario: TrialRecord with no seed
    Given a TrialRecord with no seed
    When I serialize to JSON
    Then the seed field is null

  Scenario: NaN score is rejected by validated constructor
    When I construct an Outcome::Score with NaN
    Then I get a non-finite score error

  Scenario: Infinity score is rejected by validated constructor
    When I construct an Outcome::Score with Infinity
    Then I get a non-finite score error

  Scenario: NaN temperature is rejected by JudgeConfig constructor
    When I construct a JudgeConfig with NaN temperature
    Then I get a non-finite temperature error

  Scenario: Outcome JSON uses tagged representation
    Given a TrialRecord with binary outcome true
    When I serialize to JSON
    Then the outcome JSON has a "type" field with value "Binary"
