//! PAWN — Pianosi-Wagener moment-independent sensitivity index via
//! Kolmogorov-Smirnov statistics on conditional vs unconditional CDFs.
//!
//! Per `decisions/2026-04-29-saltelli-pawn.md`.
//!
//! # Algorithm
//!
//! For each factor `i`:
//!
//! 1. Slice `X[:, i]` into `S` equal-frequency slices by ordinal rank.
//! 2. For each slice `k`: collect `Y` values whose `X[:, i]` rank
//!    falls in slice `k`. Compute the two-sample Kolmogorov-Smirnov
//!    statistic `KS_k` between the unconditional `Y` and the slice's
//!    `Y_k`:
//!
//! ```text
//! KS_k = max_y |F_Y(y) − F_{Y|slice_k}(y)|
//! ```
//!
//! 3. Aggregate `{KS_1, …, KS_S}` into the per-factor PAWN index:
//!    - `median` (Pianosi-Wagener 2018 default; bias-resilient)
//!    - `maximum` (Pianosi-Wagener 2015 default; conservative)
//!    - `mean`, `minimum`, `cv` (auxiliary statistics)
//!
//! # Why CDF-based, not PDF-based
//!
//! Borgonovo δ uses PDF-based KDE divergence; PAWN uses CDF-based
//! KS statistics. CDFs are observable directly from samples — no
//! bandwidth selection, no integration grid. Trade-off: KS is less
//! sensitive to subtle PDF changes (PAWN can underestimate
//! sensitivity for distributions that differ only in higher moments
//! while having similar CDFs).
//!
//! # 2015 vs 2018 generalization
//!
//! Pianosi-Wagener 2015 used designed input (regular slicing on
//! known marginals); the 2018 generalization works on generic
//! `(X, Y)` from any sampler. We ship the 2018 form via ordinal-
//! rank-based slicing — sampler-agnostic.
//!
//! # Determinism
//!
//! Pure under `(X, Y, n_slices)`. Stable sort + ordinal rank →
//! deterministic slice membership. KS computation is exact (no RNG).
//! Same `(X, Y, n_slices)` in → bit-identical `PawnIndices` out.

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::items_after_statements,
    clippy::needless_range_loop
)]

use std::cmp::Ordering;

use ndarray::Array2;
use salib_core::tree_sum;

/// PAWN sensitivity index estimates per factor — five aggregation
/// statistics over slice-wise KS values.
///
/// `#[non_exhaustive]` — future fields (`bootstrap_ci`, `n_slices`
/// echo, `total_variance`) land non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PawnIndices {
    /// Median KS across slices (Pianosi-Wagener 2018 default).
    pub median: Vec<f64>,
    /// Maximum KS across slices (Pianosi-Wagener 2015 default).
    pub maximum: Vec<f64>,
    /// Mean KS across slices.
    pub mean: Vec<f64>,
    /// Minimum KS across slices.
    pub minimum: Vec<f64>,
    /// Coefficient of variation `std / mean` of the KS slice values.
    pub cv: Vec<f64>,
}

impl PawnIndices {
    /// Factor count.
    #[must_use]
    pub fn d(&self) -> usize {
        self.median.len()
    }
}

/// Errors from [`estimate_pawn`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PawnError {
    #[error("PAWN: shape mismatch — X has {x_rows} rows, y has {y_len} elements")]
    ShapeMismatch { x_rows: usize, y_len: usize },
    #[error("PAWN: d must be ≥ 1, got 0")]
    ZeroD,
    #[error("PAWN: n_slices must be ≥ 2, got {n_slices}")]
    TooFewSlices { n_slices: usize },
    /// Need at least 2 samples per slice for the KS statistic to be
    /// meaningful: `N ≥ 2 · n_slices`.
    #[error("PAWN: N must be ≥ 2·n_slices (got N={n}, n_slices={n_slices}, minimum={minimum})")]
    InsufficientSamples {
        n: usize,
        n_slices: usize,
        minimum: usize,
    },
}

