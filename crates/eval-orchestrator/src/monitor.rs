use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::config::MonitorConfig;
use crate::outcome_ext::outcome_to_f64;
use crate::types::{Decision, MonitorSummary, SeriesKey};
use eval_core::TrialRecord;
use seq_anytime_valid::monitor::anytime::AnytimeMonitor;
use seq_anytime_valid::types::{DataFamily, MsprtConfig};
use spc_charts::types::ControlLimits;
use spc_charts::{EwmaChart, EwmaConfig};

/// Serde helper: serialize `BTreeMap<SeriesKey, V>` as a sequence of `(SeriesKey, V)` pairs
/// because JSON requires string map keys and `SeriesKey` is a struct.
mod btree_as_seq {
    use super::SeriesKey;
    use serde::de::Deserializer;
    use serde::ser::Serializer;
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    pub fn serialize<S, V>(map: &BTreeMap<SeriesKey, V>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        let pairs: Vec<(&SeriesKey, &V)> = map.iter().collect();
        pairs.serialize(serializer)
    }

    pub fn deserialize<'de, D, V>(deserializer: D) -> Result<BTreeMap<SeriesKey, V>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
    {
        let pairs: Vec<(SeriesKey, V)> = Vec::deserialize(deserializer)?;
        Ok(pairs.into_iter().collect())
    }
}

