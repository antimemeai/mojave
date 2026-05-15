Feature: Streaming monitor

  Scenario: SPC detects regression in streaming mode
    Given a Monitor with default config
    When 20 runs at mean=0.8 are pushed (phase I)
    And 10 runs at mean=0.2 are pushed (phase II)
    Then at least one Regression decision is emitted

  Scenario: Sequential test stops early
    Given a Monitor with sequential alpha=0.05
    When records with value=2.0 are pushed one at a time
    Then a StopEarly decision is emitted before 200 observations

  Scenario: Auto-detect discovers new series
    Given a Monitor with auto_detect=true
    When a record for a previously unseen (task, agent) arrives
    Then the series appears in active_series

  Scenario: Monitor state is serializable
    Given a Monitor with 100 observations pushed
    When serialized to JSON and deserialized
    Then pushing the same next record produces the same decisions

  Scenario: push_batch equivalent to sequential push
    Given a Monitor with default config
    When the same 50 records are processed via push_batch vs individual push
    Then the same decisions are produced