/// Estimate PAWN per factor from generic `(X, Y)` data.
///
/// `n_slices` is the conditioning slice count. `SALib` default `10`;
/// Pianosi 2020 (sensitivity-of-sensitivity analysis) recommends
/// `S ∈ [10, 20]`. Larger `S` resolves the conditional CDF more
/// finely but reduces samples per slice (raising MC noise).
///
/// # Errors
///
/// - [`PawnError::ShapeMismatch`] if `x.nrows() != y.len()`.
/// - [`PawnError::ZeroD`] if `x.ncols() == 0`.
/// - [`PawnError::TooFewSlices`] if `n_slices < 2`.
/// - [`PawnError::InsufficientSamples`] if `N < 2 · n_slices`.
pub fn estimate_pawn(
    x: &Array2<f64>,
    y: &[f64],
    n_slices: usize,
) -> Result<PawnIndices, PawnError> {
    let n = x.nrows();
    let d = x.ncols();
    if d == 0 {
        return Err(PawnError::ZeroD);
    }
    if y.len() != n {
        return Err(PawnError::ShapeMismatch {
            x_rows: n,
            y_len: y.len(),
        });
    }
    if n_slices < 2 {
        return Err(PawnError::TooFewSlices { n_slices });
    }
    let minimum = 2 * n_slices;
    if n < minimum {
        return Err(PawnError::InsufficientSamples {
            n,
            n_slices,
            minimum,
        });
    }

    // Sort Y once for the unconditional empirical CDF.
    let mut y_sorted = y.to_vec();
    y_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

    let mut median = vec![0.0_f64; d];
    let mut maximum = vec![0.0_f64; d];
    let mut mean = vec![0.0_f64; d];
    let mut minimum_arr = vec![0.0_f64; d];
    let mut cv = vec![0.0_f64; d];

    let mut x_col_buf = vec![0.0_f64; n];
    let mut ks_per_slice = vec![0.0_f64; n_slices];
    let mut slice_y_buf: Vec<f64> = Vec::with_capacity(n);

    for i in 0..d {
        for k in 0..n {
            x_col_buf[k] = x[[k, i]];
        }
        let ranks = ordinal_ranks(&x_col_buf);

        for s in 0..n_slices {
            // Equal-frequency slicing on ranks 1..=N.
            let lo = (s * n) / n_slices; // exclusive lower (so r > lo)
            let hi = ((s + 1) * n) / n_slices; // inclusive upper (r <= hi)
            slice_y_buf.clear();
            for (k, &r) in ranks.iter().enumerate() {
                if r > lo && r <= hi {
                    slice_y_buf.push(y[k]);
                }
            }
            slice_y_buf.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
            ks_per_slice[s] = ks_two_sample_sorted(&y_sorted, &slice_y_buf);
        }

        // Aggregate.
        let aggregates = summarize(&ks_per_slice);
        median[i] = aggregates.median;
        maximum[i] = aggregates.max;
        mean[i] = aggregates.mean;
        minimum_arr[i] = aggregates.min;
        cv[i] = aggregates.cv;
    }

    Ok(PawnIndices {
        median,
        maximum,
        mean,
        minimum: minimum_arr,
        cv,
    })
}

struct Summary {
    min: f64,
    max: f64,
    mean: f64,
    median: f64,
    cv: f64,
}

fn summarize(values: &[f64]) -> Summary {
    debug_assert!(!values.is_empty());
    let n = values.len() as f64;
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let min = sorted[0];
    let max = sorted[sorted.len() - 1];
    let median = if sorted.len().is_multiple_of(2) {
        0.5 * (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2])
    } else {
        sorted[sorted.len() / 2]
    };
    let mean = tree_sum(values) / n;
    let var: f64 = tree_sum(
        &values
            .iter()
            .map(|&v| (v - mean).powi(2))
            .collect::<Vec<f64>>(),
    ) / n;
    let std = var.sqrt();
    let cv = if mean.abs() > 1e-15 { std / mean } else { 0.0 };
    Summary {
        min,
        max,
        mean,
        median,
        cv,
    }
}

