//! RBD-FAST estimator — first-order Sobol' indices `Sᵢ` from
//! generic `(X, Y)` data via random balance designs (Tarantola 2006)
//! with the Plischke 2010 bias correction.
//!
//! Per `decisions/2026-04-29-saltelli-rbd-fast.md`.
//!
//! # Algorithm
//!
//! For each factor `i ∈ 0..d`:
//!
//! 1. Compute `permutation = argsort(X[:, i])`. Sorting `X[:, i]`
//!    monotonically creates a "fundamental frequency 1" pattern in
//!    the reordered output for any function of `xᵢ` alone.
//! 2. Reorder `Y` by `permutation` → `Y_perm`.
//! 3. Compute the one-sided power spectrum
//!    `Sp[k] = |FFT(Y_perm)[k]|² / N²` for `k ∈ 1..=⌊N/2⌋`.
//! 4. Total variance: `V = 2 · Σ_{k=1..=⌊N/2⌋} Sp[k]`.
//! 5. First-order numerator: `V₁ = 2 · Σ_{k=1..=M} Sp[k]`.
//! 6. **Naive estimate**: `S_naive = V₁ / V`.
//! 7. **Plischke 2010 bias correction**:
//!    `λ = 2·M / N`,  `Sᵢ = (S_naive − λ) / (1 − λ)`.
//!
//! Without the bias correction, RBD-FAST overestimates `Sᵢ` for
//! small effects (Plischke 2010 Eq 5-6). The corrected form can
//! produce slightly negative `Sᵢ` for true-zero factors — that's
//! a feature of unbiased estimation, not a bug.
//!
//! # Differences from FAST/eFAST
//!
//! - **Given-data, not designed.** Works on any `(X, Y)` — LHS,
//!   Sobol', user-provided. No special search-curve sampler.
//! - **First-order only.** Sky spec § 5.4: total-order under RBD
//!   is non-trivial and not in `SALib`; deferred.
//! - **Different harmonic budget.** `SALib` defaults `M = 10` here
//!   (vs `M = 4` for FAST). The permutation creates a different
//!   spectral landscape than the search curve.
//!
//! # Determinism
//!
//! Pure under `(X, Y, harmonic)`. Stable sort on `X[:, i]` gives a
//! reproducible permutation; tie-breaking falls back to input
//! order. `rustfft` is bit-deterministic for fixed input. Same
//! `(X, Y)` in → bit-identical `RbdFastIndices` out.

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

use std::cmp::Ordering;
use std::sync::Arc;

use ndarray::Array2;
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use salib_core::tree_sum;

/// First-order Sobol' index estimates per factor with Plischke 2010
/// bias correction.
///
/// `#[non_exhaustive]` — future fields (`bootstrap_ci`, total-order
/// extension if a literature-vetted RBD total-order lands) land
/// non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RbdFastIndices {
    /// First-order Sobol' indices, length `d`. Plischke-corrected;
    /// can be slightly negative for true-zero factors due to MC
    /// noise around the unbiased estimate.
    pub s: Vec<f64>,
}

impl RbdFastIndices {
    /// Factor count.
    #[must_use]
    pub fn d(&self) -> usize {
        self.s.len()
    }
}

/// Errors from [`estimate_rbd_fast`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RbdFastError {
    /// `X.nrows() != y.len()`.
    #[error("RBD-FAST: shape mismatch — X has {x_rows} rows, y has {y_len} elements")]
    ShapeMismatch { x_rows: usize, y_len: usize },
    /// `X.ncols() == 0` — no factors.
    #[error("RBD-FAST: d must be ≥ 1, got 0")]
    ZeroD,
    /// `harmonic == 0`.
    #[error("RBD-FAST: harmonic must be ≥ 1, got 0")]
    ZeroHarmonic,
    /// `N < 2·M + 1`. Plischke 2010 correction `λ = 2·M / N`
    /// requires `N > 2·M` for `(1 − λ)` to be positive.
    #[error(
        "RBD-FAST: N must be ≥ 2·harmonic + 1 (got N={n}, harmonic={harmonic}, \
         minimum={minimum}); else Plischke correction denominator collapses"
    )]
    InsufficientSamples {
        n: usize,
        harmonic: u32,
        minimum: usize,
    },
    /// Total variance below the FFT noise floor — model is constant.
    #[error("RBD-FAST: total variance is zero (model output is constant)")]
    ZeroVariance,
}

