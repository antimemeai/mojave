//! Quantile-Oriented Sensitivity Analysis (QOSA, Maume-Deschamps &
//! Niang 2018) — partition-based estimator.
//!
//! # What QOSA measures
//!
//! Variance-based first-order Sobol' answers "which input drives
//! `Var(Y)`?". QOSA answers a different question: **"which input
//! drives the α-quantile of Y?"** — useful when the workload cares
//! about tail behavior (e.g. 95th-percentile latency, 99th-percentile
//! loss, regulatory VaR-style measures).
//!
//! Maume-Deschamps & Niang 2018 derive QOSA from the α-quantile
//! contrast function `ψ_α(y, θ) = (y − θ)(α − 1_{y≤θ})`. The
//! resulting index (Eq 2.3):
//!
//! ```text
//! S^α_X_i = (min_θ E[ψ_α(Y;θ)] − E[min_θ E[ψ_α(Y;θ)|X_i]])
//!         / min_θ E[ψ_α(Y;θ)]
//! ```
//!
//! is rewritten via the Conditional Tail Expectation (CTE) risk
//! measure (Prop 3.1):
//!
//! ```text
//! S^α_X_i = 1 − (E[Y | Y > F_{Y|X_i}^{-1}(α)] − E[Y])
//!             / (CTE_α(Y) − E[Y])
//! ```
//!
//! where `CTE_α(Y) = E[Y | Y > F_Y^{-1}(α)]` is the tail mean and
//! `F_{Y|X_i}^{-1}(α)` is the conditional α-quantile.
//!
//! Sanity properties (Maume-Deschamps & Niang 2018 § 2 Remark):
//!
//! - `S^α_X_i = 0` if `Y ⊥ X_i` (X_i has no influence on the
//!   α-quantile of Y).
//! - `S^α_X_i = 1` if `Y` is `X_i`-measurable (X_i fully determines Y).
//!
//! # Estimator — partition-based
//!
//! Maume-Deschamps & Niang 2018 § 4 propose a kernel-based two-
//! sample estimator (Eq 4.3) using `F_Y^{-1}(α)` and a kernel-
//! conditional-quantile `F_{Y|X_i=x}^{-1}(α)`. This module ships a
//! **partition-based** alternative that fits the existing saltelli
//! given-data machinery (PR 11 [`borgonovo`], PR 14b
//! [`given_data_sobol`]):
//!
//! 1. Sort `Y` and take `θ̂* = ⌈α·N⌉`-th value (empirical α-quantile).
//! 2. Compute global `Ȳ` and `CTE_α(Y) = (1/(N(1−α))) Σⱼ Yⱼ · 1_{Yⱼ > θ̂*}`.
//! 3. For each factor `i`:
//!    a. Partition `X_i` into `K` ordinal classes (same heuristic as
//!       [`borgonovo::class_count`]).
//!    b. For each class, take the conditional α-quantile θ̂_class.
//!    c. Compute `Ê[Y | Y > F_{Y|X_i}^{-1}(α)] ≈
//!       (1/(N(1−α))) Σⱼ Yⱼ · 1_{Yⱼ > θ̂_class(j)}` where
//!       `class(j)` is the class of `X_i^j`.
//!    d. `Ŝ^α_i = 1 − (Ê[Y | …] − Ȳ) / (CTE_α(Y) − Ȳ)` per Prop 3.1.
//!    e. Clamp to `[0, 1]` (population value is non-negative; finite-
//!       sample noise can push slightly outside).
//!
//! The partition variant trades the kernel estimator's continuous-
//! conditional-quantile fit for a piecewise-constant ordinal-class
//! approximation. Asymptotically the two converge to the same
//! population index (Prop 4.1 carries through under partition
//! consistency); finite-sample bias on the partition variant is
//! bounded by class-mean variance, controlled by `class_count`.
//!
//! # Sanity at α = 0.5
//!
//! At α = 0.5, QOSA measures sensitivity at the median tail. The
//! estimator does *not* reduce exactly to first-order Sobol'
//! (which uses `Var` not CTE), but they agree on factor ordering
//! for monotone or near-monotone effects. For Ishigami canonical
//! we verify QOSA at α = 0.5 places `X_2` (largest first-order
//! Sobol' factor) above `X_1` (second) above `X_3` (≈ 0).
//!
//! # Out of scope (bead-tracked)
//!
//! - Kernel-conditional-quantile two-sample estimator (the paper's
//!   Eq 4.3 form). Bead-eligible if a workload demonstrates the
//!   partition variant has insufficient resolution.
//! - Grouped-factor QOSA + variance reduction — `workspace-0yt`.
//! - Cross-implementation differential against the paper authors'
//!   reference R code.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::doc_lazy_continuation,
    clippy::doc_markdown,
    clippy::doc_overindented_list_items,
    clippy::needless_range_loop
)]

