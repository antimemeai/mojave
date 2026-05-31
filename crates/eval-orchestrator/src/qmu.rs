use serde::{Deserialize, Serialize};

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
    pub fn evaluate(
        estimate: f64,
        expanded_uncertainty: f64,
        threshold: f64,
        guard_band: Option<f64>,
    ) -> Self {
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

        Self {
            estimate,
            expanded_uncertainty,
            threshold,
            margin,
            confidence_ratio,
            guard_band: if g == 0.0 { guard_band } else { Some(g) },
            acceptance_limit,
            decision,
        }
    }
}

/// Compute JCGM 106 guard band width for a target consumer risk.
///
/// For a normal measurement PDF with a single lower tolerance limit:
///   g = r * U  where  r = Phi^{-1}(1 - consumer_risk) / k
///
/// - `expanded_uncertainty`: U = k * u (expanded uncertainty)
/// - `coverage_factor`: k (typically 2)
/// - `consumer_risk`: target probability of accepting a non-conforming item
///
/// Returns the guard band width g.
pub fn jcgm106_guard_band(
    expanded_uncertainty: f64,
    coverage_factor: f64,
    consumer_risk: f64,
) -> f64 {
    let z = probit(1.0 - consumer_risk);
    let r = z / coverage_factor;
    r * expanded_uncertainty
}

/// Probit function: inverse of the standard normal CDF.
/// Uses the rational approximation from Abramowitz & Stegun (1964) 26.2.23.
fn probit(p: f64) -> f64 {
    assert!((0.0..=1.0).contains(&p), "p must be in [0, 1]");

    if p == 0.0 {
        return f64::NEG_INFINITY;
    }
    if p == 1.0 {
        return f64::INFINITY;
    }
    if (p - 0.5).abs() < f64::EPSILON {
        return 0.0;
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

    sign * (t - numerator / denominator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probit_known_values() {
        assert!((probit(0.5) - 0.0).abs() < 1e-6);
        assert!((probit(0.975) - 1.96).abs() < 0.01);
        assert!((probit(0.95) - 1.645).abs() < 0.01);
        assert!((probit(0.99) - 2.326).abs() < 0.01);
        assert!((probit(0.977) - 2.0).abs() < 0.01);
    }

    #[test]
    fn probit_symmetry() {
        for &p in &[0.1, 0.2, 0.3, 0.4] {
            assert!((probit(p) + probit(1.0 - p)).abs() < 1e-10);
        }
    }
}
