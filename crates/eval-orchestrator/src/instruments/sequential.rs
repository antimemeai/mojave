use crate::config::AnalysisConfig;
use crate::instrument::{Instrument, InstrumentId};
use crate::outcome_ext::outcome_to_f64;
use crate::types::{Decision, SequentialSummary, SeriesKey};
use eval_core::TrialRecord;
use seq_anytime_valid::monitor::anytime::AnytimeMonitor;
use seq_anytime_valid::types::{DataFamily, MsprtConfig};

pub struct SequentialInstrument;

impl SequentialInstrument {
    pub fn run(
        &self,
        series: &SeriesKey,
        records: &[TrialRecord],
        config: &AnalysisConfig,
    ) -> (Vec<Decision>, Option<SequentialSummary>) {
        if records.is_empty() {
            return (Vec::new(), None);
        }

        let msprt_config = MsprtConfig {
            theta_0: 0.0,
            mixing_variance: config.sequential.mixing_variance,
            family: DataFamily::Normal {
                known_variance: None,
            },
            max_samples: None,
        };

        let mut monitor = match AnytimeMonitor::new(msprt_config, config.sequential.alpha) {
            Ok(m) => m,
            Err(_) => return (Vec::new(), None),
        };

        let mut last_snapshot = None;
        for record in records {
            let value = outcome_to_f64(&record.outcome);
            match monitor.update(value) {
                Ok(snap) => last_snapshot = Some(snap),
                Err(_) => continue,
            }
        }

        let snap = match last_snapshot {
            Some(s) => s,
            None => return (Vec::new(), None),
        };

        let e_value = snap.e_value.unwrap_or(snap.log_likelihood_ratio.exp());
        let threshold = 1.0 / config.sequential.alpha;
        let stopped = e_value >= threshold;

        let ci = snap
            .confidence_interval
            .unwrap_or((f64::NEG_INFINITY, f64::INFINITY));
        let estimate = (ci.0 + ci.1) / 2.0;

        let summary = SequentialSummary {
            series: series.clone(),
            n_observations: snap.n_observations,
            current_estimate: estimate,
            ci,
            evidence: e_value,
            stopped,
        };

        let mut decisions = Vec::new();
        if stopped {
            decisions.push(Decision::StopEarly {
                series: series.clone(),
                evidence: e_value,
                estimate,
                ci,
            });
        } else {
            decisions.push(Decision::ContinueRunning {
                series: series.clone(),
                current_n: snap.n_observations,
                estimated_n_needed: 0,
                power_at_current_n: 0.0,
            });
        }

        (decisions, Some(summary))
    }
}

impl Instrument for SequentialInstrument {
    fn id(&self) -> InstrumentId {
        InstrumentId::Sequential
    }

    fn analyze(
        &self,
        series: &SeriesKey,
        records: &[TrialRecord],
        config: &AnalysisConfig,
    ) -> Vec<Decision> {
        self.run(series, records, config).0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AnalysisConfig;
    use crate::types::{Decision, SeriesKey};
    use eval_core::{Outcome, TrialRecord};
    use std::collections::BTreeMap;
    use ulid::Ulid;

    fn make_seq_record(value: f64) -> TrialRecord {
        TrialRecord {
            trial_id: Ulid::new(),
            run_id: Ulid::new(),
            task_id: "t".into(),
            task_version: None,
            agent_id: "a".into(),
            agent_version: None,
            judge_config: None,
            seed: None,
            timestamp: 1000,
            outcome: Outcome::Score(value),
            metadata: BTreeMap::new(),
        }
    }

    fn make_binary_record(value: bool) -> TrialRecord {
        TrialRecord {
            trial_id: Ulid::new(),
            run_id: Ulid::new(),
            task_id: "t".into(),
            task_version: None,
            agent_id: "a".into(),
            agent_version: None,
            judge_config: None,
            seed: None,
            timestamp: 1000,
            outcome: Outcome::Binary(value),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn strong_signal_produces_stop_early() {
        let series = SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        };
        let config = AnalysisConfig::default();
        let instrument = SequentialInstrument;

        // 100 records with a strong positive signal (Score(2.0))
        let records: Vec<_> = (0..100).map(|_| make_seq_record(2.0)).collect();

        let (decisions, summary) = instrument.run(&series, &records, &config);

        // Should produce a StopEarly decision
        assert_eq!(decisions.len(), 1);
        assert!(
            matches!(&decisions[0], Decision::StopEarly { .. }),
            "strong signal should produce StopEarly, got: {:?}",
            decisions[0]
        );

        let summary = summary.expect("summary should be present");
        assert!(summary.stopped, "summary.stopped should be true");
        assert!(
            summary.evidence > 1.0,
            "evidence should be > 1.0, got: {}",
            summary.evidence
        );
        assert_eq!(summary.n_observations, 100);
    }

    #[test]
    fn weak_signal_produces_continue() {
        let series = SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        };
        let config = AnalysisConfig::default();
        let instrument = SequentialInstrument;

        // 5 records alternating tiny values — not enough evidence to stop
        let records: Vec<_> = (0..5)
            .map(|i| {
                if i % 2 == 0 {
                    make_seq_record(0.01)
                } else {
                    make_seq_record(-0.01)
                }
            })
            .collect();

        let (decisions, summary) = instrument.run(&series, &records, &config);

        assert_eq!(decisions.len(), 1);
        assert!(
            matches!(
                &decisions[0],
                Decision::ContinueRunning { current_n: 5, .. }
            ),
            "weak signal should produce ContinueRunning, got: {:?}",
            decisions[0]
        );

        let summary = summary.expect("summary should be present");
        assert!(
            !summary.stopped,
            "summary.stopped should be false for weak signal"
        );
    }

    #[test]
    fn binary_outcomes_convert_correctly() {
        let series = SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        };
        let config = AnalysisConfig::default();
        let instrument = SequentialInstrument;

        // 50 records of Binary(true) — all convert to 1.0
        let records: Vec<_> = (0..50).map(|_| make_binary_record(true)).collect();

        let (decisions, summary) = instrument.run(&series, &records, &config);

        assert!(
            !decisions.is_empty(),
            "should produce at least one decision"
        );

        let summary = summary.expect("summary should be present");
        assert_eq!(
            summary.n_observations, 50,
            "all 50 binary records should be processed"
        );
        assert_eq!(summary.series, series);
    }
}
