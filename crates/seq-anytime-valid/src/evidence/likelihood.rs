use crate::error::SeqError;

pub fn validate_observation(x: f64) -> Result<(), SeqError> {
    if !x.is_finite() {
        return Err(SeqError::NonFiniteInput(x));
    }
    Ok(())
}

pub fn validate_observations(observations: &[f64]) -> Result<(), SeqError> {
    if observations.is_empty() {
        return Err(SeqError::EmptyObservations);
    }
    for &x in observations {
        validate_observation(x)?;
    }
    Ok(())
}

/// Log-likelihood ratio for a single Bernoulli observation.
/// LLR_i = x_i * ln(p1/p0) + (1 - x_i) * ln((1-p1)/(1-p0))
pub fn bernoulli_log_lr(x: f64, p0: f64, p1: f64) -> f64 {
    x * (p1 / p0).ln() + (1.0 - x) * ((1.0 - p1) / (1.0 - p0)).ln()
}

/// Cumulative log-likelihood ratio for Bernoulli observations.
pub fn bernoulli_cumulative_log_lr(
    observations: &[f64],
    p0: f64,
    p1: f64,
) -> Result<f64, SeqError> {
    validate_observations(observations)?;
    if (p0 - p1).abs() < f64::EPSILON {
        return Err(SeqError::DegenerateHypotheses);
    }
    Ok(observations
        .iter()
        .map(|&x| bernoulli_log_lr(x, p0, p1))
        .sum())
}

/// Log-likelihood ratio for a single Normal observation with known variance.
/// LLR_i = (mu1 - mu0) * x_i / sigma^2 - (mu1^2 - mu0^2) / (2 * sigma^2)
pub fn normal_log_lr(x: f64, mu0: f64, mu1: f64, sigma_sq: f64) -> f64 {
    (mu1 - mu0) * x / sigma_sq - (mu1.powi(2) - mu0.powi(2)) / (2.0 * sigma_sq)
}

/// Cumulative log-likelihood ratio for Normal observations (known variance).
pub fn normal_cumulative_log_lr(
    observations: &[f64],
    mu0: f64,
    mu1: f64,
    sigma_sq: f64,
) -> Result<f64, SeqError> {
    validate_observations(observations)?;
    if (mu0 - mu1).abs() < f64::EPSILON {
        return Err(SeqError::DegenerateHypotheses);
    }
    Ok(observations
        .iter()
        .map(|&x| normal_log_lr(x, mu0, mu1, sigma_sq))
        .sum())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bernoulli_log_lr_success_contributes_positive() {
        let llr = bernoulli_log_lr(1.0, 0.1, 0.2);
        assert!(llr > 0.0, "success under higher p1 should be positive");
    }

    #[test]
    fn bernoulli_log_lr_failure_contributes_negative() {
        let llr = bernoulli_log_lr(0.0, 0.1, 0.2);
        assert!(llr < 0.0, "failure under higher p1 should be negative");
    }

    #[test]
    fn bernoulli_cumulative_textbook() {
        let obs: Vec<f64> = vec![1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let llr = bernoulli_cumulative_log_lr(&obs, 0.5, 0.7).unwrap();
        let expected = 5.0 * (0.7_f64 / 0.5).ln() + 5.0 * (0.3_f64 / 0.5).ln();
        assert!((llr - expected).abs() < 1e-10);
    }

    #[test]
    fn normal_log_lr_positive_when_obs_favors_h1() {
        let llr = normal_log_lr(1.0, 0.0, 1.0, 1.0);
        assert!((llr - 0.5).abs() < 1e-10);
    }

    #[test]
    fn degenerate_returns_error() {
        let obs = vec![1.0];
        assert!(matches!(
            bernoulli_cumulative_log_lr(&obs, 0.5, 0.5),
            Err(SeqError::DegenerateHypotheses)
        ));
        assert!(matches!(
            normal_cumulative_log_lr(&obs, 0.0, 0.0, 1.0),
            Err(SeqError::DegenerateHypotheses)
        ));
    }

    #[test]
    fn empty_observations_returns_error() {
        let obs: Vec<f64> = vec![];
        assert!(matches!(
            bernoulli_cumulative_log_lr(&obs, 0.1, 0.2),
            Err(SeqError::EmptyObservations)
        ));
    }

    #[test]
    fn nan_input_returns_error() {
        let obs = vec![f64::NAN];
        assert!(matches!(
            bernoulli_cumulative_log_lr(&obs, 0.1, 0.2),
            Err(SeqError::NonFiniteInput(_))
        ));
    }
}
