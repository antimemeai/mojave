#![allow(clippy::unwrap_used, clippy::expect_used)]

//! TCK integration tests for `eval_orchestrator::Monitor` — streaming mode.
//!
//! Scenarios mirror `tck/eval-orchestrator/features/monitor.feature`.

use eval_core::{Outcome, TrialRecord};
use eval_orchestrator::{Decision, Monitor, MonitorConfig};
use std::collections::BTreeMap;
use ulid::Ulid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_record(task: &str, agent: &str, run: Ulid, value: f64, ts: i64) -> TrialRecord {
    TrialRecord {
        trial_id: Ulid::new(),
        run_id: run,
        task_id: task.into(),
        task_version: None,
        agent_id: agent.into(),
        agent_version: None,
        judge_config: None,
        seed: None,
        timestamp: ts,
        outcome: Outcome::Score(value),
        metadata: BTreeMap::new(),
    }
}

// ---------------------------------------------------------------------------
// Scenario 1: SPC detects regression in streaming mode
// ---------------------------------------------------------------------------

#[test]
fn spc_detects_regression_streaming() {
    let mut config = MonitorConfig::default();
    // Use smaller l_sigma to make detection easier in tests
    config.spc.phase1_windows = 20;
    config.spc.lambda = 0.2;
    config.spc.l_sigma = 2.5;

    let mut mon = Monitor::new(config);
    let mut decisions = Vec::new();

    // Phase 1: 21 runs with slightly varying means to produce sigma > 0.
    // Need 21 runs to complete 20 windows (each window completes when the
    // *next* run arrives).
    // Each run has 10 observations. Run ri has value = 0.8 + ri * 0.01.
    for ri in 0..21u64 {
        let run = Ulid::new();
        let val = 0.8 + (ri as f64) * 0.01;
        for obs_j in 0..10 {
            let ts = (ri * 100 + obs_j) as i64;
            let r = make_record("task", "agent", run, val, ts);
            decisions.extend(mon.push(&r));
        }
    }

    // Verify phase 1 completed: the series should now be in phase 2.
    let summary = mon.state_summary();
    assert_eq!(
        summary.series_in_phase2, 1,
        "Should be in phase 2 after 21 runs"
    );

    // Phase 2: 10 runs at dramatically lower mean to trigger regression.
    // The first phase-2 run also closes the 21st phase-1 window.
    for ri in 0..10u64 {
        let run = Ulid::new();
        let val = 0.2;
        for obs_j in 0..10 {
            let ts = (2100 + ri * 100 + obs_j) as i64;
            let r = make_record("task", "agent", run, val, ts);
            decisions.extend(mon.push(&r));
        }
    }

    let regressions: Vec<_> = decisions
        .iter()
        .filter(|d| matches!(d, Decision::Regression { .. }))
        .collect();
    assert!(
        !regressions.is_empty(),
        "Expected at least one Regression decision, got none. Total decisions: {}",
        decisions.len()
    );
}

// ---------------------------------------------------------------------------
// Scenario 2: Sequential test stops early
// ---------------------------------------------------------------------------

#[test]
fn sequential_stops_early_streaming() {
    let config = MonitorConfig::default();
    let mut mon = Monitor::new(config);
    let mut decisions = Vec::new();
    let mut stopped_at = None;

    // Push records with Score(2.0) — strong evidence away from theta_0=0.0.
    // Each observation in a separate run so SPC windowing also works, but
    // the sequential monitor is the focus here.
    for i in 0..200u64 {
        let run = Ulid::new();
        let r = make_record("t", "a", run, 2.0, i as i64);
        let d = mon.push(&r);
        if stopped_at.is_none()
            && d.iter()
                .any(|dec| matches!(dec, Decision::StopEarly { .. }))
        {
            stopped_at = Some(i + 1); // 1-indexed observation count
        }
        decisions.extend(d);
    }

    let stops: Vec<_> = decisions
        .iter()
        .filter(|d| matches!(d, Decision::StopEarly { .. }))
        .collect();
    assert!(
        !stops.is_empty(),
        "Expected StopEarly before 200 observations, got none"
    );
    assert!(
        stopped_at.unwrap() < 200,
        "StopEarly should fire before observation 200, fired at {}",
        stopped_at.unwrap()
    );
}

// ---------------------------------------------------------------------------
// Scenario 3: Auto-detect discovers new series
// ---------------------------------------------------------------------------