/// Two-sample Kolmogorov-Smirnov statistic between two **sorted**
/// samples. Matches `scipy.stats.ks_2samp(...).statistic` (two-sided)
/// modulo identical tie-handling.
///
/// `KS = max_y |F_a(y) − F_b(y)|` evaluated at every observed `y` in
/// `a ∪ b`.
fn ks_two_sample_sorted(a: &[f64], b: &[f64]) -> f64 {
    debug_assert!(!a.is_empty() && !b.is_empty());
    let n_a = a.len() as f64;
    let n_b = b.len() as f64;
    let mut i = 0;
    let mut j = 0;
    let mut max_diff = 0.0_f64;

    while i < a.len() || j < b.len() {
        // Pick the smaller next observation; on tie advance both
        // sides past the equal value to avoid double-counting at
        // ties (matches `scipy`'s `searchsorted(side='right')`).
        let v = if i >= a.len() {
            b[j]
        } else if j >= b.len() {
            a[i]
        } else {
            a[i].min(b[j])
        };
        while i < a.len() && a[i] <= v {
            i += 1;
        }
        while j < b.len() && b[j] <= v {
            j += 1;
        }
        let f_a = (i as f64) / n_a;
        let f_b = (j as f64) / n_b;
        let diff = (f_a - f_b).abs();
        if diff > max_diff {
            max_diff = diff;
        }
    }
    max_diff
}

