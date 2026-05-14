//! RS-HDMR (High-Dimensional Model Representation) via PCE
//! decomposition.
//!
//! Fits a full polynomial chaos expansion to `(x, y)` data, then
//! decomposes the PCE coefficients by interaction order to produce
//! HDMR component variances and Sobol' indices.
//!
//! # Algorithm
//!
//! 1. Map physical-domain inputs to each factor's polynomial-canonical
//!    domain (Legendre `[-1, 1]` for Uniform, Hermite `ℝ` for Normal).
//! 2. Fit a full PCE of total degree `max_degree` via OLS
//!    (`salib_surrogate::fit_full_pce`).
//! 3. For each non-constant basis function `α`, compute its variance
//!    contribution `β_α² · ∏_k ⟨Ψ_{α_k}, Ψ_{α_k}⟩`.
//! 4. Group contributions by interaction order (number of active
//!    factors) and by factor subset, accumulating into first-order,
//!    second-order, and total-order Sobol' indices.
//!
//! # Relation to `sobol_indices_from_pce`
//!
//! `sobol_indices_from_pce` (in `salib-surrogate`) computes the same
//! first-order and total-order indices from the same PCE. HDMR adds:
//! - Second-order pairwise indices `S2_{i,j}`.
//! - Per-interaction-order variance fractions.
//! - Parameterized `max_order` truncation.
//!
//! The first-order and total-order results are algebraically identical
//! to `sobol_indices_from_pce` (up to floating-point summation order).
//!
//! # References
//!
//! - Rabitz et al. 1999. General foundations of HDMR.
//! - Li et al. 2001. RS-HDMR via orthogonal polynomials.
//! - Sudret 2008. PCE-based Sobol' indices (Eq 36-39).

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::expect_used
)]

use ndarray::Array2;
use salib_core::Problem;
use salib_surrogate::{fit_full_pce, norm_squared, PceError, PolynomialChaos, PolynomialFamily};
use thiserror::Error;

/// Result of RS-HDMR variance decomposition.
///
/// Contains the fitted PCE, first- and second-order Sobol' indices,
/// total-order indices, and per-interaction-order variance fractions.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct HdmrResult {
    /// Factor count `d`.
    pub dim: usize,
    /// Total output variance estimated from the PCE (unnormalized).
    pub total_variance: f64,
    /// Variance fraction per interaction order, normalized by total
    /// variance. `order_variance[0]` = sum of all first-order
    /// component variances / D, `order_variance[1]` = sum of all
    /// second-order / D, etc. Length = `max_order`.
    pub order_variance: Vec<f64>,
    /// First-order Sobol' indices `S_i`, length `d`.
    pub first_order: Vec<f64>,
    /// Second-order Sobol' indices. `second_order[i][k] = S2_{i, i+k+1}`
    /// (upper triangle, row-major). For `d = 3`:
    /// `second_order[0] = [S2_{0,1}, S2_{0,2}]`,
    /// `second_order[1] = [S2_{1,2}]`.
    pub second_order: Vec<Vec<f64>>,
    /// Total-order Sobol' indices `S_{T_i}`, length `d`.
    pub total_order: Vec<f64>,
    /// The fitted PCE (for inspection/reuse).
    pub pce: PolynomialChaos,
}

