Feature: Generic JSONL ingestion

  Scenario: Ingest JSONL with auto field name detection
    Given a JSONL file with fields "task_id", "agent_id", "score"
    When the JsonlAdapter ingests with auto-detect
    Then records are produced with correct field mapping

  Scenario: Ingest JSONL with explicit field mapping
    Given a JSONL file with fields "item", "model", "pass"
    And a FieldMapping mapping item->task_id, model->agent_id, pass->Binary
    When the JsonlAdapter ingests the file
    Then records have correct task_id, agent_id, and Binary outcome

  Scenario: Missing optional fields get defaults
    Given a JSONL file with only required fields
    When the JsonlAdapter ingests the file
    Then run_id is generated (one per file)
    And trial_id is generated per record

  Scenario: Mixed valid/invalid lines
    Given a JSONL file where one line is malformed JSON
    When the JsonlAdapter ingests the file
    Then records are produced for all valid lines
    And warnings are emitted for invalid lines

  Scenario: Content hash is deterministic
    Given the same JSONL file ingested twice
    Then content_hash matches across both runs
