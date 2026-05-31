//! Mandel h/k consistency statistics for ISO 5725 outlier detection.
//!
//! Mandel h measures between-configuration (between-lab) consistency.
//! Mandel k measures within-configuration (within-lab) consistency.
//!
//! Reference: ISO 5725-2, Mandel (1991), Wilrich (2013) for critical values.

use serde::{Deserialize, Serialize};

/// Error type for Mandel h/k computations.
#[derive(Debug, thiserror::Error)]
pub enum MandelError {
    #[error("need at least 5 configurations (AIAG MSA ndc>=5), got {0}")]
    TooFewConfigurations(usize),
    #[error("configuration {index} has no replicates")]
    EmptyConfiguration { index: usize },
    #[error("all configurations must have at least 2 replicates for k statistic")]
    InsufficientReplicates,
    #[error("ISO 5725 requires balanced design — all configurations must have equal replicate count")]
    UnbalancedDesign,
    #[error("zero variance across configuration means; h is undefined")]
    ZeroVarianceBetween,
    #[error("zero pooled within-configuration variance; k is undefined")]
    ZeroVarianceWithin,
    #[error("alpha must be in (0, 1), got {0}")]
    InvalidAlpha(f64),
}

/// Result of Mandel h/k consistency analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[must_use]
pub struct MandelStatistics {
    /// Per-configuration between-consistency statistic h.
    /// h_i = (mean_i - grand_mean) / s_between
    pub h: Vec<f64>,
    /// Per-configuration within-consistency statistic k.
    /// k_i = s_i / s_pooled_within
    pub k: Vec<f64>,
    /// Critical value for h at the specified alpha level.
    pub h_critical: f64,
    /// Critical value for k at the specified alpha level.
    pub k_critical: f64,
    /// Indices of configurations flagged as h outliers (|h_i| > h_critical).
    pub h_outliers: Vec<usize>,
    /// Indices of configurations flagged as k outliers (k_i > k_critical).
    pub k_outliers: Vec<usize>,
}

/// Compute Mandel h and k consistency statistics.
///
/// Each element of `configs` is a slice of replicate measurements from one
/// configuration (laboratory/rater). All configurations must have at least
/// 2 replicates for k to be meaningful.
///
/// # Arguments
/// - `configs`: per-configuration replicate values
/// - `alpha`: significance level for critical values (e.g. 0.01 or 0.05)
///
/// # References
/// - ISO 5725-2:1994
/// - Wilrich (2013) "Critical values of Mandel's h and k statistics"
pub fn mandel_hk(configs: &[&[f64]], alpha: f64) -> Result<MandelStatistics, MandelError> {
    let p = configs.len();
    if p < 5 {
        return Err(MandelError::TooFewConfigurations(p));
    }
    if !(0.0 < alpha && alpha < 1.0) {
        return Err(MandelError::InvalidAlpha(alpha));
    }

    // Validate all configurations have data
    for (i, c) in configs.iter().enumerate() {
        if c.is_empty() {
            return Err(MandelError::EmptyConfiguration { index: i });
        }
    }

    // ISO 5725 assumes balanced design — all configs must have equal replicate count
    let n = configs[0].len();
    if !configs.iter().all(|c| c.len() == n) {
        return Err(MandelError::UnbalancedDesign);
    }
    if n < 2 {
        return Err(MandelError::InsufficientReplicates);
    }

    // Per-configuration means and standard deviations
    let means: Vec<f64> = configs
        .iter()
        .map(|c| c.iter().sum::<f64>() / c.len() as f64)
        .collect();

    let variances: Vec<f64> = configs
        .iter()
        .zip(means.iter())
        .map(|(c, &m)| {
            let ss: f64 = c.iter().map(|&x| (x - m).powi(2)).sum();
            ss / (c.len() as f64 - 1.0)
        })
        .collect();

    let stds: Vec<f64> = variances.iter().map(|&v| v.sqrt()).collect();

    // Grand mean of configuration means
    let grand_mean = means.iter().sum::<f64>() / p as f64;

    // Between-configuration standard deviation of means
    let s_between_sq =
        means.iter().map(|&m| (m - grand_mean).powi(2)).sum::<f64>() / (p as f64 - 1.0);
    let s_between = s_between_sq.sqrt();

    if s_between < 1e-15 {
        return Err(MandelError::ZeroVarianceBetween);
    }

    // Mandel h: (mean_i - grand_mean) / s_between
    let h: Vec<f64> = means.iter().map(|&m| (m - grand_mean) / s_between).collect();

    // Pooled within-configuration standard deviation
    // s_pooled = sqrt(mean of variances) -- uses equal-weight pooling (ISO 5725)
    let s_pooled_sq = variances.iter().sum::<f64>() / p as f64;
    let s_pooled = s_pooled_sq.sqrt();

    if s_pooled < 1e-15 {
        return Err(MandelError::ZeroVarianceWithin);
    }

    // Mandel k: s_i / s_pooled
    let k: Vec<f64> = stds.iter().map(|&s| s / s_pooled).collect();

    // Critical values: approximate using Wilrich (2013) approach
    // For h: approximate critical value based on t-distribution
    // h_crit ~ t_{alpha/(2p), p-2} * sqrt((p-1)/p)
    // For k: approximate using F-distribution
    // k_crit ~ sqrt(F_{alpha/p, n-1, (p-1)(n-1)})
    // We use simplified approximations suitable for p >= 3.
    let h_critical = mandel_h_critical(p, alpha);
    let k_critical = mandel_k_critical(p, configs[0].len(), alpha);

    let h_outliers: Vec<usize> = h
        .iter()
        .enumerate()
        .filter(|(_, &hi)| hi.abs() > h_critical)
        .map(|(i, _)| i)
        .collect();

    let k_outliers: Vec<usize> = k
        .iter()
        .enumerate()
        .filter(|(_, &ki)| ki > k_critical)
        .map(|(i, _)| i)
        .collect();

    Ok(MandelStatistics {
        h,
        k,
        h_critical,
        k_critical,
        h_outliers,
        k_outliers,
    })
}

