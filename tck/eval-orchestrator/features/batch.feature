Feature: Batch analysis

  Scenario: Auto-detect IRR from multi-judge records
    Given 100 records for task "math" agent "gpt-4o" with 3 distinct judges
    When analyze is called with default config
    Then IrrInstrument is in instruments_run
    And irr_results contains alpha value

  Scenario: Auto-detect sequential from repeated observations
    Given 50 records for task "code" agent "claude" with one judge
    When analyze is called with default config
    Then SequentialInstrument is in instruments_run
    And a StopEarly or ContinueRunning decision is emitted

  Scenario: Auto-detect SPC from multi-run records
    Given 200 records spanning 10 runs for task "safety" agent "gpt-4o"
    When analyze is called with default config
    Then SpcInstrument is in instruments_run
    And spc_results contains chart state

  Scenario: Force-disable overrides auto-detect
    Given multi-judge records that would trigger IRR
    When analyze is called with force_disable = ["irr"]
    Then IrrInstrument is NOT in instruments_run

  Scenario: Force-enable overrides auto-detect
    Given single-run records that would NOT trigger SPC
    When analyze is called with force_enable = ["spc"]
    Then SpcInstrument is in instruments_run

  Scenario: Empty input returns error
    Given no records
    When analyze is called
    Then OrchestratorError::EmptyInput is returned
