//! Regression-based sensitivity indices: SRC, SRRC, PCC, PRCC,
//! plus `R²` diagnostics for the linear and rank-linear regressions.
//!
//! Per `decisions/2026-04-29-saltelli-regression.md`.
//!
//! # The four estimators
//!
//! - **SRC (Standardized Regression Coefficient)** —
//!   `β̂ᵢ · σ_{Xᵢ} / σ_Y` from the OLS fit `Y ≈ β₀ + β·X`.
//!   Trustworthy when the model is approximately linear; pin by
//!   `r²_linear > 0.7`.
//!
//! - **SRRC (Standardized Rank Regression Coefficient)** — SRC
//!   computed on rank-transformed `(X, Y)`. Trustworthy when the
//!   model is approximately monotonic; pin by `r²_rank > 0.7`.
//!
//! - **PCC (Partial Correlation Coefficient)** — Pearson
//!   correlation between residuals of `Xᵢ` and `Y` after each is
//!   regressed on the *other* `X` factors. Captures `Xᵢ`'s unique
//!   linear contribution after partialing out other factors.
//!
//! - **PRCC (Partial Rank Correlation Coefficient)** — PCC on
//!   ranks. Captures monotonic partial contribution.
//!
//! All four are sampler-agnostic — work on any `(X, Y)` from any
//! sampler. **None recover Sobol' indices** unless the model is
//! linear (SRC) or monotonic (SRRC/PRCC). The `R²` diagnostics are
//! the load-bearing trust signal.
//!
//! # Why these alongside Sobol'/Morris/etc.
//!
//! Cheap relative to variance-based methods. The PCC/PRCC path
//! re-fits OLS once per factor (residualizing on every other
//! factor), so the asymptotic is `O(N · d³)` for typical `d`.
//! Could be reduced to `O(N · d² + d³)` by inverting `XᵀX` once
//! and deriving partial correlations from the inverse — bead-
//! eligible if a workload pushes `d` past ~50.
//!
//! # Determinism
//!
//! Pure under `(X, Y)`. OLS via normal equations + Cholesky
//! solve (`nalgebra`). Stable rank with `partial_cmp(...)
//! .unwrap_or(Equal)`. Same `(X, Y)` in → bit-identical output.

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::needless_range_loop
)]

use std::cmp::Ordering;

use nalgebra::{DMatrix, DVector};
use ndarray::Array2;
use salib_core::tree_sum;

/// Regression-based sensitivity indices and `R²` diagnostics.
///
/// `#[non_exhaustive]` — future fields (`bootstrap_ci`,
/// `condition_number` of the design matrix for ill-conditioning
/// detection) land non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RegressionIndices {
    /// Standardized regression coefficients, length `d`. Trust if
    /// `r²_linear > 0.7`.
    pub src: Vec<f64>,
    /// Standardized rank regression coefficients, length `d`.
    /// Trust if `r²_rank > 0.7`.
    pub srrc: Vec<f64>,
    /// Partial correlation coefficients, length `d`.
    pub pcc: Vec<f64>,
    /// Partial rank correlation coefficients, length `d`.
    pub prcc: Vec<f64>,
    /// `R²` of the linear OLS fit `Y ≈ β₀ + β·X`. Diagnostic for
    /// SRC and PCC trustworthiness.
    pub r2_linear: f64,
    /// `R²` of the rank-linear OLS fit. Diagnostic for SRRC/PRCC.
    pub r2_rank: f64,
}

impl RegressionIndices {
    /// Factor count.
    #[must_use]
    pub fn d(&self) -> usize {
        self.src.len()
    }
}

/// Errors from [`estimate_regression_indices`].
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum RegressionError {
    #[error("regression: shape mismatch — X has {x_rows} rows, y has {y_len} elements")]
    ShapeMismatch { x_rows: usize, y_len: usize },
    #[error("regression: d must be ≥ 1, got 0")]
    ZeroD,
    /// Need at least `d + 2` samples to fit `d` regression
    /// coefficients + intercept and have ≥ 1 residual DOF.
    #[error("regression: N must be ≥ d + 2 (got N={n}, d={d}, minimum={minimum})")]
    InsufficientSamples { n: usize, d: usize, minimum: usize },
    /// Total variance of `Y` is zero — model is constant; no
    /// regression signal to recover.
    #[error("regression: Var(Y) is zero (model output is constant)")]
    ZeroVariance,
    /// Total variance of `Xᵢ` is zero for some factor — design
    /// matrix is rank-deficient.
    #[error("regression: Var(X[:, {factor}]) is zero — design matrix rank-deficient")]
    ZeroFactorVariance { factor: usize },
    /// `(XᵀX)` is singular — design matrix is rank-deficient
    /// despite per-factor variance checks. Possible collinearity.
    #[error("regression: design matrix XᵀX is singular (factor collinearity?)")]
    SingularDesignMatrix,
}

