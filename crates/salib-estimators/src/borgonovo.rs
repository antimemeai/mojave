//! Borgonovo δ — moment-independent sensitivity index via the
//! Plischke-Borgonovo-Smith 2013 given-data algorithm with KDE-based
//! density estimation.
//!
//! Per `decisions/2026-04-29-saltelli-borgonovo-delta.md`.
//!
//! # Definition (Borgonovo 2007)
//!
//! ```text
//! δᵢ = (1/2) · E_{Xᵢ} [ ∫ |f_Y(y) − f_{Y|Xᵢ}(y)| dy ]
//! ```
//!
//! `δᵢ ∈ [0, 1]` measures the average absolute area between the
//! unconditional density of `Y` and the conditional density of `Y`
//! given `Xᵢ`. Unlike `Sᵢ` and `Sᵀᵢ`, `δ` is moment-independent —
//! sensitive to *any* change in the output distribution shape, not
//! just variance. Two factors with the same `Sᵢ` can have very
//! different `δ` if one shifts the distribution location while the
//! other only changes the scale.
//!
//! # Algorithm — Plischke-Borgonovo-Smith 2013 Eq 26
//!
//! Given `(X, Y)`:
//!
//! 1. Build an unconditional `f_Y` via Gaussian KDE with Silverman's
//!    bandwidth rule.
//! 2. Partition `X[:, i]` into `M` equal-frequency classes by rank.
//!    `M = round(min(⌈N^exp⌉, 48))`, where
//!    `exp = 2 / (7 + tanh((1500 − N) / 500))`. Matches `SALib`.
//! 3. For each class `j`:
//!    - Build conditional `f_{Y|class_j}` via Gaussian KDE on `Y[ix_j]`.
//!    - Trapezoidal integration: `area_j = ∫ |f_Y(y) − f_{Y|class_j}(y)| dy`.
//!    - Class contribution: `(|ix_j| / (2·N)) · area_j`.
//! 4. `δᵢ = Σⱼ contribution_j`.
//!
//! Y-grid for integration: 100 points uniformly spaced from
//! `min(Y)` to `max(Y)`. Matches `SALib`'s default.
//!
//! # Differences from `SALib`
//!
//! `SALib`'s `analyze.delta` wraps `calc_delta` in a `bias_reduced_delta`
//! Plischke 2013 Eq 30 jackknife-style correction (one bootstrap
//! re-estimate plus 100 bootstrap CI samples). We ship the
//! uncorrected `calc_delta` per Eq 26; the bias correction is
//! bead-eligible (requires bootstrap RNG plumbing). At `N = 4096`
//! on Ishigami, raw `calc_delta` and `bias_reduced_delta` differ by
//! `~0.04` per factor — both within the analytic-recovery
//! tolerance.
//!
//! # First-order only
//!
//! `SALib`'s `delta.analyze` also returns the Sobol' `S₁` from a
//! correlation-based estimator on the same `(X, Y)`. We don't —
//! `S₁` lives in `saltelli2010` (designed) and `rbd_fast` (given-
//! data); the Borgonovo module focuses on `δ`.
//!
//! # Determinism
//!
//! Pure under `(X, Y)`. KDE evaluation is fully deterministic
//! (closed-form Gaussian sum). Partitioning uses ordinal ranking;
//! ties broken by input order. Same `(X, Y)` in → bit-identical
//! `BorgonovoIndices` out.

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::items_after_statements
)]

use std::cmp::Ordering;
use std::f64::consts::PI;

use ndarray::Array2;
use salib_core::tree_sum;

/// Borgonovo `δ` estimates per factor.
///
/// `#[non_exhaustive]` — future fields (`bootstrap_ci`,
/// `bias_reduced` flag, `total_variance` echo for downstream GUM
/// contribution) land non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct BorgonovoIndices {
    /// Borgonovo δ per factor, length `d`. `δᵢ ∈ [0, 1]`.
    pub delta: Vec<f64>,
}