#[test]
fn auto_detect_discovers_series() {
    let config = MonitorConfig {
        auto_detect: true,
        ..MonitorConfig::default()
    };
    let mut mon = Monitor::new(config);

    // Push 1 record for (task1, agent1)
    let run1 = Ulid::new();
    let r1 = make_record("task1", "agent1", run1, 0.5, 1_000_000);
    let _ = mon.push(&r1);
    assert_eq!(
        mon.active_series().len(),
        1,
        "Should have 1 active series after first push"
    );

    // Push 1 record for (task2, agent2)
    let run2 = Ulid::new();
    let r2 = make_record("task2", "agent2", run2, 0.7, 2_000_000);
    let _ = mon.push(&r2);
    assert_eq!(
        mon.active_series().len(),
        2,
        "Should have 2 active series after second push"
    );

    // Verify the series keys contain the expected task/agent pairs
    let keys = mon.active_series();
    let has_task1 = keys
        .iter()
        .any(|k| k.task_id == "task1" && k.agent_id == "agent1");
    let has_task2 = keys
        .iter()
        .any(|k| k.task_id == "task2" && k.agent_id == "agent2");
    assert!(has_task1, "active_series should contain (task1, agent1)");
    assert!(has_task2, "active_series should contain (task2, agent2)");
}

// ---------------------------------------------------------------------------
// Scenario 4: Monitor state is serializable
// ---------------------------------------------------------------------------

#[test]
fn monitor_serde_roundtrip_produces_same_decisions() {
    let config = MonitorConfig::default();
    let mut mon1 = Monitor::new(config);

    // Push 100 observations (all in one run so sequential accumulates evidence)
    let run = Ulid::new();
    for i in 0..100 {
        let r = make_record("t", "a", run, 0.5, i);
        let _ = mon1.push(&r);
    }

    // Serialize and deserialize
    let json = serde_json::to_string(&mon1).expect("serialize should succeed");
    let mut mon2: Monitor = serde_json::from_str(&json).expect("deserialize should succeed");

    // Push the same next record to both
    let next_run = Ulid::new();
    let next_record = make_record("t", "a", next_run, 0.5, 100);
    let d1 = mon1.push(&next_record);
    let d2 = mon2.push(&next_record);

    // Same number of decisions
    assert_eq!(
        d1.len(),
        d2.len(),
        "Deserialized monitor should produce same number of decisions"
    );

    // Same observation counts
    assert_eq!(
        mon1.state_summary().observations_seen,
        mon2.state_summary().observations_seen,
        "Observation counts should match after roundtrip"
    );

    // Same total decisions emitted
    assert_eq!(
        mon1.state_summary().total_decisions_emitted,
        mon2.state_summary().total_decisions_emitted,
        "Total decisions emitted should match after roundtrip"
    );
}

// ---------------------------------------------------------------------------
// Scenario 5: push_batch equivalent to sequential push
// ---------------------------------------------------------------------------

#[test]
fn push_batch_matches_sequential_push() {
    let config = MonitorConfig::default();
    let mut mon_seq = Monitor::new(config.clone());
    let mut mon_batch = Monitor::new(config);

    // Build 50 records across several runs so SPC windowing also engages
    let mut records = Vec::new();
    for i in 0..50u64 {
        // Change run every 10 records
        let run = Ulid::from_parts((i / 10) + 1, 0);
        let r = make_record("t", "a", run, 0.5 + (i as f64) * 0.001, i as i64);
        records.push(r);
    }

    // Sequential push
    let mut seq_decisions = Vec::new();
    for r in &records {
        seq_decisions.extend(mon_seq.push(r));
    }

    // Batch push
    let batch_decisions = mon_batch.push_batch(&records);

    // Same decision count
    assert_eq!(
        seq_decisions.len(),
        batch_decisions.len(),
        "push_batch should produce same number of decisions as sequential push ({} vs {})",
        seq_decisions.len(),
        batch_decisions.len()
    );

    // Same observations seen
    assert_eq!(
        mon_seq.state_summary().observations_seen,
        mon_batch.state_summary().observations_seen,
        "observations_seen should match"
    );

    // Same active series count
    assert_eq!(
        mon_seq.state_summary().active_series,
        mon_batch.state_summary().active_series,
        "active_series count should match"
    );

    // Same total decisions emitted
    assert_eq!(
        mon_seq.state_summary().total_decisions_emitted,
        mon_batch.state_summary().total_decisions_emitted,
        "total_decisions_emitted should match"
    );
}
