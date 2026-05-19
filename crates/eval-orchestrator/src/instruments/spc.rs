use std::collections::BTreeMap;

use crate::config::{AnalysisConfig, SpcChartType, WindowSize};
use crate::instrument::{Instrument, InstrumentId};
use crate::outcome_ext::outcome_to_f64;
use crate::types::{Decision, SeriesKey, SpcSummary};
use eval_core::TrialRecord;
use spc_charts::types::{ChartSignal, ControlLimits};
use spc_charts::{
    CombinedChart, CombinedConfig, CusumChart, CusumConfig, EwmaChart, EwmaConfig, ShewhartChart,
    ShewhartConfig, SpcError,
};
use ulid::Ulid;

pub struct SpcInstrument;

impl SpcInstrument {
    pub fn run(
        &self,
        series: &SeriesKey,
        records: &[TrialRecord],
        config: &AnalysisConfig,
    ) -> (Vec<Decision>, Option<SpcSummary>) {
        if records.is_empty() {
            return (Vec::new(), None);
        }

        let windows = compute_windows(records, &config.spc.window_size);
        if windows.is_empty() {
            return (Vec::new(), None);
        }

        let n_windows = windows.len();
        let chart_type_name = chart_type_name(&config.spc.chart_type);

        // Split into phase 1 and phase 2
        let phase1_count = config.spc.phase1_windows.min(windows.len());
        let phase1 = &windows[..phase1_count];
        let phase2 = &windows[phase1_count..];

        // If still in phase 1 (no phase 2 data), return summary only
        if phase2.is_empty() {
            return (
                Vec::new(),
                Some(SpcSummary {
                    series: series.clone(),
                    n_windows,
                    chart_type: chart_type_name,
                    in_control: true,
                    signals: Vec::new(),
                    control_limits: (0.0, 0.0),
                }),
            );
        }

        // Phase 1: compute control limits from sample mean and std dev
        let phase1_means: Vec<f64> = phase1.iter().map(|w| w.mean).collect();
        let mu_0 = phase1_means.iter().sum::<f64>() / phase1_means.len() as f64;
        let sigma = sample_std(&phase1_means, mu_0);

        // If sigma is zero or not finite, we can't establish control limits
        if sigma == 0.0 || !sigma.is_finite() {
            return (
                Vec::new(),
                Some(SpcSummary {
                    series: series.clone(),
                    n_windows,
                    chart_type: chart_type_name,
                    in_control: true,
                    signals: Vec::new(),
                    control_limits: (mu_0, mu_0),
                }),
            );
        }

        let limits = match ControlLimits::new(mu_0, sigma) {
            Ok(l) => l,
            Err(_) => {
                return (
                    Vec::new(),
                    Some(SpcSummary {
                        series: series.clone(),
                        n_windows,
                        chart_type: chart_type_name,
                        in_control: true,
                        signals: Vec::new(),
                        control_limits: (mu_0, mu_0),
                    }),
                );
            }
        };

        // Phase 2: create chart and observe
        let (decisions, signals) =
            run_phase2(series, phase2, &config.spc.chart_type, &limits, config);

        let in_control = signals.iter().all(|s| !s.is_out_of_control());

        let cl_lower = mu_0 - config.spc.l_sigma * sigma;
        let cl_upper = mu_0 + config.spc.l_sigma * sigma;

        let summary = SpcSummary {
            series: series.clone(),
            n_windows,
            chart_type: chart_type_name,
            in_control,
            signals,
            control_limits: (cl_lower, cl_upper),
        };

        (decisions, Some(summary))
    }
}

