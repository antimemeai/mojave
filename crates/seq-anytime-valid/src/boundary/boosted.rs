use crate::boundary::wald;
use crate::error::SeqError;

/// Fischer 2024 truncation function T_alpha(x; M).
/// T_alpha(x; M) = x if M*x <= 1/alpha, else 1/(M*alpha).
pub fn truncation(x: f64, prior_mass: f64, alpha: f64) -> f64 {
    let threshold = 1.0 / alpha;
    if prior_mass * x <= threshold {
        x
    } else {
        1.0 / (prior_mass * alpha)
    }
}

/// Compute the boosted test supermartingale from a sequence of
/// likelihood ratio factors L_t = Lambda_t / Lambda_{t-1}.
///
/// Returns the boosted process values at each step.
/// The boosted process M_t^boost never overshoots 1/alpha.
pub fn boosted_process(lr_factors: &[f64], alpha: f64) -> Result<Vec<f64>, SeqError> {
    wald::validate_error_rates(alpha, 0.5)?;
    if lr_factors.is_empty() {
        return Err(SeqError::EmptyObservations);
    }
    let mut m_boost = 1.0_f64;
    let mut values = Vec::with_capacity(lr_factors.len());
    for &l_t in lr_factors {
        let truncated = truncation(l_t, m_boost, alpha);
        m_boost *= truncated;
        values.push(m_boost);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_identity_below_threshold() {
        let result = truncation(10.0, 0.8, 0.05);
        assert!((result - 10.0).abs() < 1e-10);
    }

    #[test]
    fn truncation_caps_above_threshold() {
        let result = truncation(30.0, 0.8, 0.05);
        assert!((result - 25.0).abs() < 1e-10);
    }

    #[test]
    fn boosted_process_never_overshoots() {
        let factors = vec![2.0, 3.0, 5.0, 10.0, 8.0];
        let alpha = 0.05;
        let threshold = 1.0 / alpha;
        let values = boosted_process(&factors, alpha).unwrap();
        for &v in &values {
            assert!(v <= threshold + 1e-10, "boosted value {v} > {threshold}");
        }
    }
}
