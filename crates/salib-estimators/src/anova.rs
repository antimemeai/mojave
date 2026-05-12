//! Balanced crossed-design ANOVA decomposition.
//!
//! Two surfaces land here:
//! - [`estimate_anova_two_way`] for an unreplicated balanced `A x B` grid.
//! - [`estimate_anova_three_way`] for an unreplicated balanced
//!   `A x B x C` grid.
//!
//! The estimators compute exact sums-of-squares decompositions on the
//! supplied balanced grids. Because the grids are unreplicated, the
//! residual term is identically zero. Inferential statistics therefore
//! rely on explicit denominator choices ratified in ADRs rather than an
//! invented residual term.

#![allow(
    clippy::cast_precision_loss,
    clippy::similar_names,
    clippy::too_many_lines
)]

use ndarray::{Array2, Array3};
use rand::RngCore;
use salib_core::RngState;
use statrs::distribution::{ContinuousCDF, FisherSnedecor};

use crate::bootstrap::percentile_ci;
use crate::bootstrap_given_data::{BootstrapCi, BootstrapGivenDataError};

/// Errors from the ANOVA estimators.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AnovaError {
    #[error("anova: axis {axis} must have length >= 2, got {len}")]
    DegenerateAxis { axis: &'static str, len: usize },
    #[error("anova: expected {expected} varying factors, got {got}")]
    FactorCountMismatch { expected: usize, got: usize },
    #[error("anova: balanced design expected {expected_cells} cells, got {actual_cells}")]
    UnbalancedDesign {
        expected_cells: usize,
        actual_cells: usize,
    },
    #[error("anova: duplicate observation for one balanced-grid cell")]
    DuplicateCell,
    #[error("anova: missing observation for one balanced-grid cell")]
    MissingCell,
    #[error("anova: total variance is zero")]
    ZeroVariance,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum AnovaBootstrapError {
    #[error("anova bootstrap: estimator failed: {0}")]
    Anova(#[from] AnovaError),
    #[error("anova bootstrap: invalid bootstrap params: {0}")]
    Bootstrap(#[from] BootstrapGivenDataError),
    #[error("anova bootstrap: every resample failed")]
    AllResamplesFailed,
}

/// Balanced `A x B` ANOVA result.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AnovaTwoWayResult {
    pub v_row: f64,
    pub v_column: f64,
    pub v_interaction: f64,
    pub v_residual: f64,
    pub ms_row: f64,
    pub ms_column: f64,
    pub ms_interaction: f64,
    pub ms_residual: f64,
    pub f_row: Option<f64>,
    pub f_column: Option<f64>,
    pub f_interaction: Option<f64>,
    pub p_row: Option<f64>,
    pub p_column: Option<f64>,
    pub p_interaction: Option<f64>,
    pub variance_fraction_ci_low: Option<Vec<f64>>,
    pub variance_fraction_ci_high: Option<Vec<f64>>,
    pub bootstrap_iterations: Option<usize>,
    pub bootstrap_alpha: Option<f64>,
}

/// Balanced `A x B x C` ANOVA result.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AnovaThreeWayResult {
    pub v_data: f64,
    pub v_brittleness: f64,
    pub v_inference: f64,
    pub v_data_brittleness: f64,
    pub v_data_inference: f64,
    pub v_brittleness_inference: f64,
    pub v_data_brittleness_inference: f64,
    pub v_residual: f64,
    pub ms_data: f64,
    pub ms_brittleness: f64,
    pub ms_inference: f64,
    pub ms_data_brittleness: f64,
    pub ms_data_inference: f64,
    pub ms_brittleness_inference: f64,
    pub ms_data_brittleness_inference: f64,
    pub ms_residual: f64,
    pub f_data: Option<f64>,
    pub f_brittleness: Option<f64>,
    pub f_inference: Option<f64>,
    pub f_data_brittleness: Option<f64>,
    pub f_data_inference: Option<f64>,
    pub f_brittleness_inference: Option<f64>,
    pub f_data_brittleness_inference: Option<f64>,
    pub p_data: Option<f64>,
    pub p_brittleness: Option<f64>,
    pub p_inference: Option<f64>,
    pub p_data_brittleness: Option<f64>,
    pub p_data_inference: Option<f64>,
    pub p_brittleness_inference: Option<f64>,
    pub p_data_brittleness_inference: Option<f64>,
    pub variance_fraction_ci_low: Option<Vec<f64>>,
    pub variance_fraction_ci_high: Option<Vec<f64>>,
    pub bootstrap_iterations: Option<usize>,
    pub bootstrap_alpha: Option<f64>,
}

pub fn estimate_anova_two_way(grid: &Array2<f64>) -> Result<AnovaTwoWayResult, AnovaError> {
    let a = grid.nrows();
    let b = grid.ncols();
    if a < 2 {
        return Err(AnovaError::DegenerateAxis {
            axis: "row",
            len: a,
        });
    }
    if b < 2 {
        return Err(AnovaError::DegenerateAxis {
            axis: "column",
            len: b,
        });
    }

    let grand = grid.iter().sum::<f64>() / (a * b) as f64;
    let row_means: Vec<f64> = (0..a)
        .map(|i| grid.row(i).iter().sum::<f64>() / b as f64)
        .collect();
    let col_means: Vec<f64> = (0..b)
        .map(|j| grid.column(j).iter().sum::<f64>() / a as f64)
        .collect();

    let ss_total = grid.iter().map(|y| (y - grand).powi(2)).sum::<f64>();
    if ss_total <= 1.0e-15 {
        return Err(AnovaError::ZeroVariance);
    }

    let ss_row = (b as f64)
        * row_means
            .iter()
            .map(|mean| (mean - grand).powi(2))
            .sum::<f64>();
    let ss_column = (a as f64)
        * col_means
            .iter()
            .map(|mean| (mean - grand).powi(2))
            .sum::<f64>();

    let mut ss_interaction = 0.0_f64;
    for i in 0..a {
        for j in 0..b {
            let fitted = row_means[i] + col_means[j] - grand;
            ss_interaction += (grid[[i, j]] - fitted).powi(2);
        }
    }

    let df_row = (a - 1) as f64;
    let df_column = (b - 1) as f64;
    let df_interaction = ((a - 1) * (b - 1)) as f64;
    let ms_row = ss_row / df_row;
    let ms_column = ss_column / df_column;
    let ms_interaction = ss_interaction / df_interaction;
    let ms_residual = 0.0;
    let (f_row, p_row) = inferential_value(ms_row, ms_interaction, df_row, df_interaction)?;
    let (f_column, p_column) =
        inferential_value(ms_column, ms_interaction, df_column, df_interaction)?;

    Ok(AnovaTwoWayResult {
        v_row: ss_row / ss_total,
        v_column: ss_column / ss_total,
        v_interaction: ss_interaction / ss_total,
        v_residual: 0.0,
        ms_row,
        ms_column,
        ms_interaction,
        ms_residual,
        f_row,
        f_column,
        f_interaction: None,
        p_row,
        p_column,
        p_interaction: None,
        variance_fraction_ci_low: None,
        variance_fraction_ci_high: None,
        bootstrap_iterations: None,
        bootstrap_alpha: None,
    })
}

pub fn estimate_anova_three_way(grid: &Array3<f64>) -> Result<AnovaThreeWayResult, AnovaError> {
    let a = grid.shape()[0];
    let b = grid.shape()[1];
    let c = grid.shape()[2];
    if a < 2 {
        return Err(AnovaError::DegenerateAxis {
            axis: "data",
            len: a,
        });
    }
    if b < 2 {
        return Err(AnovaError::DegenerateAxis {
            axis: "brittleness",
            len: b,
        });
    }
    if c < 2 {
        return Err(AnovaError::DegenerateAxis {
            axis: "inference",
            len: c,
        });
    }

    let n_total = (a * b * c) as f64;
    let grand = grid.iter().sum::<f64>() / n_total;
    let ss_total = grid.iter().map(|y| (y - grand).powi(2)).sum::<f64>();
    if ss_total <= 1.0e-15 {
        return Err(AnovaError::ZeroVariance);
    }

    let mean_a: Vec<f64> = (0..a)
        .map(|i| {
            let mut sum = 0.0;
            for j in 0..b {
                for k in 0..c {
                    sum += grid[[i, j, k]];
                }
            }
            sum / (b * c) as f64
        })
        .collect();
    let mean_b: Vec<f64> = (0..b)
        .map(|j| {
            let mut sum = 0.0;
            for i in 0..a {
                for k in 0..c {
                    sum += grid[[i, j, k]];
                }
            }
            sum / (a * c) as f64
        })
        .collect();
    let mean_c: Vec<f64> = (0..c)
        .map(|k| {
            let mut sum = 0.0;
            for i in 0..a {
                for j in 0..b {
                    sum += grid[[i, j, k]];
                }
            }
            sum / (a * b) as f64
        })
        .collect();

    let mean_ab: Vec<Vec<f64>> = (0..a)
        .map(|i| {
            (0..b)
                .map(|j| {
                    let mut sum = 0.0;
                    for k in 0..c {
                        sum += grid[[i, j, k]];
                    }
                    sum / c as f64
                })
                .collect()
        })
        .collect();
    let mean_ac: Vec<Vec<f64>> = (0..a)
        .map(|i| {
            (0..c)
                .map(|k| {
                    let mut sum = 0.0;
                    for j in 0..b {
                        sum += grid[[i, j, k]];
                    }
                    sum / b as f64
                })
                .collect()
        })
        .collect();
    let mean_bc: Vec<Vec<f64>> = (0..b)
        .map(|j| {
            (0..c)
                .map(|k| {
                    let mut sum = 0.0;
                    for i in 0..a {
                        sum += grid[[i, j, k]];
                    }
                    sum / a as f64
                })
                .collect()
        })
        .collect();

    let ss_data = (b * c) as f64 * mean_a.iter().map(|m| (m - grand).powi(2)).sum::<f64>();
    let ss_brittleness = (a * c) as f64 * mean_b.iter().map(|m| (m - grand).powi(2)).sum::<f64>();
    let ss_inference = (a * b) as f64 * mean_c.iter().map(|m| (m - grand).powi(2)).sum::<f64>();

    let mut ss_data_brittleness = 0.0;
    for i in 0..a {
        for j in 0..b {
            let term = mean_ab[i][j] - mean_a[i] - mean_b[j] + grand;
            ss_data_brittleness += (c as f64) * term.powi(2);
        }
    }
    let mut ss_data_inference = 0.0;
    for i in 0..a {
        for k in 0..c {
            let term = mean_ac[i][k] - mean_a[i] - mean_c[k] + grand;
            ss_data_inference += (b as f64) * term.powi(2);
        }
    }
    let mut ss_brittleness_inference = 0.0;
    for j in 0..b {
        for k in 0..c {
            let term = mean_bc[j][k] - mean_b[j] - mean_c[k] + grand;
            ss_brittleness_inference += (a as f64) * term.powi(2);
        }
    }

    let mut ss_data_brittleness_inference = 0.0;
    for i in 0..a {
        for j in 0..b {
            for k in 0..c {
                let term = grid[[i, j, k]] - mean_ab[i][j] - mean_ac[i][k] - mean_bc[j][k]
                    + mean_a[i]
                    + mean_b[j]
                    + mean_c[k]
                    - grand;
                ss_data_brittleness_inference += term.powi(2);
            }
        }
    }

    let ms_data = ss_data / ((a - 1) as f64);
    let ms_brittleness = ss_brittleness / ((b - 1) as f64);
    let ms_inference = ss_inference / ((c - 1) as f64);
    let ms_data_brittleness = ss_data_brittleness / (((a - 1) * (b - 1)) as f64);
    let ms_data_inference = ss_data_inference / (((a - 1) * (c - 1)) as f64);
    let ms_brittleness_inference = ss_brittleness_inference / (((b - 1) * (c - 1)) as f64);
    let ms_data_brittleness_inference =
        ss_data_brittleness_inference / (((a - 1) * (b - 1) * (c - 1)) as f64);
    let ms_residual = 0.0;
    let df_pir = ((a - 1) * (b - 1) * (c - 1)) as f64;
    let (f_data, p_data) = inferential_value(
        ms_data,
        ms_data_brittleness_inference,
        (a - 1) as f64,
        df_pir,
    )?;
    let (f_brittleness, p_brittleness) = inferential_value(
        ms_brittleness,
        ms_data_brittleness_inference,
        (b - 1) as f64,
        df_pir,
    )?;
    let (f_inference, p_inference) = inferential_value(
        ms_inference,
        ms_data_brittleness_inference,
        (c - 1) as f64,
        df_pir,
    )?;
    let (f_data_brittleness, p_data_brittleness) = inferential_value(
        ms_data_brittleness,
        ms_data_brittleness_inference,
        ((a - 1) * (b - 1)) as f64,
        df_pir,
    )?;
    let (f_data_inference, p_data_inference) = inferential_value(
        ms_data_inference,
        ms_data_brittleness_inference,
        ((a - 1) * (c - 1)) as f64,
        df_pir,
    )?;
    let (f_brittleness_inference, p_brittleness_inference) = inferential_value(
        ms_brittleness_inference,
        ms_data_brittleness_inference,
        ((b - 1) * (c - 1)) as f64,
        df_pir,
    )?;

    Ok(AnovaThreeWayResult {
        v_data: ss_data / ss_total,
        v_brittleness: ss_brittleness / ss_total,
        v_inference: ss_inference / ss_total,
        v_data_brittleness: ss_data_brittleness / ss_total,
        v_data_inference: ss_data_inference / ss_total,
        v_brittleness_inference: ss_brittleness_inference / ss_total,
        v_data_brittleness_inference: ss_data_brittleness_inference / ss_total,
        v_residual: 0.0,
        ms_data,
        ms_brittleness,
        ms_inference,
        ms_data_brittleness,
        ms_data_inference,
        ms_brittleness_inference,
        ms_data_brittleness_inference,
        ms_residual,
        f_data,
        f_brittleness,
        f_inference,
        f_data_brittleness,
        f_data_inference,
        f_brittleness_inference,
        f_data_brittleness_inference: None,
        p_data,
        p_brittleness,
        p_inference,
        p_data_brittleness,
        p_data_inference,
        p_brittleness_inference,
        p_data_brittleness_inference: None,
        variance_fraction_ci_low: None,
        variance_fraction_ci_high: None,
        bootstrap_iterations: None,
        bootstrap_alpha: None,
    })
}

pub fn estimate_anova_two_way_with_bootstrap(
    grid: &Array2<f64>,
    n_resamples: usize,
    alpha: f64,
    rng: &mut RngState,
) -> Result<AnovaTwoWayResult, AnovaBootstrapError> {
    let mut result = estimate_anova_two_way(grid)?;
    let ci = bootstrap_anova_two_way(grid, n_resamples, alpha, rng)?;
    result.variance_fraction_ci_low = Some(ci.ci_low);
    result.variance_fraction_ci_high = Some(ci.ci_high);
    result.bootstrap_iterations = Some(ci.n_resamples);
    result.bootstrap_alpha = Some(ci.alpha);
    Ok(result)
}

pub fn estimate_anova_three_way_with_bootstrap(
    grid: &Array3<f64>,
    n_resamples: usize,
    alpha: f64,
    rng: &mut RngState,
) -> Result<AnovaThreeWayResult, AnovaBootstrapError> {
    let mut result = estimate_anova_three_way(grid)?;
    let ci = bootstrap_anova_three_way(grid, n_resamples, alpha, rng)?;
    result.variance_fraction_ci_low = Some(ci.ci_low);
    result.variance_fraction_ci_high = Some(ci.ci_high);
    result.bootstrap_iterations = Some(ci.n_resamples);
    result.bootstrap_alpha = Some(ci.alpha);
    Ok(result)
}

pub fn bootstrap_anova_two_way(
    grid: &Array2<f64>,
    n_resamples: usize,
    alpha: f64,
    rng: &mut RngState,
) -> Result<BootstrapCi, AnovaBootstrapError> {
    let _ = estimate_anova_two_way(grid)?;
    validate_bootstrap_params(grid.nrows() * grid.ncols(), n_resamples, alpha)?;
    let a = grid.nrows();
    let b = grid.ncols();
    let mut chacha = rng.clone().into_chacha();
    let mut per_component = (0..4)
        .map(|_| Vec::with_capacity(n_resamples))
        .collect::<Vec<_>>();
    let mut n_skipped = 0usize;
    let mut row_idx = vec![0usize; a];
    let mut col_idx = vec![0usize; b];
    let mut resampled = Array2::<f64>::zeros((a, b));

    for _ in 0..n_resamples {
        for slot in &mut row_idx {
            *slot = (chacha.next_u32() as usize) % a;
        }
        for slot in &mut col_idx {
            *slot = (chacha.next_u32() as usize) % b;
        }
        for out_row in 0..a {
            for out_col in 0..b {
                resampled[[out_row, out_col]] = grid[[row_idx[out_row], col_idx[out_col]]];
            }
        }
        match estimate_anova_two_way(&resampled) {
            Ok(est) => {
                let vals = [est.v_row, est.v_column, est.v_interaction, est.v_residual];
                for (samples, value) in per_component.iter_mut().zip(vals) {
                    samples.push(value);
                }
            }
            Err(_) => n_skipped += 1,
        }
    }
    *rng = RngState::snapshot(&chacha, rng);
    build_bootstrap_ci(&per_component, n_resamples, alpha, n_skipped)
}

pub fn bootstrap_anova_three_way(
    grid: &Array3<f64>,
    n_resamples: usize,
    alpha: f64,
    rng: &mut RngState,
) -> Result<BootstrapCi, AnovaBootstrapError> {
    let _ = estimate_anova_three_way(grid)?;
    validate_bootstrap_params(grid.len(), n_resamples, alpha)?;
    let a = grid.shape()[0];
    let b = grid.shape()[1];
    let c = grid.shape()[2];
    let mut chacha = rng.clone().into_chacha();
    let mut per_component = (0..8)
        .map(|_| Vec::with_capacity(n_resamples))
        .collect::<Vec<_>>();
    let mut n_skipped = 0usize;
    let mut a_idx = vec![0usize; a];
    let mut b_idx = vec![0usize; b];
    let mut c_idx = vec![0usize; c];
    let mut resampled = Array3::<f64>::zeros((a, b, c));

    for _ in 0..n_resamples {
        for slot in &mut a_idx {
            *slot = (chacha.next_u32() as usize) % a;
        }
        for slot in &mut b_idx {
            *slot = (chacha.next_u32() as usize) % b;
        }
        for slot in &mut c_idx {
            *slot = (chacha.next_u32() as usize) % c;
        }
        for out_a in 0..a {
            for out_b in 0..b {
                for out_c in 0..c {
                    resampled[[out_a, out_b, out_c]] =
                        grid[[a_idx[out_a], b_idx[out_b], c_idx[out_c]]];
                }
            }
        }
        match estimate_anova_three_way(&resampled) {
            Ok(est) => {
                let vals = [
                    est.v_data,
                    est.v_brittleness,
                    est.v_inference,
                    est.v_data_brittleness,
                    est.v_data_inference,
                    est.v_brittleness_inference,
                    est.v_data_brittleness_inference,
                    est.v_residual,
                ];
                for (samples, value) in per_component.iter_mut().zip(vals) {
                    samples.push(value);
                }
            }
            Err(_) => n_skipped += 1,
        }
    }
    *rng = RngState::snapshot(&chacha, rng);
    build_bootstrap_ci(&per_component, n_resamples, alpha, n_skipped)
}

fn validate_bootstrap_params(
    sample_size: usize,
    n_resamples: usize,
    alpha: f64,
) -> Result<(), BootstrapGivenDataError> {
    if n_resamples == 0 {
        return Err(BootstrapGivenDataError::ZeroResamples);
    }
    if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
        return Err(BootstrapGivenDataError::OutOfRangeAlpha { alpha });
    }
    if sample_size == 0 {
        return Err(BootstrapGivenDataError::EmptySample);
    }
    Ok(())
}