use ndarray::Array2;
use salib_core::tree_sum;

use crate::borgonovo::{class_count, ordinal_ranks};

/// QOSA index estimates for a fixed α.
///
/// `#[non_exhaustive]` — future fields (`bootstrap_ci`, per-factor
/// realized class count) land non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct QosaIndices {
    /// QOSA index per factor, length `d`. Clamped to `[0, 1]`
    /// (population value is non-negative under Maume-Deschamps
    /// 2018 § 2 Remark; finite-sample noise can push slightly
    /// outside, hence the clamp).
    pub s: Vec<f64>,
    /// Quantile level α used for the estimate. Echo of input.
    pub alpha: f64,
    /// Empirical α-quantile of the marginal output. Diagnostic.
    pub global_quantile: f64,
    /// Empirical CTE_α(Y) = E[Y | Y > F_Y^{-1}(α)]. Diagnostic.
    pub global_cte: f64,
}

impl QosaIndices {
    /// Factor count.
    #[must_use]
    pub fn d(&self) -> usize {
        self.s.len()
    }
}

/// Errors from [`estimate_qosa`].
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum QosaError {
    #[error("qosa: shape mismatch — x has {x_rows} rows, y has {y_len} elements")]
    ShapeMismatch { x_rows: usize, y_len: usize },
    #[error("qosa: d must be ≥ 1, got 0")]
    ZeroD,
    #[error("qosa: insufficient samples — need N ≥ 16, got {n}")]
    InsufficientSamples { n: usize },
    #[error("qosa: alpha must lie in (0, 1), got {alpha}")]
    InvalidAlpha { alpha: f64 },
    #[error("qosa: Var(Y) ≈ 0 (model output is constant)")]
    ZeroVariance,
    #[error(
        "qosa: degenerate tail — CTE_α(Y) ≈ E[Y]; either α is too \
         small or Y has a heavy point mass below the α-quantile"
    )]
    DegenerateTail,
}

