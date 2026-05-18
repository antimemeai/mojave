#![allow(clippy::unwrap_used, clippy::expect_used)]

//! TCK integration tests for `eval_orchestrator::analyze()` — batch mode.
//!
//! Scenarios mirror `tck/eval-orchestrator/features/batch.feature`.

use eval_core::{JudgeConfig, Outcome, TrialRecord};
use eval_orchestrator::{analyze, AnalysisConfig, Decision, OrchestratorError};
use std::collections::BTreeMap;
use ulid::Ulid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_record(
    task: &str,
    agent: &str,
    run: Ulid,
    judge: Option<JudgeConfig>,
    ts: i64,
    outcome: Outcome,
) -> TrialRecord {
    TrialRecord {
        trial_id: Ulid::new(),
        run_id: run,
        task_id: task.into(),
        task_version: None,
        agent_id: agent.into(),
        agent_version: None,
        judge_config: judge,
        seed: None,
        timestamp: ts,
        outcome,
        metadata: BTreeMap::new(),
    }
}

fn make_record_with_metadata(
    task: &str,
    agent: &str,
    run: Ulid,
    judge: Option<JudgeConfig>,
    ts: i64,
    outcome: Outcome,
    metadata: BTreeMap<String, serde_json::Value>,
) -> TrialRecord {
    TrialRecord {
        trial_id: Ulid::new(),
        run_id: run,
        task_id: task.into(),
        task_version: None,
        agent_id: agent.into(),
        agent_version: None,
        judge_config: judge,
        seed: None,
        timestamp: ts,
        outcome,
        metadata,
    }
}

fn judge(model: &str) -> JudgeConfig {
    JudgeConfig::new(model.into(), "gpt".into(), "a".repeat(64), 0.0, None).unwrap()
}

// ---------------------------------------------------------------------------
// Scenario 1: Auto-detect IRR from multi-judge records
// ---------------------------------------------------------------------------

#[test]
fn auto_detect_irr_from_multi_judge() {
    let run = Ulid::new();
    let judges = ["judge-A", "judge-B", "judge-C"];

    // 100 records: 3 judges rating items round-robin.
    // Each item gets all 3 judges.  Mix true/false to avoid degenerate
    // single-category data (Krippendorff alpha is undefined for that).
    let n_items = 34; // 34 items × 3 judges = 102 ≥ 100
    let mut records = Vec::new();
    for item_idx in 0..n_items {
        let item_id = format!("item_{item_idx}");
        let label = item_idx % 2 == 0; // alternating per item, all judges agree
        for (j_idx, &model) in judges.iter().enumerate() {
            let mut metadata = BTreeMap::new();
            metadata.insert(
                "sample_id".to_string(),
                serde_json::Value::String(item_id.clone()),
            );
            records.push(make_record_with_metadata(
                "math",
                "gpt-4o",
                run,
                Some(judge(model)),
                1_700_000_000 + (item_idx as i64) * 10 + (j_idx as i64),
                Outcome::Binary(label),
                metadata,
            ));
        }
    }

    let config = AnalysisConfig::default();
    let report = analyze(&records, &config).expect("analyze should succeed");

    assert!(
        report.instruments_run.contains(&"irr".to_string()),
        "instruments_run should contain 'irr', got: {:?}",
        report.instruments_run
    );

    let irr = report.irr_results.expect("irr_results should be Some");
    assert!(irr.alpha.is_finite(), "alpha should be finite");
    assert_eq!(irr.n_raters, 3, "should detect 3 raters");
}

// ---------------------------------------------------------------------------
// Scenario 2: Auto-detect sequential from repeated observations
// ---------------------------------------------------------------------------

#[test]
fn auto_detect_sequential_from_repeated_obs() {
    let run = Ulid::new();

    // 50 records with Score(2.0), same run — triggers has_repeated_observations (>= 10)
    let records: Vec<TrialRecord> = (0..50)
        .map(|i| {
            make_record(
                "code",
                "claude",
                run,
                None,
                1_700_000_000 + i,
                Outcome::Score(2.0),
            )
        })
        .collect();

    let config = AnalysisConfig::default();
    let report = analyze(&records, &config).expect("analyze should succeed");

    assert!(
        report.instruments_run.contains(&"sequential".to_string()),
        "instruments_run should contain 'sequential', got: {:?}",
        report.instruments_run
    );

    // Should emit either StopEarly or ContinueRunning
    let has_seq_decision = report.decisions.iter().any(|d| {
        matches!(
            d,
            Decision::StopEarly { .. } | Decision::ContinueRunning { .. }
        )
    });
    assert!(
        has_seq_decision,
        "should emit StopEarly or ContinueRunning decision, got: {:?}",
        report.decisions
    );
}

