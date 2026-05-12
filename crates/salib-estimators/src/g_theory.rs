//! Crossed `p x i x r` generalizability-theory decomposition.

#![allow(
    clippy::cast_precision_loss,
    clippy::similar_names,
    clippy::too_many_lines
)]

use ndarray::Array3;
use rand::RngCore;
use salib_core::RngState;

use crate::bootstrap::percentile_ci;
use crate::bootstrap_given_data::{BootstrapCi, BootstrapGivenDataError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GTheoryDesign {
    Crossed,
    Nested,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum GTheoryError {
    #[error("g-theory: only crossed design is implemented in V1")]
    UnsupportedDesign,
    #[error("g-theory: expected 3 varying factors, got {got}")]
    FactorCountMismatch { got: usize },
    #[error("g-theory: balanced design expected {expected_cells} cells, got {actual_cells}")]
    UnbalancedDesign {
        expected_cells: usize,
        actual_cells: usize,
    },
    #[error("g-theory: duplicate observation for one balanced-grid cell")]
    DuplicateCell,
    #[error("g-theory: missing observation for one balanced-grid cell")]
    MissingCell,
    #[error("g-theory: axis {axis} must have length >= 2, got {len}")]
    DegenerateAxis { axis: &'static str, len: usize },
    #[error("g-theory: total variance is zero")]
    ZeroVariance,
    #[error(
        "g-theory: {coefficient} is undefined because its denominator collapsed to zero while sigma_p remained non-zero"
    )]
    UndefinedReliability { coefficient: &'static str },
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum GTheoryBootstrapError {
    #[error("g-theory bootstrap: estimator failed: {0}")]
    GTheory(#[from] GTheoryError),
    #[error("g-theory bootstrap: invalid bootstrap params: {0}")]
    Bootstrap(#[from] BootstrapGivenDataError),
    #[error("g-theory bootstrap: every resample failed")]
    AllResamplesFailed,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GTheoryResult {
    pub sigma_p: f64,
    pub sigma_i: f64,
    pub sigma_r: f64,
    pub sigma_pi: f64,
    pub sigma_pr: f64,
    pub sigma_ir: f64,
    pub sigma_pir: f64,
    pub g_coefficient: f64,
    pub phi_coefficient: f64,
    pub variance_component_ci_low: Option<Vec<f64>>,
    pub variance_component_ci_high: Option<Vec<f64>>,
    pub g_coefficient_ci_low: Option<f64>,
    pub g_coefficient_ci_high: Option<f64>,
    pub phi_coefficient_ci_low: Option<f64>,
    pub phi_coefficient_ci_high: Option<f64>,
    pub bootstrap_iterations: Option<usize>,
    pub bootstrap_alpha: Option<f64>,
    pub bootstrap_skipped: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DStudyPoint {
    pub n_items: usize,
    pub n_raters: usize,
    pub g_coefficient: f64,
    pub phi_coefficient: f64,
}

pub fn estimate_g_theory_pir(
    grid: &Array3<f64>,
    design: GTheoryDesign,
) -> Result<GTheoryResult, GTheoryError> {
    if !matches!(design, GTheoryDesign::Crossed) {
        return Err(GTheoryError::UnsupportedDesign);
    }

    let p = grid.shape()[0];
    let i = grid.shape()[1];
    let r = grid.shape()[2];
    if p < 2 {
        return Err(GTheoryError::DegenerateAxis { axis: "p", len: p });
    }
    if i < 2 {
        return Err(GTheoryError::DegenerateAxis { axis: "i", len: i });
    }
    if r < 2 {
        return Err(GTheoryError::DegenerateAxis { axis: "r", len: r });
    }

    let grand = grid.iter().sum::<f64>() / (p * i * r) as f64;
    let ss_total = grid.iter().map(|y| (y - grand).powi(2)).sum::<f64>();
    if ss_total <= 1.0e-15 {
        return Err(GTheoryError::ZeroVariance);
    }

    let mean_p: Vec<f64> = (0..p)
        .map(|pp| {
            let mut sum = 0.0;
            for ii in 0..i {
                for rr in 0..r {
                    sum += grid[[pp, ii, rr]];
                }
            }
            sum / (i * r) as f64
        })
        .collect();
    let mean_i: Vec<f64> = (0..i)
        .map(|ii| {
            let mut sum = 0.0;
            for pp in 0..p {
                for rr in 0..r {
                    sum += grid[[pp, ii, rr]];
                }
            }
            sum / (p * r) as f64
        })
        .collect();
    let mean_r: Vec<f64> = (0..r)
        .map(|rr| {
            let mut sum = 0.0;
            for pp in 0..p {
                for ii in 0..i {
                    sum += grid[[pp, ii, rr]];
                }
            }
            sum / (p * i) as f64
        })
        .collect();

    let mean_pi: Vec<Vec<f64>> = (0..p)
        .map(|pp| {
            (0..i)
                .map(|ii| {
                    let mut sum = 0.0;
                    for rr in 0..r {
                        sum += grid[[pp, ii, rr]];
                    }
                    sum / r as f64
                })
                .collect()
        })
        .collect();
    let mean_pr: Vec<Vec<f64>> = (0..p)
        .map(|pp| {
            (0..r)
                .map(|rr| {
                    let mut sum = 0.0;
                    for ii in 0..i {
                        sum += grid[[pp, ii, rr]];
                    }
                    sum / i as f64
                })
                .collect()
        })
        .collect();
    let mean_ir: Vec<Vec<f64>> = (0..i)
        .map(|ii| {
            (0..r)
                .map(|rr| {
                    let mut sum = 0.0;
                    for pp in 0..p {
                        sum += grid[[pp, ii, rr]];
                    }
                    sum / p as f64
                })
                .collect()
        })
        .collect();

    let ss_p = (i * r) as f64 * mean_p.iter().map(|m| (m - grand).powi(2)).sum::<f64>();
    let ss_i = (p * r) as f64 * mean_i.iter().map(|m| (m - grand).powi(2)).sum::<f64>();
    let ss_r = (p * i) as f64 * mean_r.iter().map(|m| (m - grand).powi(2)).sum::<f64>();

    let mut ss_pi = 0.0;
    for pp in 0..p {
        for ii in 0..i {
            let term = mean_pi[pp][ii] - mean_p[pp] - mean_i[ii] + grand;
            ss_pi += (r as f64) * term.powi(2);
        }
    }
    let mut ss_pr = 0.0;
    for pp in 0..p {
        for rr in 0..r {
            let term = mean_pr[pp][rr] - mean_p[pp] - mean_r[rr] + grand;
            ss_pr += (i as f64) * term.powi(2);
        }
    }
    let mut ss_ir = 0.0;
    for ii in 0..i {
        for rr in 0..r {
            let term = mean_ir[ii][rr] - mean_i[ii] - mean_r[rr] + grand;
            ss_ir += (p as f64) * term.powi(2);
        }
    }

    let mut ss_pir = 0.0;
    for pp in 0..p {
        for ii in 0..i {
            for rr in 0..r {
                let term = grid[[pp, ii, rr]] - mean_pi[pp][ii] - mean_pr[pp][rr] - mean_ir[ii][rr]
                    + mean_p[pp]
                    + mean_i[ii]
                    + mean_r[rr]
                    - grand;
                ss_pir += term.powi(2);
            }
        }
    }

    let ms_p = ss_p / ((p - 1) as f64);
    let ms_i = ss_i / ((i - 1) as f64);
    let ms_r = ss_r / ((r - 1) as f64);
    let ms_pi = ss_pi / (((p - 1) * (i - 1)) as f64);
    let ms_pr = ss_pr / (((p - 1) * (r - 1)) as f64);
    let ms_ir = ss_ir / (((i - 1) * (r - 1)) as f64);
    let ms_pir = ss_pir / (((p - 1) * (i - 1) * (r - 1)) as f64);

    let sigma_p = (ms_p - ms_pi - ms_pr + ms_pir) / ((i * r) as f64);
    let sigma_i = (ms_i - ms_pi - ms_ir + ms_pir) / ((p * r) as f64);
    let sigma_r = (ms_r - ms_pr - ms_ir + ms_pir) / ((p * i) as f64);
    let sigma_pi = (ms_pi - ms_pir) / (r as f64);
    let sigma_pr = (ms_pr - ms_pir) / (i as f64);
    let sigma_ir = (ms_ir - ms_pir) / (p as f64);
    let sigma_pir = ms_pir;

    let g_den =
        sigma_p + sigma_pi / (i as f64) + sigma_pr / (r as f64) + sigma_pir / ((i * r) as f64);
    let phi_den = sigma_p
        + sigma_i / (i as f64)
        + sigma_r / (r as f64)
        + sigma_pi / (i as f64)
        + sigma_pr / (r as f64)
        + sigma_ir / ((i * r) as f64)
        + sigma_pir / ((i * r) as f64);

    Ok(GTheoryResult {
        sigma_p,
        sigma_i,
        sigma_r,
        sigma_pi,
        sigma_pr,
        sigma_ir,
        sigma_pir,
        g_coefficient: reliability_ratio("g_coefficient", sigma_p, g_den)?,
        phi_coefficient: reliability_ratio("phi_coefficient", sigma_p, phi_den)?,
        variance_component_ci_low: None,
        variance_component_ci_high: None,
        g_coefficient_ci_low: None,
        g_coefficient_ci_high: None,
        phi_coefficient_ci_low: None,
        phi_coefficient_ci_high: None,
        bootstrap_iterations: None,
        bootstrap_alpha: None,
        bootstrap_skipped: None,
    })
}

pub fn estimate_g_theory_pir_with_bootstrap(
    grid: &Array3<f64>,
    design: GTheoryDesign,
    n_resamples: usize,
    alpha: f64,
    rng: &mut RngState,
) -> Result<GTheoryResult, GTheoryBootstrapError> {
    let mut result = estimate_g_theory_pir(grid, design)?;
    let (component_ci, g_ci, phi_ci) =
        bootstrap_g_theory_pir(grid, design, n_resamples, alpha, rng)?;
    result.variance_component_ci_low = Some(component_ci.ci_low);
    result.variance_component_ci_high = Some(component_ci.ci_high);
    result.g_coefficient_ci_low = Some(g_ci.0);
    result.g_coefficient_ci_high = Some(g_ci.1);
    result.phi_coefficient_ci_low = Some(phi_ci.0);
    result.phi_coefficient_ci_high = Some(phi_ci.1);
    result.bootstrap_iterations = Some(component_ci.n_resamples);
    result.bootstrap_alpha = Some(component_ci.alpha);
    result.bootstrap_skipped = Some(component_ci.n_skipped);
    Ok(result)
}

#[allow(clippy::type_complexity)]
pub fn bootstrap_g_theory_pir(
    grid: &Array3<f64>,
    design: GTheoryDesign,
    n_resamples: usize,
    alpha: f64,
    rng: &mut RngState,
) -> Result<(BootstrapCi, (f64, f64), (f64, f64)), GTheoryBootstrapError> {
    let _ = estimate_g_theory_pir(grid, design)?;
    if n_resamples == 0 {
        return Err(GTheoryBootstrapError::Bootstrap(
            BootstrapGivenDataError::ZeroResamples,
        ));
    }
    if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
        return Err(GTheoryBootstrapError::Bootstrap(
            BootstrapGivenDataError::OutOfRangeAlpha { alpha },
        ));
    }

    let p = grid.shape()[0];
    let i = grid.shape()[1];
    let r = grid.shape()[2];
    let mut chacha = rng.clone().into_chacha();
    let mut p_idx = vec![0usize; p];
    let mut i_idx = vec![0usize; i];
    let mut r_idx = vec![0usize; r];
    let mut resampled = Array3::<f64>::zeros((p, i, r));
    let mut component_samples = (0..7)
        .map(|_| Vec::with_capacity(n_resamples))
        .collect::<Vec<_>>();
    let mut g_samples = Vec::with_capacity(n_resamples);
    let mut phi_samples = Vec::with_capacity(n_resamples);
    let mut n_skipped = 0usize;

    for _ in 0..n_resamples {
        for idx in &mut p_idx {
            *idx = (chacha.next_u32() as usize) % p;
        }
        for idx in &mut i_idx {
            *idx = (chacha.next_u32() as usize) % i;
        }
        for idx in &mut r_idx {
            *idx = (chacha.next_u32() as usize) % r;
        }
        for out_p in 0..p {
            for out_i in 0..i {
                for out_r in 0..r {
                    resampled[[out_p, out_i, out_r]] =
                        grid[[p_idx[out_p], i_idx[out_i], r_idx[out_r]]];
                }
            }
        }
        match estimate_g_theory_pir(&resampled, design) {
            Ok(est) => {
                let vals = [
                    est.sigma_p,
                    est.sigma_i,
                    est.sigma_r,
                    est.sigma_pi,
                    est.sigma_pr,
                    est.sigma_ir,
                    est.sigma_pir,
                ];
                for (samples, value) in component_samples.iter_mut().zip(vals) {
                    samples.push(value);
                }
                g_samples.push(est.g_coefficient);
                phi_samples.push(est.phi_coefficient);
            }
            Err(_) => n_skipped += 1,
        }
    }
    *rng = RngState::snapshot(&chacha, rng);
    if component_samples.iter().all(Vec::is_empty) {
        return Err(GTheoryBootstrapError::AllResamplesFailed);
    }
    let component_ci = build_bootstrap_ci(&component_samples, n_resamples, alpha, n_skipped)?;
    let (g_low, g_high) = percentile_ci(&g_samples, alpha / 2.0, 1.0 - alpha / 2.0);
    let (phi_low, phi_high) = percentile_ci(&phi_samples, alpha / 2.0, 1.0 - alpha / 2.0);
    Ok((component_ci, (g_low, g_high), (phi_low, phi_high)))
}

pub fn project_g_theory_d_study(
    result: &GTheoryResult,
    n_items: usize,
    n_raters: usize,
) -> Result<DStudyPoint, GTheoryError> {
    if n_items < 1 {
        return Err(GTheoryError::DegenerateAxis {
            axis: "n_items",
            len: n_items,
        });
    }
    if n_raters < 1 {
        return Err(GTheoryError::DegenerateAxis {
            axis: "n_raters",
            len: n_raters,
        });
    }
    let ni = n_items as f64;
    let nr = n_raters as f64;
    let g_den =
        result.sigma_p + result.sigma_pi / ni + result.sigma_pr / nr + result.sigma_pir / (ni * nr);
    let phi_den = result.sigma_p
        + result.sigma_i / ni
        + result.sigma_r / nr
        + result.sigma_pi / ni
        + result.sigma_pr / nr
        + result.sigma_ir / (ni * nr)
        + result.sigma_pir / (ni * nr);
    Ok(DStudyPoint {
        n_items,
        n_raters,
        g_coefficient: reliability_ratio("g_coefficient", result.sigma_p, g_den)?,
        phi_coefficient: reliability_ratio("phi_coefficient", result.sigma_p, phi_den)?,
    })
}

fn reliability_ratio(
    coefficient: &'static str,
    numerator: f64,
    denominator: f64,
) -> Result<f64, GTheoryError> {
    if denominator.abs() <= 1.0e-15 {
        if numerator.abs() <= 1.0e-15 {
            Ok(0.0)
        } else {
            Err(GTheoryError::UndefinedReliability { coefficient })
        }
    } else {
        Ok(numerator / denominator)
    }
}

fn build_bootstrap_ci(
    per_component: &[Vec<f64>],
    n_resamples: usize,
    alpha: f64,
    n_skipped: usize,
) -> Result<BootstrapCi, GTheoryBootstrapError> {
    if per_component.iter().all(Vec::is_empty) {
        return Err(GTheoryBootstrapError::AllResamplesFailed);
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

#[cfg(test)]
mod tests {
    use ndarray::Array3;
    use salib_core::RngState;

    use super::*;

    fn grid() -> Array3<f64> {
        let mut grid = Array3::<f64>::zeros((2, 2, 2));
        let levels = [-1.0_f64, 1.0_f64];
        for (ip, &p) in levels.iter().enumerate() {
            for (ii, &i) in levels.iter().enumerate() {
                for (ir, &r) in levels.iter().enumerate() {
                    grid[[ip, ii, ir]] = 50.0
                        + 6.0 * p
                        + 4.0 * i
                        + 2.0 * r
                        + 3.0 * p * i
                        + 1.5 * p * r
                        + 1.0 * i * r
                        + 0.5 * p * i * r;
                }
            }
        }
        grid
    }

    #[test]
    fn zero_variance_errors() {
        let grid = Array3::<f64>::from_elem((2, 2, 2), 1.0);
        assert_eq!(
            estimate_g_theory_pir(&grid, GTheoryDesign::Crossed).unwrap_err(),
            GTheoryError::ZeroVariance
        );
    }

    #[test]
    fn unsupported_design_errors() {
        let err = estimate_g_theory_pir(&grid(), GTheoryDesign::Nested).unwrap_err();
        assert_eq!(err, GTheoryError::UnsupportedDesign);
    }

    #[test]
    fn zero_object_variance_yields_zero_reliability_not_nan() {
        let mut grid = Array3::<f64>::zeros((2, 2, 2));
        let levels = [-1.0_f64, 1.0_f64];
        for (ip, _) in levels.iter().enumerate() {
            for (ii, &i) in levels.iter().enumerate() {
                for (ir, _) in levels.iter().enumerate() {
                    grid[[ip, ii, ir]] = 50.0 + 4.0 * i;
                }
            }
        }
        let result = estimate_g_theory_pir(&grid, GTheoryDesign::Crossed).unwrap();
        assert!(result.sigma_p.abs() < 1.0e-12);
        assert!(result.g_coefficient.abs() < 1.0e-12);
        assert!(result.phi_coefficient.abs() < 1.0e-12);
        assert!(result.g_coefficient.is_finite());
        assert!(result.phi_coefficient.is_finite());
    }

    #[test]
    fn nonzero_sigma_p_with_zero_denominator_errors() {
        let err = reliability_ratio("g_coefficient", 1.0, 0.0).unwrap_err();
        assert_eq!(
            err,
            GTheoryError::UndefinedReliability {
                coefficient: "g_coefficient"
            }
        );
    }

    #[test]
    fn bootstrap_invalid_params_error() {
        let mut rng = RngState::from_seed([0x55; 32]);
        let err =
            bootstrap_g_theory_pir(&grid(), GTheoryDesign::Crossed, 0, 0.05, &mut rng).unwrap_err();
        assert_eq!(
            err,
            GTheoryBootstrapError::Bootstrap(BootstrapGivenDataError::ZeroResamples)
        );
    }

    #[test]
    fn bootstrap_all_failed_errors() {
        let per_component = vec![Vec::new(), Vec::new()];
        let err = build_bootstrap_ci(&per_component, 32, 0.05, 32).unwrap_err();
        assert_eq!(err, GTheoryBootstrapError::AllResamplesFailed);
    }

    #[test]
    fn d_study_projection_increases_with_more_items_and_raters() {
        let base = estimate_g_theory_pir(&grid(), GTheoryDesign::Crossed).unwrap();
        let current = project_g_theory_d_study(&base, 2, 2).unwrap();
        let expanded = project_g_theory_d_study(&base, 4, 4).unwrap();
        assert!(expanded.g_coefficient > current.g_coefficient);
        assert!(expanded.phi_coefficient > current.phi_coefficient);
        assert!(current.g_coefficient > current.phi_coefficient);
        assert!(expanded.g_coefficient > expanded.phi_coefficient);
    }
}