/// Estimate SRC / SRRC / PCC / PRCC plus `R²` diagnostics.
///
/// `x` is the `(N, d)` input matrix; `y` is the `N`-element model
/// output. Sampler-agnostic.
///
/// # Errors
///
/// - [`RegressionError::ShapeMismatch`] if `x.nrows() != y.len()`.
/// - [`RegressionError::ZeroD`] if `x.ncols() == 0`.
/// - [`RegressionError::InsufficientSamples`] if `N < d + 2`.
/// - [`RegressionError::ZeroVariance`] if `Var(Y) ≈ 0`.
/// - [`RegressionError::ZeroFactorVariance`] if any column of `X`
///   is constant.
/// - [`RegressionError::SingularDesignMatrix`] if `XᵀX` is
///   non-invertible.
pub fn estimate_regression_indices(
    x: &Array2<f64>,
    y: &[f64],
) -> Result<RegressionIndices, RegressionError> {
    let n = x.nrows();
    let d = x.ncols();
    if d == 0 {
        return Err(RegressionError::ZeroD);
    }
    if y.len() != n {
        return Err(RegressionError::ShapeMismatch {
            x_rows: n,
            y_len: y.len(),
        });
    }
    let minimum = d + 2;
    if n < minimum {
        return Err(RegressionError::InsufficientSamples { n, d, minimum });
    }

    // Per-column variance checks.
    let var_y = sample_variance(y);
    if var_y < 1e-15 {
        return Err(RegressionError::ZeroVariance);
    }
    for j in 0..d {
        let col: Vec<f64> = (0..n).map(|k| x[[k, j]]).collect();
        if sample_variance(&col) < 1e-15 {
            return Err(RegressionError::ZeroFactorVariance { factor: j });
        }
    }

    // ── Linear regression: SRC + PCC + R²_linear ──────────────────
    let (src, pcc, r2_linear) = compute_src_pcc_r2(x, y)?;

    // ── Rank regression: SRRC + PRCC + R²_rank ────────────────────
    // Build rank-transformed X and Y.
    let mut x_rank = Array2::<f64>::zeros((n, d));
    for j in 0..d {
        let col: Vec<f64> = (0..n).map(|k| x[[k, j]]).collect();
        let r = ordinal_ranks_f64(&col);
        for k in 0..n {
            x_rank[[k, j]] = r[k];
        }
    }
    let y_rank = ordinal_ranks_f64(y);

    let (srrc, prcc, r2_rank) = compute_src_pcc_r2(&x_rank, &y_rank)?;

    Ok(RegressionIndices {
        src,
        srrc,
        pcc,
        prcc,
        r2_linear,
        r2_rank,
    })
}

/// Compute SRC, PCC, and `R²` for the regression of `y` on `x`.
///
/// SRC: `β̂ᵢ · σ_{Xᵢ} / σ_y` from `Y ≈ β₀ + β·X`.
/// PCC: Pearson correlation between residuals of `Xᵢ` and `y`
///      regressed on the *other* X columns.
fn compute_src_pcc_r2(
    x: &Array2<f64>,
    y: &[f64],
) -> Result<(Vec<f64>, Vec<f64>, f64), RegressionError> {
    let n = x.nrows();
    let d = x.ncols();

    // OLS: Y = β₀ + β·X. Build augmented design matrix [1, X].
    let design = build_design_matrix(x, n, d);
    let y_vec = DVector::from_iterator(n, y.iter().copied());

    let beta = solve_ols(&design, &y_vec)?;

    // Residuals and R².
    let y_hat = &design * &beta;
    let residuals = &y_vec - &y_hat;
    let ss_res = residuals.dot(&residuals);
    let y_mean = y_vec.mean();
    let centered = y_vec.map(|v| v - y_mean);
    let ss_tot = centered.dot(&centered);
    let r2 = if ss_tot > 1e-15 {
        1.0 - ss_res / ss_tot
    } else {
        0.0
    };

    // SRC = β̂_j · σ_X_j / σ_Y for j = 1..=d (skipping intercept).
    let sigma_y = (ss_tot / (n as f64)).sqrt();
    let mut src = vec![0.0_f64; d];
    for j in 0..d {
        let col: Vec<f64> = (0..n).map(|k| x[[k, j]]).collect();
        let sigma_xj = sample_variance(&col).sqrt();
        // β[0] is intercept; β[j+1] is coefficient on X_j.
        src[j] = beta[j + 1] * sigma_xj / sigma_y;
    }

    // PCC: for each factor j, regress X_j on the *other* factors,
    // regress y on the other factors, then correlate residuals.
    let mut pcc = vec![0.0_f64; d];
    for j in 0..d {
        pcc[j] = partial_correlation(x, y, j)?;
    }

    Ok((src, pcc, r2))
}

