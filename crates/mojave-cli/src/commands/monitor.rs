use std::io::{self, BufRead, Write};
use std::path::Path;

use eval_core::TrialRecord;
use eval_orchestrator::Monitor;

use crate::config::{load_monitor_config, ConfigOverrides};
use crate::error::CliError;
use crate::hint::decision_hint;

pub fn run_monitor_stdin(
    config_path: Option<&Path>,
    overrides: &ConfigOverrides,
) -> Result<(), CliError> {
    let config = load_monitor_config(config_path, overrides)?;
    let mut monitor = Monitor::new(config);
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    for line_result in stdin.lock().lines() {
        let line = line_result?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let record: TrialRecord = serde_json::from_str(trimmed)
            .map_err(|e| CliError::Io(io::Error::other(e.to_string())))?;

        let decisions = monitor.push(&record);
        for decision in decisions {
            let hint = decision_hint(&decision);
            let enriched = serde_json::json!({
                "decision": decision,
                "hint": hint,
            });
            let json_line = serde_json::to_string(&enriched)
                .map_err(|e| CliError::Io(io::Error::other(e.to_string())))?;
            writeln!(stdout_lock, "{json_line}")?;
            stdout_lock.flush()?;
        }
    }

    let summary = monitor.state_summary();
    let summary_json = serde_json::to_string(&summary)
        .map_err(|e| CliError::Io(io::Error::other(e.to_string())))?;
    writeln!(stdout_lock, "{summary_json}")?;

    Ok(())
}

pub fn run_monitor_watch(
    watch_path: &Path,
    config_path: Option<&Path>,
    overrides: &ConfigOverrides,
) -> Result<(), CliError> {
    let config = load_monitor_config(config_path, overrides)?;
    let mut monitor = Monitor::new(config);
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    let content = std::fs::read_to_string(watch_path)?;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let record: TrialRecord = serde_json::from_str(trimmed)
            .map_err(|e| CliError::Io(io::Error::other(e.to_string())))?;

        let decisions = monitor.push(&record);
        for decision in decisions {
            let hint = decision_hint(&decision);
            let enriched = serde_json::json!({
                "decision": decision,
                "hint": hint,
            });
            let json_line = serde_json::to_string(&enriched)
                .map_err(|e| CliError::Io(io::Error::other(e.to_string())))?;
            writeln!(stdout_lock, "{json_line}")?;
        }
    }

    let summary = monitor.state_summary();
    let summary_json = serde_json::to_string(&summary)
        .map_err(|e| CliError::Io(io::Error::other(e.to_string())))?;
    writeln!(stdout_lock, "{summary_json}")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eval_core::{Outcome, TrialRecord};
    use std::collections::BTreeMap;
    use ulid::Ulid;

    fn make_record(task: &str, agent: &str, score: f64, run_id: Ulid) -> TrialRecord {
        TrialRecord {
            trial_id: Ulid::new(),
            run_id,
            task_id: task.into(),
            task_version: None,
            agent_id: agent.into(),
            agent_version: None,
            judge_config: None,
            seed: None,
            timestamp: 1717200000,
            outcome: Outcome::Score(score),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn monitor_produces_summary() {
        let config = eval_orchestrator::MonitorConfig::default();
        let mut monitor = Monitor::new(config);
        let run_id = Ulid::new();
        for _ in 0..5 {
            let record = make_record("t", "a", 0.8, run_id);
            let _ = monitor.push(&record);
        }
        let summary = monitor.state_summary();
        assert_eq!(summary.observations_seen, 5);
    }

    #[test]
    fn monitor_json_line_output_shape() {
        let decision = eval_orchestrator::Decision::ContinueRunning {
            series: eval_orchestrator::SeriesKey {
                task_id: "t".into(),
                agent_id: "a".into(),
                scorer: None,
            },
            current_n: 5,
            estimated_n_needed: 0,
            power_at_current_n: 0.0,
        };
        let hint = decision_hint(&decision);
        let enriched = serde_json::json!({
            "decision": decision,
            "hint": hint,
        });
        let json = serde_json::to_string(&enriched).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["hint"].is_string());
        assert!(parsed["decision"].is_object());
    }
}