/// Streaming monitor that tracks SPC and sequential evidence across series.
///
/// Because `EwmaChart` and `AnytimeMonitor` do not implement `Serialize`/`Deserialize`,
/// we store only raw configs and accumulated observations in serializable state structs.
/// Chart objects are reconstructed and replayed on each `push()` call. This is O(n) per
/// push but acceptable for eval workloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monitor {
    config: MonitorConfig,
    #[serde(with = "btree_as_seq")]
    spc_state: BTreeMap<SeriesKey, SpcStreamState>,
    #[serde(with = "btree_as_seq")]
    seq_state: BTreeMap<SeriesKey, SeqStreamState>,
    observations_seen: u64,
    total_decisions_emitted: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpcStreamState {
    /// Values in the current (incomplete) window.
    window_values: Vec<f64>,
    /// Run ID of the current window (stored as string for serde).
    current_window_run: Option<String>,
    /// Means of completed windows.
    completed_window_means: Vec<f64>,
    /// Whether phase 1 calibration is complete.
    phase1_complete: bool,
    /// Phase-1 center line (set once phase1 is complete).
    mu_0: f64,
    /// Phase-1 standard deviation (set once phase1 is complete).
    sigma: f64,
    /// Phase-2 window means (replayed through chart on each push).
    chart_observations: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SeqStreamState {
    /// All observations seen (replayed through `AnytimeMonitor` on each push).
    observations: Vec<f64>,
    /// Whether we have already emitted `StopEarly`.
    stopped: bool,
}

impl Monitor {
    /// Create a new monitor with empty state.
    #[must_use]
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            config,
            spc_state: BTreeMap::new(),
            seq_state: BTreeMap::new(),
            observations_seen: 0,
            total_decisions_emitted: 0,
        }
    }

    /// Process a single trial record and return any decisions emitted.
    pub fn push(&mut self, record: &TrialRecord) -> Vec<Decision> {
        self.observations_seen += 1;
        let series = SeriesKey::from_record(record);
        let value = outcome_to_f64(&record.outcome);
        let run_id_str = record.run_id.to_string();

        let is_known = self.spc_state.contains_key(&series) || self.seq_state.contains_key(&series);
        if !is_known && !self.config.auto_detect {
            return Vec::new();
        }

        let mut decisions = Vec::new();

        // Sequential monitoring
        decisions.extend(self.push_sequential(&series, value));

        // SPC monitoring
        decisions.extend(self.push_spc(&series, value, &run_id_str));

        self.total_decisions_emitted += decisions.len() as u64;
        decisions
    }

    /// Process a batch of trial records and return all decisions emitted.
    pub fn push_batch(&mut self, records: &[TrialRecord]) -> Vec<Decision> {
        let mut all_decisions = Vec::new();
        for record in records {
            all_decisions.extend(self.push(record));
        }
        all_decisions
    }

    /// Return all active series (union of SPC and sequential state keys).
    #[must_use]
    pub fn active_series(&self) -> Vec<&SeriesKey> {
        let mut keys: Vec<&SeriesKey> = self.spc_state.keys().collect();
        for k in self.seq_state.keys() {
            if !self.spc_state.contains_key(k) {
                keys.push(k);
            }
        }
        keys
    }

    /// Summary of the monitor's current state.
    #[must_use]
    pub fn state_summary(&self) -> MonitorSummary {
        let active = self.active_series().len();
        let in_phase1 = self
            .spc_state
            .values()
            .filter(|s| !s.phase1_complete)
            .count();
        let in_phase2 = self
            .spc_state
            .values()
            .filter(|s| s.phase1_complete)
            .count();

        MonitorSummary {
            observations_seen: self.observations_seen,
            active_series: active,
            series_in_phase1: in_phase1,
            series_in_phase2: in_phase2,
            total_decisions_emitted: self.total_decisions_emitted,
        }
    }

    /// Push a value into the sequential monitor for a series.
    /// Replays all observations through a fresh `AnytimeMonitor` each time.
    fn push_sequential(&mut self, series: &SeriesKey, value: f64) -> Vec<Decision> {
        let state = self
            .seq_state
            .entry(series.clone())
            .or_insert_with(|| SeqStreamState {
                observations: Vec::new(),
                stopped: false,
            });

        if state.stopped {
            return Vec::new();
        }

        state.observations.push(value);

        let alpha = self.config.sequential.alpha;
        let msprt_config = MsprtConfig {
            theta_0: 0.0,
            mixing_variance: self.config.sequential.mixing_variance,
            family: DataFamily::Normal {
                known_variance: None,
            },
            max_samples: None,
        };

        // Reconstruct and replay
        let monitor = AnytimeMonitor::new(msprt_config, alpha);
        let mut monitor = match monitor {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };

        let mut last_snapshot = None;
        for &obs in &state.observations {
            match monitor.update(obs) {
                Ok(snap) => last_snapshot = Some(snap),
                Err(_) => return Vec::new(),
            }
        }

        if let Some(snap) = last_snapshot {
            let e_val = snap.e_value.unwrap_or(snap.log_likelihood_ratio.exp());
            let threshold = 1.0 / alpha;

            if e_val >= threshold {
                state.stopped = true;
                let ci = snap
                    .confidence_interval
                    .unwrap_or((f64::NEG_INFINITY, f64::INFINITY));
                let estimate = (ci.0 + ci.1) / 2.0;
                return vec![Decision::StopEarly {
                    series: series.clone(),
                    evidence: e_val,
                    estimate,
                    ci,
                }];
            }
        }

        Vec::new()
    }

    /// Push a value into the SPC stream for a series.
    /// When a new window completes in phase 2, replays all chart observations
    /// through a fresh `EwmaChart`.
    fn push_spc(&mut self, series: &SeriesKey, value: f64, run_id: &str) -> Vec<Decision> {
        let phase1_windows = self.config.spc.phase1_windows;
        let lambda = self.config.spc.lambda;
        let l_sigma = self.config.spc.l_sigma;

        let state = self
            .spc_state
            .entry(series.clone())
            .or_insert_with(|| SpcStreamState {
                window_values: Vec::new(),
                current_window_run: None,
                completed_window_means: Vec::new(),
                phase1_complete: false,
                mu_0: 0.0,
                sigma: 0.0,
                chart_observations: Vec::new(),
            });

        // Detect window boundary: if run changed and we have pending values
        let mut just_completed_window = false;
        if let Some(ref current_run) = state.current_window_run {
            if current_run != run_id && !state.window_values.is_empty() {
                // Complete the current window
                let mean =
                    state.window_values.iter().sum::<f64>() / state.window_values.len() as f64;
                state.completed_window_means.push(mean);
                state.window_values.clear();
                just_completed_window = true;
            }
        }

        state.current_window_run = Some(run_id.to_string());
        state.window_values.push(value);

        // Check if phase 1 is now complete
        if !state.phase1_complete && state.completed_window_means.len() >= phase1_windows {
            let n = state.completed_window_means.len() as f64;
            let mu = state.completed_window_means.iter().sum::<f64>() / n;
            let variance = state
                .completed_window_means
                .iter()
                .map(|x| (x - mu).powi(2))
                .sum::<f64>()
                / (n - 1.0);
            let sigma = variance.sqrt();

            if sigma > 0.0 && sigma.is_finite() {
                state.mu_0 = mu;
                state.sigma = sigma;
                state.phase1_complete = true;
            }
        }

        // Phase-2 chart logic: only when phase1 complete and a window just completed
        if state.phase1_complete
            && just_completed_window
            && state.sigma > 0.0
            && state.sigma.is_finite()
        {
            // The latest completed window mean goes into chart_observations
            let latest_mean = state.completed_window_means.last().copied().unwrap_or(0.0);
            state.chart_observations.push(latest_mean);

            // Reconstruct and replay
            let limits = ControlLimits::new(state.mu_0, state.sigma);
            let limits = match limits {
                Ok(l) => l,
                Err(_) => return Vec::new(),
            };
            let ewma_config = EwmaConfig {
                limits,
                lambda,
                l_sigma,
            };
            let chart = EwmaChart::new(ewma_config);
            let mut chart = match chart {
                Ok(c) => c,
                Err(_) => return Vec::new(),
            };

            let mut last_signal = spc_charts::types::ChartSignal::InControl;
            for &obs in &state.chart_observations {
                last_signal = chart.observe(obs);
            }

            if last_signal.is_out_of_control() {
                let ucl = state.mu_0 + l_sigma * state.sigma;
                let lcl = state.mu_0 - l_sigma * state.sigma;
                return vec![Decision::Regression {
                    series: series.clone(),
                    signal: last_signal,
                    observation_value: latest_mean,
                    control_limits: (lcl, ucl),
                }];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eval_core::{Outcome, TrialRecord};
    use std::collections::BTreeMap;
    use ulid::Ulid;

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

    #[test]
    fn new_monitor_is_empty() {
        let mon = Monitor::new(MonitorConfig::default());
        assert!(mon.active_series().is_empty());
        let summary = mon.state_summary();
        assert_eq!(summary.observations_seen, 0);
        assert_eq!(summary.active_series, 0);
        assert_eq!(summary.series_in_phase1, 0);
        assert_eq!(summary.series_in_phase2, 0);
        assert_eq!(summary.total_decisions_emitted, 0);
    }

    #[test]
    fn push_auto_detects_series() {
        let mut mon = Monitor::new(MonitorConfig::default());
        let run = Ulid::new();
        let record = make_record("task-1", "agent-1", run, 0.5, 1_000_000);
        let _decisions = mon.push(&record);

        assert_eq!(mon.active_series().len(), 1);
        assert_eq!(mon.state_summary().observations_seen, 1);
    }

    #[test]
    fn push_batch_equivalent_to_sequential() {
        let config = MonitorConfig::default();
        let mut mon_batch = Monitor::new(config.clone());
        let mut mon_seq = Monitor::new(config);

        let run = Ulid::new();
        let records: Vec<_> = (0..20)
            .map(|i| make_record("t", "a", run, 0.5, i))
            .collect();

        let _ = mon_batch.push_batch(&records);
        for r in &records {
            let _ = mon_seq.push(r);
        }

        assert_eq!(
            mon_batch.state_summary().observations_seen,
            mon_seq.state_summary().observations_seen
        );
    }

    #[test]
    fn monitor_serde_roundtrip() {
        let config = MonitorConfig::default();
        let mut mon1 = Monitor::new(config);

        let run = Ulid::new();
        let records: Vec<_> = (0..30)
            .map(|i| make_record("t", "a", run, 0.5, i))
            .collect();

        for r in &records {
            let _ = mon1.push(r);
        }

        let json = serde_json::to_string(&mon1).unwrap();
        let mut mon2: Monitor = serde_json::from_str(&json).unwrap();

        // Push one more record to both
        let extra = make_record("t", "a", run, 0.5, 100);
        let d1 = mon1.push(&extra);
        let d2 = mon2.push(&extra);

        assert_eq!(d1.len(), d2.len());
        assert_eq!(
            mon1.state_summary().observations_seen,
            mon2.state_summary().observations_seen
        );
    }

    #[test]
    fn spc_regression_in_streaming() {
        let mut config = MonitorConfig::default();
        // Use 20 phase-1 windows with smaller l_sigma to make detection easier
        config.spc.phase1_windows = 20;
        config.spc.lambda = 0.2;
        config.spc.l_sigma = 2.5;

        let mut mon = Monitor::new(config);
        let mut decisions = Vec::new();

        // Phase 1: 21 runs with slightly varying means to get sigma > 0.
        // Windows are completed when the *next* run starts, so we need 21
        // runs to produce 20 completed windows.
        // Each run has 5 observations. Run i has value = 0.8 + i * 0.01.
        for ri in 0..21u64 {
            let run = Ulid::new();
            let val = 0.8 + (ri as f64) * 0.01;
            for obs_j in 0..5 {
                let ts = (ri * 100 + obs_j) as i64;
                let r = make_record("task", "agent", run, val, ts);
                decisions.extend(mon.push(&r));
            }
        }

        // After phase 1, the series should be in phase 2.
        // (21 runs → 20 completed windows, the 21st window is still open.)
        let summary = mon.state_summary();
        assert_eq!(
            summary.series_in_phase2, 1,
            "Should be in phase 2 after 21 runs"
        );

        // Phase 2: 10 runs with dramatically lower values to trigger regression.
        // The first phase-2 run also closes the 21st phase-1 window.
        for ri in 0..10u64 {
            let run = Ulid::new();
            let val = 0.2;
            for obs_j in 0..5 {
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

    #[test]
    fn sequential_stop_in_streaming() {
        let config = MonitorConfig::default();
        let mut mon = Monitor::new(config);
        let mut decisions = Vec::new();

        // Push records with Score(2.0) — strong evidence away from theta_0=0.0
        // Each record in a separate run so it counts as a new observation
        for i in 0..200u64 {
            let run = Ulid::new();
            let r = make_record("t", "a", run, 2.0, i as i64);
            decisions.extend(mon.push(&r));
        }

        let stops: Vec<_> = decisions
            .iter()
            .filter(|d| matches!(d, Decision::StopEarly { .. }))
            .collect();
        assert!(
            !stops.is_empty(),
            "Expected StopEarly before 200 observations, got none"
        );
    }
}