/// Errors from [`estimate_hdmr`].
#[derive(Debug, Clone, Error)]
pub enum HdmrError {
    /// Total variance is zero or negative — model output is constant
    /// or the PCE fit collapsed.
    #[error("total variance is zero or negative")]
    ZeroVariance,
    /// PCE fit failed (delegated from `salib-surrogate`).
    #[error("PCE fit failed: {0}")]
    PceFitFailed(#[from] PceError),
}

/// RS-HDMR via PCE decomposition.
///
/// Fits a full polynomial chaos expansion to `(x, y)` data, then
/// decomposes the PCE coefficients by interaction order to produce
/// HDMR component variances and Sobol' indices.
///
/// # Arguments
///
/// * `x` — `N × d` sample matrix in the physical domain (each
///   factor's support).
/// * `y` — model output vector of length `N`.
/// * `problem` — defines factor distributions (for canonical-domain
///   mapping + family selection).
/// * `max_order` — maximum interaction order to track (2 = up to
///   pairwise).
/// * `max_degree` — PCE polynomial truncation degree.
///
/// # Errors
///
/// - [`HdmrError::PceFitFailed`] — delegated from `fit_full_pce`.
/// - [`HdmrError::ZeroVariance`] — total PCE variance is zero.
pub fn estimate_hdmr(
    x: &Array2<f64>,
    y: &[f64],
    problem: &Problem,
    max_order: usize,
    max_degree: usize,
) -> Result<HdmrResult, HdmrError> {
    let d = problem.dim();
    let n = x.nrows();

    // Choose polynomial families based on factor distributions.
    let families: Vec<PolynomialFamily> = problem
        .factors()
        .iter()
        .map(|f| match f.distribution {
            salib_core::Distribution::Normal { .. } => PolynomialFamily::Hermite,
            _ => PolynomialFamily::Legendre,
        })
        .collect();

    // Map physical inputs to canonical domain.
    // For Legendre (Uniform(lo, hi)): canonical = 2*(x - lo)/(hi - lo) - 1 ∈ [-1, 1]
    // For Hermite (Normal(mu, sigma)): canonical = (x - mu) / sigma
    let mut x_canonical = Array2::<f64>::zeros((n, d));
    for i in 0..n {
        for j in 0..d {
            x_canonical[[i, j]] = match families[j] {
                PolynomialFamily::Hermite => {
                    if let salib_core::Distribution::Normal { mu, sigma } =
                        problem.factors()[j].distribution
                    {
                        (x[[i, j]] - mu) / sigma
                    } else {
                        x[[i, j]]
                    }
                }
                _ => {
                    let (lo, hi) = problem.factors()[j].distribution.support();
                    2.0 * (x[[i, j]] - lo) / (hi - lo) - 1.0
                }
            };
        }
    }

    // Fit PCE.
    let pce = fit_full_pce(&x_canonical, y, &families, max_degree)?;

    // Per-basis-function variance contribution: β_α² · ∏_k ⟨Ψ_{α_k}, Ψ_{α_k}⟩
    let contributions: Vec<f64> = pce
        .multi_indices
        .iter()
        .zip(pce.coefficients.iter())
        .map(|(alpha, &beta)| {
            let mut ns = 1.0;
            for (k, &deg) in alpha.indices.iter().enumerate() {
                ns *= norm_squared(pce.families[k], deg);
            }
            beta * beta * ns
        })
        .collect();

    // Total variance = sum over all non-constant basis functions.
    let total_variance: f64 = pce
        .multi_indices
        .iter()
        .zip(contributions.iter())
        .filter(|(alpha, _)| !alpha.is_zero())
        .map(|(_, &c)| c)
        .sum();

    if total_variance < 1e-15 {
        return Err(HdmrError::ZeroVariance);
    }

    // Accumulate by interaction order and by factor.
    let mut first_order = vec![0.0_f64; d];
    let mut total_order = vec![0.0_f64; d];
    let mut order_variance = vec![0.0_f64; max_order];
    let mut s2: Vec<Vec<f64>> = (0..d).map(|i| vec![0.0_f64; d - i - 1]).collect();

    for (alpha, &c) in pce.multi_indices.iter().zip(contributions.iter()) {
        if alpha.is_zero() {
            continue;
        }
        let active = alpha.active_factors();
        let order = active.len();

        // Accumulate order variance (capped at max_order).
        if order <= max_order {
            order_variance[order - 1] += c;
        }

        // First-order: exactly one active factor.
        if order == 1 {
            first_order[active[0]] += c;
        }

        // Second-order: exactly two active factors.
        if order == 2 {
            let (i, j) = (active[0], active[1]);
            s2[i][j - i - 1] += c;
        }

        // Total-order: every active factor gets the contribution.
        for &i in &active {
            total_order[i] += c;
        }
    }

    // Normalize by total variance.
    for v in &mut first_order {
        *v = (*v / total_variance).clamp(0.0, 1.0);
    }
    for v in &mut total_order {
        *v = (*v / total_variance).clamp(0.0, 1.0);
    }
    for row in &mut s2 {
        for v in row.iter_mut() {
            *v = (*v / total_variance).clamp(0.0, 1.0);
        }
    }
    for v in &mut order_variance {
        *v /= total_variance;
    }

    Ok(HdmrResult {
        dim: d,
        total_variance,
        order_variance,
        first_order,
        second_order: s2,
        total_order,
        pce,
    })
}
