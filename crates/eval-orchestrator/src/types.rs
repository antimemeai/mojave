use eval_core::TrialRecord;
use serde::{Deserialize, Serialize};
use spc_charts::ChartSignal;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SeriesKey {
    pub task_id: String,
    pub agent_id: String,
    pub scorer: Option<String>,
}

impl SeriesKey {
    pub fn from_record(record: &TrialRecord) -> Self {
        let scorer = record
            .metadata
            .get("scorer_name")
            .and_then(|v| v.as_str())
            .map(String::from);
        Self {
            task_id: record.task_id.clone(),
            agent_id: record.agent_id.clone(),
            scorer,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Decision {
    StopEarly {
        series: SeriesKey,
        evidence: f64,
        estimate: f64,
        ci: (f64, f64),
    },
    ContinueRunning {
        series: SeriesKey,
        current_n: usize,
        estimated_n_needed: usize,
        power_at_current_n: f64,
    },
    Regression {
        series: SeriesKey,
        signal: ChartSignal,
        observation_value: f64,
        control_limits: (f64, f64),
    },
    MeasurementWarning {
        series: SeriesKey,
        issue: MeasurementIssue,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeasurementIssue {
    LowAgreement { kappa: f64, threshold: f64 },
    InsufficientRaters { have: usize, need: usize },
    InsufficientSamples { have: usize, need: usize },
    HighVariance { cv: f64, threshold: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub decisions: Vec<Decision>,
    pub irr_results: Option<IrrSummary>,
    pub sequential_results: Vec<SequentialSummary>,
    pub spc_results: Vec<SpcSummary>,
    pub series_detected: Vec<SeriesKey>,
    pub instruments_run: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrrSummary {
    pub series: SeriesKey,
    pub alpha: f64,
    pub n_raters: usize,
    pub n_items: usize,
    pub metric: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialSummary {
    pub series: SeriesKey,
    pub n_observations: usize,
    pub current_estimate: f64,
    pub ci: (f64, f64),
    pub evidence: f64,
    pub stopped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpcSummary {
    pub series: SeriesKey,
    pub n_windows: usize,
    pub chart_type: String,
    pub in_control: bool,
    pub signals: Vec<ChartSignal>,
    pub control_limits: (f64, f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorSummary {
    pub observations_seen: u64,
    pub active_series: usize,
    pub series_in_phase1: usize,
    pub series_in_phase2: usize,
    pub total_decisions_emitted: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("no records provided")]
    EmptyInput,
    #[error("no instruments applicable and none force-enabled")]
    NoInstrumentsApplicable,
}

#[cfg(test)]
mod tests {
    use super::*;
    use eval_core::Outcome;
    use std::collections::{BTreeMap, HashSet};
    use ulid::Ulid;

    fn make_record(scorer_name: Option<&str>) -> TrialRecord {
        let mut metadata = BTreeMap::new();
        if let Some(name) = scorer_name {
            metadata.insert(
                "scorer_name".to_string(),
                serde_json::Value::String(name.to_string()),
            );
        }
        TrialRecord {
            trial_id: Ulid::new(),
            run_id: Ulid::new(),
            task_id: "task-abc".to_string(),
            task_version: None,
            agent_id: "agent-xyz".to_string(),
            agent_version: None,
            judge_config: None,
            seed: None,
            timestamp: 1_700_000_000,
            outcome: Outcome::Binary(true),
            metadata,
        }
    }

    #[test]
    fn series_key_from_record_no_scorer() {
        let record = make_record(None);
        let key = SeriesKey::from_record(&record);
        assert_eq!(key.task_id, "task-abc");
        assert_eq!(key.agent_id, "agent-xyz");
        assert_eq!(key.scorer, None);
    }

    #[test]
    fn series_key_from_record_with_scorer() {
        let record = make_record(Some("exact_match"));
        let key = SeriesKey::from_record(&record);
        assert_eq!(key.task_id, "task-abc");
        assert_eq!(key.agent_id, "agent-xyz");
        assert_eq!(key.scorer, Some("exact_match".to_string()));
    }

    #[test]
    fn series_key_equality_and_hash() {
        let a = SeriesKey {
            task_id: "t1".to_string(),
            agent_id: "a1".to_string(),
            scorer: Some("s1".to_string()),
        };
        let b = SeriesKey {
            task_id: "t1".to_string(),
            agent_id: "a1".to_string(),
            scorer: Some("s1".to_string()),
        };
        assert_eq!(a, b);

        let mut set = HashSet::new();
        set.insert(a);
        set.insert(b);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn series_key_roundtrip_serde() {
        let key = SeriesKey {
            task_id: "task-1".to_string(),
            agent_id: "agent-2".to_string(),
            scorer: Some("bleu".to_string()),
        };
        let json = serde_json::to_string(&key).unwrap();
        let back: SeriesKey = serde_json::from_str(&json).unwrap();
        assert_eq!(key, back);
    }

    #[test]
    fn decision_roundtrip_serde() {
        let decision = Decision::StopEarly {
            series: SeriesKey {
                task_id: "t".to_string(),
                agent_id: "a".to_string(),
                scorer: None,
            },
            evidence: 12.5,
            estimate: 0.85,
            ci: (0.75, 0.95),
        };
        let json = serde_json::to_string(&decision).unwrap();
        let back: Decision = serde_json::from_str(&json).unwrap();
        // Decision doesn't derive PartialEq (ChartSignal contains f64),
        // so check via re-serialization.
        let json2 = serde_json::to_string(&back).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn measurement_issue_roundtrip_serde() {
        let issue = MeasurementIssue::LowAgreement {
            kappa: 0.45,
            threshold: 0.6,
        };
        let json = serde_json::to_string(&issue).unwrap();
        let back: MeasurementIssue = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&back).unwrap();
        assert_eq!(json, json2);
    }
}
