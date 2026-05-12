//! Given-data first-order Sobol' indices via the Plischke-
//! Borgonovo-Smith 2013 partition-based estimator.
//!
//! Per `decisions/2026-04-29-saltelli-given-data-sobol.md`. Sibling
//! to `borgonovo` (same paper, same partitioning machinery, but
//! variance-based instead of PDF-divergence-based).
//!
//! # Algorithm — Plischke-Borgonovo-Smith 2013 + law of total variance
//!
//! ```text
//! Var(Y) = E[Var(Y|X_i)] + Var(E[Y|X_i])
//! S_1_i  = Var(E[Y|X_i]) / Var(Y)
//!        = 1 − E[Var(Y|X_i)] / Var(Y)              (the form we compute)
//! ```
//!
//! For each factor `i`:
//!
//! 1. Partition `X[:, i]` into `M` equal-frequency classes by
//!    ordinal rank (same partition as `borgonovo::class_count`,
//!    matches `SALib`).
//! 2. For each class `j`: compute `Var(Y | X_i ∈ class_j)` (population
//!    variance, 1/n divisor).
//! 3. `E[Var(Y|X_i)] = Σ_j (|class_j| / N) · Var_j`.
//! 4. `S_1_i = 1 − E[Var(Y|X_i)] / Var(Y)`, clamped to `[0, 1]`.
//!
//! Computing via `1 − E[Var(Y|X)]/Var(Y)` avoids materializing
//! conditional means and matches `SALib`'s `delta.analyze`'s `S1`
//! output exactly.
//!
//! # Differences from RBD-FAST
//!
//! Both produce first-order `S_1` from given `(X, Y)`. RBD-FAST
//! uses *spectral* analysis on rank-permuted output (FFT, Plischke
//! 2010 bias correction). This estimator uses *direct* variance
//! decomposition by partition. Pros and cons:
//!
//! | | RBD-FAST | Given-data Sobol' |
//! |---|---|---|
//! | Mechanism | Spectral (FFT) | Variance partition |
//! | Bias correction | Plischke 2010 (`λ = 2M/N`) | None — Eq 7 is unbiased asymptotically |
//! | Tunable | Harmonic order `M` | Class count `M` (auto from `N`) |
//! | Cost | `O(N log N · d)` | `O(N · d)` |
//!
//! Both are sampler-agnostic. Differ in numerical behavior at
//! finite `N`; converge to the same true `S_1` as `N → ∞`.
//!
//! # Determinism
//!
//! Pure under `(X, Y)`. Stable sort + ordinal ranking → deterministic
//! class membership. All sums route through `tree_sum`. Same `(X, Y)`
//! in → bit-identical `S1` out.

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::needless_range_loop
)]

use std::cmp::Ordering;

use ndarray::Array2;
use salib_core::tree_sum;

/// First-order Sobol' index estimates from given-data partition.
///
/// `#[non_exhaustive]` — future fields (`bootstrap_ci`, total-order
/// extension if a literature-vetted variant lands) land non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GivenDataSobolIndices {
    /// First-order Sobol' index per factor, length `d`. Clamped to
    /// `[0, 1]` (the law of total variance guarantees the
    /// population value is non-negative; finite-sample noise can
    /// push the raw computation slightly outside that range, hence
    /// the clamp).
    pub s1: Vec<f64>,
}

impl GivenDataSobolIndices {
    /// Factor count.
    #[must_use]
    pub fn d(&self) -> usize {
        self.s1.len()
    }
}

/// Errors from [`estimate_given_data_sobol`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum GivenDataSobolError {
    #[error("given-data Sobol': shape mismatch — X has {x_rows} rows, y has {y_len} elements")]
    ShapeMismatch { x_rows: usize, y_len: usize },
    #[error("given-data Sobol': d must be ≥ 1, got 0")]
    ZeroD,
    /// `N < 16` — too few samples for meaningful partitioning.
    /// Floor matches `borgonovo` for consistency across the
    /// partition-based given-data estimator family.
    #[error("given-data Sobol': N must be ≥ 16, got {n}")]
    InsufficientSamples { n: usize },
    #[error("given-data Sobol': Var(Y) is zero (model output is constant)")]
    ZeroVariance,
}