impl BorgonovoIndices {
    /// Factor count.
    #[must_use]
    pub fn d(&self) -> usize {
        self.delta.len()
    }
}

/// Errors from [`estimate_borgonovo_delta`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BorgonovoError {
    #[error("Borgonovo δ: shape mismatch — X has {x_rows} rows, y has {y_len} elements")]
    ShapeMismatch { x_rows: usize, y_len: usize },
    #[error("Borgonovo δ: d must be ≥ 1, got 0")]
    ZeroD,
    /// `N < 16` — too few samples for meaningful KDE + partitioning.
    /// `SALib`'s default `M ≥ 2` requires at least a few samples per
    /// class; we floor at `N = 16` to keep the estimator stable.
    #[error("Borgonovo δ: N must be ≥ 16, got {n}")]
    InsufficientSamples { n: usize },
    /// Y has zero range (constant model).
    #[error("Borgonovo δ: Y has zero range (model output is constant)")]
    ZeroVariance,
}

/// Estimate Borgonovo `δ` per factor from generic `(X, Y)` data.
///
/// `x` is the `(N, d)` input matrix; `y` is the `N`-element model
/// output vector. The estimator is sampler-agnostic — LHS, Sobol',
/// Saltelli matrix, user data all work.
///
/// # Errors
///
/// - [`BorgonovoError::ShapeMismatch`] if `x.nrows() != y.len()`.
/// - [`BorgonovoError::ZeroD`] if `x.ncols() == 0`.
/// - [`BorgonovoError::InsufficientSamples`] if `N < 16`.
/// - [`BorgonovoError::ZeroVariance`] if `Y` has zero range.
pub fn estimate_borgonovo_delta(
    x: &Array2<f64>,
    y: &[f64],
) -> Result<BorgonovoIndices, BorgonovoError> {
    let n = x.nrows();
    let d = x.ncols();
    if d == 0 {
        return Err(BorgonovoError::ZeroD);
    }
    if y.len() != n {
        return Err(BorgonovoError::ShapeMismatch {
            x_rows: n,
            y_len: y.len(),
        });
    }
    if n < 16 {
        return Err(BorgonovoError::InsufficientSamples { n });
    }

    let (y_min, y_max) = min_max(y);
    if !(y_max - y_min).is_finite() || (y_max - y_min) < 1e-15 {
        return Err(BorgonovoError::ZeroVariance);
    }

    // Y grid: 100 points from y_min to y_max inclusive (matches
    // `SALib`'s `np.linspace(min, max, 100)`).
    const Y_GRID_POINTS: usize = 100;
    let y_grid: Vec<f64> = (0..Y_GRID_POINTS)
        .map(|k| y_min + (y_max - y_min) * (k as f64) / ((Y_GRID_POINTS - 1) as f64))
        .collect();

    // Adaptive class count per `SALib` / Plischke 2013.
    let n_classes = class_count(n);

    // Pre-compute the unconditional KDE — it's reused across factors.
    let h_y = silverman_bandwidth(y);
    let fy = gaussian_kde(y, h_y, &y_grid);

    let mut delta = vec![0.0_f64; d];
    let mut x_col_buf = vec![0.0_f64; n];
    for i in 0..d {
        for k in 0..n {
            x_col_buf[k] = x[[k, i]];
        }
        delta[i] = calc_delta(y, &y_grid, &fy, &x_col_buf, n_classes);
    }

    Ok(BorgonovoIndices { delta })
}