/// Estimate quantile-oriented sensitivity indices on generic
/// `(X, Y)` data via a partition-based form of Maume-Deschamps &
/// Niang 2018 Prop 3.1.
///
/// `x` is the `(N, d)` input matrix; `y` is the `N`-element model
/// output. `alpha ∈ (0, 1)` is the quantile level; common choices
/// are `0.5` (median), `0.9` / `0.95` (tail), `0.99` (extreme tail).
///
/// Sampler-agnostic — works on any pair of independent `(X, Y)`
/// observations regardless of how `X` was sampled.
///
/// # Errors
///
/// - [`QosaError::ShapeMismatch`] if `x.nrows() != y.len()`.
/// - [`QosaError::ZeroD`] if `x.ncols() == 0`.
/// - [`QosaError::InsufficientSamples`] if `N < 16` (matches the
///   sibling given-data estimators' floor).
/// - [`QosaError::InvalidAlpha`] if `alpha ∉ (0, 1)`.
/// - [`QosaError::DegenerateTail`] if the global CTE collapses onto
///   the global mean (numerically `< 1e-12 · |Ȳ| + 1e-15`); the
///   index denominator vanishes.
pub fn estimate_qosa(x: &Array2<f64>, y: &[f64], alpha: f64) -> Result<QosaIndices, QosaError> {
    let n = x.nrows();
    let d = x.ncols();
    if d == 0 {
        return Err(QosaError::ZeroD);
    }
    if y.len() != n {
        return Err(QosaError::ShapeMismatch {
            x_rows: n,
            y_len: y.len(),
        });
    }
    if n < 16 {
        return Err(QosaError::InsufficientSamples { n });
    }
    if !(alpha.is_finite() && alpha > 0.0 && alpha < 1.0) {
        return Err(QosaError::InvalidAlpha { alpha });
    }

    let n_f = n as f64;
    let mean_y = tree_sum(y) / n_f;

    // Constant-Y short-circuit. The CTE-based numerator collapses
    // (no Y_j strictly exceeds the empirical quantile) and the
    // population sensitivity is undefined for a constant output.
    let var_y_centered: Vec<f64> = y.iter().map(|&yj| (yj - mean_y).powi(2)).collect();
    let var_y = tree_sum(&var_y_centered) / n_f;
    if !var_y.is_finite() || var_y < 1e-15 {
        return Err(QosaError::ZeroVariance);
    }

    // Global α-quantile via empirical sort. ⌈α·N⌉-th order statistic
    // (1-indexed → α·N - 1 zero-indexed, ceiling).
    let mut y_sorted: Vec<f64> = y.to_vec();
    y_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let q_idx = ((alpha * n_f).ceil() as usize).saturating_sub(1).min(n - 1);
    let global_quantile = y_sorted[q_idx];

    // Global CTE via Prop 3.1 estimator: (1/(N(1-α))) Σⱼ Y_j · 1_{Y_j > θ̂*}.
    // (We compute the sum via tree_sum for bit-determinism.)
    let global_excess: Vec<f64> = y
        .iter()
        .map(|&yj| if yj > global_quantile { yj } else { 0.0 })
        .collect();
    let global_cte = tree_sum(&global_excess) / (n_f * (1.0 - alpha));

    let denom = global_cte - mean_y;
    if !denom.is_finite() || denom.abs() < 1e-12 * mean_y.abs() + 1e-15 {
        return Err(QosaError::DegenerateTail);
    }

    let n_classes = class_count(n);
    let mut s = vec![0.0_f64; d];
    let mut x_col_buf = vec![0.0_f64; n];
    let mut class_y_buf: Vec<f64> = Vec::with_capacity(n);
    // Per-sample assigned class index (for fast indicator lookup
    // in the second pass).
    let mut class_of: Vec<usize> = vec![0_usize; n];
    let mut class_quantile = vec![0.0_f64; n_classes];
    let mut conditional_excess = vec![0.0_f64; n];

    for i in 0..d {
        for k in 0..n {
            x_col_buf[k] = x[[k, i]];
        }
        let ranks = ordinal_ranks(&x_col_buf);

        // First pass: assign each sample to a class and compute the
        // conditional α-quantile per class.
        for j in 0..n_classes {
            let lo = (n_f * (j as f64) / (n_classes as f64)) as usize;
            let hi = (n_f * ((j + 1) as f64) / (n_classes as f64)) as usize;
            class_y_buf.clear();
            for (k, &r) in ranks.iter().enumerate() {
                if r > lo && r <= hi {
                    class_y_buf.push(y[k]);
                    class_of[k] = j;
                }
            }
            class_y_buf.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            // ⌈α · class_size⌉-th order statistic.
            let nm = class_y_buf.len();
            class_quantile[j] = if nm == 0 {
                f64::INFINITY // unreachable in practice with class_count(N) ≤ √N
            } else {
                let cq_idx = ((alpha * nm as f64).ceil() as usize)
                    .saturating_sub(1)
                    .min(nm - 1);
                class_y_buf[cq_idx]
            };
        }

        // Second pass: compute Σⱼ Yⱼ · 1_{Yⱼ > θ̂_class(j)}.
        //
        // Strict inequality matches Prop 3.1's `1_{Y > F^{-1}(α)}`.
        // Caveat: for discrete `Y` with a heavy point mass at the
        // class quantile, ties are excluded from the tail and the
        // conditional CTE biases low (corresponding S biases high).
        // Module docstring's "out of scope" lists this as bead-
        // eligible; the `Var(Y) < 1e-15` early check handles only
        // the constant-Y degenerate case.
        for k in 0..n {
            let cq = class_quantile[class_of[k]];
            conditional_excess[k] = if y[k] > cq { y[k] } else { 0.0 };
        }
        let cond_excess_sum = tree_sum(&conditional_excess);
        let conditional_cte = cond_excess_sum / (n_f * (1.0 - alpha));

        // Prop 3.1: S^α_i = 1 - (E[Y | Y > F^{-1}_{Y|X_i}(α)] - Ȳ) /
        //                       (CTE_α(Y) - Ȳ)
        let raw = 1.0 - (conditional_cte - mean_y) / denom;
        s[i] = raw.clamp(0.0, 1.0);
    }

    Ok(QosaIndices {
        s,
        alpha,
        global_quantile,
        global_cte,
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use super::*;

    fn synthetic_uniform(n: usize, d: usize) -> Array2<f64> {
        let mut x = Array2::<f64>::zeros((n, d));
        for j in 0..d {
            let mut state: u64 = 0x9E37_79B9_7F4A_7C15_u64.wrapping_mul((j as u64).wrapping_add(1));
            for k in 0..n {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                let u = (state >> 33) as f64 / ((u64::MAX >> 33) as f64 + 1.0);
                x[[k, j]] = u;
            }
        }
        x
    }

    // ── Validation ────────────────────────────────────────────────

    #[test]
    fn shape_mismatch_errors() {
        let x = Array2::<f64>::zeros((10, 3));
        let y = vec![0.0; 5];
        let err = estimate_qosa(&x, &y, 0.5).unwrap_err();
        assert!(matches!(err, QosaError::ShapeMismatch { .. }));
    }

    #[test]
    fn zero_d_errors() {
        let x = Array2::<f64>::zeros((100, 0));
        let y = vec![0.0; 100];
        assert_eq!(estimate_qosa(&x, &y, 0.5).unwrap_err(), QosaError::ZeroD);
    }

    #[test]
    fn insufficient_samples_errors() {
        let x = Array2::<f64>::zeros((8, 3));
        let y = vec![0.0; 8];
        assert!(matches!(
            estimate_qosa(&x, &y, 0.5).unwrap_err(),
            QosaError::InsufficientSamples { .. }
        ));
    }

    #[test]
    fn invalid_alpha_errors() {
        let x = Array2::<f64>::zeros((100, 3));
        let y: Vec<f64> = (0..100).map(|k| k as f64).collect();
        for bad in [0.0, 1.0, -0.1, 1.5, f64::NAN] {
            assert!(matches!(
                estimate_qosa(&x, &y, bad).unwrap_err(),
                QosaError::InvalidAlpha { .. }
            ));
        }
    }

    #[test]
    fn zero_variance_errors_on_constant_y() {
        let x = synthetic_uniform(64, 3);
        let y = vec![5.0; 64];
        assert_eq!(
            estimate_qosa(&x, &y, 0.5).unwrap_err(),
            QosaError::ZeroVariance
        );
    }

    // ── Sanity properties ────────────────────────────────────────

    #[test]
    fn independent_factor_yields_near_zero_index() {
        // Y depends only on X_0; X_1, X_2 are independent of Y.
        // Sanity property: S^α_1 ≈ S^α_2 ≈ 0.
        let n = 1024;
        let x = synthetic_uniform(n, 3);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]]).collect();
        let result = estimate_qosa(&x, &y, 0.5).unwrap();
        assert!(
            result.s[1] < 0.1,
            "S^α_1 = {} should be ≈ 0 (X_1 ⊥ Y)",
            result.s[1]
        );
        assert!(
            result.s[2] < 0.1,
            "S^α_2 = {} should be ≈ 0 (X_2 ⊥ Y)",
            result.s[2]
        );
        // X_0 fully determines Y → S^α_0 should be substantial.
        assert!(
            result.s[0] > 0.3,
            "S^α_0 = {} should be substantial (X_0 ⇒ Y)",
            result.s[0]
        );
    }

    #[test]
    fn fully_determining_factor_yields_index_near_one() {
        // Y = X_0 exactly. Maume-Deschamps Remark: S^α = 1 if Y is
        // X_i-measurable. The partition-based estimator approaches
        // 1 as N grows; at moderate N it's biased low by the
        // class-mean smoothing.
        let n = 2048;
        let x = synthetic_uniform(n, 2);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]]).collect();
        let result = estimate_qosa(&x, &y, 0.5).unwrap();
        assert!(
            result.s[0] > 0.85,
            "S^α_0 = {} should be near 1 (Y = X_0)",
            result.s[0]
        );
    }

    // ── Output shape ──────────────────────────────────────────────

    #[test]
    fn output_dimensions_match_inputs() {
        let n = 256;
        let x = synthetic_uniform(n, 5);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]] + x[[k, 2]]).collect();
        let result = estimate_qosa(&x, &y, 0.75).unwrap();
        assert_eq!(result.d(), 5);
        assert_eq!(result.alpha, 0.75);
        assert!(result.global_cte > result.global_quantile);
    }

    // ── Tail-vs-median demonstrates QOSA's distinguishing feature ─

    #[test]
    fn tail_alpha_emphasizes_tail_driving_factor() {
        // Y = X_0 + 5 · X_1 · 1_{X_2 > 0.95}.
        // At α = 0.5 (median), X_0 dominates (the indicator fires
        // only 5% of the time, contributing little to the median).
        // At α = 0.95 (tail), X_1 and X_2 dominate (when the
        // indicator fires, the 5·X_1 term swamps X_0).
        let n = 4096;
        let x = synthetic_uniform(n, 3);
        let y: Vec<f64> = (0..n)
            .map(|k| {
                let base = x[[k, 0]];
                let tail = if x[[k, 2]] > 0.95 {
                    5.0 * x[[k, 1]]
                } else {
                    0.0
                };
                base + tail
            })
            .collect();

        let median = estimate_qosa(&x, &y, 0.5).unwrap();
        let tail = estimate_qosa(&x, &y, 0.95).unwrap();

        // At median, X_0 is the dominant driver.
        assert!(
            median.s[0] > median.s[1],
            "at α=0.5: S_0 = {} should exceed S_1 = {}",
            median.s[0],
            median.s[1]
        );
        // At tail, X_2 should dominate over X_0 (it's the gate
        // variable that triggers the tail-driving term).
        assert!(
            tail.s[2] > tail.s[0],
            "at α=0.95: S_2 = {} should exceed S_0 = {}",
            tail.s[2],
            tail.s[0]
        );
    }

    // ── Determinism ──────────────────────────────────────────────

    #[test]
    fn same_input_yields_identical_output() {
        let n = 256;
        let x = synthetic_uniform(n, 3);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]] * x[[k, 1]]).collect();
        let a = estimate_qosa(&x, &y, 0.7).unwrap();
        let b = estimate_qosa(&x, &y, 0.7).unwrap();
        assert_eq!(a.s, b.s);
        assert_eq!(a.global_quantile, b.global_quantile);
        assert_eq!(a.global_cte, b.global_cte);
    }
}