impl Instrument for SpcInstrument {
    fn id(&self) -> InstrumentId {
        InstrumentId::Spc
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

struct WindowStat {
    mean: f64,
}

fn compute_windows(records: &[TrialRecord], window_size: &WindowSize) -> Vec<WindowStat> {
    match window_size {
        WindowSize::PerRun => compute_per_run_windows(records),
        WindowSize::Fixed(n) => compute_fixed_windows(records, *n),
    }
}

fn compute_per_run_windows(records: &[TrialRecord]) -> Vec<WindowStat> {
    // Group by run_id
    let mut by_run: BTreeMap<Ulid, Vec<&TrialRecord>> = BTreeMap::new();
    for record in records {
        by_run.entry(record.run_id).or_default().push(record);
    }

    // Sort runs by earliest timestamp
    let mut runs: Vec<(i64, Ulid, Vec<&TrialRecord>)> = by_run
        .into_iter()
        .map(|(run_id, recs)| {
            let earliest = recs.iter().map(|r| r.timestamp).min().unwrap_or(0);
            (earliest, run_id, recs)
        })
        .collect();
    runs.sort_by_key(|(ts, id, _)| (*ts, *id));

    runs.into_iter()
        .map(|(_, _, recs)| {
            let sum: f64 = recs.iter().map(|r| outcome_to_f64(&r.outcome)).sum();
            let mean = sum / recs.len() as f64;
            WindowStat { mean }
        })
        .collect()
}

fn compute_fixed_windows(records: &[TrialRecord], chunk_size: usize) -> Vec<WindowStat> {
    if chunk_size == 0 {
        return Vec::new();
    }
    records
        .chunks(chunk_size)
        .map(|chunk| {
            let sum: f64 = chunk.iter().map(|r| outcome_to_f64(&r.outcome)).sum();
            let mean = sum / chunk.len() as f64;
            WindowStat { mean }
        })
        .collect()
}

fn sample_std(values: &[f64], mean: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let sum_sq: f64 = values.iter().map(|&v| (v - mean).powi(2)).sum();
    (sum_sq / (values.len() - 1) as f64).sqrt()
}

fn chart_type_name(ct: &SpcChartType) -> String {
    match ct {
        SpcChartType::Ewma => "ewma".to_string(),
        SpcChartType::Cusum => "cusum".to_string(),
        SpcChartType::Shewhart => "shewhart".to_string(),
        SpcChartType::Combined => "combined".to_string(),
    }
}

fn run_phase2(
    series: &SeriesKey,
    phase2_windows: &[WindowStat],
    chart_type: &SpcChartType,
    limits: &ControlLimits,
    config: &AnalysisConfig,
) -> (Vec<Decision>, Vec<ChartSignal>) {
    match chart_type {
        SpcChartType::Ewma => run_phase2_ewma(series, phase2_windows, limits, config),
        SpcChartType::Cusum => run_phase2_cusum(series, phase2_windows, limits),
        SpcChartType::Shewhart => run_phase2_shewhart(series, phase2_windows, limits),
        SpcChartType::Combined => run_phase2_combined(series, phase2_windows, limits),
    }
}

fn run_phase2_ewma(
    series: &SeriesKey,
    phase2_windows: &[WindowStat],
    limits: &ControlLimits,
    config: &AnalysisConfig,
) -> (Vec<Decision>, Vec<ChartSignal>) {
    let ewma_config = EwmaConfig {
        limits: limits.clone(),
        lambda: config.spc.lambda,
        l_sigma: config.spc.l_sigma,
    };
    let mut chart = match EwmaChart::new(ewma_config) {
        Ok(c) => c,
        Err(_) => return (Vec::new(), Vec::new()),
    };

    observe_and_collect(series, phase2_windows, limits, config.spc.l_sigma, |x| {
        chart.observe(x)
    })
}

fn run_phase2_cusum(
    series: &SeriesKey,
    phase2_windows: &[WindowStat],
    limits: &ControlLimits,
) -> (Vec<Decision>, Vec<ChartSignal>) {
    let cusum_config = CusumConfig::default_for(limits.clone());
    let mut chart = match CusumChart::new(cusum_config) {
        Ok(c) => c,
        Err(_) => return (Vec::new(), Vec::new()),
    };

    observe_and_collect(series, phase2_windows, limits, 3.0, |x| chart.observe(x))
}

fn run_phase2_shewhart(
    series: &SeriesKey,
    phase2_windows: &[WindowStat],
    limits: &ControlLimits,
) -> (Vec<Decision>, Vec<ChartSignal>) {
    let shewhart_config = ShewhartConfig::default_for(limits.clone());
    let mut chart = match ShewhartChart::new(shewhart_config) {
        Ok(c) => c,
        Err(_) => return (Vec::new(), Vec::new()),
    };

    observe_and_collect(series, phase2_windows, limits, 3.0, |x| chart.observe(x))
}

fn run_phase2_combined(
    series: &SeriesKey,
    phase2_windows: &[WindowStat],
    limits: &ControlLimits,
) -> (Vec<Decision>, Vec<ChartSignal>) {
    let combined_config = CombinedConfig::default_for(limits.clone());
    let mut chart = match CombinedChart::new(combined_config) {
        Ok(c) => c,
        Err(_) => return (Vec::new(), Vec::new()),
    };

    observe_and_collect(series, phase2_windows, limits, 3.0, |x| chart.observe(x))
}

fn observe_and_collect(
    series: &SeriesKey,
    phase2_windows: &[WindowStat],
    limits: &ControlLimits,
    l_sigma: f64,
    mut observe_fn: impl FnMut(f64) -> Result<ChartSignal, SpcError>,
) -> (Vec<Decision>, Vec<ChartSignal>) {
    let mut decisions = Vec::new();
    let mut signals = Vec::new();

    let cl_lower = limits.mu_0 - l_sigma * limits.sigma;
    let cl_upper = limits.mu_0 + l_sigma * limits.sigma;

    for window in phase2_windows {
        let signal = match observe_fn(window.mean) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if signal.is_out_of_control() {
            decisions.push(Decision::Regression {
                series: series.clone(),
                signal: signal.clone(),
                observation_value: window.mean,
                control_limits: (cl_lower, cl_upper),
            });
        }
        signals.push(signal);
    }

    (decisions, signals)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AnalysisConfig;
    use eval_core::{Outcome, TrialRecord};
    use std::collections::BTreeMap;
    use ulid::Ulid;

    fn make_spc_records(n_runs: usize, per_run: usize, mean: f64) -> Vec<TrialRecord> {
        let mut records = Vec::new();
        for ri in 0..n_runs {
            let run_id = Ulid::new();
            for si in 0..per_run {
                // Use a deterministic pseudo-random pattern that produces
                // run-to-run variation centred on `mean`. The sin-based
                // pattern ensures roughly equal positive/negative deviations
                // so phase-2 windows remain in control.
                let angle = (ri * 7 + si * 3) as f64;
                let value = mean + 0.02 * angle.sin();
                records.push(TrialRecord {
                    trial_id: Ulid::new(),
                    run_id,
                    task_id: "t".into(),
                    task_version: None,
                    agent_id: "a".into(),
                    agent_version: None,
                    judge_config: None,
                    seed: None,
                    timestamp: (ri * 1000 + si) as i64,
                    outcome: Outcome::Score(value),
                    metadata: BTreeMap::new(),
                });
            }
        }
        records
    }

    fn default_series() -> SeriesKey {
        SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        }
    }

    #[test]
    fn stable_process_no_regression() {
        let series = default_series();
        let config = AnalysisConfig::default();
        let instrument = SpcInstrument;

        // 25 runs x 10 records, all close to 0.8
        let records = make_spc_records(25, 10, 0.8);
        let (decisions, summary) = instrument.run(&series, &records, &config);

        // No regressions for a stable process
        let regressions: Vec<_> = decisions
            .iter()
            .filter(|d| matches!(d, Decision::Regression { .. }))
            .collect();
        assert!(
            regressions.is_empty(),
            "stable process should produce no regressions, got {}",
            regressions.len()
        );

        let summary = summary.expect("summary should be present");
        assert!(summary.in_control, "stable process should be in control");
        assert_eq!(summary.n_windows, 25);
        assert_eq!(summary.chart_type, "ewma");
    }

    #[test]
    fn mean_shift_detects_regression() {
        let series = default_series();
        let config = AnalysisConfig::default(); // phase1_windows=20, EWMA

        let instrument = SpcInstrument;

        // Phase 1: 20 runs around 0.8 with small per-run variation
        let mut records = Vec::new();
        for ri in 0..20 {
            let run_id = Ulid::new();
            for si in 0..10 {
                // Small variation per run so sigma > 0
                let value = 0.8 + (ri as f64 - 10.0) * 0.003;
                records.push(TrialRecord {
                    trial_id: Ulid::new(),
                    run_id,
                    task_id: "t".into(),
                    task_version: None,
                    agent_id: "a".into(),
                    agent_version: None,
                    judge_config: None,
                    seed: None,
                    timestamp: (ri * 1000 + si) as i64,
                    outcome: Outcome::Score(value),
                    metadata: BTreeMap::new(),
                });
            }
        }

        // Phase 2: 10 runs with dramatic shift down to 0.2
        for ri in 20..30 {
            let run_id = Ulid::new();
            for si in 0..10 {
                records.push(TrialRecord {
                    trial_id: Ulid::new(),
                    run_id,
                    task_id: "t".into(),
                    task_version: None,
                    agent_id: "a".into(),
                    agent_version: None,
                    judge_config: None,
                    seed: None,
                    timestamp: (ri * 1000 + si) as i64,
                    outcome: Outcome::Score(0.2),
                    metadata: BTreeMap::new(),
                });
            }
        }

        let (decisions, summary) = instrument.run(&series, &records, &config);

        let regressions: Vec<_> = decisions
            .iter()
            .filter(|d| matches!(d, Decision::Regression { .. }))
            .collect();
        assert!(
            !regressions.is_empty(),
            "mean shift from ~0.8 to 0.2 should detect at least one regression"
        );

        let summary = summary.expect("summary should be present");
        assert!(
            !summary.in_control,
            "process with mean shift should not be in control"
        );
    }

    #[test]
    fn insufficient_windows_still_ok() {
        let series = default_series();
        let config = AnalysisConfig::default(); // phase1_windows=20
        let instrument = SpcInstrument;

        // Only 3 runs x 10 records — fewer than phase1_windows
        let records = make_spc_records(3, 10, 0.8);
        let (decisions, summary) = instrument.run(&series, &records, &config);

        // No regressions when still in phase 1
        assert!(
            decisions.is_empty(),
            "should produce no decisions when still in phase 1, got {}",
            decisions.len()
        );

        let summary = summary.expect("summary should be present even in phase 1");
        assert!(summary.in_control, "phase 1 only should report in_control");
        assert_eq!(summary.n_windows, 3);
    }

    #[test]
    fn per_run_windowing_groups_by_run_id() {
        let series = default_series();
        let config = AnalysisConfig::default();
        let instrument = SpcInstrument;

        // 25 runs x 5 records each
        let records = make_spc_records(25, 5, 0.8);
        let (_, summary) = instrument.run(&series, &records, &config);

        let summary = summary.expect("summary should be present");
        assert_eq!(
            summary.n_windows, 25,
            "PerRun windowing should produce one window per run_id"
        );
    }
}