fn build_bootstrap_ci(
    per_component: &[Vec<f64>],
    n_resamples: usize,
    alpha: f64,
    n_skipped: usize,
) -> Result<BootstrapCi, AnovaBootstrapError> {
    if per_component.iter().all(Vec::is_empty) {
        return Err(AnovaBootstrapError::AllResamplesFailed);
    }
    let low_p = alpha / 2.0;
    let high_p = 1.0 - alpha / 2.0;
    let mut ci_low = Vec::with_capacity(per_component.len());
    let mut ci_high = Vec::with_capacity(per_component.len());
    for samples in per_component {
        if samples.is_empty() {
            ci_low.push(f64::NAN);
            ci_high.push(f64::NAN);
        } else {
            let (lo, hi) = percentile_ci(samples, low_p, high_p);
            ci_low.push(lo);
            ci_high.push(hi);
        }
    }
    Ok(BootstrapCi {
        ci_low,
        ci_high,
        n_resamples,
        alpha,
        n_skipped,
    })
}

fn f_tail(statistic: f64, df_num: f64, df_den: f64) -> Result<f64, AnovaError> {
    let dist = FisherSnedecor::new(df_num, df_den).map_err(|_| AnovaError::ZeroVariance)?;
    Ok(1.0 - dist.cdf(statistic))
}

