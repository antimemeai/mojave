use std::f64::consts::PI;

use crate::error::SeqError;

/// Log-gamma via Lanczos approximation (g=7, n=9).
pub(crate) fn lngamma(z: f64) -> f64 {
    const G: f64 = 7.0;
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];

    if z < 0.5 {
        return (PI / (PI * z).sin()).ln() - lngamma(1.0 - z);
    }

    let z = z - 1.0;
    let mut x = C[0];
    for (i, &c) in C[1..].iter().enumerate() {
        x += c / (z + i as f64 + 1.0);
    }
    let t = z + G + 0.5;
    0.5 * (2.0 * PI).ln() + (z + 0.5) * t.ln() - t + x.ln()
}

/// Log of the Beta function: ln B(a,b) = lngamma(a) + lngamma(b) - lngamma(a+b).
pub(crate) fn lnbeta(a: f64, b: f64) -> f64 {
    lngamma(a) + lngamma(b) - lngamma(a + b)
}

/// Marginalized log-likelihood ratio for Bernoulli data with Beta(a, b) mixing.
///
/// Λ_n = B(s + a, f + b) / (B(a, b) · p0^s · (1 - p0)^f)
///
/// where s = number of successes, f = n - s = number of failures.
/// Reference: Johari et al. (2022) §3.1.
pub fn bernoulli_msprt_log_lr(
    observations: &[f64],
    p0: f64,
    beta_a: f64,
    beta_b: f64,
) -> Result<f64, SeqError> {
    if observations.is_empty() {
        return Ok(0.0);
    }
    if p0 <= 0.0 || p0 >= 1.0 {
        return Err(SeqError::InvalidNullProportion(p0));
    }
    if beta_a <= 0.0 || beta_b <= 0.0 {
        return Err(SeqError::InvalidBetaParams {
            a: beta_a,
            b: beta_b,
        });
    }

    let mut s: f64 = 0.0;
    let mut n: f64 = 0.0;
    for &x in observations {
        if !x.is_finite() || !(0.0..=1.0).contains(&x) {
            return Err(SeqError::InvalidBernoulliObservation(x));
        }
        s += x;
        n += 1.0;
    }
    let f = n - s;

    Ok(lnbeta(s + beta_a, f + beta_b) - lnbeta(beta_a, beta_b) - s * p0.ln() - f * (1.0 - p0).ln())
}

/// Always-valid p-value for Bernoulli mSPRT: p_n = min(1, 1/Λ_n).
pub fn bernoulli_always_valid_p(
    observations: &[f64],
    p0: f64,
    beta_a: f64,
    beta_b: f64,
) -> Result<f64, SeqError> {
    let log_lr = bernoulli_msprt_log_lr(observations, p0, beta_a, beta_b)?;
    let lr = log_lr.exp();
    Ok((1.0 / lr).min(1.0))
}

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

    #[test]
    fn lngamma_known_values() {
        assert!((lngamma(1.0) - 0.0).abs() < 1e-10);
        assert!((lngamma(2.0) - 0.0).abs() < 1e-10);
        assert!((lngamma(5.0) - 24.0_f64.ln()).abs() < 1e-10);
        assert!((lngamma(0.5) - std::f64::consts::PI.sqrt().ln()).abs() < 1e-10);
    }

    #[test]
    fn lnbeta_known_values() {
        assert!((lnbeta(1.0, 1.0) - 0.0).abs() < 1e-10);
        assert!((lnbeta(2.0, 2.0) - (-6.0_f64.ln())).abs() < 1e-10);
        assert!((lnbeta(0.5, 0.5) - std::f64::consts::PI.ln()).abs() < 1e-10);
    }

    #[test]
    fn bernoulli_msprt_empty_gives_zero() {
        let log_lr = bernoulli_msprt_log_lr(&[], 0.25, 1.0, 1.0).unwrap();
        assert!((log_lr - 0.0).abs() < 1e-10);
    }

    #[test]
    fn bernoulli_msprt_strong_signal() {
        let obs: Vec<f64> = [vec![1.0; 50], vec![0.0; 50]].concat();
        let log_lr = bernoulli_msprt_log_lr(&obs, 0.25, 1.0, 1.0).unwrap();
        assert!(
            log_lr > 5.0,
            "strong signal should give large log-LR, got {log_lr}"
        );
    }

    #[test]
    fn bernoulli_msprt_null_data() {
        let obs: Vec<f64> = [vec![1.0; 25], vec![0.0; 75]].concat();
        let log_lr = bernoulli_msprt_log_lr(&obs, 0.25, 1.0, 1.0).unwrap();
        assert!(
            log_lr.abs() < 3.0,
            "null data should give small log-LR, got {log_lr}"
        );
    }

    #[test]
    fn bernoulli_msprt_invalid_p0() {
        let obs = vec![1.0];
        assert!(matches!(
            bernoulli_msprt_log_lr(&obs, 0.0, 1.0, 1.0),
            Err(SeqError::InvalidNullProportion(_))
        ));
        assert!(matches!(
            bernoulli_msprt_log_lr(&obs, 1.0, 1.0, 1.0),
            Err(SeqError::InvalidNullProportion(_))
        ));
    }

    #[test]
    fn bernoulli_msprt_invalid_beta_params() {
        let obs = vec![1.0];
        assert!(matches!(
            bernoulli_msprt_log_lr(&obs, 0.5, 0.0, 1.0),
            Err(SeqError::InvalidBetaParams { .. })
        ));
    }

    #[test]
    fn bernoulli_always_valid_p_empty_is_one() {
        let p = bernoulli_always_valid_p(&[], 0.25, 1.0, 1.0).unwrap();
        assert!((p - 1.0).abs() < 1e-10);
    }

    #[test]
    fn bernoulli_always_valid_p_strong_signal() {
        let obs: Vec<f64> = [vec![1.0; 50], vec![0.0; 50]].concat();
        let p = bernoulli_always_valid_p(&obs, 0.25, 1.0, 1.0).unwrap();
        assert!(p < 0.001, "strong evidence should give p < 0.001, got {p}");
    }

    #[test]
    fn bernoulli_always_valid_p_null_data() {
        let obs: Vec<f64> = [vec![1.0; 25], vec![0.0; 75]].concat();
        let p = bernoulli_always_valid_p(&obs, 0.25, 1.0, 1.0).unwrap();
        assert!(p > 0.3, "null data should give high p, got {p}");
    }
}