/// Approximate critical value for Mandel h at significance level alpha.
///
/// Uses the exact relationship h_crit = ((p-1) * t_crit) / sqrt(p * (p - 2 + t_crit^2))
/// where t_crit is the upper alpha/(2p) quantile of t_{p-2}.
/// We approximate the t-quantile using the Abramowitz & Stegun rational approximation
/// of the normal quantile, with finite-df correction.
fn mandel_h_critical(p: usize, alpha: f64) -> f64 {
    // Bonferroni-adjusted alpha for two-sided test across p labs
    let a = alpha / (2.0 * p as f64);
    let df = (p - 2) as f64;

    // Approximate t-quantile via normal approximation with df correction
    let z = normal_quantile(1.0 - a);
    // Cornish-Fisher approximation for t from z
    let t_crit = z + (z.powi(3) + z) / (4.0 * df)
        + (5.0 * z.powi(5) + 16.0 * z.powi(3) + 3.0 * z) / (96.0 * df.powi(2));

    let pf = p as f64;
    (pf - 1.0) * t_crit / (pf * (pf - 2.0 + t_crit.powi(2))).sqrt()
}

/// Approximate critical value for Mandel k at significance level alpha.
///
/// ISO 5725-2 Annex B.3:
/// k_crit = sqrt(p * F_crit / (p - 1 + F_crit))
/// where F_crit is the upper alpha/p quantile of F_{n-1, (p-1)(n-1)}.
/// We use a Wilson-Hilferty approximation for the F-quantile.
fn mandel_k_critical(p: usize, n: usize, alpha: f64) -> f64 {
    let a = alpha / p as f64;
    let df1 = (n - 1) as f64;
    let df2 = ((p - 1) * (n - 1)) as f64;

    let f_crit = f_quantile_approx(1.0 - a, df1, df2);

    // ISO 5725-2 Annex B.3: k_crit = sqrt(p * F / (p - 1 + F))
    let pf = p as f64;
    (pf * f_crit / (pf - 1.0 + f_crit)).sqrt()
}

/// Abramowitz & Stegun rational approximation for the standard normal quantile.
/// Accurate to ~4.5e-4 for p in (0, 1).
fn normal_quantile(p: f64) -> f64 {
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }

    let sign;
    let pp;
    if p < 0.5 {
        sign = -1.0;
        pp = p;
    } else {
        sign = 1.0;
        pp = 1.0 - p;
    }

    let t = (-2.0 * pp.ln()).sqrt();

    // Coefficients from Abramowitz & Stegun 26.2.23
    let c0 = 2.515517;
    let c1 = 0.802853;
    let c2 = 0.010328;
    let d1 = 1.432788;
    let d2 = 0.189269;
    let d3 = 0.001308;

    let z = t - (c0 + c1 * t + c2 * t * t) / (1.0 + d1 * t + d2 * t * t + d3 * t * t * t);

    sign * z
}

/// Wilson-Hilferty approximation for the F-distribution quantile.
fn f_quantile_approx(p: f64, df1: f64, df2: f64) -> f64 {
    let z = normal_quantile(p);

    // Chi-squared approximation via Wilson-Hilferty for each df
    let chi2_1 = df1 * (1.0 - 2.0 / (9.0 * df1) + z * (2.0 / (9.0 * df1)).sqrt()).powi(3);
    let chi2_2 = df2 * (1.0 - 2.0 / (9.0 * df2) - z * (2.0 / (9.0 * df2)).sqrt()).powi(3);

    if chi2_2 <= 0.0 {
        return f64::INFINITY;
    }

    (chi2_1 / df1) / (chi2_2 / df2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_fewer_than_5_configurations() {
        // 3 configs: should fail now that minimum is 5 (AIAG MSA ndc>=5)
        let c1 = [1.0, 2.0, 3.0];
        let c2 = [1.1, 2.1, 3.1];
        let c3 = [1.2, 2.2, 3.2];
        let configs: Vec<&[f64]> = vec![&c1, &c2, &c3];
        let result = mandel_hk(&configs, 0.05);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MandelError::TooFewConfigurations(3)
        ));
    }

    #[test]
    fn rejects_4_configurations() {
        let c1 = [1.0, 2.0, 3.0];
        let c2 = [1.1, 2.1, 3.1];
        let c3 = [1.2, 2.2, 3.2];
        let c4 = [1.3, 2.3, 3.3];
        let configs: Vec<&[f64]> = vec![&c1, &c2, &c3, &c4];
        let result = mandel_hk(&configs, 0.05);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MandelError::TooFewConfigurations(4)
        ));
    }

    #[test]
    fn accepts_5_configurations() {
        let c1 = [10.1, 10.3, 10.2, 10.0];
        let c2 = [10.0, 10.2, 10.1, 10.3];
        let c3 = [14.0, 14.2, 14.1, 13.9];
        let c4 = [10.2, 10.0, 10.1, 10.3];
        let c5 = [10.1, 10.2, 10.0, 10.3];
        let configs: Vec<&[f64]> = vec![&c1, &c2, &c3, &c4, &c5];
        let result = mandel_hk(&configs, 0.05);
        assert!(result.is_ok());
    }
}
