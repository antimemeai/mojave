use std::collections::BTreeSet;

use crate::config::{AnalysisConfig, IrrMetric};
use crate::instrument::{Instrument, InstrumentId};
use crate::outcome_ext::outcome_to_ordinal;
use crate::types::{Decision, IrrSummary, MeasurementIssue, SeriesKey};
use eval_core::TrialRecord;
use irr::types::{AnnotationTriple, MetricLevel, RatingMatrix};

pub struct IrrInstrument;

impl IrrInstrument {
    pub fn run(
        &self,
        series: &SeriesKey,
        records: &[TrialRecord],
        config: &AnalysisConfig,
    ) -> (Vec<Decision>, Option<IrrSummary>) {
        // 1. Count distinct judges
        let mut judges: BTreeSet<String> = BTreeSet::new();
        for r in records {
            if let Some(ref jc) = r.judge_config {
                judges.insert(jc.model.clone());
            }
        }

        if judges.len() < config.irr.min_raters {
            return (
                vec![Decision::MeasurementWarning {
                    series: series.clone(),
                    issue: MeasurementIssue::InsufficientRaters {
                        have: judges.len(),
                        need: config.irr.min_raters,
                    },
                }],
                None,
            );
        }

        // 2. Build triples
        let triples = build_triples(records);
        if triples.is_empty() {
            return (Vec::new(), None);
        }

        // 3. Build matrix
        let matrix = match RatingMatrix::from_triples(&triples) {
            Ok(m) => m,
            Err(_) => {
                return (
                    vec![Decision::MeasurementWarning {
                        series: series.clone(),
                        issue: MeasurementIssue::InsufficientSamples { have: 0, need: 1 },
                    }],
                    None,
                );
            }
        };

        // 4. Run metric (handle each error type separately)
        let irr_result = match config.irr.metric {
            IrrMetric::Krippendorff => {
                match irr::krippendorff::alpha(&matrix, Some(MetricLevel::Nominal)) {
                    Ok(r) => r,
                    Err(_) => return (Vec::new(), None),
                }
            }
            IrrMetric::Fleiss => match irr::fleiss::kappa(&matrix) {
                Ok(r) => r,
                Err(_) => return (Vec::new(), None),
            },
            IrrMetric::Gwet => match irr::gwet::ac(&matrix, None) {
                Ok(r) => r,
                Err(_) => return (Vec::new(), None),
            },
        };

        let metric_name = match config.irr.metric {
            IrrMetric::Krippendorff => "krippendorff_alpha",
            IrrMetric::Fleiss => "fleiss_kappa",
            IrrMetric::Gwet => "gwet_ac",
        };

        // 5. Build summary
        let summary = IrrSummary {
            series: series.clone(),
            alpha: irr_result.value,
            n_raters: irr_result.n_raters,
            n_items: irr_result.n_items,
            metric: metric_name.to_string(),
        };

        // 6. Check threshold
        let mut decisions = Vec::new();
        if irr_result.value < config.irr.threshold {
            decisions.push(Decision::MeasurementWarning {
                series: series.clone(),
                issue: MeasurementIssue::LowAgreement {
                    kappa: irr_result.value,
                    threshold: config.irr.threshold,
                },
            });
        }

        (decisions, Some(summary))
    }
}

