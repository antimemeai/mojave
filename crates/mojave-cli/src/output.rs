use eval_orchestrator::types::{AnalysisReport, Decision};
use serde::Serialize;

use crate::hint::decision_hint;

#[derive(Serialize)]
pub struct AnalyzeOutput {
    pub series_detected: Vec<eval_orchestrator::types::SeriesKey>,
    pub instruments_run: Vec<String>,
    pub decisions: Vec<DecisionWithHint>,
    pub summaries: Summaries,
}

#[derive(Serialize)]
pub struct DecisionWithHint {
    #[serde(flatten)]
    pub decision: Decision,
    pub hint: String,
}

#[derive(Serialize)]
pub struct Summaries {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub irr: Option<eval_orchestrator::types::IrrSummary>,
    pub sequential: Vec<eval_orchestrator::types::SequentialSummary>,
    pub spc: Vec<eval_orchestrator::types::SpcSummary>,
}

impl AnalyzeOutput {
    pub fn from_report(report: AnalysisReport) -> Self {
        let decisions = report
            .decisions
            .into_iter()
            .map(|d| {
                let hint = decision_hint(&d);
                DecisionWithHint { decision: d, hint }
            })
            .collect();

        AnalyzeOutput {
            series_detected: report.series_detected,
            instruments_run: report.instruments_run,
            decisions,
            summaries: Summaries {
                irr: report.irr_results,
                sequential: report.sequential_results,
                spc: report.spc_results,
            },
        }
    }
}

pub fn write_json<T: Serialize>(value: &T) -> Result<(), crate::error::CliError> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| crate::error::CliError::Io(std::io::Error::other(e.to_string())))?;
    println!("{json}");
    Ok(())
}

pub fn write_error(error: &crate::error::CliError) {
    let err_json = serde_json::json!({
        "error": error.to_string(),
        "kind": error.kind(),
    });
    if let Ok(s) = serde_json::to_string(&err_json) {
        eprintln!("{s}");
    } else {
        eprintln!("{error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eval_orchestrator::types::{AnalysisReport, SeriesKey};

    #[test]
    fn analyze_output_from_empty_report() {
        let report = AnalysisReport {
            decisions: vec![],
            irr_results: None,
            sequential_results: vec![],
            spc_results: vec![],
            series_detected: vec![],
            instruments_run: vec![],
        };
        let output = AnalyzeOutput::from_report(report);
        assert!(output.decisions.is_empty());
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"decisions\":[]"));
    }

    #[test]
    fn analyze_output_includes_hints() {
        let report = AnalysisReport {
            decisions: vec![Decision::ContinueRunning {
                series: SeriesKey {
                    task_id: "t".into(),
                    agent_id: "a".into(),
                    scorer: None,
                },
                current_n: 5,
                estimated_n_needed: 0,
                power_at_current_n: 0.0,
            }],
            irr_results: None,
            sequential_results: vec![],
            spc_results: vec![],
            series_detected: vec![],
            instruments_run: vec!["sequential".into()],
        };
        let output = AnalyzeOutput::from_report(report);
        assert_eq!(output.decisions.len(), 1);
        assert!(output.decisions[0].hint.contains("5 observations"));
    }
}
