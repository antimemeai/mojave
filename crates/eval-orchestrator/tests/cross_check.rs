#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Gate 2 cross-check tests: verify that orchestrator instrument adapters
//! produce identical results to calling the underlying math crates directly.

use eval_core::{JudgeConfig, Outcome, TrialRecord};
use eval_orchestrator::{analyze, AnalysisConfig};
use irr::krippendorff;
use irr::types::{AnnotationTriple, MetricLevel, RatingMatrix};
use seq_anytime_valid::monitor::anytime::AnytimeMonitor;
use seq_anytime_valid::types::{DataFamily, MsprtConfig};
use std::collections::BTreeMap;
use ulid::Ulid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

fn make_record(task: &str, agent: &str, run: Ulid, ts: i64, outcome: Outcome) -> TrialRecord {
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
        outcome,
        metadata: BTreeMap::new(),
    }
}

fn judge(model: &str) -> JudgeConfig {
    JudgeConfig::new(model.into(), "gpt".into(), "a".repeat(64), 0.0, None).unwrap()
}

// ---------------------------------------------------------------------------
// Test 1: IRR cross-check — orchestrator vs direct Krippendorff alpha
// ---------------------------------------------------------------------------

#[test]
fn irr_cross_check_matches_direct_krippendorff() {
    // Dataset: 5 items, 3 judges, known labels
    let items = ["q1", "q2", "q3", "q4", "q5"];
    let judges = ["j1", "j2", "j3"];
    let labels: [[u32; 3]; 5] = [
        [1, 1, 1], // q1: all agree correct
        [1, 0, 1], // q2: j2 disagrees
        [0, 0, 0], // q3: all agree incorrect
        [1, 1, 0], // q4: j3 disagrees
        [0, 0, 1], // q5: j3 disagrees
    ];

    // --- Path A: Orchestrator ---
    let run = Ulid::new();
    let mut records = Vec::new();
    for (item_idx, item_name) in items.iter().enumerate() {
        for (j_idx, &judge_name) in judges.iter().enumerate() {
            let label = labels[item_idx][j_idx];
            let mut metadata = BTreeMap::new();
            metadata.insert(
                "sample_id".to_string(),
                serde_json::Value::String((*item_name).to_string()),
            );
            records.push(make_record_with_metadata(
                "math",
                "gpt-4o",
                run,
                Some(judge(judge_name)),
                1_700_000_000 + (item_idx as i64) * 10 + (j_idx as i64),
                Outcome::Binary(label == 1),
                metadata,
            ));
        }
    }

    let config = AnalysisConfig::default();
    let report = analyze(&records, &config).expect("analyze should succeed");
    let irr = report.irr_results.expect("irr_results should be Some");
    let alpha_orchestrator = irr.alpha;

    // --- Path B: Direct math crate ---
    let mut triples = Vec::new();
    for (item_idx, item_name) in items.iter().enumerate() {
        for (j_idx, &judge_name) in judges.iter().enumerate() {
            triples.push(AnnotationTriple {
                item_id: (*item_name).to_string(),
                annotator_id: judge_name.to_string(),
                label: labels[item_idx][j_idx],
            });
        }
    }

    let matrix = RatingMatrix::from_triples(&triples).expect("matrix should build");
    let result =
        krippendorff::alpha(&matrix, Some(MetricLevel::Nominal)).expect("alpha should compute");
    let alpha_direct = result.value;

    // --- Assert both paths produce the same alpha ---
    assert!(
        (alpha_orchestrator - alpha_direct).abs() < 1e-10,
        "orchestrator alpha ({alpha_orchestrator}) should match direct alpha ({alpha_direct}) within 1e-10"
    );
}

// ---------------------------------------------------------------------------
// Test 2: Sequential cross-check — orchestrator vs direct AnytimeMonitor
// ---------------------------------------------------------------------------

#[test]
fn sequential_cross_check_matches_direct_monitor() {
    let n = 50;
    let value = 2.0;

    // --- Path A: Orchestrator ---
    let run = Ulid::new();
    let records: Vec<TrialRecord> = (0..n)
        .map(|i| {
            make_record(
                "code",
                "claude",
                run,
                1_700_000_000 + i,
                Outcome::Score(value),
            )
        })
        .collect();

    let config = AnalysisConfig::default();
    let report = analyze(&records, &config).expect("analyze should succeed");

    assert!(
        !report.sequential_results.is_empty(),
        "sequential_results should be non-empty"
    );
    let e_value_orchestrator = report.sequential_results[0].evidence;

    // --- Path B: Direct math crate ---
    let msprt_config = MsprtConfig {
        theta_0: 0.0,
        mixing_variance: 1.0,
        family: DataFamily::Normal {
            known_variance: None,
        },
        max_samples: None,
    };
    let mut monitor = AnytimeMonitor::new(msprt_config, 0.05).expect("monitor should construct");

    let mut last_snapshot = None;
    for _ in 0..n {
        last_snapshot = Some(monitor.update(value).expect("update should succeed"));
    }

    let snap = last_snapshot.expect("should have at least one snapshot");
    let e_value_direct = snap.e_value.expect("e_value should be Some");

    // --- Assert both paths produce the same e-value ---
    assert!(
        (e_value_orchestrator - e_value_direct).abs() < 1e-10,
        "orchestrator e_value ({e_value_orchestrator}) should match direct e_value ({e_value_direct}) within 1e-10"
    );
}
