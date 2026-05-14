//! Discrepancy indices — space-filling quality metrics.
//!
//! Measures how uniformly a point set fills the unit hypercube
//! `[0, 1]^d`. Lower discrepancy ≈ better space-filling. Four
//! variants shipped:
//!
//! - **Centered Discrepancy (CD)** — Hickernell 1998, Eq 3.8.
//! - **Wrap-around Discrepancy (WD)** — Hickernell 1998, Eq 3.10.
//! - **Modified Discrepancy (MD)** — Fang et al. 2006.
//! - **L2-star Discrepancy** — Niederreiter classical.
//!
//! All returned values are the square root of the squared
//! discrepancy (i.e., the discrepancy itself, not its square).
//!
//! # References
//!
//! - Hickernell, F. J. (1998). A generalized discrepancy and
//!   quadrature error bound. *Mathematics of Computation*, 67(221),
//!   299-322.
//! - Fang, K.-T., Li, R., & Sudjianto, A. (2006). *Design and
//!   Modeling for Computer Experiments*. Chapman & Hall/CRC.
//! - Niederreiter, H. (1992). *Random Number Generation and
//!   Quasi-Monte Carlo Methods*. SIAM.

use ndarray::Array2;
use thiserror::Error;

/// Discrepancy results for a sample matrix in `[0, 1]^d`.
#[derive(Debug, Clone)]
pub struct DiscrepancyResult {
    /// Centered discrepancy (Hickernell 1998 Eq 3.8).
    pub centered: f64,
    /// Wrap-around discrepancy (Hickernell 1998 Eq 3.10).
    pub wrap_around: f64,
    /// Modified discrepancy (Fang et al. 2006).
    pub modified: f64,
    /// L2-star discrepancy (Niederreiter).
    pub l2_star: f64,
}

/// Errors from [`compute_discrepancy`].
#[derive(Debug, Clone, Error)]
pub enum DiscrepancyError {
    /// The sample matrix has zero rows.
    #[error("sample matrix is empty")]
    EmptyMatrix,
    /// A sample value lies outside `[0, 1]`.
    #[error("sample values must be in [0, 1], found {0}")]
    NotUnitInterval(f64),
}

/// Compute all four discrepancy indices for `sample` (shape `N × d`,
/// values in `[0, 1]`).
///
/// Returns `Err` if the matrix is empty or any value is outside
/// `[0, 1]`.
#[allow(clippy::cast_precision_loss)]
pub fn compute_discrepancy(sample: &Array2<f64>) -> Result<DiscrepancyResult, DiscrepancyError> {
    let n = sample.nrows();
    let d = sample.ncols();
    if n == 0 {
        return Err(DiscrepancyError::EmptyMatrix);
    }
    for &v in sample.iter() {
        if !(0.0..=1.0).contains(&v) {
            return Err(DiscrepancyError::NotUnitInterval(v));
        }
    }
    let n_f = n as f64;
    Ok(DiscrepancyResult {
        centered: centered_discrepancy(sample, n, d, n_f),
        wrap_around: wrap_around_discrepancy(sample, n, d, n_f),
        modified: modified_discrepancy(sample, n, d, n_f),
        l2_star: l2_star_discrepancy(sample, n, d, n_f),
    })
}

/// Centered Discrepancy — Hickernell 1998, Eq 3.8.
///
/// ```text
/// CD² = (13/12)^d
///     - (2/N) Σ_i Π_k [1 + |x_{ik} - 0.5|/2 - |x_{ik} - 0.5|²/2]
///     + (1/N²) Σ_i Σ_j Π_k [1 + |x_{ik} - 0.5|/2
///                                + |x_{jk} - 0.5|/2
///                                - |x_{ik} - x_{jk}|/2]
/// ```
#[allow(clippy::similar_names)]
fn centered_discrepancy(sample: &Array2<f64>, n: usize, d: usize, n_f: f64) -> f64 {
    let term1 = (13.0_f64 / 12.0).powi(d as i32);

    let mut sum2 = 0.0;
    for i in 0..n {
        let mut prod = 1.0;
        for k in 0..d {
            let xik = sample[[i, k]];
            let abs_half = (xik - 0.5).abs();
            prod *= 1.0 + abs_half / 2.0 - abs_half * abs_half / 2.0;
        }
        sum2 += prod;
    }

    let mut sum3 = 0.0;
    for i in 0..n {
        for j in 0..n {
            let mut prod = 1.0;
            for k in 0..d {
                let xik = sample[[i, k]];
                let xjk = sample[[j, k]];
                prod *= 1.0 + (xik - 0.5).abs() / 2.0 + (xjk - 0.5).abs() / 2.0
                    - (xik - xjk).abs() / 2.0;
            }
            sum3 += prod;
        }
    }

    let cd_sq = term1 - (2.0 / n_f) * sum2 + (1.0 / (n_f * n_f)) * sum3;
    cd_sq.max(0.0).sqrt()
}

