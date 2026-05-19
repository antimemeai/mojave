use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::types::RatingMatrix;

#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    #[error("empty rating matrix")]
    EmptyData,
    #[error("confidence level must be in (0, 1), got {0}")]
    InvalidConfidence(f64),
    #[error("n_resamples must be > 0")]
    InvalidResamples,
    #[error("statistic computation failed on resample: {0}")]
    StatisticFailed(String),
    #[error("insufficient valid resamples: {succeeded} of {requested} succeeded (<50%), CI estimate unreliable")]
    InsufficientResamples { succeeded: usize, requested: usize },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[must_use]
pub struct BootstrapCiResult {
    pub ci_lower: f64,
    pub ci_upper: f64,
    pub point_estimate: f64,
    pub n_resamples: usize,
    pub confidence_level: f64,
}

/// Percentile bootstrap CI for an IRR statistic.
///
/// Item-level resampling with replacement, deterministic via seed.
/// Reference: Efron & Tibshirani (1993), Chapter 13.
pub fn bootstrap_ci<F>(
    matrix: &RatingMatrix,
    statistic_fn: F,
    n_resamples: usize,
    confidence_level: f64,
    seed: u64,
) -> Result<BootstrapCiResult, BootstrapError>
where
    F: Fn(&RatingMatrix) -> Result<f64, String>,
{
    if matrix.n_items() == 0 {
        return Err(BootstrapError::EmptyData);
    }
    if !(0.0 < confidence_level && confidence_level < 1.0) {
        return Err(BootstrapError::InvalidConfidence(confidence_level));
    }
    if n_resamples == 0 {
        return Err(BootstrapError::InvalidResamples);
    }

    let point_estimate = statistic_fn(matrix).map_err(BootstrapError::StatisticFailed)?;

    let mut rng = StdRng::seed_from_u64(seed);
    let n = matrix.n_items();

    let mut boot_stats = Vec::with_capacity(n_resamples);
    for _ in 0..n_resamples {
        let indices: Vec<usize> = (0..n).map(|_| rng.random_range(0..n)).collect();
        let resampled = resample_matrix(matrix, &indices);
        match statistic_fn(&resampled) {
            Ok(val) if val.is_finite() => boot_stats.push(val),
            _ => continue,
        }
    }

    if boot_stats.is_empty() {
        return Err(BootstrapError::StatisticFailed(
            "all resamples failed".to_string(),
        ));
    }

    // If fewer than 50% of requested resamples succeeded, the CI is unreliable.
    if boot_stats.len() * 2 < n_resamples {
        return Err(BootstrapError::InsufficientResamples {
            succeeded: boot_stats.len(),
            requested: n_resamples,
        });
    }

    boot_stats.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let b = boot_stats.len();
    let alpha = 1.0 - confidence_level;
    // Efron & Tibshirani (1993), p. 171: percentile interval uses
    // the ceil(alpha/2 * B)-th and ceil((1-alpha/2) * B)-th order statistics (1-indexed).
    let lower_idx = ((alpha / 2.0) * b as f64).ceil() as usize;
    let lower_idx = if lower_idx == 0 { 0 } else { lower_idx - 1 };
    let upper_idx = ((1.0 - alpha / 2.0) * b as f64).ceil() as usize;
    let upper_idx = if upper_idx == 0 { 0 } else { upper_idx - 1 };

    let ci_lower = boot_stats[lower_idx.min(b - 1)];
    let ci_upper = boot_stats[upper_idx.min(b - 1)];

    Ok(BootstrapCiResult {
        ci_lower,
        ci_upper,
        point_estimate,
        n_resamples: b,
        confidence_level,
    })
}

fn resample_matrix(matrix: &RatingMatrix, indices: &[usize]) -> RatingMatrix {
    let ratings: Vec<Vec<Option<u32>>> =
        indices.iter().map(|&i| matrix.ratings[i].clone()).collect();
    let items: Vec<String> = (0..indices.len())
        .map(|i| format!("resample-{i}"))
        .collect();
    RatingMatrix {
        items,
        raters: matrix.raters.clone(),
        ratings,
    }
}