/// Estimate first-order Sobol' indices from generic `(X, Y)` data
/// via RBD-FAST with Plischke 2010 bias correction.
///
/// `x` is the `(N, d)` input matrix (each row a sample, each column
/// a factor). `y` is the corresponding model output vector of
/// length `N`. `harmonic` is the spectral truncation order `M`
/// (`SALib` default `10`).
///
/// `X` may be from any sampler — LHS, Sobol', user data — provided
/// the marginal distribution of each column is known. RBD-FAST is
/// invariant to monotonic transformations of `X[:, i]` (the sort
/// step removes them), so non-uniform marginals are fine.
///
/// # Errors
///
/// - [`RbdFastError::ShapeMismatch`] if `x.nrows() != y.len()`.
/// - [`RbdFastError::ZeroD`] if `x.ncols() == 0`.
/// - [`RbdFastError::ZeroHarmonic`] if `harmonic == 0`.
/// - [`RbdFastError::InsufficientSamples`] if `N < 2·harmonic + 1`.
/// - [`RbdFastError::ZeroVariance`] if the model is constant.
///
/// # `NaN` handling
///
/// `X` containing `NaN` values triggers undefined-but-deterministic
/// sort order (stable sort with `partial_cmp` falling back to
/// `Equal` for `NaN` comparisons). The estimate will be valid only
/// if all column values are well-ordered. The caller is responsible
/// for `NaN`-free input.
pub fn estimate_rbd_fast(
    x: &Array2<f64>,
    y: &[f64],
    harmonic: u32,
) -> Result<RbdFastIndices, RbdFastError> {
    let n = x.nrows();
    let d = x.ncols();
    if d == 0 {
        return Err(RbdFastError::ZeroD);
    }
    if y.len() != n {
        return Err(RbdFastError::ShapeMismatch {
            x_rows: n,
            y_len: y.len(),
        });
    }
    if harmonic == 0 {
        return Err(RbdFastError::ZeroHarmonic);
    }
    let minimum = 2 * (harmonic as usize) + 1;
    if n < minimum {
        return Err(RbdFastError::InsufficientSamples {
            n,
            harmonic,
            minimum,
        });
    }

    let fft = build_fft_planner(n);

    let m_usize = harmonic as usize;
    let lambda = 2.0 * f64::from(harmonic) / (n as f64);
    let one_minus_lambda = 1.0 - lambda;

    let mut s = vec![0.0_f64; d];

    let mut permutation: Vec<usize> = (0..n).collect();
    let mut y_perm: Vec<f64> = vec![0.0; n];

    for i in 0..d {
        // argsort(X[:, i]) with stable order.
        permutation
            .iter_mut()
            .enumerate()
            .for_each(|(k, slot)| *slot = k);
        permutation.sort_by(|&a, &b| x[[a, i]].partial_cmp(&x[[b, i]]).unwrap_or(Ordering::Equal));

        // Apply permutation to Y.
        for (k, &p) in permutation.iter().enumerate() {
            y_perm[k] = y[p];
        }

        // One-sided power spectrum.
        let spectrum = power_spectrum_one_sided(&y_perm, fft.as_ref());

        // Total variance: V = 2 · Σ Sp[k].
        let v_total = 2.0 * tree_sum(&spectrum);
        if !v_total.is_finite() || v_total < 1e-15 {
            return Err(RbdFastError::ZeroVariance);
        }

        // First-order: V₁ = 2 · Σ_{k=1..=M} Sp[k]. Sp is 0-indexed
        // with Sp[0] corresponding to frequency 1, so the first M
        // bins are Sp[0..M].
        let take = m_usize.min(spectrum.len());
        let v1 = 2.0 * tree_sum(&spectrum[..take]);

        let s_naive = v1 / v_total;
        // Plischke 2010: S = (S_naive − λ) / (1 − λ).
        s[i] = (s_naive - lambda) / one_minus_lambda;
    }

    Ok(RbdFastIndices { s })
}

fn build_fft_planner(n: usize) -> Arc<dyn Fft<f64>> {
    let mut planner: FftPlanner<f64> = FftPlanner::new();
    planner.plan_fft_forward(n)
}

