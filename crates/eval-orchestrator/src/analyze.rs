use crate::config::AnalysisConfig;
use crate::instrument::InstrumentId;
use crate::instruments::irr::IrrInstrument;
use crate::instruments::sequential::SequentialInstrument;
use crate::instruments::spc::SpcInstrument;
use crate::router::{detect_instruments, group_by_series};
use crate::types::{AnalysisReport, OrchestratorError};
use eval_core::TrialRecord;

/// Run all applicable instruments over the supplied trial records and return
/// a consolidated [`AnalysisReport`].
///
/// The function auto-detects which instruments apply (based on record
/// characteristics and `config.force_enable` / `config.force_disable`), groups
/// records into series, then runs each instrument on every series.
pub fn analyze(
    records: &[TrialRecord],
    config: &AnalysisConfig,
) -> Result<AnalysisReport, OrchestratorError> {
    if records.is_empty() {
        return Err(OrchestratorError::EmptyInput);
    }

    let instrument_ids = detect_instruments(records, config);
    if instrument_ids.is_empty() {
        return Err(OrchestratorError::NoInstrumentsApplicable);
    }

    let groups = group_by_series(records);
    let series_detected: Vec<_> = groups.keys().cloned().collect();

    let mut all_decisions = Vec::new();
    let mut irr_results = None;
    let mut sequential_results = Vec::new();
    let mut spc_results = Vec::new();
    let mut instruments_run = Vec::new();

    for &id in &instrument_ids {
        instruments_run.push(id.name().to_string());

        for (series, series_records) in &groups {
            // group_by_series returns Vec<&TrialRecord>, but .run() expects
            // &[TrialRecord]. Clone the records into an owned vec.
            let owned_records: Vec<TrialRecord> =
                series_records.iter().map(|r| (*r).clone()).collect();

            match id {
                InstrumentId::Irr => {
                    let (decisions, summary) = IrrInstrument.run(series, &owned_records, config);
                    all_decisions.extend(decisions);
                    if let Some(s) = summary {
                        irr_results = Some(s);
                    }
                }
                InstrumentId::Sequential => {
                    let (decisions, summary) =
                        SequentialInstrument.run(series, &owned_records, config);
                    all_decisions.extend(decisions);
                    if let Some(s) = summary {
                        sequential_results.push(s);
                    }
                }
                InstrumentId::Spc => {
                    let (decisions, summary) = SpcInstrument.run(series, &owned_records, config);
                    all_decisions.extend(decisions);
                    if let Some(s) = summary {
                        spc_results.push(s);
                    }
                }
            }
        }
    }

    Ok(AnalysisReport {
        decisions: all_decisions,
        irr_results,
        sequential_results,
        spc_results,
        series_detected,
        instruments_run,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AnalysisConfig;
    use eval_core::{JudgeConfig, Outcome, TrialRecord};
    use std::collections::BTreeMap;
    use ulid::Ulid;

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

    fn default_config() -> AnalysisConfig {
        AnalysisConfig::default()
    }

    // ---- 1. empty input ----

    #[test]
    fn empty_input_returns_error() {
        let config = default_config();
        let result = analyze(&[], &config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, OrchestratorError::EmptyInput),
            "expected EmptyInput, got: {err}"
        );
    }

    // ---- 2. no instruments applicable ----

    #[test]
    fn no_instruments_applicable_returns_error() {
        let config = default_config();
        let run = Ulid::new();
        // Single record, no judge, one run => nothing triggers
        let records = vec![make_record(
            "task-1",
            "agent-1",
            run,
            None,
            1_700_000_000,
            Outcome::Binary(true),
        )];

        let result = analyze(&records, &config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, OrchestratorError::NoInstrumentsApplicable),
            "expected NoInstrumentsApplicable, got: {err}"
        );
    }

    // ---- 3. multi-judge triggers IRR ----

    #[test]
    fn multi_judge_triggers_irr() {
        let config = default_config();
        let run = Ulid::new();

        // 10 records with 2 alternating judges, using sample_id metadata
        // so the IRR instrument can build proper rating matrices.
        // Mix true/false outcomes so Krippendorff alpha is computable
        // (single-category data produces a degenerate matrix).
        let records: Vec<TrialRecord> = (0..10)
            .map(|i| {
                let j = if i % 2 == 0 {
                    judge("gpt-4o")
                } else {
                    judge("claude-3-opus")
                };
                let item_id = format!("item_{}", i / 2); // 5 items, 2 judges each
                let label = (i / 2) % 2 == 0; // alternating per item, both judges agree
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

        let report = analyze(&records, &config).expect("should succeed");
        assert!(
            report.instruments_run.contains(&"irr".to_string()),
            "instruments_run should contain 'irr', got: {:?}",
            report.instruments_run
        );
        assert!(
            report.irr_results.is_some(),
            "irr_results should be Some for multi-judge data"
        );
    }

    // ---- 4. many observations triggers sequential ----

    #[test]
    fn many_observations_triggers_sequential() {
        let config = default_config();
        let run = Ulid::new();

        // 50 records with Score(2.0) — same run, same task/agent
        // has_repeated_observations threshold is 10
        let records: Vec<TrialRecord> = (0..50)
            .map(|i| {
                make_record(
                    "math",
                    "gpt-4o",
                    run,
                    None,
                    1_700_000_000 + i,
                    Outcome::Score(2.0),
                )
            })
            .collect();

        let report = analyze(&records, &config).expect("should succeed");
        assert!(
            report.instruments_run.contains(&"sequential".to_string()),
            "instruments_run should contain 'sequential', got: {:?}",
            report.instruments_run
        );
        assert!(
            !report.sequential_results.is_empty(),
            "sequential_results should be non-empty"
        );
    }

    // ---- 5. multi-run triggers SPC ----

    #[test]
    fn multi_run_triggers_spc() {
        let config = default_config();

        // 25 runs x 10 records each, slight variation per run
        let mut records = Vec::new();
        for run_idx in 0..25_u64 {
            let run = Ulid::new();
            for obs in 0..10_i64 {
                let value = 0.8 + (run_idx as f64) * 0.01;
                records.push(make_record(
                    "math",
                    "gpt-4o",
                    run,
                    None,
                    1_700_000_000 + (run_idx as i64) * 1000 + obs,
                    Outcome::Score(value),
                ));
            }
        }

        let report = analyze(&records, &config).expect("should succeed");
        assert!(
            report.instruments_run.contains(&"spc".to_string()),
            "instruments_run should contain 'spc', got: {:?}",
            report.instruments_run
        );
        assert!(
            !report.spc_results.is_empty(),
            "spc_results should be non-empty"
        );
    }

    // ---- 6. force_disable prevents instrument ----

    #[test]
    fn force_disable_prevents_instrument() {
        let mut config = default_config();
        config.force_disable.push("irr".into());

        let run = Ulid::new();

        // Multi-judge data that would normally trigger IRR
        let records: Vec<TrialRecord> = (0..10)
            .map(|i| {
                let j = if i % 2 == 0 {
                    judge("gpt-4o")
                } else {
                    judge("claude-3-opus")
                };
                let mut metadata = BTreeMap::new();
                metadata.insert(
                    "sample_id".to_string(),
                    serde_json::Value::String(format!("item_{}", i / 2)),
                );
                make_record_with_metadata(
                    "math",
                    "gpt-4o",
                    run,
                    Some(j),
                    1_700_000_000 + i,
                    Outcome::Binary(true),
                    metadata,
                )
            })
            .collect();

        let result = analyze(&records, &config);

        // With IRR disabled and only 10 records from 1 run, sequential
        // is the only candidate (has_repeated_observations >= 10).
        // If sequential fires, instruments_run should not contain "irr".
        // If nothing fires, we get NoInstrumentsApplicable.
        match result {
            Err(OrchestratorError::NoInstrumentsApplicable) => {
                // acceptable — disabling IRR left no applicable instruments
            }
            Ok(report) => {
                assert!(
                    !report.instruments_run.contains(&"irr".to_string()),
                    "instruments_run should not contain 'irr' when force_disable includes it"
                );
            }
            Err(other) => panic!("unexpected error: {other}"),
        }
    }

    // ---- 7. report contains detected series ----

    #[test]
    fn report_contains_detected_series() {
        let config = default_config();
        let run = Ulid::new();

        // 50 records all for task="math", agent="gpt-4o"
        let records: Vec<TrialRecord> = (0..50)
            .map(|i| {
                make_record(
                    "math",
                    "gpt-4o",
                    run,
                    None,
                    1_700_000_000 + i,
                    Outcome::Score(0.9),
                )
            })
            .collect();

        let report = analyze(&records, &config).expect("should succeed");
        assert_eq!(
            report.series_detected.len(),
            1,
            "expected exactly 1 series, got: {}",
            report.series_detected.len()
        );

        let series = &report.series_detected[0];
        assert_eq!(series.task_id, "math");
        assert_eq!(series.agent_id, "gpt-4o");
    }
}
