Feature: mojave CLI — measurement engine entry point

  Scenario: Ingest Inspect AI log to JSON
    Given an Inspect AI eval log at "fixtures/inspect_binary.json"
    When I run "mojave ingest" on the file
    Then stdout is valid JSON
    And the JSON has a "records" array with at least 1 element
    And the JSON has a "source_meta" object with "runner_name" equal to "inspect_ai"
    And the JSON has a "warnings" array
    And the exit code is 0

  Scenario: Ingest JSONL log to JSON
    Given a JSONL file at "fixtures/basic.jsonl"
    When I run "mojave ingest" on the file
    Then stdout is valid JSON
    And the JSON has a "records" array with 5 elements
    And the exit code is 0

  Scenario: Analyze produces decisions with hints
    Given an Inspect AI eval log with multiple judges
    When I run "mojave analyze" on the file
    Then stdout is valid JSON
    And the JSON has a "decisions" array
    And each decision has a "hint" string field
    And the JSON has a "series_detected" array
    And the JSON has an "instruments_run" array
    And the exit code is 0

  Scenario: Analyze with config file override
    Given a config file setting irr.threshold to 0.9
    And an Inspect AI eval log with multiple judges
    When I run "mojave analyze --config=config.yaml" on the file
    Then the analysis uses irr threshold 0.9

  Scenario: Monitor reads TrialRecord JSONL from stdin
    Given a stream of 5 TrialRecord JSON lines
    When I pipe them to "mojave monitor"
    Then stdout contains one JSON object per line
    And the exit code is 0

  Scenario: Monitor emits summary on EOF
    Given a stream of 15 TrialRecord JSON lines for the same series
    When I pipe them to "mojave monitor"
    Then the last line is a MonitorSummary JSON object
    And the exit code is 0

  Scenario: Bad input file returns exit code 1
    Given a file "nonexistent.json" that does not exist
    When I run "mojave analyze nonexistent.json"
    Then the exit code is 1
    And stderr contains a JSON error with "kind" field

  Scenario: Invalid flag returns exit code 2
    When I run "mojave analyze --nonexistent-flag"
    Then the exit code is 2

  Scenario: Format auto-detection picks Inspect for .json
    Given an Inspect AI eval log at "fixtures/inspect_binary.json"
    When I run "mojave ingest" with no --format flag
    Then the ingest succeeds with runner_name "inspect_ai"

  Scenario: Format auto-detection picks JSONL for .jsonl
    Given a JSONL file at "fixtures/basic.jsonl"
    When I run "mojave ingest" with no --format flag
    Then the ingest succeeds with 5 records