/// `(N, d+1)` design matrix `[1, X]`.
fn build_design_matrix(x: &Array2<f64>, n: usize, d: usize) -> DMatrix<f64> {
    let mut design = DMatrix::<f64>::zeros(n, d + 1);
    for k in 0..n {
        design[(k, 0)] = 1.0;
        for j in 0..d {
            design[(k, j + 1)] = x[[k, j]];
        }
    }
    design
}

/// Solve `(XᵀX) β = Xᵀ y` via Cholesky.
fn solve_ols(design: &DMatrix<f64>, y: &DVector<f64>) -> Result<DVector<f64>, RegressionError> {
    let xt = design.transpose();
    let xtx = &xt * design;
    let xty = &xt * y;
    xtx.cholesky()
        .ok_or(RegressionError::SingularDesignMatrix)
        .map(|chol| chol.solve(&xty))
}

/// Partial correlation between `X[:, j]` and `y`, controlling for
/// the other columns of `X`.
fn partial_correlation(x: &Array2<f64>, y: &[f64], j: usize) -> Result<f64, RegressionError> {
    let n = x.nrows();
    let d = x.ncols();

    // Build "other factors" matrix Z = X with column j removed.
    let z = if d > 1 {
        let mut z_arr = Array2::<f64>::zeros((n, d - 1));
        let mut col_in_z = 0;
        for j_other in 0..d {
            if j_other == j {
                continue;
            }
            for k in 0..n {
                z_arr[[k, col_in_z]] = x[[k, j_other]];
            }
            col_in_z += 1;
        }
        z_arr
    } else {
        Array2::<f64>::zeros((n, 0))
    };

    // Residuals from regressing X[:, j] on Z (with intercept) and
    // y on Z.
    let xj: Vec<f64> = (0..n).map(|k| x[[k, j]]).collect();
    let xj_resid = if d > 1 {
        residuals_from_regression(&z, &xj, n, d - 1)?
    } else {
        // d = 1: no other factors → residuals = X_j - mean(X_j).
        let mean_xj = tree_sum(&xj) / (n as f64);
        xj.iter().map(|&v| v - mean_xj).collect()
    };
    let y_resid = if d > 1 {
        residuals_from_regression(&z, y, n, d - 1)?
    } else {
        let mean_y = tree_sum(y) / (n as f64);
        y.iter().map(|&v| v - mean_y).collect()
    };

    Ok(pearson_correlation(&xj_resid, &y_resid))
}

/// Fit `target ≈ α₀ + α·z` via OLS and return the residuals.
fn residuals_from_regression(
    z: &Array2<f64>,
    target: &[f64],
    n: usize,
    d_z: usize,
) -> Result<Vec<f64>, RegressionError> {
    let design = build_design_matrix(z, n, d_z);
    let y_vec = DVector::from_iterator(n, target.iter().copied());
    let beta = solve_ols(&design, &y_vec)?;
    let predicted = &design * &beta;
    let resid = (0..n).map(|k| y_vec[k] - predicted[k]).collect();
    Ok(resid)
}

/// Pearson correlation. Returns 0.0 if either input has zero variance.
fn pearson_correlation(a: &[f64], b: &[f64]) -> f64 {
    debug_assert_eq!(a.len(), b.len());
    let n = a.len() as f64;
    let mean_a = tree_sum(a) / n;
    let mean_b = tree_sum(b) / n;
    let mut cov = 0.0;
    let mut var_a = 0.0;
    let mut var_b = 0.0;
    for (av, bv) in a.iter().zip(b.iter()) {
        let da = av - mean_a;
        let db = bv - mean_b;
        cov += da * db;
        var_a += da * da;
        var_b += db * db;
    }
    let denom = (var_a * var_b).sqrt();
    if denom < 1e-15 {
        0.0
    } else {
        cov / denom
    }
}

/// Population variance (1/n divisor; not Bessel) — matches the
/// scaling used in `SALib`'s regression module so SRC values are
/// directly comparable.
fn sample_variance(v: &[f64]) -> f64 {
    let n = v.len() as f64;
    let mean = tree_sum(v) / n;
    let sq_sum: f64 = v.iter().map(|x| (x - mean).powi(2)).sum();
    sq_sum / n
}