impl Instrument for IrrInstrument {
    fn id(&self) -> InstrumentId {
        InstrumentId::Irr
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

fn build_triples(records: &[TrialRecord]) -> Vec<AnnotationTriple> {
    let mut triples = Vec::new();
    for record in records {
        let annotator_id = match &record.judge_config {
            Some(jc) => jc.model.clone(),
            None => continue,
        };
        let item_id = record
            .metadata
            .get("sample_id")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| record.trial_id.to_string());
        let label = outcome_to_ordinal(&record.outcome);
        triples.push(AnnotationTriple {
            item_id,
            annotator_id,
            label,
        });
    }
    triples
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AnalysisConfig;
    use eval_core::{JudgeConfig, Outcome, TrialRecord};
    use std::collections::BTreeMap;
    use ulid::Ulid;

    fn make_irr_record(
        task: &str,
        agent: &str,
        judge_model: &str,
        item_id: &str,
        label: bool,
    ) -> TrialRecord {
        let jc =
            JudgeConfig::new(judge_model.into(), "gpt".into(), "a".repeat(64), 0.0, None).unwrap();
        let mut metadata = BTreeMap::new();
        metadata.insert(
            "sample_id".into(),
            serde_json::Value::String(item_id.into()),
        );
        TrialRecord {
            trial_id: Ulid::new(),
            run_id: Ulid::new(),
            task_id: task.into(),
            task_version: None,
            agent_id: agent.into(),
            agent_version: None,
            judge_config: Some(jc),
            seed: None,
            timestamp: 1000,
            outcome: Outcome::Binary(label),
            metadata,
        }
    }

    #[test]
    fn perfect_agreement_no_warning() {
        let series = SeriesKey {
            task_id: "t1".into(),
            agent_id: "a1".into(),
            scorer: None,
        };
        let config = AnalysisConfig::default();
        let instrument = IrrInstrument;

        // 5 items, 2 judges, both judges agree on each item (mix of true/false
        // to avoid degenerate single-category data)
        let mut records = Vec::new();
        for i in 0..5 {
            let item = format!("item_{i}");
            let label = i % 2 == 0; // alternating true/false, but both judges agree
            records.push(make_irr_record("t1", "a1", "judge-A", &item, label));
            records.push(make_irr_record("t1", "a1", "judge-B", &item, label));
        }

        let (decisions, summary) = instrument.run(&series, &records, &config);

        // No LowAgreement warning when agreement is perfect
        let has_low_agreement = decisions.iter().any(|d| {
            matches!(
                d,
                Decision::MeasurementWarning {
                    issue: MeasurementIssue::LowAgreement { .. },
                    ..
                }
            )
        });
        assert!(
            !has_low_agreement,
            "perfect agreement should not emit LowAgreement warning"
        );

        let summary = summary.expect("summary should be present for valid data");
        assert_eq!(summary.n_raters, 2);
        assert_eq!(summary.n_items, 5);
    }

    #[test]
    fn low_agreement_emits_warning() {
        let series = SeriesKey {
            task_id: "t1".into(),
            agent_id: "a1".into(),
            scorer: None,
        };
        let config = AnalysisConfig::default(); // threshold = 0.67
        let instrument = IrrInstrument;

        // 5 items, 2 judges, alternating disagreement
        let mut records = Vec::new();
        for i in 0..5 {
            let item = format!("item_{i}");
            let judge_a_label = i % 2 == 0;
            let judge_b_label = i % 2 != 0; // opposite
            records.push(make_irr_record("t1", "a1", "judge-A", &item, judge_a_label));
            records.push(make_irr_record("t1", "a1", "judge-B", &item, judge_b_label));
        }

        let (decisions, _summary) = instrument.run(&series, &records, &config);

        let has_low_agreement = decisions.iter().any(|d| {
            matches!(
                d,
                Decision::MeasurementWarning {
                    issue: MeasurementIssue::LowAgreement { .. },
                    ..
                }
            )
        });
        assert!(
            has_low_agreement,
            "alternating disagreement should emit LowAgreement warning"
        );
    }

    #[test]
    fn insufficient_raters_emits_warning() {
        let series = SeriesKey {
            task_id: "t1".into(),
            agent_id: "a1".into(),
            scorer: None,
        };
        let config = AnalysisConfig::default(); // min_raters = 2
        let instrument = IrrInstrument;

        // Only 1 judge
        let records = vec![make_irr_record("t1", "a1", "judge-A", "item_0", true)];

        let (decisions, summary) = instrument.run(&series, &records, &config);

        assert!(
            summary.is_none(),
            "summary should be None when insufficient raters"
        );

        let has_insufficient = decisions.iter().any(|d| {
            matches!(
                d,
                Decision::MeasurementWarning {
                    issue: MeasurementIssue::InsufficientRaters { .. },
                    ..
                }
            )
        });
        assert!(
            has_insufficient,
            "single judge should emit InsufficientRaters warning"
        );
    }
}