/// Plischke 2013 Eq 26 estimator for a single factor's `δ`.
fn calc_delta(y: &[f64], y_grid: &[f64], fy: &[f64], x_col: &[f64], n_classes: usize) -> f64 {
    let n = y.len();
    let ranks = ordinal_ranks(x_col);

    let n_f = n as f64;
    let mut d_hat = 0.0_f64;
    let mut class_y_buf: Vec<f64> = Vec::with_capacity(n);
    let mut diff_buf = vec![0.0_f64; y_grid.len()];

    for j in 0..n_classes {
        // Class j has rank-bounds (lo, hi]. Equal-frequency partitioning.
        let lo = (n_f * (j as f64) / (n_classes as f64)) as usize;
        let hi = (n_f * ((j + 1) as f64) / (n_classes as f64)) as usize;
        // Collect Y values whose rank falls in (lo, hi].
        class_y_buf.clear();
        for (k, &r) in ranks.iter().enumerate() {
            if r > lo && r <= hi {
                class_y_buf.push(y[k]);
            }
        }
        let nm = class_y_buf.len();
        if nm == 0 {
            continue;
        }

        // Peak-to-peak: if the class's Y is constant, the conditional
        // density collapses to a δ-distribution; treat the divergence
        // as |fy| (matches `SALib`'s degenerate-class fallback).
        let (cy_min, cy_max) = min_max(&class_y_buf);
        if (cy_max - cy_min) > 0.0 {
            let h_yc = silverman_bandwidth(&class_y_buf);
            // Re-use diff_buf as the conditional KDE then convert
            // in-place to |fy − fyc|.
            gaussian_kde_into(&class_y_buf, h_yc, y_grid, &mut diff_buf);
            for (k, slot) in diff_buf.iter_mut().enumerate() {
                *slot = (fy[k] - *slot).abs();
            }
        } else {
            for (k, slot) in diff_buf.iter_mut().enumerate() {
                *slot = fy[k].abs();
            }
        }

        let integral = trapz(&diff_buf, y_grid);
        d_hat += (nm as f64 / (2.0 * n_f)) * integral;
    }
    d_hat
}

/// `SALib` / Plischke 2013 adaptive partition count.
///
/// `M = round(min(⌈N^exp⌉, 48))` where
/// `exp = 2 / (7 + tanh((1500 − N) / 500))`. For `N = 1024`, `M = 6`;
/// for `N = 4096`, `M = 16`; capped at `48` for very large `N`.
///
/// `pub(crate)` so sibling partition-based estimators (e.g. `qosa`)
/// can share the same heuristic without re-deriving it.
pub(crate) fn class_count(n: usize) -> usize {
    let n_f = n as f64;
    let tanh_arg = (1500.0 - n_f) / 500.0;
    let exp = 2.0 / (7.0 + tanh_arg.tanh());
    let raw = n_f.powf(exp).ceil() as usize;
    raw.clamp(2, 48)
}

/// Silverman's rule-of-thumb bandwidth for univariate Gaussian KDE,
/// matching `scipy.stats.gaussian_kde(bw_method="silverman")`:
///
/// `h = (n · 3/4)^(−1/5) · σ`
///
/// where `σ` is the sample standard deviation (with `1/n` divisor,
/// not Bessel `1/(n−1)` — `scipy.gaussian_kde` uses the population
/// std for the bandwidth derivation).
fn silverman_bandwidth(data: &[f64]) -> f64 {
    let n = data.len() as f64;
    let mean = tree_sum(data) / n;
    let var: f64 = tree_sum(
        &data
            .iter()
            .map(|&v| (v - mean).powi(2))
            .collect::<Vec<f64>>(),
    ) / n;
    let sigma = var.sqrt();
    let factor = (n * 3.0 / 4.0).powf(-0.2);
    let h = factor * sigma;
    // Numerical safety: KDE bandwidth must be strictly positive. If
    // the data is exactly constant (caught upstream by ZeroVariance
    // or per-class fallback), this code path shouldn't fire — but
    // guard against denormal-tiny σ from numerical noise.
    h.max(1e-15)
}