/// Ordinal ranks `1..=N` matching `scipy.stats.rankdata(method="ordinal")`.
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
        assert_eq!(estimate_pawn(&x, &y, 10).unwrap_err(), PawnError::ZeroD);
    }

    #[test]
    fn shape_mismatch_errors() {
        let x = Array2::<f64>::zeros((100, 3));
        let y = vec![0.0; 50];
        let err = estimate_pawn(&x, &y, 10).unwrap_err();
        assert!(matches!(err, PawnError::ShapeMismatch { .. }));
    }

    #[test]
    fn too_few_slices_errors() {
        let x = synthetic_x(100, 3);
        let y = vec![0.0; 100];
        assert_eq!(
            estimate_pawn(&x, &y, 1).unwrap_err(),
            PawnError::TooFewSlices { n_slices: 1 }
        );
    }

    #[test]
    fn insufficient_samples_errors() {
        let x = synthetic_x(15, 3);
        let y = vec![0.0; 15];
        let err = estimate_pawn(&x, &y, 10).unwrap_err();
        assert!(matches!(err, PawnError::InsufficientSamples { .. }));
    }

    // ── Output shape ──────────────────────────────────────────────

    #[test]
    fn output_length_matches_d() {
        let n = 256;
        let x = synthetic_x(n, 5);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]]).collect();
        let est = estimate_pawn(&x, &y, 10).unwrap();
        assert_eq!(est.d(), 5);
        assert_eq!(est.median.len(), 5);
        assert_eq!(est.maximum.len(), 5);
        assert_eq!(est.mean.len(), 5);
        assert_eq!(est.minimum.len(), 5);
        assert_eq!(est.cv.len(), 5);
    }

    // ── Indices in [0, 1] ─────────────────────────────────────────

    #[test]
    fn indices_in_unit_interval() {
        // KS statistic is always in [0, 1].
        let n = 256;
        let x = synthetic_x(n, 3);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]] + 0.5 * x[[k, 1]]).collect();
        let est = estimate_pawn(&x, &y, 10).unwrap();
        for i in 0..3 {
            assert!(
                (0.0..=1.0).contains(&est.median[i]),
                "median_{i} = {} not in [0, 1]",
                est.median[i]
            );
            assert!(
                (0.0..=1.0).contains(&est.maximum[i]),
                "max_{i} = {} not in [0, 1]",
                est.maximum[i]
            );
            assert!(
                est.minimum[i] >= 0.0 && est.minimum[i] <= est.maximum[i],
                "min_{i} {} not in [0, max_{i}={}]",
                est.minimum[i],
                est.maximum[i]
            );
        }
    }

    // ── min ≤ median ≤ max ───────────────────────────────────────

    #[test]
    fn aggregate_ordering_holds() {
        let n = 256;
        let x = synthetic_x(n, 3);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]] + x[[k, 1]] * x[[k, 2]]).collect();
        let est = estimate_pawn(&x, &y, 10).unwrap();
        for i in 0..3 {
            assert!(
                est.minimum[i] <= est.median[i] + 1e-12,
                "min_{i} {} > median_{i} {}",
                est.minimum[i],
                est.median[i]
            );
            assert!(
                est.median[i] <= est.maximum[i] + 1e-12,
                "median_{i} {} > max_{i} {}",
                est.median[i],
                est.maximum[i]
            );
            assert!(
                est.minimum[i] <= est.mean[i] + 1e-12,
                "min_{i} {} > mean_{i} {}",
                est.minimum[i],
                est.mean[i]
            );
            assert!(
                est.mean[i] <= est.maximum[i] + 1e-12,
                "mean_{i} {} > max_{i} {}",
                est.mean[i],
                est.maximum[i]
            );
        }
    }

    // ── Linear single-factor: factor 0 dominates ─────────────────

    #[test]
    fn linear_single_factor_dominates_pawn() {
        let n = 512;
        let x = synthetic_x(n, 3);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]]).collect();
        let est = estimate_pawn(&x, &y, 10).unwrap();
        assert!(
            est.median[0] > est.median[1],
            "median_0 = {} should exceed median_1 = {}",
            est.median[0],
            est.median[1]
        );
        assert!(
            est.median[0] > est.median[2],
            "median_0 = {} should exceed median_2 = {}",
            est.median[0],
            est.median[2]
        );
    }

    // ── Determinism ───────────────────────────────────────────────

    #[test]
    fn same_input_yields_identical_output() {
        let n = 64;
        let x = synthetic_x(n, 3);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]] + x[[k, 1]] * x[[k, 2]]).collect();
        let a = estimate_pawn(&x, &y, 8).unwrap();
        let b = estimate_pawn(&x, &y, 8).unwrap();
        assert_eq!(a.median, b.median);
        assert_eq!(a.maximum, b.maximum);
    }

    // ── KS statistic unit tests ───────────────────────────────────

    #[test]
    fn ks_identical_samples_zero() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(ks_two_sample_sorted(&a, &a), 0.0);
    }

    #[test]
    fn ks_disjoint_samples_one() {
        let a = vec![0.0, 1.0, 2.0];
        let b = vec![10.0, 11.0, 12.0];
        // F_a(2) = 1, F_b(2) = 0 → KS = 1.
        assert_eq!(ks_two_sample_sorted(&a, &b), 1.0);
    }

    #[test]
    fn ks_known_distance_one_third() {
        // a = [0, 1, 2, 3, 4, 5], b = [3, 4, 5, 6, 7, 8].
        // At y=2: F_a = 3/6 = 0.5, F_b = 0/6 = 0   → diff 0.5.
        // At y=5: F_a = 6/6 = 1.0, F_b = 3/6 = 0.5 → diff 0.5.
        // Max diff = 0.5.
        let a: Vec<f64> = (0..6).map(f64::from).collect();
        let b: Vec<f64> = (3..9).map(f64::from).collect();
        let ks = ks_two_sample_sorted(&a, &b);
        assert!((ks - 0.5).abs() < 1e-12, "KS = {ks}, expected 0.5");
    }

    #[test]
    fn ks_handles_ties() {
        let a = vec![1.0, 1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 2.0, 3.0];
        // KS at observed values: at y=1: F_a=2/4=0.5, F_b=1/4=0.25 → 0.25.
        // At y=2: F_a=3/4=0.75, F_b=3/4=0.75 → 0.
        // At y=3: F_a=1, F_b=1 → 0.
        // Max = 0.25.
        let ks = ks_two_sample_sorted(&a, &b);
        assert!((ks - 0.25).abs() < 1e-12, "KS = {ks}, expected 0.25");
    }

    // ── Helpers ───────────────────────────────────────────────────

    #[test]
    fn summarize_basic_stats() {
        let s = summarize(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(s.min, 1.0);
        assert_eq!(s.max, 5.0);
        assert_eq!(s.median, 3.0);
        assert_eq!(s.mean, 3.0);
    }

    #[test]
    fn summarize_even_count_median_averages() {
        let s = summarize(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(s.median, 2.5);
    }

    #[test]
    fn ordinal_ranks_one_indexed() {
        let data = [3.0, 1.0, 2.0];
        assert_eq!(ordinal_ranks(&data), vec![3, 1, 2]);
    }
}