/// Estimate first-order Sobol' indices from generic `(X, Y)` data
/// via the Plischke-Borgonovo-Smith 2013 partition estimator.
///
/// `x` is the `(N, d)` input matrix; `y` is the `N`-element model
/// output. Sampler-agnostic — LHS, Sobol', Saltelli matrix, user
/// data all work.
///
/// # Errors
///
/// - [`GivenDataSobolError::ShapeMismatch`] if `x.nrows() != y.len()`.
/// - [`GivenDataSobolError::ZeroD`] if `x.ncols() == 0`.
/// - [`GivenDataSobolError::InsufficientSamples`] if `N < 16`.
/// - [`GivenDataSobolError::ZeroVariance`] if `Var(Y) ≈ 0`.
pub fn estimate_given_data_sobol(
    x: &Array2<f64>,
    y: &[f64],
) -> Result<GivenDataSobolIndices, GivenDataSobolError> {
    let n = x.nrows();
    let d = x.ncols();
    if d == 0 {
        return Err(GivenDataSobolError::ZeroD);
    }
    if y.len() != n {
        return Err(GivenDataSobolError::ShapeMismatch {
            x_rows: n,
            y_len: y.len(),
        });
    }
    if n < 16 {
        return Err(GivenDataSobolError::InsufficientSamples { n });
    }

    let var_y = population_variance(y);
    if !var_y.is_finite() || var_y < 1e-15 {
        return Err(GivenDataSobolError::ZeroVariance);
    }

    let n_classes = class_count(n);
    let n_f = n as f64;
    let mut s1 = vec![0.0_f64; d];

    let mut x_col_buf = vec![0.0_f64; n];
    let mut class_y_buf: Vec<f64> = Vec::with_capacity(n);
    let mut weighted_intra = vec![0.0_f64; n_classes];

    for i in 0..d {
        for k in 0..n {
            x_col_buf[k] = x[[k, i]];
        }
        let ranks = ordinal_ranks(&x_col_buf);

        for j in 0..n_classes {
            let lo = (n_f * (j as f64) / (n_classes as f64)) as usize;
            let hi = (n_f * ((j + 1) as f64) / (n_classes as f64)) as usize;
            class_y_buf.clear();
            for (k, &r) in ranks.iter().enumerate() {
                if r > lo && r <= hi {
                    class_y_buf.push(y[k]);
                }
            }
            let nm = class_y_buf.len();
            if nm > 0 {
                let weight = (nm as f64) / n_f;
                let var_class = population_variance(&class_y_buf);
                weighted_intra[j] = weight * var_class;
            } else {
                weighted_intra[j] = 0.0;
            }
        }
        let e_var_given_x = tree_sum(&weighted_intra);
        // Law of total variance: S_1 = 1 - E[Var(Y|X_i)] / Var(Y).
        let raw = 1.0 - e_var_given_x / var_y;
        s1[i] = raw.clamp(0.0, 1.0);
    }

    Ok(GivenDataSobolIndices { s1 })
}

/// Equal-frequency partition count, matching `SALib` /
/// `borgonovo::class_count`.
fn class_count(n: usize) -> usize {
    let n_f = n as f64;
    let tanh_arg = (1500.0 - n_f) / 500.0;
    let exp = 2.0 / (7.0 + tanh_arg.tanh());
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let raw = n_f.powf(exp).ceil() as usize;
    raw.clamp(2, 48)
}

/// Population variance (1/n divisor; not Bessel) — matches the
/// scaling used by `SALib`'s `delta.sobol_first`.
fn population_variance(v: &[f64]) -> f64 {
    let n = v.len() as f64;
    let mean = tree_sum(v) / n;
    let sq_sum: f64 = v.iter().map(|x| (x - mean).powi(2)).sum();
    sq_sum / n
}

