use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub force_enable: Vec<String>,
    pub force_disable: Vec<String>,
    pub irr: IrrConfig,
    pub sequential: SequentialConfig,
    pub spc: SpcConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrrConfig {
    pub threshold: f64,
    pub metric: IrrMetric,
    pub min_raters: usize,
}

impl Default for IrrConfig {
    fn default() -> Self {
        Self {
            threshold: 0.67,
            metric: IrrMetric::Krippendorff,
            min_raters: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrrMetric {
    Krippendorff,
    Fleiss,
    Gwet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialConfig {
    pub alpha: f64,
    pub min_effect_size: f64,
    pub method: SequentialMethod,
    pub mixing_variance: f64,
}

impl Default for SequentialConfig {
    fn default() -> Self {
        Self {
            alpha: 0.05,
            min_effect_size: 0.1,
            method: SequentialMethod::Msprt,
            mixing_variance: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SequentialMethod {
    Msprt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpcConfig {
    pub chart_type: SpcChartType,
    pub phase1_windows: usize,
    pub window_size: WindowSize,
    pub lambda: f64,
    pub l_sigma: f64,
}

impl Default for SpcConfig {
    fn default() -> Self {
        Self {
            chart_type: SpcChartType::Ewma,
            phase1_windows: 20,
            window_size: WindowSize::PerRun,
            lambda: 0.2,
            l_sigma: 2.962,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpcChartType {
    Ewma,
    Cusum,
    Shewhart,
    Combined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowSize {
    PerRun,
    Fixed(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub spc: SpcConfig,
    pub sequential: SequentialConfig,
    pub irr: IrrConfig,
    pub irr_recompute_interval: usize,
    pub auto_detect: bool,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            spc: SpcConfig::default(),
            sequential: SequentialConfig::default(),
            irr: IrrConfig::default(),
            irr_recompute_interval: 50,
            auto_detect: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_config_default_has_sane_values() {
        let cfg = AnalysisConfig::default();
        assert!(cfg.force_enable.is_empty());
        assert!(cfg.force_disable.is_empty());
        assert!((cfg.irr.threshold - 0.67).abs() < f64::EPSILON);
        assert!((cfg.sequential.alpha - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn irr_config_default() {
        let cfg = IrrConfig::default();
        assert!(matches!(cfg.metric, IrrMetric::Krippendorff));
        assert_eq!(cfg.min_raters, 2);
        assert!((cfg.threshold - 0.67).abs() < f64::EPSILON);
    }

    #[test]
    fn sequential_config_default() {
        let cfg = SequentialConfig::default();
        assert!((cfg.alpha - 0.05).abs() < f64::EPSILON);
        assert!((cfg.min_effect_size - 0.1).abs() < f64::EPSILON);
        assert!(matches!(cfg.method, SequentialMethod::Msprt));
        assert!((cfg.mixing_variance - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn spc_config_default() {
        let cfg = SpcConfig::default();
        assert!(matches!(cfg.chart_type, SpcChartType::Ewma));
        assert_eq!(cfg.phase1_windows, 20);
        assert!(matches!(cfg.window_size, WindowSize::PerRun));
        assert!((cfg.lambda - 0.2).abs() < f64::EPSILON);
        assert!((cfg.l_sigma - 2.962).abs() < f64::EPSILON);
    }

    #[test]
    fn monitor_config_default() {
        let cfg = MonitorConfig::default();
        assert_eq!(cfg.irr_recompute_interval, 50);
        assert!(cfg.auto_detect);
    }

    #[test]
    fn analysis_config_roundtrip_serde() {
        let original = AnalysisConfig::default();
        let json = serde_json::to_string(&original).unwrap();
        let recovered: AnalysisConfig = serde_json::from_str(&json).unwrap();
        assert!((recovered.irr.threshold - original.irr.threshold).abs() < f64::EPSILON);
        assert!((recovered.sequential.alpha - original.sequential.alpha).abs() < f64::EPSILON);
        assert_eq!(recovered.force_enable, original.force_enable);
        assert_eq!(recovered.force_disable, original.force_disable);
    }

    #[test]
    fn monitor_config_roundtrip_serde() {
        let original = MonitorConfig::default();
        let json = serde_json::to_string(&original).unwrap();
        let recovered: MonitorConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(
            recovered.irr_recompute_interval,
            original.irr_recompute_interval
        );
        assert_eq!(recovered.auto_detect, original.auto_detect);
        assert!((recovered.spc.lambda - original.spc.lambda).abs() < f64::EPSILON);
    }
}