// ---------------------------------------------------------------------------
// Scenario 3: Auto-detect SPC from multi-run records
// ---------------------------------------------------------------------------

#[test]
fn auto_detect_spc_from_multi_run() {
    // 25 runs × 10 records each with varying means per run (non-zero sigma).
    // Use a sin-based pattern to produce small per-run variation.
    let mut records = Vec::new();
    for run_idx in 0..25_u64 {
        let run = Ulid::new();
        for obs in 0..10_i64 {
            let angle = (run_idx * 7 + obs as u64 * 3) as f64;
            let value = 0.8 + 0.02 * angle.sin();
            records.push(make_record(
                "safety",
                "gpt-4o",
                run,
                None,
                1_700_000_000 + (run_idx as i64) * 1000 + obs,
                Outcome::Score(value),
            ));
        }
    }

    let config = AnalysisConfig::default();
    let report = analyze(&records, &config).expect("analyze should succeed");

    assert!(
        report.instruments_run.contains(&"spc".to_string()),
        "instruments_run should contain 'spc', got: {:?}",
        report.instruments_run
    );

    assert!(
        !report.spc_results.is_empty(),
        "spc_results should be non-empty"
    );

    // Verify chart state fields are populated
    let spc = &report.spc_results[0];
    assert!(spc.n_windows > 0, "n_windows should be > 0");
    assert!(
        !spc.chart_type.is_empty(),
        "chart_type should be non-empty string"
    );
}

// ---------------------------------------------------------------------------
// Scenario 4: Force-disable overrides auto-detect
// ---------------------------------------------------------------------------

#[test]
fn force_disable_overrides_detection() {
    let run = Ulid::new();

    // 20 records with 2 alternating judges — would normally trigger IRR.
    // Use sample_id + mixed labels so IRR detection is unambiguous.
    let records: Vec<TrialRecord> = (0..20)
        .map(|i| {
            let j = if i % 2 == 0 {
                judge("gpt-4o")
            } else {
                judge("claude-3-opus")
            };
            let item_id = format!("item_{}", i / 2);
            let label = (i / 2) % 2 == 0;
            let mut metadata = BTreeMap::new();
            metadata.insert("sample_id".to_string(), serde_json::Value::String(item_id));
            make_record_with_metadata(
                "math",
                "gpt-4o",
                run,
                Some(j),
                1_700_000_000 + i,
                Outcome::Binary(label),
                metadata,
            )
        })
        .collect();

    let mut config = AnalysisConfig::default();
    config.force_disable.push("irr".into());

    let result = analyze(&records, &config);

    // Disabling IRR should either leave no applicable instruments (error)
    // or produce a report without IRR in instruments_run.
    match result {
        Err(OrchestratorError::NoInstrumentsApplicable) => {
            // Acceptable — disabling IRR left no applicable instruments
            // (20 records from 1 run and no repeated-obs threshold met
            //  because binary outcomes with judges may or may not hit
            //  the sequential threshold depending on detection logic).
        }
        Ok(report) => {
            assert!(
                !report.instruments_run.contains(&"irr".to_string()),
                "instruments_run should NOT contain 'irr' when force_disable includes it, got: {:?}",
                report.instruments_run
            );
        }
        Err(other) => panic!("unexpected error: {other}"),
    }
}

// ---------------------------------------------------------------------------
// Scenario 5: Force-enable overrides auto-detect
// ---------------------------------------------------------------------------

#[test]
fn force_enable_overrides_detection() {
    let run = Ulid::new();

    // 50 records from a single run — would NOT trigger SPC (needs >= 2 runs).
    let records: Vec<TrialRecord> = (0..50)
        .map(|i| {
            make_record(
                "code",
                "claude",
                run,
                None,
                1_700_000_000 + i,
                Outcome::Score(0.75),
            )
        })
        .collect();

    let mut config = AnalysisConfig::default();
    config.force_enable.push("spc".into());

    let report = analyze(&records, &config).expect("analyze should succeed");

    assert!(
        report.instruments_run.contains(&"spc".to_string()),
        "instruments_run should contain 'spc' when force_enable includes it, got: {:?}",
        report.instruments_run
    );
}

// ---------------------------------------------------------------------------
// Scenario 6: Empty input returns error
// ---------------------------------------------------------------------------

#[test]
fn empty_input_error() {
    let config = AnalysisConfig::default();
    let result = analyze(&[], &config);

    assert!(result.is_err(), "empty input should return an error");
    let err = result.unwrap_err();
    assert!(
        matches!(err, OrchestratorError::EmptyInput),
        "expected OrchestratorError::EmptyInput, got: {err}"
    );
}