/// Evaluate Gaussian KDE at each grid point, writing into `out`.
///
/// `f̂(y) = (1 / (n · h · √(2π))) · Σᵢ exp(− ½ ((y − xᵢ) / h)²)`
fn gaussian_kde_into(data: &[f64], h: f64, grid: &[f64], out: &mut [f64]) {
    debug_assert_eq!(grid.len(), out.len());
    let n = data.len() as f64;
    let norm = 1.0 / (n * h * (2.0 * PI).sqrt());
    for (k, &y) in grid.iter().enumerate() {
        let mut acc = 0.0;
        for &xi in data {
            let z = (y - xi) / h;
            acc += (-0.5 * z * z).exp();
        }
        out[k] = norm * acc;
    }
}

/// Convenience wrapper: allocate and return.
fn gaussian_kde(data: &[f64], h: f64, grid: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; grid.len()];
    gaussian_kde_into(data, h, grid, &mut out);
    out
}

/// Trapezoidal integration of `y` against `x` (both length `n`).
fn trapz(y: &[f64], x: &[f64]) -> f64 {
    debug_assert_eq!(y.len(), x.len());
    if y.len() < 2 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 1..y.len() {
        sum += 0.5 * (y[i] + y[i - 1]) * (x[i] - x[i - 1]);
    }
    sum
}

/// `scipy.stats.rankdata(method='ordinal')` — ranks are
/// `1..=N` with stable tie-breaking by input order.
///
/// `pub(crate)` so sibling partition-based estimators can reuse it.
pub(crate) fn ordinal_ranks(data: &[f64]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..data.len()).collect();
    idx.sort_by(|&a, &b| data[a].partial_cmp(&data[b]).unwrap_or(Ordering::Equal));
    let mut ranks = vec![0_usize; data.len()];
    for (rank, &i) in idx.iter().enumerate() {
        ranks[i] = rank + 1;
    }
    ranks
}

