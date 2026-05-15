use std::collections::{BTreeMap, BTreeSet};

use crate::config::AnalysisConfig;
use crate::instrument::InstrumentId;
use crate::types::SeriesKey;
use eval_core::TrialRecord;

pub fn detect_instruments(records: &[TrialRecord], config: &AnalysisConfig) -> Vec<InstrumentId> {
    let mut enabled = Vec::new();

    if !config.force_disable.iter().any(|s| s == "irr") && has_multiple_judges(records) {
        enabled.push(InstrumentId::Irr);
    }

    if !config.force_disable.iter().any(|s| s == "sequential") && has_repeated_observations(records)
    {
        enabled.push(InstrumentId::Sequential);
    }

    if !config.force_disable.iter().any(|s| s == "spc") && has_temporal_runs(records) {
        enabled.push(InstrumentId::Spc);
    }

    for name in &config.force_enable {
        if let Some(id) = InstrumentId::from_name(name) {
            if !enabled.contains(&id) {
                enabled.push(id);
            }
        }
    }

    enabled
}

pub fn group_by_series(records: &[TrialRecord]) -> BTreeMap<SeriesKey, Vec<&TrialRecord>> {
    let mut groups: BTreeMap<SeriesKey, Vec<&TrialRecord>> = BTreeMap::new();
    for record in records {
        let key = SeriesKey::from_record(record);
        groups.entry(key).or_default().push(record);
    }
    groups
}

fn has_multiple_judges(records: &[TrialRecord]) -> bool {
    let mut by_series: BTreeMap<SeriesKey, BTreeSet<String>> = BTreeMap::new();
    for record in records {
        if let Some(ref jc) = record.judge_config {
            let key = SeriesKey::from_record(record);
            by_series.entry(key).or_default().insert(jc.model.clone());
        }
    }
    by_series.values().any(|judges| judges.len() >= 2)
}

fn has_repeated_observations(records: &[TrialRecord]) -> bool {
    let mut counts: BTreeMap<SeriesKey, usize> = BTreeMap::new();
    for record in records {
        let key = SeriesKey::from_record(record);
        *counts.entry(key).or_default() += 1;
    }
    counts.values().any(|&n| n >= 10)
}

fn has_temporal_runs(records: &[TrialRecord]) -> bool {
    let mut by_series: BTreeMap<SeriesKey, BTreeSet<ulid::Ulid>> = BTreeMap::new();
    for record in records {
        let key = SeriesKey::from_record(record);
        by_series.entry(key).or_default().insert(record.run_id);
    }
    by_series.values().any(|runs| runs.len() >= 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eval_core::{JudgeConfig, Outcome, TrialRecord};
    use std::collections::BTreeMap;
    use ulid::Ulid;

    fn make_record(
        task: &str,
        agent: &str,
        run: Ulid,
        judge: Option<JudgeConfig>,
        ts: i64,
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
            outcome: Outcome::Binary(true),
            metadata: BTreeMap::new(),
        }
    }

    fn judge(model: &str) -> JudgeConfig {
        JudgeConfig::new(model.into(), "gpt".into(), "a".repeat(64), 0.0, None).unwrap()
    }

    fn default_config() -> AnalysisConfig {
        AnalysisConfig::default()
    }

    #[test]
    fn detects_irr_from_multiple_judges() {
        let run = Ulid::new();
        let records: Vec<TrialRecord> = (0..10)
            .map(|i| {
                let j = if i % 2 == 0 {
                    judge("gpt-4o")
                } else {
                    judge("claude-3-opus")
                };
                make_record("task-1", "agent-1", run, Some(j), 1_700_000_000 + i)
            })
            .collect();

        let instruments = detect_instruments(&records, &default_config());
        assert!(
            instruments.contains(&InstrumentId::Irr),
            "expected Irr to be detected from alternating judges"
        );
    }

    #[test]
    fn detects_sequential_from_repeated_obs() {
        let run = Ulid::new();
        let records: Vec<TrialRecord> = (0..50)
            .map(|i| make_record("task-1", "agent-1", run, None, 1_700_000_000 + i))
            .collect();

        let instruments = detect_instruments(&records, &default_config());
        assert!(
            instruments.contains(&InstrumentId::Sequential),
            "expected Sequential to be detected from 50 repeated observations"
        );
    }

    #[test]
    fn detects_spc_from_multiple_runs() {
        let mut records = Vec::new();
        for _ in 0..5 {
            let run = Ulid::new();
            for i in 0..10 {
                records.push(make_record(
                    "task-1",
                    "agent-1",
                    run,
                    None,
                    1_700_000_000 + i,
                ));
            }
        }

        let instruments = detect_instruments(&records, &default_config());
        assert!(
            instruments.contains(&InstrumentId::Spc),
            "expected Spc to be detected from 5 distinct runs"
        );
    }

    #[test]
    fn force_disable_overrides_detection() {
        let run = Ulid::new();
        let records: Vec<TrialRecord> = (0..10)
            .map(|i| {
                let j = if i % 2 == 0 {
                    judge("gpt-4o")
                } else {
                    judge("claude-3-opus")
                };
                make_record("task-1", "agent-1", run, Some(j), 1_700_000_000 + i)
            })
            .collect();

        let mut config = default_config();
        config.force_disable.push("irr".into());

        let instruments = detect_instruments(&records, &config);
        assert!(
            !instruments.contains(&InstrumentId::Irr),
            "Irr should be disabled when force_disable contains 'irr'"
        );
    }

    #[test]
    fn force_enable_adds_instrument() {
        let run = Ulid::new();
        let records: Vec<TrialRecord> = (0..3)
            .map(|i| make_record("task-1", "agent-1", run, None, 1_700_000_000 + i))
            .collect();

        let mut config = default_config();
        config.force_enable.push("spc".into());

        let instruments = detect_instruments(&records, &config);
        assert!(
            instruments.contains(&InstrumentId::Spc),
            "Spc should be present when force_enable contains 'spc'"
        );
    }

    #[test]
    fn no_instruments_for_tiny_dataset() {
        let run = Ulid::new();
        let records = vec![make_record("task-1", "agent-1", run, None, 1_700_000_000)];

        let instruments = detect_instruments(&records, &default_config());
        assert!(
            instruments.is_empty(),
            "expected no instruments for a single record, got: {instruments:?}"
        );
    }
}