/// Wrap-around Discrepancy — Hickernell 1998, Eq 3.10.
///
/// ```text
/// WD² = -(4/3)^d
///     + (1/N²) Σ_i Σ_j Π_k [3/2 - |x_{ik} - x_{jk}| · (1 - |x_{ik} - x_{jk}|)]
/// ```
fn wrap_around_discrepancy(sample: &Array2<f64>, n: usize, d: usize, n_f: f64) -> f64 {
    let term1 = -((4.0_f64 / 3.0).powi(d as i32));

    let mut sum = 0.0;
    for i in 0..n {
        for j in 0..n {
            let mut prod = 1.0;
            for k in 0..d {
                let diff = (sample[[i, k]] - sample[[j, k]]).abs();
                prod *= 1.5 - diff * (1.0 - diff);
            }
            sum += prod;
        }
    }

    let wd_sq = term1 + (1.0 / (n_f * n_f)) * sum;
    wd_sq.max(0.0).sqrt()
}

/// L2-star Discrepancy — Niederreiter.
///
/// ```text
/// L2*² = (1/3)^d
///      - (2^{1-d}/N) Σ_i Π_k (1 - x_{ik}²)
///      + (1/N²) Σ_i Σ_j Π_k [1 - max(x_{ik}, x_{jk})]
/// ```
fn l2_star_discrepancy(sample: &Array2<f64>, n: usize, d: usize, n_f: f64) -> f64 {
    let term1 = (1.0_f64 / 3.0).powi(d as i32);
    let coeff2 = 2.0_f64.powi(1 - d as i32) / n_f;

    let mut sum2 = 0.0;
    for i in 0..n {
        let mut prod = 1.0;
        for k in 0..d {
            let xik = sample[[i, k]];
            prod *= 1.0 - xik * xik;
        }
        sum2 += prod;
    }

    let mut sum3 = 0.0;
    for i in 0..n {
        for j in 0..n {
            let mut prod = 1.0;
            for k in 0..d {
                prod *= 1.0 - sample[[i, k]].max(sample[[j, k]]);
            }
            sum3 += prod;
        }
    }

    let l2_sq = term1 - coeff2 * sum2 + (1.0 / (n_f * n_f)) * sum3;
    l2_sq.max(0.0).sqrt()
}

/// Modified Discrepancy — Fang et al. 2006.
///
/// ```text
/// MD² = (19/12)^d
///     - (2/N) Σ_i Π_k [(19 - 5|2x_{ik}-1| - 5|2x_{ik}-1|²) / 12]
///     + (1/N²) Σ_i Σ_j Π_k [(19 - 5|2x_{ik}-1| - 5|2x_{jk}-1|
///                              + 5|x_{ik}-x_{jk}|) / 12]
/// ```
///
/// Cross-check: for a single point at `(0.5, 0.5)`, the single-sum
/// kernel should be `(19/12)^2` and the double-sum kernel should be
/// `(19/12)^2`, yielding `MD² = (19/12)^2 - 2·(19/12)^2 + (19/12)^2 = 0`.
#[allow(clippy::similar_names)]
fn modified_discrepancy(sample: &Array2<f64>, n: usize, d: usize, n_f: f64) -> f64 {
    let term1 = (19.0_f64 / 12.0).powi(d as i32);

    let mut sum2 = 0.0;
    for i in 0..n {
        let mut prod = 1.0;
        for k in 0..d {
            let xik = sample[[i, k]];
            let t = (2.0 * xik - 1.0).abs();
            prod *= (19.0 - 5.0 * t - 5.0 * t * t) / 12.0;
        }
        sum2 += prod;
    }

    let mut sum3 = 0.0;
    for i in 0..n {
        for j in 0..n {
            let mut prod = 1.0;
            for k in 0..d {
                let xik = sample[[i, k]];
                let xjk = sample[[j, k]];
                let ti = (2.0 * xik - 1.0).abs();
                let tj = (2.0 * xjk - 1.0).abs();
                let dij = (xik - xjk).abs();
                prod *= (19.0 - 5.0 * ti - 5.0 * tj + 5.0 * dij) / 12.0;
            }
            sum3 += prod;
        }
    }

    let md_sq = term1 - (2.0 / n_f) * sum2 + (1.0 / (n_f * n_f)) * sum3;
    md_sq.max(0.0).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    /// Single point at center: MD should be zero (cross-check from
    /// the docstring).
    #[test]
    fn modified_single_center_is_zero() {
        let sample = array![[0.5, 0.5]];
        let r = compute_discrepancy(&sample).unwrap();
        assert!(r.modified < 1e-12, "MD = {} should be ~0", r.modified);
    }

    /// All four metrics are non-negative for a trivial 1-point sample.
    #[test]
    fn all_non_negative_single_point() {
        let sample = array![[0.3, 0.7]];
        let r = compute_discrepancy(&sample).unwrap();
        assert!(r.centered >= 0.0);
        assert!(r.wrap_around >= 0.0);
        assert!(r.modified >= 0.0);
        assert!(r.l2_star >= 0.0);
    }

    /// Empty matrix returns `EmptyMatrix`.
    #[test]
    fn empty_matrix_error() {
        let sample = Array2::<f64>::zeros((0, 3));
        assert!(matches!(
            compute_discrepancy(&sample),
            Err(DiscrepancyError::EmptyMatrix)
        ));
    }

    /// Out-of-range value returns `NotUnitInterval`.
    #[test]
    fn out_of_range_error() {
        let sample = array![[0.5, 1.5]];
        assert!(matches!(
            compute_discrepancy(&sample),
            Err(DiscrepancyError::NotUnitInterval(v)) if (v - 1.5).abs() < 1e-15
        ));
    }
}