fn min_max(data: &[f64]) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for &v in data {
        if v < lo {
            lo = v;
        }
        if v > hi {
            hi = v;
        }
    }
    (lo, hi)
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use super::*;

    fn synthetic_x(n: usize, d: usize) -> Array2<f64> {
        // Per-column independent permutations of (k+0.5)/n.
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
            estimate_borgonovo_delta(&x, &y).unwrap_err(),
            BorgonovoError::ZeroD
        );
    }

    #[test]
    fn shape_mismatch_errors() {
        let x = Array2::<f64>::zeros((100, 3));
        let y = vec![0.0; 50];
        let err = estimate_borgonovo_delta(&x, &y).unwrap_err();
        assert!(matches!(err, BorgonovoError::ShapeMismatch { .. }));
    }

    #[test]
    fn insufficient_samples_errors() {
        let x = synthetic_x(10, 3);
        let y = vec![0.0; 10];
        let err = estimate_borgonovo_delta(&x, &y).unwrap_err();
        assert!(matches!(err, BorgonovoError::InsufficientSamples { .. }));
    }

    #[test]
    fn constant_model_errors() {
        let x = synthetic_x(64, 3);
        let y = vec![1.0; 64];
        let err = estimate_borgonovo_delta(&x, &y).unwrap_err();
        assert_eq!(err, BorgonovoError::ZeroVariance);
    }

    // ── Class-count formula matches `SALib` ───────────────────────

    #[test]
    fn class_count_matches_salib_table() {
        // SALib `np.round(min(int(np.ceil(N**exp)), 48))` for the
        // same N → M values:
        //   N=1024  M=6
        //   N=4096  M=16
        assert_eq!(class_count(1024), 6);
        assert_eq!(class_count(4096), 16);
    }

    #[test]
    fn class_count_capped_at_48() {
        assert_eq!(class_count(1_000_000), 48);
    }

    #[test]
    fn class_count_floors_at_2() {
        assert!(class_count(16) >= 2);
    }

    // ── Output shape ──────────────────────────────────────────────

    #[test]
    fn output_length_matches_d() {
        let n = 256;
        let x = synthetic_x(n, 5);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]]).collect();
        let est = estimate_borgonovo_delta(&x, &y).unwrap();
        assert_eq!(est.d(), 5);
    }

    // ── δ ∈ [0, 1] ────────────────────────────────────────────────

    #[test]
    fn delta_within_unit_interval_with_slack() {
        // δ is bounded in [0, 1] by definition. Empirical KDE-based
        // estimates can slip slightly outside due to numerical
        // integration error; allow ε slack.
        let n = 512;
        let x = synthetic_x(n, 3);
        let y: Vec<f64> = (0..n)
            .map(|k| x[[k, 0]] + 0.5 * x[[k, 1]] * x[[k, 2]])
            .collect();
        let est = estimate_borgonovo_delta(&x, &y).unwrap();
        for &v in &est.delta {
            assert!((-0.05..=1.05).contains(&v), "δ = {v} outside [-0.05, 1.05]");
        }
    }

    // ── Linear single-factor: factor 0 has highest δ ─────────────

    #[test]
    fn linear_single_factor_dominates_delta() {
        // Y = X[:, 0] — factor 0 should have the largest δ.
        let n = 512;
        let x = synthetic_x(n, 3);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]]).collect();
        let est = estimate_borgonovo_delta(&x, &y).unwrap();
        assert!(
            est.delta[0] > est.delta[1],
            "δ_0 = {} should exceed δ_1 = {}",
            est.delta[0],
            est.delta[1]
        );
        assert!(
            est.delta[0] > est.delta[2],
            "δ_0 = {} should exceed δ_2 = {}",
            est.delta[0],
            est.delta[2]
        );
    }

    // ── Determinism ───────────────────────────────────────────────

    #[test]
    fn same_input_yields_identical_output() {
        let n = 64;
        let x = synthetic_x(n, 3);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]] + x[[k, 1]] * x[[k, 2]]).collect();
        let a = estimate_borgonovo_delta(&x, &y).unwrap();
        let b = estimate_borgonovo_delta(&x, &y).unwrap();
        assert_eq!(a.delta, b.delta);
    }

    // ── Helpers: KDE / trapz / ranks ──────────────────────────────

    #[test]
    fn silverman_bandwidth_matches_formula() {
        // For data with σ = 1, n = 100: h = (75)^(−1/5) · 1 ≈ 0.4156.
        let data: Vec<f64> = (0..100_i32)
            .map(|i| (f64::from(i) - 49.5) / 28.866_07)
            .collect();
        // ^ pre-scaled so σ ≈ 1.
        let h = silverman_bandwidth(&data);
        let expected = 75.0_f64.powf(-0.2);
        assert!(
            (h - expected).abs() < 0.01,
            "h = {h}, expected ≈ {expected}"
        );
    }

    #[test]
    fn gaussian_kde_normalizes_to_unity() {
        // ∫ f̂(y) dy ≈ 1 for any KDE with sufficient grid coverage.
        let data: Vec<f64> = (-50..50).map(|i| f64::from(i) * 0.1).collect();
        let h = silverman_bandwidth(&data);
        let grid: Vec<f64> = (-200..200).map(|i| f64::from(i) * 0.05).collect();
        let pdf = gaussian_kde(&data, h, &grid);
        let area = trapz(&pdf, &grid);
        assert!(
            (area - 1.0).abs() < 0.01,
            "KDE integral = {area}, expected ≈ 1"
        );
    }

    #[test]
    fn trapz_simple_cases() {
        // ∫₀² 2x dx = 4.
        let x = [0.0, 1.0, 2.0];
        let y = [0.0, 2.0, 4.0];
        assert!((trapz(&y, &x) - 4.0).abs() < 1e-12);
    }

    #[test]
    fn ordinal_ranks_assigns_one_through_n() {
        let data = [3.0, 1.0, 2.0];
        let ranks = ordinal_ranks(&data);
        assert_eq!(ranks, vec![3, 1, 2]);
    }

    #[test]
    fn ordinal_ranks_breaks_ties_by_input_order() {
        let data = [1.0, 1.0, 1.0];
        let ranks = ordinal_ranks(&data);
        assert_eq!(ranks, vec![1, 2, 3]);
    }
}
