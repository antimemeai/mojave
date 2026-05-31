use crate::types::SequentialSummary;
use serde::{Deserialize, Serialize};

/// Errors from QMU conformity assessment.
#[derive(Debug, thiserror::Error)]
pub enum QmuError {
    #[error("expanded_uncertainty must be non-negative, got {0}")]
    NegativeUncertainty(f64),
    #[error("inverted confidence interval: ci_hi ({ci_hi}) < ci_lo ({ci_lo})")]
    InvertedCi { ci_lo: f64, ci_hi: f64 },
    #[error("coverage_factor must be positive, got {0}")]
    NonPositiveCoverageFactor(f64),
    #[error("consumer_risk must be in (0, 1), got {0}")]
    InvalidConsumerRisk(f64),
    #[error("probit argument p must be in [0, 1], got {0}")]
    ProbitOutOfRange(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QmuAssessment {
    pub estimate: f64,
    pub expanded_uncertainty: f64,
    pub threshold: f64,
    pub margin: f64,
    pub confidence_ratio: f64,
    pub guard_band: Option<f64>,
    pub acceptance_limit: f64,
    pub decision: ConformityDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConformityDecision {
    Accept,
    Reject,
    Investigate { reason: String },
}

impl QmuAssessment {
    /// Evaluate a QMU conformity assessment.
    ///
    /// For a single lower tolerance limit (performance must exceed `threshold`):
    /// - margin = estimate - threshold
    /// - CR = margin / expanded_uncertainty  (Sharp & Wood-Schultz 2003)
    /// - acceptance_limit = threshold + guard_band  (JCGM 106 Section 8.3.2)
    ///
    /// Decision rule (three-tier, incorporating CI):
    /// - Accept:  estimate - U >= acceptance_limit  (entire CI above guarded threshold)
    /// - Reject:  estimate + U < threshold           (entire CI below raw threshold)
    /// - Investigate: otherwise
    ///
    /// Returns `Err` if `expanded_uncertainty` is negative.
    pub fn evaluate(
        estimate: f64,
        expanded_uncertainty: f64,
        threshold: f64,
        guard_band: Option<f64>,
    ) -> Result<Self, QmuError> {
        if expanded_uncertainty < 0.0 {
            return Err(QmuError::NegativeUncertainty(expanded_uncertainty));
        }

        let margin = estimate - threshold;
        let confidence_ratio = if expanded_uncertainty > 0.0 {
            margin / expanded_uncertainty
        } else if margin > 0.0 {
            f64::INFINITY
        } else if margin < 0.0 {
            f64::NEG_INFINITY
        } else {
            f64::NAN
        };

        let g = guard_band.unwrap_or(0.0);
        let acceptance_limit = threshold + g;

        let decision = if expanded_uncertainty == 0.0 {
            if estimate >= acceptance_limit {
                ConformityDecision::Accept
            } else if estimate < threshold {
                ConformityDecision::Reject
            } else {
                ConformityDecision::Investigate {
                    reason: format!(
                        "CR={confidence_ratio:.2}, zero uncertainty at guard band boundary"
                    ),
                }
            }
        } else if estimate - expanded_uncertainty >= acceptance_limit {
            ConformityDecision::Accept
        } else if estimate + expanded_uncertainty < threshold {
            ConformityDecision::Reject
        } else {
            ConformityDecision::Investigate {
                reason: format!(
                    "CR={confidence_ratio:.2}, CI [{:.4}, {:.4}] overlaps decision boundary",
                    estimate - expanded_uncertainty,
                    estimate + expanded_uncertainty
                ),
            }
        };

        Ok(Self {
            estimate,
            expanded_uncertainty,
            threshold,
            margin,
            confidence_ratio,
            guard_band,
            acceptance_limit,
            decision,
        })
    }

    /// Construct a QMU assessment from pipeline outputs.
    ///
    /// Derives estimate and expanded uncertainty from the confidence interval
    /// in a [`SequentialSummary`]:
    /// - estimate = (ci_lo + ci_hi) / 2
    /// - expanded_uncertainty = (ci_hi - ci_lo) / 2
    ///
    /// This is the primary composition point connecting the sequential testing
    /// pipeline to the QMU decision framework.
    ///
    /// Returns `Err` if `ci_hi < ci_lo` (inverted interval) or if the derived
    /// expanded uncertainty is negative.
    pub fn from_pipeline(
        sequential: &SequentialSummary,
        threshold: f64,
        guard_band: Option<f64>,
    ) -> Result<Self, QmuError> {
        let (ci_lo, ci_hi) = sequential.ci;
        if ci_hi < ci_lo {
            return Err(QmuError::InvertedCi { ci_lo, ci_hi });
        }
        let estimate = (ci_lo + ci_hi) / 2.0;
        let expanded_uncertainty = (ci_hi - ci_lo) / 2.0;
        Self::evaluate(estimate, expanded_uncertainty, threshold, guard_band)
    }
}

/// Compute JCGM 106 guard band width for a target consumer risk.
///
/// For a normal measurement PDF with a single lower tolerance limit:
///   g = r * U  where  r = Phi^{-1}(1 - consumer_risk) / k
///
/// - `expanded_uncertainty`: U = k * u (expanded uncertainty)
/// - `coverage_factor`: k (typically 2), must be > 0
/// - `consumer_risk`: target probability of accepting a non-conforming item, must be in (0, 1)
///
/// Returns the guard band width g, or an error if inputs are invalid.
pub fn jcgm106_guard_band(
    expanded_uncertainty: f64,
    coverage_factor: f64,
    consumer_risk: f64,
) -> Result<f64, QmuError> {
    if coverage_factor <= 0.0 {
        return Err(QmuError::NonPositiveCoverageFactor(coverage_factor));
    }
    if !(0.0 < consumer_risk && consumer_risk < 1.0) {
        return Err(QmuError::InvalidConsumerRisk(consumer_risk));
    }
    let z = probit(1.0 - consumer_risk)?;
    let r = z / coverage_factor;
    Ok(r * expanded_uncertainty)
}

/// Probit function: inverse of the standard normal CDF.
/// Uses the rational approximation from Abramowitz & Stegun (1964) 26.2.23.
///
/// Returns `Err` if `p` is outside [0, 1].
fn probit(p: f64) -> Result<f64, QmuError> {
    if !(0.0..=1.0).contains(&p) {
        return Err(QmuError::ProbitOutOfRange(p));
    }

    if p == 0.0 {
        return Ok(f64::NEG_INFINITY);
    }
    if p == 1.0 {
        return Ok(f64::INFINITY);
    }
    if (p - 0.5).abs() < f64::EPSILON {
        return Ok(0.0);
    }

    let sign;
    let t_input;
    if p < 0.5 {
        sign = -1.0;
        t_input = p;
    } else {
        sign = 1.0;
        t_input = 1.0 - p;
    };

    let t = (-2.0 * t_input.ln()).sqrt();

    // Abramowitz & Stegun 26.2.23 rational approximation
    const C0: f64 = 2.515517;
    const C1: f64 = 0.802853;
    const C2: f64 = 0.010328;
    const D1: f64 = 1.432788;
    const D2: f64 = 0.189269;
    const D3: f64 = 0.001308;

    let numerator = C0 + C1 * t + C2 * t * t;
    let denominator = 1.0 + D1 * t + D2 * t * t + D3 * t * t * t;

    Ok(sign * (t - numerator / denominator))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probit_known_values() {
        assert!((probit(0.5).unwrap() - 0.0).abs() < 1e-6);
        assert!((probit(0.975).unwrap() - 1.96).abs() < 0.01);
        assert!((probit(0.95).unwrap() - 1.645).abs() < 0.01);
        assert!((probit(0.99).unwrap() - 2.326).abs() < 0.01);
        assert!((probit(0.977).unwrap() - 2.0).abs() < 0.01);
    }

    #[test]
    fn probit_symmetry() {
        for &p in &[0.1, 0.2, 0.3, 0.4] {
            assert!((probit(p).unwrap() + probit(1.0 - p).unwrap()).abs() < 1e-10);
        }
    }

    #[test]
    fn probit_out_of_range_returns_error() {
        assert!(probit(-0.1).is_err());
        assert!(probit(1.5).is_err());
    }

    #[test]
    fn evaluate_rejects_negative_uncertainty() {
        let result = QmuAssessment::evaluate(0.80, -0.04, 0.70, None);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("non-negative"), "error: {msg}");
    }

    #[test]
    fn from_pipeline_rejects_inverted_ci() {
        let summary = SequentialSummary {
            series: crate::types::SeriesKey {
                task_id: "t".into(),
                agent_id: "a".into(),
                scorer: None,
            },
            n_observations: 100,
            current_estimate: 0.80,
            ci: (0.90, 0.70), // inverted: ci_hi < ci_lo
            evidence: 50.0,
            stopped: true,
        };
        let result = QmuAssessment::from_pipeline(&summary, 0.70, None);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("inverted"), "error: {msg}");
    }

    #[test]
    fn guard_band_rejects_non_positive_coverage_factor() {
        let result = jcgm106_guard_band(0.10, 0.0, 0.05);
        assert!(result.is_err());
        let result = jcgm106_guard_band(0.10, -1.0, 0.05);
        assert!(result.is_err());
    }

    #[test]
    fn guard_band_rejects_invalid_consumer_risk() {
        let result = jcgm106_guard_band(0.10, 2.0, 0.0);
        assert!(result.is_err());
        let result = jcgm106_guard_band(0.10, 2.0, 1.0);
        assert!(result.is_err());
        let result = jcgm106_guard_band(0.10, 2.0, -0.1);
        assert!(result.is_err());
    }
}