fn inferential_value(
    numerator_ms: f64,
    denominator_ms: f64,
    df_num: f64,
    df_den: f64,
) -> Result<(Option<f64>, Option<f64>), AnovaError> {
    if denominator_ms.abs() <= 1.0e-15 {
        Ok((None, None))
    } else {
        let statistic = numerator_ms / denominator_ms;
        Ok((Some(statistic), Some(f_tail(statistic, df_num, df_den)?)))
    }
}

#[cfg(test)]
mod tests {
    use ndarray::arr2;
    use salib_core::RngState;

    use super::{
        bootstrap_anova_two_way, build_bootstrap_ci, estimate_anova_three_way,
        estimate_anova_two_way, AnovaBootstrapError, AnovaError,
    };

    #[test]
    fn two_way_zero_variance_errors() {
        let grid = arr2(&[[1.0, 1.0], [1.0, 1.0]]);
        assert_eq!(
            estimate_anova_two_way(&grid).unwrap_err(),
            AnovaError::ZeroVariance
        );
    }

    #[test]
    fn three_way_degenerate_axis_errors() {
        let grid = ndarray::Array3::<f64>::zeros((1, 2, 2));
        assert_eq!(
            estimate_anova_three_way(&grid).unwrap_err(),
            AnovaError::DegenerateAxis {
                axis: "data",
                len: 1,
            }
        );
    }

    #[test]
    fn bootstrap_two_way_invalid_grid_errors_instead_of_nan_ci() {
        let grid = arr2(&[[1.0, 1.0], [1.0, 1.0]]);
        let mut rng = RngState::from_seed([0x33; 32]);
        let err = bootstrap_anova_two_way(&grid, 32, 0.05, &mut rng).unwrap_err();
        assert_eq!(err, AnovaBootstrapError::Anova(AnovaError::ZeroVariance));
    }

    #[test]
    fn all_skipped_bootstrap_errors_instead_of_nan_ci() {
        let err = build_bootstrap_ci(&[Vec::new(), Vec::new()], 32, 0.05, 32).unwrap_err();
        assert_eq!(err, AnovaBootstrapError::AllResamplesFailed);
    }
}
