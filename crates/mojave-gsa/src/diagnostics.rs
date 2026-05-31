use serde::Serialize;

use crate::analyze::SobolIndexEntry;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SobolDiagnosticEntry {
    pub factor: String,
    pub kind: DiagnosticKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum DiagnosticKind {
    NegativeS1,
    CiCrossesBound,
    SumStExceedsThreshold,
    CiWidthExceedsThreshold,
    RecommendDoubleN,
}

pub struct DiagnosticConfig {
    pub ci_width_ratio_threshold: f64,
    pub sum_st_threshold: f64,
}

impl Default for DiagnosticConfig {
    fn default() -> Self {
        Self {
            ci_width_ratio_threshold: 0.10,
            sum_st_threshold: 1.3,
        }
    }
}

pub fn run_diagnostics(
    indices: &[SobolIndexEntry],
    config: &DiagnosticConfig,
) -> Vec<SobolDiagnosticEntry> {
    let mut diagnostics = Vec::new();

    for entry in indices {
        if entry.s1 < 0.0 {
            diagnostics.push(SobolDiagnosticEntry {
                factor: entry.axis.clone(),
                kind: DiagnosticKind::NegativeS1,
                message: format!(
                    "S1_{} = {:.4} is negative, indicating insufficient N or model misspecification",
                    entry.axis, entry.s1
                ),
            });
        }

        if entry.s1_ci_low < 0.0 && entry.s1_ci_high > 0.0 {
            diagnostics.push(SobolDiagnosticEntry {
                factor: entry.axis.clone(),
                kind: DiagnosticKind::CiCrossesBound,
                message: format!(
                    "S1_{} CI [{:.4}, {:.4}] crosses zero — index sign is uncertain",
                    entry.axis, entry.s1_ci_low, entry.s1_ci_high
                ),
            });
        }

        if entry.st_ci_low < 0.0 || entry.st_ci_high > 1.0 {
            diagnostics.push(SobolDiagnosticEntry {
                factor: entry.axis.clone(),
                kind: DiagnosticKind::CiCrossesBound,
                message: format!(
                    "ST_{} CI [{:.4}, {:.4}] crosses [0,1] boundary",
                    entry.axis, entry.st_ci_low, entry.st_ci_high
                ),
            });
        }

        let s1_width = entry.s1_ci_high - entry.s1_ci_low;
        if entry.s1.abs() > 1e-10 && s1_width / entry.s1.abs() > config.ci_width_ratio_threshold {
            diagnostics.push(SobolDiagnosticEntry {
                factor: entry.axis.clone(),
                kind: DiagnosticKind::CiWidthExceedsThreshold,
                message: format!(
                    "S1_{} CI width {:.4} exceeds {:.0}% of point estimate {:.4}",
                    entry.axis,
                    s1_width,
                    config.ci_width_ratio_threshold * 100.0,
                    entry.s1
                ),
            });
        }
    }

    let sum_st: f64 = indices.iter().map(|e| e.st).sum();
    if sum_st > config.sum_st_threshold {
        diagnostics.push(SobolDiagnosticEntry {
            factor: "(global)".to_string(),
            kind: DiagnosticKind::SumStExceedsThreshold,
            message: format!(
                "sum(ST) = {sum_st:.4} > {:.1} — substantial factor interactions or insufficient N",
                config.sum_st_threshold
            ),
        });
    }

    let needs_double_n = diagnostics.iter().any(|d| {
        matches!(
            d.kind,
            DiagnosticKind::NegativeS1 | DiagnosticKind::CiWidthExceedsThreshold
        )
    });
    if needs_double_n {
        diagnostics.push(SobolDiagnosticEntry {
            factor: "(global)".to_string(),
            kind: DiagnosticKind::RecommendDoubleN,
            message: "Convergence issues detected — recommend doubling N_base".to_string(),
        });
    }

    diagnostics
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn make_entry(
        axis: &str,
        s1: f64,
        s1_ci: (f64, f64),
        st: f64,
        st_ci: (f64, f64),
    ) -> SobolIndexEntry {
        SobolIndexEntry {
            axis: axis.to_string(),
            s1,
            s1_ci_low: s1_ci.0,
            s1_ci_high: s1_ci.1,
            st,
            st_ci_low: st_ci.0,
            st_ci_high: st_ci.1,
        }
    }

    #[test]
    fn negative_s1_triggers_warning() {
        let indices = vec![
            make_entry("quantization", -0.070, (-0.12, -0.02), 0.05, (0.01, 0.10)),
            make_entry("prompt_template", 0.85, (0.70, 0.95), 0.90, (0.80, 0.95)),
        ];
        let diags = run_diagnostics(&indices, &DiagnosticConfig::default());
        assert!(
            diags
                .iter()
                .any(|d| d.kind == DiagnosticKind::NegativeS1 && d.factor == "quantization"),
            "expected NegativeS1 for quantization: {diags:?}"
        );
    }

    #[test]
    fn ci_crossing_zero_triggers_warning() {
        let indices = vec![make_entry(
            "decoding",
            0.02,
            (-0.05, 0.08),
            0.04,
            (0.01, 0.09),
        )];
        let diags = run_diagnostics(&indices, &DiagnosticConfig::default());
        assert!(
            diags
                .iter()
                .any(|d| d.kind == DiagnosticKind::CiCrossesBound && d.factor == "decoding"),
            "expected CiCrossesBound for decoding: {diags:?}"
        );
    }

    #[test]
    fn sum_st_exceeding_threshold_triggers_warning() {
        let indices = vec![
            make_entry("a", 0.40, (0.35, 0.45), 0.50, (0.45, 0.55)),
            make_entry("b", 0.30, (0.25, 0.35), 0.45, (0.40, 0.50)),
            make_entry("c", 0.20, (0.15, 0.25), 0.40, (0.35, 0.45)),
        ];
        let config = DiagnosticConfig {
            sum_st_threshold: 1.3,
            ..Default::default()
        };
        let diags = run_diagnostics(&indices, &config);
        assert!(
            diags
                .iter()
                .any(|d| d.kind == DiagnosticKind::SumStExceedsThreshold),
            "expected SumStExceedsThreshold: sum_st=1.35, {diags:?}"
        );
    }

    #[test]
    fn ci_width_exceeding_threshold_triggers_doubling() {
        let indices = vec![make_entry(
            "prompt_template",
            0.85,
            (0.41, 0.85),
            0.90,
            (0.80, 0.95),
        )];
        let config = DiagnosticConfig {
            ci_width_ratio_threshold: 0.10,
            ..Default::default()
        };
        let diags = run_diagnostics(&indices, &config);
        assert!(
            diags
                .iter()
                .any(|d| d.kind == DiagnosticKind::CiWidthExceedsThreshold),
            "expected CiWidthExceedsThreshold: {diags:?}"
        );
        assert!(
            diags
                .iter()
                .any(|d| d.kind == DiagnosticKind::RecommendDoubleN),
            "expected RecommendDoubleN: {diags:?}"
        );
    }

    #[test]
    fn clean_results_produce_no_diagnostics() {
        let indices = vec![
            make_entry("a", 0.50, (0.48, 0.52), 0.55, (0.52, 0.58)),
            make_entry("b", 0.30, (0.29, 0.31), 0.35, (0.32, 0.38)),
        ];
        let config = DiagnosticConfig::default();
        let diags = run_diagnostics(&indices, &config);
        assert!(diags.is_empty(), "expected no diagnostics: {diags:?}");
    }
}
