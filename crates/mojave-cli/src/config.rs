use eval_orchestrator::config::{AnalysisConfig, IrrMetric, MonitorConfig, SpcChartType};

use crate::error::ConfigError;

pub fn load_config(
    config_path: Option<&std::path::Path>,
    overrides: &ConfigOverrides,
) -> Result<AnalysisConfig, ConfigError> {
    let mut config = AnalysisConfig::default();

    if let Some(path) = config_path {
        let contents = std::fs::read_to_string(path).map_err(ConfigError::FileReadError)?;
        let partial: serde_json::Value =
            serde_yaml::from_str(&contents).map_err(|e| ConfigError::ParseError(e.to_string()))?;
        let base =
            serde_json::to_value(&config).map_err(|e| ConfigError::ParseError(e.to_string()))?;
        let merged = merge_json(base, partial);
        config =
            serde_json::from_value(merged).map_err(|e| ConfigError::ParseError(e.to_string()))?;
    }

    apply_overrides(&mut config, overrides);
    Ok(config)
}

pub fn load_monitor_config(
    config_path: Option<&std::path::Path>,
    overrides: &ConfigOverrides,
) -> Result<MonitorConfig, ConfigError> {
    let mut config = MonitorConfig::default();

    if let Some(path) = config_path {
        let contents = std::fs::read_to_string(path).map_err(ConfigError::FileReadError)?;
        let partial: serde_json::Value =
            serde_yaml::from_str(&contents).map_err(|e| ConfigError::ParseError(e.to_string()))?;
        let base =
            serde_json::to_value(&config).map_err(|e| ConfigError::ParseError(e.to_string()))?;
        let merged = merge_json(base, partial);
        config =
            serde_json::from_value(merged).map_err(|e| ConfigError::ParseError(e.to_string()))?;
    }

    if let Some(v) = overrides.irr_threshold {
        config.irr.threshold = v;
    }
    if let Some(ref m) = overrides.irr_metric {
        if let Some(parsed) = parse_irr_metric(m) {
            config.irr.metric = parsed;
        }
    }
    if let Some(ref ct) = overrides.spc_chart {
        if let Some(parsed) = parse_spc_chart(ct) {
            config.spc.chart_type = parsed;
        }
    }
    if let Some(v) = overrides.spc_phase1_windows {
        config.spc.phase1_windows = v;
    }
    if let Some(v) = overrides.sequential_alpha {
        config.sequential.alpha = v;
    }

    Ok(config)
}

#[derive(Debug, Default)]
pub struct ConfigOverrides {
    pub irr_threshold: Option<f64>,
    pub irr_metric: Option<String>,
    pub spc_chart: Option<String>,
    pub spc_phase1_windows: Option<usize>,
    pub sequential_alpha: Option<f64>,
    pub force_enable: Option<String>,
    pub force_disable: Option<String>,
}

fn merge_json(base: serde_json::Value, patch: serde_json::Value) -> serde_json::Value {
    match (base, patch) {
        (serde_json::Value::Object(mut base_map), serde_json::Value::Object(patch_map)) => {
            for (k, v) in patch_map {
                let base_val = base_map.remove(&k).unwrap_or(serde_json::Value::Null);
                base_map.insert(k, merge_json(base_val, v));
            }
            serde_json::Value::Object(base_map)
        }
        (_, patch) => patch,
    }
}

fn apply_overrides(config: &mut AnalysisConfig, overrides: &ConfigOverrides) {
    if let Some(v) = overrides.irr_threshold {
        config.irr.threshold = v;
    }
    if let Some(ref m) = overrides.irr_metric {
        if let Some(parsed) = parse_irr_metric(m) {
            config.irr.metric = parsed;
        }
    }
    if let Some(ref ct) = overrides.spc_chart {
        if let Some(parsed) = parse_spc_chart(ct) {
            config.spc.chart_type = parsed;
        }
    }
    if let Some(v) = overrides.spc_phase1_windows {
        config.spc.phase1_windows = v;
    }
    if let Some(v) = overrides.sequential_alpha {
        config.sequential.alpha = v;
    }
    if let Some(ref fe) = overrides.force_enable {
        config.force_enable = fe.split(',').map(|s| s.trim().to_string()).collect();
    }
    if let Some(ref fd) = overrides.force_disable {
        config.force_disable = fd.split(',').map(|s| s.trim().to_string()).collect();
    }
}

fn parse_irr_metric(s: &str) -> Option<IrrMetric> {
    match s.to_lowercase().as_str() {
        "krippendorff" => Some(IrrMetric::Krippendorff),
        "fleiss" => Some(IrrMetric::Fleiss),
        "gwet" => Some(IrrMetric::Gwet),
        _ => None,
    }
}

fn parse_spc_chart(s: &str) -> Option<SpcChartType> {
    match s.to_lowercase().as_str() {
        "ewma" => Some(SpcChartType::Ewma),
        "cusum" => Some(SpcChartType::Cusum),
        "shewhart" => Some(SpcChartType::Shewhart),
        "combined" => Some(SpcChartType::Combined),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn default_config_when_no_file() {
        let overrides = ConfigOverrides::default();
        let config = load_config(None, &overrides).unwrap();
        assert!((config.irr.threshold - 0.67).abs() < f64::EPSILON);
        assert!((config.sequential.alpha - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn flag_overrides_default() {
        let overrides = ConfigOverrides {
            irr_threshold: Some(0.9),
            sequential_alpha: Some(0.01),
            ..Default::default()
        };
        let config = load_config(None, &overrides).unwrap();
        assert!((config.irr.threshold - 0.9).abs() < f64::EPSILON);
        assert!((config.sequential.alpha - 0.01).abs() < f64::EPSILON);
    }

    #[test]
    fn yaml_file_loads() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "irr:\n  threshold: 0.8\nspc:\n  chart_type: Cusum\n").unwrap();
        let overrides = ConfigOverrides::default();
        let config = load_config(Some(tmp.path()), &overrides).unwrap();
        assert!((config.irr.threshold - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn flag_overrides_yaml_file() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "irr:\n  threshold: 0.8\n").unwrap();
        let overrides = ConfigOverrides {
            irr_threshold: Some(0.95),
            ..Default::default()
        };
        let config = load_config(Some(tmp.path()), &overrides).unwrap();
        assert!((config.irr.threshold - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn bad_yaml_returns_parse_error() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "{{{{invalid yaml").unwrap();
        let overrides = ConfigOverrides::default();
        let result = load_config(Some(tmp.path()), &overrides);
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }

    #[test]
    fn force_enable_splits_comma_list() {
        let overrides = ConfigOverrides {
            force_enable: Some("irr, spc".into()),
            ..Default::default()
        };
        let config = load_config(None, &overrides).unwrap();
        assert_eq!(config.force_enable, vec!["irr", "spc"]);
    }

    #[test]
    fn parse_irr_metric_variants() {
        assert!(matches!(
            parse_irr_metric("krippendorff"),
            Some(IrrMetric::Krippendorff)
        ));
        assert!(matches!(
            parse_irr_metric("Fleiss"),
            Some(IrrMetric::Fleiss)
        ));
        assert!(matches!(parse_irr_metric("gwet"), Some(IrrMetric::Gwet)));
        assert!(parse_irr_metric("bogus").is_none());
    }

    #[test]
    fn parse_spc_chart_variants() {
        assert!(matches!(parse_spc_chart("ewma"), Some(SpcChartType::Ewma)));
        assert!(matches!(
            parse_spc_chart("CUSUM"),
            Some(SpcChartType::Cusum)
        ));
        assert!(parse_spc_chart("bogus").is_none());
    }
}