/// Ordinal ranks `1..=N`, stable tie-break by input order.
fn ordinal_ranks(data: &[f64]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..data.len()).collect();
    idx.sort_by(|&a, &b| data[a].partial_cmp(&data[b]).unwrap_or(Ordering::Equal));
    let mut ranks = vec![0_usize; data.len()];
    for (rank, &i) in idx.iter().enumerate() {
        ranks[i] = rank + 1;
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
                #[allow(clippy::cast_possible_truncation)]
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
            estimate_given_data_sobol(&x, &y).unwrap_err(),
            GivenDataSobolError::ZeroD
        );
    }

    #[test]
    fn shape_mismatch_errors() {
        let x = Array2::<f64>::zeros((100, 3));
        let y = vec![0.0; 50];
        let err = estimate_given_data_sobol(&x, &y).unwrap_err();
        assert!(matches!(err, GivenDataSobolError::ShapeMismatch { .. }));
    }

    #[test]
    fn insufficient_samples_errors() {
        let x = synthetic_x(10, 3);
        let y = vec![0.0; 10];
        let err = estimate_given_data_sobol(&x, &y).unwrap_err();
        assert!(matches!(
            err,
            GivenDataSobolError::InsufficientSamples { .. }
        ));
    }

    #[test]
    fn constant_model_errors() {
        let x = synthetic_x(64, 3);
        let y = vec![1.0; 64];
        let err = estimate_given_data_sobol(&x, &y).unwrap_err();
        assert_eq!(err, GivenDataSobolError::ZeroVariance);
    }

    // ── Output shape ──────────────────────────────────────────────

    #[test]
    fn output_length_matches_d() {
        let n = 256;
        let x = synthetic_x(n, 5);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]]).collect();
        let est = estimate_given_data_sobol(&x, &y).unwrap();
        assert_eq!(est.d(), 5);
        assert_eq!(est.s1.len(), 5);
    }

    // ── S_1 in [0, 1] ─────────────────────────────────────────────

    #[test]
    fn indices_in_unit_interval() {
        let n = 512;
        let x = synthetic_x(n, 3);
        let y: Vec<f64> = (0..n)
            .map(|k| x[[k, 0]] + 0.5 * x[[k, 1]] * x[[k, 2]])
            .collect();
        let est = estimate_given_data_sobol(&x, &y).unwrap();
        for &v in &est.s1 {
            assert!((0.0..=1.0).contains(&v), "S_1 = {v} not in [0, 1]");
        }
    }

    // ── Linear single-factor: S_1[0] should dominate ─────────────

    #[test]
    fn linear_single_factor_dominates() {
        // Y = X[:, 0] — first-order S_1[0] should be near 1.
        let n = 512;
        let x = synthetic_x(n, 3);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]]).collect();
        let est = estimate_given_data_sobol(&x, &y).unwrap();
        assert!(
            est.s1[0] > 0.7,
            "S_1[0] = {} should dominate for Y = X_0",
            est.s1[0]
        );
        assert!(
            est.s1[1] < 0.3,
            "S_1[1] = {} should be small (factor 1 not in model)",
            est.s1[1]
        );
    }

    // ── Additive sum of two: S_1 should split proportionally ─────

    #[test]
    fn additive_two_factors_split_proportionally() {
        // Y = 2·X[:, 0] + X[:, 1] over independent uniform X.
        // Var(Y) = 4·Var(X_0) + Var(X_1) = 5·(1/12) ≈ 0.417.
        // Var(E[Y|X_0]) = Var(2·X_0) = 4/12 = 1/3.
        // Var(E[Y|X_1]) = Var(X_1) = 1/12.
        // S_1[0] = (1/3) / (5/12) = 4/5 = 0.8.
        // S_1[1] = (1/12) / (5/12) = 1/5 = 0.2.
        let n = 4096;
        let x = synthetic_x(n, 2);
        let y: Vec<f64> = (0..n).map(|k| 2.0 * x[[k, 0]] + x[[k, 1]]).collect();
        let est = estimate_given_data_sobol(&x, &y).unwrap();
        assert!(
            (est.s1[0] - 0.8).abs() < 0.05,
            "S_1[0] = {} should ≈ 0.8",
            est.s1[0]
        );
        assert!(
            (est.s1[1] - 0.2).abs() < 0.05,
            "S_1[1] = {} should ≈ 0.2",
            est.s1[1]
        );
    }

    // ── Determinism ───────────────────────────────────────────────

    #[test]
    fn same_input_yields_identical_output() {
        let n = 64;
        let x = synthetic_x(n, 3);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]] + x[[k, 1]] * x[[k, 2]]).collect();
        let a = estimate_given_data_sobol(&x, &y).unwrap();
        let b = estimate_given_data_sobol(&x, &y).unwrap();
        assert_eq!(a.s1, b.s1);
    }

    // ── Helpers ───────────────────────────────────────────────────

    #[test]
    fn class_count_matches_borgonovo_table() {
        // Same formula as borgonovo::class_count.
        assert_eq!(class_count(1024), 6);
        assert_eq!(class_count(4096), 16);
        assert_eq!(class_count(1_000_000), 48);
    }

    #[test]
    fn population_variance_zero_for_constant() {
        assert_eq!(population_variance(&[1.0, 1.0, 1.0]), 0.0);
    }

    #[test]
    fn population_variance_matches_formula() {
        // Var([1, 2, 3]) population = ((1-2)² + (2-2)² + (3-2)²)/3
        //                           = 2/3 ≈ 0.6667.
        let v = vec![1.0, 2.0, 3.0];
        let expected = 2.0 / 3.0;
        assert!((population_variance(&v) - expected).abs() < 1e-12);
    }

    #[test]
    fn ordinal_ranks_one_indexed() {
        let data = [3.0, 1.0, 2.0];
        assert_eq!(ordinal_ranks(&data), vec![3, 1, 2]);
    }
}
