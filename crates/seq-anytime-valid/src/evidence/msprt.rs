use crate::error::SeqError;

/// Compute the marginalized log-likelihood ratio Lambda_n for Gaussian data
/// with Gaussian mixing distribution N(theta_0, tau^2).
///
/// For known-variance sigma^2 = 1, the closed-form is:
/// log(Lambda_n) = -0.5 * ln(1 + n*tau^2) + n^2 * xbar^2 * tau^2 / (2 * (1 + n*tau^2))
pub fn gaussian_msprt_log_lr(
    observations: &[f64],
    theta_0: f64,
    mixing_variance: f64,
) -> Result<f64, SeqError> {
    if observations.is_empty() {
        return Ok(0.0);
    }
    if mixing_variance <= 0.0 {
        return Err(SeqError::InvalidMixingVariance(mixing_variance));
    }
    for &x in observations {
        if !x.is_finite() {
            return Err(SeqError::NonFiniteInput(x));
        }
    }
    let n = observations.len() as f64;
    let tau_sq = mixing_variance;
    let x_bar: f64 = observations.iter().map(|&x| x - theta_0).sum::<f64>() / n;
    let log_lr = -0.5 * (1.0 + n * tau_sq).ln()
        + n.powi(2) * x_bar.powi(2) * tau_sq / (2.0 * (1.0 + n * tau_sq));
    Ok(log_lr)
}

/// Always-valid p-value: p_n = min(1, 1/Lambda_n).
pub fn always_valid_p(
    observations: &[f64],
    theta_0: f64,
    mixing_variance: f64,
) -> Result<f64, SeqError> {
    let log_lr = gaussian_msprt_log_lr(observations, theta_0, mixing_variance)?;
    let lr = log_lr.exp();
    Ok((1.0 / lr).min(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_observations_gives_p_one() {
        let p = always_valid_p(&[], 0.0, 1.0).unwrap();
        assert!((p - 1.0).abs() < 1e-10);
    }

    #[test]
    fn strong_signal_gives_low_p() {
        let obs: Vec<f64> = vec![2.0; 10];
        let p = always_valid_p(&obs, 0.0, 1.0).unwrap();
        assert!(p < 0.05, "strong signal should give p < 0.05, got {p}");
    }

    #[test]
    fn null_data_gives_high_p() {
        let obs: Vec<f64> = vec![0.0; 10];
        let p = always_valid_p(&obs, 0.0, 1.0).unwrap();
        assert!(p >= 0.5, "null data should give high p, got {p}");
    }

    #[test]
    fn invalid_mixing_variance() {
        let obs = vec![1.0];
        assert!(matches!(
            always_valid_p(&obs, 0.0, -1.0),
            Err(SeqError::InvalidMixingVariance(_))
        ));
    }
}
