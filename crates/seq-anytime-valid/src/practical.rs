use crate::error::SeqError;

/// Max likelihood ratio test for practical significance.
///
/// Tests H0': |theta| < delta vs H1': |theta| >= delta. Computes the
/// maximum of the likelihood ratios for theta = +delta and theta = -delta
/// (both vs theta = 0, normal with sigma^2 = 1), and returns the reciprocal
/// as a p-value analog capped at 1.0. Smaller values indicate stronger
/// evidence for practical significance.
pub fn practical_significance_p(observations: &[f64], delta: f64) -> Result<f64, SeqError> {
    if delta <= 0.0 {
        return Err(SeqError::InvalidPracticalDelta(delta));
    }
    if observations.is_empty() {
        return Ok(1.0);
    }
    for &x in observations {
        if !x.is_finite() {
            return Err(SeqError::NonFiniteInput(x));
        }
    }
    let n = observations.len() as f64;
    let x_bar: f64 = observations.iter().sum::<f64>() / n;
    // LR for theta = +delta vs theta = 0 (normal, sigma^2 = 1)
    let log_lr_pos = n * delta * x_bar - n * delta.powi(2) / 2.0;
    // LR for theta = -delta vs theta = 0
    let log_lr_neg = -n * delta * x_bar - n * delta.powi(2) / 2.0;
    let log_lr_max = log_lr_pos.max(log_lr_neg);
    let lr_max = log_lr_max.exp();
    Ok((1.0 / lr_max).min(1.0))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn large_effect_detected() {
        let obs: Vec<f64> = vec![2.0; 20];
        let p = practical_significance_p(&obs, 0.5).unwrap();
        assert!(p < 0.05, "large effect should give p < 0.05, got {p}");
    }

    #[test]
    fn small_effect_not_detected() {
        let obs: Vec<f64> = vec![0.1; 20];
        let p = practical_significance_p(&obs, 1.0).unwrap();
        assert!(p > 0.05, "small effect should give p > 0.05, got {p}");
    }

    #[test]
    fn invalid_delta() {
        assert!(matches!(
            practical_significance_p(&[1.0], -0.5),
            Err(SeqError::InvalidPracticalDelta(_))
        ));
    }

    #[test]
    fn empty_observations_gives_p_one() {
        let p = practical_significance_p(&[], 0.5).unwrap();
        assert!((p - 1.0).abs() < 1e-10);
    }
}
