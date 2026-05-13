use crate::error::SeqError;

/// Siegmund 1985 Ch.4: First-order bias correction for the MLE at
/// stopping time of a normal-mean SPRT.
///
/// bias ~ (mu1 - mu0) / (2 * n)
pub fn bias_corrected_mle(mle: f64, n: usize, mu0: f64, mu1: f64) -> Result<f64, SeqError> {
    if n == 0 {
        return Err(SeqError::EmptyObservations);
    }
    if (mu0 - mu1).abs() < f64::EPSILON {
        return Err(SeqError::DegenerateHypotheses);
    }
    if !mle.is_finite() {
        return Err(SeqError::NonFiniteInput(mle));
    }
    let bias = (mu1 - mu0) / (2.0 * n as f64);
    let sign = if mle >= mu0 { 1.0 } else { -1.0 };
    Ok(mle - sign * bias.abs())
}

/// Median-unbiased estimator: shrinkage toward the null.
/// Simplified: (n-1)/n shrinkage factor.
pub fn median_unbiased_estimate(mle: f64, n: usize, mu0: f64) -> Result<f64, SeqError> {
    if n == 0 {
        return Err(SeqError::EmptyObservations);
    }
    if !mle.is_finite() {
        return Err(SeqError::NonFiniteInput(mle));
    }
    let shrinkage = (n as f64 - 1.0) / n as f64;
    Ok(mu0 + (mle - mu0) * shrinkage)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn bias_correction_reduces_magnitude() {
        let corrected = bias_corrected_mle(0.8, 20, 0.0, 0.5).unwrap();
        assert!(
            corrected.abs() < 0.8,
            "corrected {corrected} should be < 0.8"
        );
        assert!(corrected > 0.0, "corrected should still be positive");
    }

    #[test]
    fn median_unbiased_between_null_and_mle() {
        let estimate = median_unbiased_estimate(0.8, 20, 0.0).unwrap();
        assert!(
            estimate > 0.0 && estimate < 0.8,
            "median-unbiased {estimate} should be in (0, 0.8)"
        );
    }

    #[test]
    fn degenerate_hypotheses_error() {
        assert!(matches!(
            bias_corrected_mle(0.5, 10, 0.5, 0.5),
            Err(SeqError::DegenerateHypotheses)
        ));
    }
}
