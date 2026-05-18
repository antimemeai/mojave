Feature: Inspect eval log ingestion

  Scenario: Ingest a v2 eval log with binary outcomes
    Given an Inspect v2 log file with 10 samples scored as "C"/"I"
    When the InspectAdapter ingests the file
    Then 10 TrialRecords are produced
    And each record has outcome Binary(true) or Binary(false)
    And each record has a valid trial_id and run_id
    And task_id matches the eval spec task_id
    And agent_id matches the eval spec model

  Scenario: Ingest a log with model-graded scorer
    Given an Inspect log with model_graded_fact scorer
    When the InspectAdapter ingests the file
    Then each record has judge_config populated
    And judge_config.model is the grading model name
    And judge_config.prompt_template_hash is a SHA-256 hex string

  Scenario: Ingest a log with multiple scorers
    Given an Inspect log with 5 samples and 2 scorers
    When the InspectAdapter ingests the file
    Then 10 TrialRecords are produced
    And metadata contains scorer_name for each record

  Scenario: Ingest a log with epochs
    Given an Inspect log with 5 samples and 3 epochs
    When the InspectAdapter ingests the file
    Then 15 TrialRecords are produced
    And records from different epochs share sample.id but differ in trial_id
    And metadata contains epoch number

  Scenario: Malformed samples produce warnings not errors
    Given an Inspect log where some samples have null scores or unmappable values
    When the InspectAdapter ingests the file
    Then the result contains records for all valid samples
    And the result contains warnings for unmappable samples

  Scenario: Content hash is computed for provenance
    Given an Inspect log file
    When the InspectAdapter ingests the file
    Then source_meta.content_hash is the SHA-256 hex of the file bytes
    And source_meta.runner_name is "inspect_ai"

  Scenario: Deterministic IDs across runs
    Given the same Inspect log file ingested twice
    Then trial_ids and run_ids match across both runs
