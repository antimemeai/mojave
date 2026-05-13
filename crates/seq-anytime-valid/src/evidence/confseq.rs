use crate::error::SeqError;

/// Normal-mixture confidence sequence (known variance).
///
/// Howard et al. 2021 "stitched" boundary (Theorem 1) for N(mu, sigma^2) with
/// **known** sigma:
///
///   xbar_n ± sigma * sqrt(2 * (1 + 1/n) * ln(sqrt(n + 1) / alpha) / n)
///
/// This form guarantees anytime-valid coverage at level 1 − alpha. Passing the
/// true population sigma is required; using a sample estimate does not preserve
/// the coverage guarantee (use `normal_mixture_cs_known_sigma` when sigma is known).
///
/// For backward compatibility this function continues to estimate sigma from the
/// sample, but callers should prefer `normal_mixture_cs_known_sigma` when the
/// population standard deviation is available.
pub fn normal_mixture_cs(observations: &[f64], alpha: f64) -> Result<(f64, f64), SeqError> {
    if observations.is_empty() {
        return Err(SeqError::EmptyObservations);
    }
    if alpha <= 0.0 || alpha >= 1.0 {
        return Err(SeqError::InvalidAlpha(alpha));
    }
    for &x in observations {
        if !x.is_finite() {
            return Err(SeqError::NonFiniteInput(x));
        }
    }
    let n = observations.len() as f64;
    let x_bar: f64 = observations.iter().sum::<f64>() / n;
    let variance: f64 = observations
        .iter()
        .map(|&x| (x - x_bar).powi(2))
        .sum::<f64>()
        / n;
    let sigma = variance.sqrt().max(1e-10);
    let width = sigma * (2.0 * (1.0 + 1.0 / n) * ((n + 1.0).sqrt() / alpha).ln() / n).sqrt();
    Ok((x_bar - width, x_bar + width))
}

/// Normal-mixture confidence sequence with **known** population standard deviation.
///
/// Howard et al. 2021 "stitched" boundary with known sigma:
///
///   xbar_n ± sigma * sqrt(2 * (1 + 1/n) * ln(sqrt(n + 1) / alpha) / n)
///
/// This is the calibration-valid form. Anytime-valid coverage at level 1 − alpha
/// is guaranteed when `sigma` equals the true population standard deviation.
pub fn normal_mixture_cs_known_sigma(
    observations: &[f64],
    sigma: f64,
    alpha: f64,
) -> Result<(f64, f64), SeqError> {
    if observations.is_empty() {
        return Err(SeqError::EmptyObservations);
    }
    if alpha <= 0.0 || alpha >= 1.0 {
        return Err(SeqError::InvalidAlpha(alpha));
    }
    if sigma <= 0.0 || !sigma.is_finite() {
        return Err(SeqError::NonFiniteInput(sigma));
    }
    for &x in observations {
        if !x.is_finite() {
            return Err(SeqError::NonFiniteInput(x));
        }
    }
    let n = observations.len() as f64;
    let x_bar: f64 = observations.iter().sum::<f64>() / n;
    let width = sigma * (2.0 * (1.0 + 1.0 / n) * ((n + 1.0).sqrt() / alpha).ln() / n).sqrt();
    Ok((x_bar - width, x_bar + width))
}

/// Width of the confidence sequence (analytic, given sigma).
pub fn cs_width(n: usize, sigma: f64, alpha: f64) -> Result<f64, SeqError> {
    if n == 0 {
        return Err(SeqError::EmptyObservations);
    }
    if alpha <= 0.0 || alpha >= 1.0 {
        return Err(SeqError::InvalidAlpha(alpha));
    }
    let n_f = n as f64;
    Ok(2.0 * sigma * (2.0 * (1.0 + 1.0 / n_f) * ((n_f + 1.0).sqrt() / alpha).ln() / n_f).sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_decreases_with_n() {
        let w100 = cs_width(100, 1.0, 0.05).unwrap();
        let w1000 = cs_width(1000, 1.0, 0.05).unwrap();
        assert!(w1000 < w100, "width should decrease: {w1000} >= {w100}");
    }

    #[test]
    fn cs_contains_true_mean() {
        let obs: Vec<f64> = vec![0.1, -0.2, 0.3, -0.1, 0.05, 0.15, -0.05, 0.2, -0.3, 0.1];
        let (lo, hi) = normal_mixture_cs(&obs, 0.05).unwrap();
        assert!(lo <= 0.0 && hi >= 0.0, "CS [{lo}, {hi}] should contain 0");
    }

    #[test]
    fn empty_observations_error() {
        assert!(matches!(
            normal_mixture_cs(&[], 0.05),
            Err(SeqError::EmptyObservations)
        ));
    }
}
