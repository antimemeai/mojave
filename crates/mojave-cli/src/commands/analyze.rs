use std::path::{Path, PathBuf};

use crate::commands::ingest::run_ingest;
use crate::config::{load_config, ConfigOverrides};
use crate::error::CliError;
use crate::output::AnalyzeOutput;

pub fn run_analyze(
    paths: &[PathBuf],
    config_path: Option<&Path>,
    overrides: &ConfigOverrides,
) -> Result<AnalyzeOutput, CliError> {
    let ingest_output = run_ingest(paths, "auto", None)?;

    if ingest_output.records.is_empty() {
        return Err(CliError::Orchestrator(
            eval_orchestrator::OrchestratorError::EmptyInput,
        ));
    }

    let config = load_config(config_path, overrides)?;
    let report = eval_orchestrator::analyze(&ingest_output.records, &config)?;

    Ok(AnalyzeOutput::from_report(report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigOverrides;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../eval-ingest/tests/fixtures")
            .join(name)
    }

    #[test]
    fn analyze_inspect_binary() {
        let paths = vec![fixture_path("inspect_binary.json")];
        let overrides = ConfigOverrides::default();
        let output = run_analyze(&paths, None, &overrides).unwrap();
        assert!(
            !output.series_detected.is_empty(),
            "should detect at least one series"
        );
        let json = serde_json::to_string(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["decisions"].is_array());
        assert!(parsed["instruments_run"].is_array());
        assert!(parsed["summaries"].is_object());
    }

    #[test]
    fn analyze_inspect_series_detected() {
        let paths = vec![fixture_path("inspect_binary.json")];
        let overrides = ConfigOverrides::default();
        let output = run_analyze(&paths, None, &overrides).unwrap();
        assert!(!output.series_detected.is_empty());
    }

    #[test]
    fn analyze_with_config_override() {
        let paths = vec![fixture_path("inspect_binary.json")];
        let overrides = ConfigOverrides {
            sequential_alpha: Some(0.01),
            ..Default::default()
        };
        let _output = run_analyze(&paths, None, &overrides).unwrap();
    }

    #[test]
    fn analyze_decisions_have_hints() {
        let paths = vec![fixture_path("inspect_binary.json")];
        let overrides = ConfigOverrides::default();
        let output = run_analyze(&paths, None, &overrides).unwrap();
        for d in &output.decisions {
            assert!(
                !d.hint.is_empty(),
                "every decision should have a non-empty hint"
            );
        }
    }
}