/// Ordinal ranks (1..=N, stable tie-break by input order). Same
/// posture as `pawn::ordinal_ranks` but typed to `f64` directly
/// for use as regression input.
fn ordinal_ranks_f64(data: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..data.len()).collect();
    idx.sort_by(|&a, &b| data[a].partial_cmp(&data[b]).unwrap_or(Ordering::Equal));
    let mut ranks = vec![0.0_f64; data.len()];
    for (rank, &i) in idx.iter().enumerate() {
        ranks[i] = (rank + 1) as f64;
    }
    ranks
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use super::*;

    fn synthetic_x(n: usize, d: usize) -> Array2<f64> {
        let mut x = Array2::<f64>::zeros((n, d));
        for j in 0..d {
            let mut perm: Vec<usize> = (0..n).collect();
            let mut state: u64 = 0x9E37_79B9_7F4A_7C15_u64.wrapping_mul((j as u64).wrapping_add(1));
            for i in (1..n).rev() {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                let k = (state >> 33) as usize % (i + 1);
                perm.swap(i, k);
            }
            for i in 0..n {
                x[[i, j]] = (perm[i] as f64 + 0.5) / (n as f64);
            }
        }
        x
    }

    // ── Validation ────────────────────────────────────────────────

    #[test]
    fn zero_d_errors() {
        let x = Array2::<f64>::zeros((100, 0));
        let y = vec![0.0; 100];
        assert_eq!(
            estimate_regression_indices(&x, &y).unwrap_err(),
            RegressionError::ZeroD
        );
    }

    #[test]
    fn shape_mismatch_errors() {
        let x = Array2::<f64>::zeros((100, 3));
        let y = vec![0.0; 50];
        let err = estimate_regression_indices(&x, &y).unwrap_err();
        assert!(matches!(err, RegressionError::ShapeMismatch { .. }));
    }

    #[test]
    fn insufficient_samples_errors() {
        // d=3, need N ≥ 5.
        let x = synthetic_x(4, 3);
        let y = vec![0.0; 4];
        let err = estimate_regression_indices(&x, &y).unwrap_err();
        assert!(matches!(err, RegressionError::InsufficientSamples { .. }));
    }

    #[test]
    fn constant_model_errors() {
        let x = synthetic_x(64, 3);
        let y = vec![1.0; 64];
        let err = estimate_regression_indices(&x, &y).unwrap_err();
        assert_eq!(err, RegressionError::ZeroVariance);
    }

    #[test]
    fn zero_factor_variance_errors() {
        let mut x = synthetic_x(64, 3);
        for k in 0..64 {
            x[[k, 1]] = 0.5; // factor 1 constant
        }
        let y: Vec<f64> = (0..64).map(|k| x[[k, 0]] + x[[k, 2]]).collect();
        let err = estimate_regression_indices(&x, &y).unwrap_err();
        assert_eq!(err, RegressionError::ZeroFactorVariance { factor: 1 });
    }

    // ── Output shape ──────────────────────────────────────────────

    #[test]
    fn output_lengths_match_d() {
        let n = 64;
        let x = synthetic_x(n, 4);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]] + 2.0 * x[[k, 1]]).collect();
        let est = estimate_regression_indices(&x, &y).unwrap();
        assert_eq!(est.d(), 4);
        assert_eq!(est.src.len(), 4);
        assert_eq!(est.srrc.len(), 4);
        assert_eq!(est.pcc.len(), 4);
        assert_eq!(est.prcc.len(), 4);
    }

    // ── Linear model: SRC + PCC near 1 for active factor ─────────

    #[test]
    fn linear_model_recovers_high_r2_and_dominant_src() {
        // Y = X[:, 0] — perfectly linear, factor 0 dominant.
        let n = 256;
        let x = synthetic_x(n, 3);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]]).collect();
        let est = estimate_regression_indices(&x, &y).unwrap();
        assert!(est.r2_linear > 0.99, "R²_linear = {}", est.r2_linear);
        assert!(
            est.src[0].abs() > 0.95,
            "|SRC_0| = {} should dominate for Y = X_0",
            est.src[0].abs()
        );
        assert!(
            est.src[1].abs() < 0.1,
            "|SRC_1| = {} should be small",
            est.src[1].abs()
        );
        assert!(
            est.pcc[0].abs() > 0.95,
            "|PCC_0| = {} should dominate",
            est.pcc[0].abs()
        );
    }

    // ── Linear with multiple factors ─────────────────────────────

    #[test]
    fn linear_combination_yields_proportional_src() {
        // Y = 2·X[:, 0] + X[:, 1]. With independent uniform X,
        // SRC scales with the coefficient times std ratio. Both
        // factors have the same std, so SRC_0 should be ~2× SRC_1.
        let n = 1024;
        let x = synthetic_x(n, 3);
        let y: Vec<f64> = (0..n).map(|k| 2.0 * x[[k, 0]] + x[[k, 1]]).collect();
        let est = estimate_regression_indices(&x, &y).unwrap();
        assert!(est.r2_linear > 0.99);
        // Ratio SRC_0 / SRC_1 should be ≈ 2 within MC noise.
        let ratio = est.src[0].abs() / est.src[1].abs();
        assert!(
            (ratio - 2.0).abs() < 0.2,
            "SRC ratio = {ratio}, expected ≈ 2"
        );
    }

    // ── Non-linear: low R² flags untrustworthy SRC ────────────────

    #[test]
    fn nonlinear_model_yields_low_r2_linear() {
        // Y = X[:, 0]² is non-linear. R²_linear should be lower
        // than R²_rank since rank captures monotonic-in-X²
        // structure.
        let n = 512;
        let x = synthetic_x(n, 2);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]].powi(2)).collect();
        let est = estimate_regression_indices(&x, &y).unwrap();
        // R²_linear is reduced because Y vs X is not linear.
        // (It's still > 0 since X² is correlated with X over [0, 1].)
        assert!(est.r2_linear < 0.99, "R²_linear = {}", est.r2_linear);
        // R²_rank should be high — Y is monotonic in X over [0, 1].
        assert!(est.r2_rank > 0.95, "R²_rank = {}", est.r2_rank);
    }

    // ── PRCC catches monotonicity that PCC misses on cubed input ─

    #[test]
    fn prcc_catches_strong_monotonic_relationship_with_high_magnitude() {
        // Y = (X[:, 0] - 0.5)³ — strictly monotonic in X_0 over
        // [0, 1] (cube preserves order). PRCC_0 should be ~1.
        let n = 1024;
        let x = synthetic_x(n, 2);
        let y: Vec<f64> = (0..n).map(|k| (x[[k, 0]] - 0.5).powi(3)).collect();
        let est = estimate_regression_indices(&x, &y).unwrap();
        assert!(
            est.prcc[0].abs() > 0.95,
            "|PRCC_0| = {} should be near 1 (strict monotonic)",
            est.prcc[0].abs()
        );
    }

    // ── Determinism ───────────────────────────────────────────────

    #[test]
    fn same_input_yields_identical_output() {
        let n = 64;
        let x = synthetic_x(n, 3);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]] + x[[k, 1]] * x[[k, 2]]).collect();
        let a = estimate_regression_indices(&x, &y).unwrap();
        let b = estimate_regression_indices(&x, &y).unwrap();
        assert_eq!(a.src, b.src);
        assert_eq!(a.srrc, b.srrc);
        assert_eq!(a.pcc, b.pcc);
        assert_eq!(a.prcc, b.prcc);
        assert_eq!(a.r2_linear, b.r2_linear);
        assert_eq!(a.r2_rank, b.r2_rank);
    }

    // ── Pearson + ranking unit ────────────────────────────────────

    #[test]
    fn pearson_correlation_perfect_positive() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        assert!((pearson_correlation(&a, &b) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn pearson_correlation_perfect_negative() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        assert!((pearson_correlation(&a, &b) + 1.0).abs() < 1e-12);
    }

    #[test]
    fn pearson_correlation_uncorrelated() {
        // Constant b → zero variance → returns 0.
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![5.0, 5.0, 5.0];
        assert_eq!(pearson_correlation(&a, &b), 0.0);
    }

    #[test]
    fn ordinal_ranks_one_indexed() {
        let data = [3.0, 1.0, 2.0];
        assert_eq!(ordinal_ranks_f64(&data), vec![3.0, 1.0, 2.0]);
    }

    // ── d=1 special case (no "other factors" for partial corr) ───

    #[test]
    fn d_one_pcc_equals_pearson_with_y() {
        // For d=1, PCC reduces to Pearson(X_0, Y).
        let n = 100;
        let x = synthetic_x(n, 1);
        let y: Vec<f64> = (0..n).map(|k| 2.0 * x[[k, 0]]).collect();
        let est = estimate_regression_indices(&x, &y).unwrap();
        // Y is perfectly linear in X_0 → PCC ≈ 1.
        assert!(
            (est.pcc[0].abs() - 1.0).abs() < 1e-6,
            "|PCC_0| = {}",
            est.pcc[0].abs()
        );
    }
}