fn power_spectrum_one_sided(y: &[f64], fft: &dyn Fft<f64>) -> Vec<f64> {
    let n = y.len();
    let mut buffer: Vec<Complex<f64>> = y.iter().map(|&v| Complex::new(v, 0.0)).collect();
    fft.process(&mut buffer);
    let half = n / 2;
    let n_sq = (n as f64).powi(2);
    (1..=half)
        .map(|k| {
            let c = buffer[k];
            (c.re * c.re + c.im * c.im) / n_sq
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use super::*;

    fn lhs_x(n: usize, d: usize) -> Array2<f64> {
        // Deterministic per-column independent permutation of the
        // grid `(k + 0.5)/n` — one stratum per cell, factor columns
        // uncorrelated. Permutation seed varies by `j` so columns
        // are independent in the rank-correlation sense.
        let mut x = Array2::<f64>::zeros((n, d));
        for j in 0..d {
            // Generate a permutation deterministically via a small
            // linear-congruential walk.
            let mut perm: Vec<usize> = (0..n).collect();
            // Fisher-Yates with a per-column LCG seed.
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
                #[allow(clippy::cast_precision_loss)]
                let v = (perm[i] as f64 + 0.5) / (n as f64);
                x[[i, j]] = v;
            }
        }
        x
    }

    // ── Validation ────────────────────────────────────────────────

    #[test]
    fn zero_d_errors() {
        let x = Array2::<f64>::zeros((10, 0));
        let y = vec![0.0; 10];
        assert_eq!(
            estimate_rbd_fast(&x, &y, 4).unwrap_err(),
            RbdFastError::ZeroD
        );
    }

    #[test]
    fn shape_mismatch_errors() {
        let x = Array2::<f64>::zeros((10, 3));
        let y = vec![0.0; 9];
        let err = estimate_rbd_fast(&x, &y, 4).unwrap_err();
        assert!(matches!(err, RbdFastError::ShapeMismatch { .. }));
    }

    #[test]
    fn zero_harmonic_errors() {
        let x = lhs_x(20, 3);
        let y = vec![0.0; 20];
        assert_eq!(
            estimate_rbd_fast(&x, &y, 0).unwrap_err(),
            RbdFastError::ZeroHarmonic
        );
    }

    #[test]
    fn insufficient_samples_errors() {
        // N=8, M=4 → need ≥ 9.
        let x = lhs_x(8, 3);
        let y = vec![0.0; 8];
        let err = estimate_rbd_fast(&x, &y, 4).unwrap_err();
        assert!(matches!(err, RbdFastError::InsufficientSamples { .. }));
    }

    #[test]
    fn constant_model_errors() {
        let x = lhs_x(64, 3);
        let y = vec![1.0; 64];
        let err = estimate_rbd_fast(&x, &y, 4).unwrap_err();
        assert_eq!(err, RbdFastError::ZeroVariance);
    }

    // ── Output shape ──────────────────────────────────────────────

    #[test]
    fn output_length_matches_d() {
        let x = lhs_x(64, 5);
        let y: Vec<f64> = (0..64_u32).map(f64::from).collect();
        let est = estimate_rbd_fast(&x, &y, 4).unwrap();
        assert_eq!(est.d(), 5);
        assert_eq!(est.s.len(), 5);
    }

    // ── Linear single-factor: factor 0 dominant ───────────────────

    #[test]
    fn linear_single_factor_concentrates_variance() {
        // Y = X[:, 0] — all variance from factor 0.
        let n = 256;
        let d = 3;
        let x = lhs_x(n, d);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]]).collect();
        let est = estimate_rbd_fast(&x, &y, 10).unwrap();
        assert!(
            est.s[0] > 0.5,
            "S_0 should dominate for Y = X_0, got {}",
            est.s[0]
        );
        assert!(
            est.s[1].abs() < 0.2,
            "S_1 should be small (factor 1 absent), got {}",
            est.s[1]
        );
        assert!(
            est.s[2].abs() < 0.2,
            "S_2 should be small, got {}",
            est.s[2]
        );
    }

    // ── Plischke correction can produce small negatives ───────────

    #[test]
    fn plischke_correction_allows_small_negatives() {
        // For a model where factor 2 has zero true effect, the
        // bias-corrected estimate can be slightly negative due to
        // MC noise around 0. We don't clamp.
        let n = 256;
        let x = lhs_x(n, 3);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]]).collect();
        let est = estimate_rbd_fast(&x, &y, 10).unwrap();
        // Just assert finite — negatives are allowed by design.
        for &v in &est.s {
            assert!(v.is_finite(), "estimate non-finite: {v}");
        }
    }

    // ── Determinism ───────────────────────────────────────────────

    #[test]
    fn same_input_yields_identical_output() {
        let n = 64;
        let x = lhs_x(n, 3);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]] + x[[k, 1]] * x[[k, 2]]).collect();
        let a = estimate_rbd_fast(&x, &y, 4).unwrap();
        let b = estimate_rbd_fast(&x, &y, 4).unwrap();
        assert_eq!(a.s, b.s);
    }

    // ── Permutation invariance ────────────────────────────────────

    #[test]
    fn rbd_fast_invariant_to_input_row_order() {
        // RBD-FAST sorts X internally; permuting rows of (X, Y)
        // together must yield the same estimate.
        let n = 256;
        let x = lhs_x(n, 3);
        let y: Vec<f64> = (0..n).map(|k| x[[k, 0]] + 2.0 * x[[k, 1]]).collect();
        let est_a = estimate_rbd_fast(&x, &y, 10).unwrap();

        // Reverse the input row order (equivalent permutation).
        let mut x_rev = Array2::<f64>::zeros((n, 3));
        let mut y_rev = vec![0.0; n];
        for k in 0..n {
            for j in 0..3 {
                x_rev[[k, j]] = x[[n - 1 - k, j]];
            }
            y_rev[k] = y[n - 1 - k];
        }
        let est_b = estimate_rbd_fast(&x_rev, &y_rev, 10).unwrap();

        // Permutation invariance is exact: argsort gives the same
        // post-sort sample order regardless of input row order, so
        // Y_perm and the resulting spectrum are bit-identical.
        for i in 0..3 {
            assert!(
                (est_a.s[i] - est_b.s[i]).abs() < 1e-10,
                "factor {i}: a={} b={}",
                est_a.s[i],
                est_b.s[i]
            );
        }
    }
}
