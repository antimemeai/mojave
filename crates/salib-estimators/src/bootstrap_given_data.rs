//! Generic percentile-bootstrap confidence intervals for *given-data*
//! sensitivity estimators. Sibling to [`crate::bootstrap`], which
//! covers the *designed-sample* (Saltelli 2010) path.
//!
//! # Why a separate module
//!
//! Designed-sample Sobol' bootstrap can cache `fa, fb, fab` once and
//! resample by row-index without re-evaluating the model — the
//! `(A, B, A_Bⁱ)` row-alignment is preserved per Saltelli 2002 § 5.
//!
//! Given-data estimators (Borgonovo δ, regression, given-data Sobol',
//! PAWN, RBD-FAST, QOSA, …) take a single `(X, Y)` pair. Bootstrap
//! is row-resampling on `(X, Y)` and re-running the estimator on
//! each resample. There is no cache to reuse — the estimator's
//! internal partitioning / ranking / spectral work is recomputed per
//! draw. This module ships the generic loop so each estimator's
//! per-factor index function is the only thing the caller plugs in.
//!
//! # Algorithm
//!
//! Standard percentile bootstrap, single output:
//!
//! 1. For each bootstrap resample `k = 1..=B`:
//!    - Draw `n` row-indices `idx[k]` ~ Uniform{0..n} with replacement.
//!    - Build `x_re = X[idx[k], :]`, `y_re = Y[idx[k]]`.
//!    - Call `estimator_fn(&x_re, &y_re)` → per-factor `Vec<f64>`
//!      of length `d`.
//! 2. The `(1 − α)` CI for factor `i` is the
//!    `[α/2, 1 − α/2]` percentile of the `B − n_skipped` resamples.
//!
//! # Multi-output
//!
//! This helper is single-output. `y` is one output column. Callers
//! with `(N, k_outputs)` shaped output iterate over output columns
//! externally and call this helper once per column. Keeping the
//! signature single-output avoids prescribing an output-column
//! storage shape that not every estimator can honor.
//!
//! # Failed-resample handling
//!
//! Each bootstrap draw is a fresh `(X', Y')` of size `n` with row
//! repetition, drawn uniformly. A degenerate draw can violate an
//! estimator's preconditions: a Borgonovo or given-data-Sobol' draw
//! that lands all rows in one X-class will see zero conditional
//! variance; an RBD-FAST draw with all-equal `Y` triggers
//! `ZeroVariance`; a regression draw can yield a singular design
//! matrix.
//!
//! **Decision:** failed resamples are *skipped*, not propagated as a
//! whole-bootstrap error. Tracked in `BootstrapCi::n_skipped`. The
//! percentile pool is the surviving `B − n_skipped` re-estimates.
//!
//! Tradeoff:
//!
//! - **Skip-and-track (chosen)**: a single pathological resample
//!   doesn't kill an otherwise-valid bootstrap of B = 1000. Caller
//!   inspects `n_skipped` to decide whether the CI is trustworthy
//!   (`n_skipped / B` near 1 → bootstrap is uninformative; the CI
//!   becomes `(NaN, NaN)` if all are skipped).
//! - **Early-return**: simpler contract, but a degenerate small-N
//!   draw of bad luck would torpedo a long bootstrap run; surfaces
//!   as an error rather than as a tracked count.
//!
//! Skip-and-track matches `SALib`'s posture (`SALib` silently uses
//! `np.nan` for failed resamples and `np.nanpercentile` for the CI;
//! we make the count explicit in the result type).
//!
//! # Determinism
//!
//! Pure under `(x, y, n_resamples, alpha, rng_state, estimator_fn)`.
//! Caller supplies the RNG (`salib_core::RngState`, the workspace
//! pattern). Same `RngState` in → bit-identical CIs out, *provided*
//! `estimator_fn` is itself deterministic for a given `(X, Y)`.
//!
//! # Cost
//!
//! `B` model-free re-estimates. The expensive call is `estimator_fn`,
//! not the resampling. Cost is `B × cost(estimator_fn(N, d))`.

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

use std::error::Error as StdError;

use ndarray::Array2;
use rand::RngCore;
use salib_core::RngState;

use crate::bootstrap::{percentile_ci, percentile_value};

/// Boxed estimator-error returned by the user's `estimator_fn`.
/// Each estimator has its own error type; the caller adapts via
/// `.map_err(|e| Box::new(e) as BoxedEstimatorError)`.
pub type BoxedEstimatorError = Box<dyn StdError + Send + Sync>;

/// Per-factor percentile bootstrap CIs. Length `d` (number of
/// factors).
///
/// `#[non_exhaustive]` — future fields land non-breaking.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct BootstrapCi {
    /// Lower CI bound per factor — the `α/2` percentile of the
    /// surviving re-estimates. Length `d`.
    pub ci_low: Vec<f64>,
    /// Upper CI bound per factor — the `1 − α/2` percentile. Length
    /// `d`.
    pub ci_high: Vec<f64>,
    /// Number of bootstrap resamples requested.
    pub n_resamples: usize,
    /// Significance level — CI is `(1 − alpha)`.
    pub alpha: f64,
    /// Number of resamples whose `estimator_fn` call returned `Err`.
    /// The percentile pool is `n_resamples − n_skipped` per factor.
    /// `n_skipped == n_resamples` produces `NaN` CIs.
    pub n_skipped: usize,
}

/// Errors from [`bootstrap_given_data`].
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum BootstrapGivenDataError {
    /// `n_resamples == 0` — at least one resample is required.
    #[error("bootstrap-given-data: n_resamples must be ≥ 1, got 0")]
    ZeroResamples,
    /// `alpha` outside the open interval `(0, 1)`.
    #[error("bootstrap-given-data: alpha must be in (0, 1), got {alpha}")]
    OutOfRangeAlpha { alpha: f64 },
    /// `x.nrows() != y.len()`.
    #[error("bootstrap-given-data: shape mismatch — X has {x_rows} rows, y has {y_len} elements")]
    ShapeMismatch { x_rows: usize, y_len: usize },
    /// `x.nrows() == 0` — cannot bootstrap an empty sample.
    #[error("bootstrap-given-data: N must be ≥ 1, got 0")]
    EmptySample,
}

/// Per-factor percentile bootstrap CIs from a given-data estimator.
///
/// Resamples row-indices uniformly with replacement from `(X, Y)`,
/// re-runs `estimator_fn` on each resample, and returns the
/// element-wise percentile CIs over the surviving re-estimates.
///
/// `estimator_fn` returns `Vec<f64>` of length `d` — one index per
/// factor. The caller is responsible for passing the same `(X, Y)`
/// shape and factor ordering that they used for the point estimate;
/// this helper does no correlation between point estimates and the
/// bootstrap (it is purely a CI generator).
///
/// # Multi-output
///
/// Single-output. For multi-output `(N, k)` data, call this once per
/// output column.
///
/// # Failed resamples
///
/// `estimator_fn` returning `Err` for a resample increments
/// `n_skipped` and excludes that draw from the percentile pool. See
/// the module-level "Failed-resample handling" section.
///
/// # Determinism
///
/// Pure under `(x, y, n_resamples, alpha, rng_state, estimator_fn)`.
/// Same `RngState` in → bit-identical `BootstrapCi` out, provided
/// `estimator_fn` is itself deterministic for a given `(X, Y)`.
///
/// # Errors
///
/// - [`BootstrapGivenDataError::ZeroResamples`] if `n_resamples == 0`.
/// - [`BootstrapGivenDataError::OutOfRangeAlpha`] if
///   `alpha <= 0 || alpha >= 1` (or non-finite).
/// - [`BootstrapGivenDataError::ShapeMismatch`] if
///   `x.nrows() != y.len()`.
/// - [`BootstrapGivenDataError::EmptySample`] if `x.nrows() == 0`.
pub fn bootstrap_given_data<F>(
    x: &Array2<f64>,
    y: &[f64],
    n_resamples: usize,
    alpha: f64,
    rng: &mut RngState,
    mut estimator_fn: F,
) -> Result<BootstrapCi, BootstrapGivenDataError>
where
    F: FnMut(&Array2<f64>, &[f64]) -> Result<Vec<f64>, BoxedEstimatorError>,
{
    // ── Input validation ────────────────────────────────────────────
    if n_resamples == 0 {
        return Err(BootstrapGivenDataError::ZeroResamples);
    }
    if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
        return Err(BootstrapGivenDataError::OutOfRangeAlpha { alpha });
    }
    let n = x.nrows();
    if n == 0 {
        return Err(BootstrapGivenDataError::EmptySample);
    }
    if y.len() != n {
        return Err(BootstrapGivenDataError::ShapeMismatch {
            x_rows: n,
            y_len: y.len(),
        });
    }
    let d_in = x.ncols();

    // ── Bootstrap loop ──────────────────────────────────────────────
    let mut chacha = rng.clone().into_chacha();

    // `per_factor_resamples[i]` collects the surviving estimates for
    // factor `i`. Length `d` is determined by the first successful
    // resample. If no resample succeeds, we fall back to `d_in` so
    // the result has consistent shape with the input.
    let mut per_factor_resamples: Option<Vec<Vec<f64>>> = None;
    let mut n_skipped = 0usize;

    let mut idx = vec![0usize; n];
    let mut x_re = Array2::<f64>::zeros((n, d_in));
    let mut y_re = vec![0.0_f64; n];

    for _ in 0..n_resamples {
        // Draw row indices uniformly with replacement.
        for slot in &mut idx {
            *slot = (chacha.next_u32() as usize) % n;
        }

        // Materialize the resampled (X', Y'). Building owned
        // structures (rather than views) is the simplest portable
        // contract for `estimator_fn` — every existing estimator
        // accepts `&Array2<f64>` and `&[f64]`.
        for (row_out, &src) in idx.iter().enumerate() {
            for col in 0..d_in {
                x_re[[row_out, col]] = x[[src, col]];
            }
            y_re[row_out] = y[src];
        }

        match estimator_fn(&x_re, &y_re) {
            Ok(per_factor) => {
                let acc = per_factor_resamples
                    .get_or_insert_with(|| vec![Vec::with_capacity(n_resamples); per_factor.len()]);
                // Defensive: estimator should return the same `d`
                // every call. If a misbehaving estimator returns
                // a different length, treat it as a failed resample
                // rather than panicking.
                if per_factor.len() != acc.len() {
                    n_skipped += 1;
                    continue;
                }
                for (sink, val) in acc.iter_mut().zip(per_factor) {
                    sink.push(val);
                }
            }
            Err(_) => {
                n_skipped += 1;
            }
        }
    }

    *rng = RngState::snapshot(&chacha, rng);

    // ── Percentile CIs ──────────────────────────────────────────────
    let low_p = alpha / 2.0;
    let high_p = 1.0 - alpha / 2.0;

    let (ci_low, ci_high) = match per_factor_resamples {
        Some(per_factor) => {
            let mut lo = Vec::with_capacity(per_factor.len());
            let mut hi = Vec::with_capacity(per_factor.len());
            for samples in &per_factor {
                let (l, h) = percentile_ci(samples, low_p, high_p);
                lo.push(l);
                hi.push(h);
            }
            (lo, hi)
        }
        None => {
            // Every resample failed. Return NaN CIs at the input's
            // factor count so the result has predictable shape.
            (vec![f64::NAN; d_in], vec![f64::NAN; d_in])
        }
    };

    // Touch `percentile_value` so the re-export from `bootstrap` is
    // exercised at least once via the public surface — keeps the
    // `pub(crate)` boundary honest under dead-code analysis.
    debug_assert!(percentile_value(&[0.0_f64], 0.5).is_finite());

    Ok(BootstrapCi {
        ci_low,
        ci_high,
        n_resamples,
        alpha,
        n_skipped,
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::given_data_sobol::estimate_given_data_sobol;
    use ndarray::Array2;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    fn fresh_rng() -> RngState {
        RngState::from_seed([0xab; 32])
    }

    /// Synthesize `(X, Y)` with `Y = 2·X_0 + 1·X_1 + ε`, `X_2`
    /// inert. `X` ~ U[0, 1]^d via a fixed-seed `ChaCha20`.
    fn linear_gaussian(n: usize, seed: u8) -> (Array2<f64>, Vec<f64>) {
        let d = 3;
        let mut rng = ChaCha20Rng::from_seed([seed; 32]);
        let mut x = Array2::<f64>::zeros((n, d));
        let scale = f64::from(u32::MAX) + 1.0;
        for i in 0..n {
            for j in 0..d {
                // Uniform [0, 1) from the high 32 bits.
                let u = f64::from(rng.next_u32()) / scale;
                x[[i, j]] = u;
            }
        }
        let mut y = vec![0.0_f64; n];
        for i in 0..n {
            // Tiny noise term so Var(Y) is non-degenerate even on
            // unlucky resamples.
            let eps = (f64::from(rng.next_u32()) / scale - 0.5) * 0.01;
            y[i] = 2.0 * x[[i, 0]] + 1.0 * x[[i, 1]] + eps;
        }
        (x, y)
    }

    fn boxed<E: StdError + Send + Sync + 'static>(e: E) -> BoxedEstimatorError {
        Box::new(e)
    }

    // ── Determinism ─────────────────────────────────────────────────

    #[test]
    fn same_inputs_produce_bit_identical_output() {
        let (x, y) = linear_gaussian(128, 0x42);
        let estimator = |xx: &Array2<f64>, yy: &[f64]| {
            estimate_given_data_sobol(xx, yy)
                .map(|r| r.s1)
                .map_err(boxed)
        };

        let mut r1 = fresh_rng();
        let mut r2 = fresh_rng();
        let ci1 = bootstrap_given_data(&x, &y, 50, 0.05, &mut r1, estimator).unwrap();
        let ci2 = bootstrap_given_data(&x, &y, 50, 0.05, &mut r2, estimator).unwrap();
        assert_eq!(ci1, ci2);
        assert_eq!(r1, r2);
    }

    // ── Linear-Gaussian sanity ──────────────────────────────────────

    #[test]
    fn linear_gaussian_ci_separates_strong_from_weak_factor() {
        let (x, y) = linear_gaussian(512, 0x42);
        let mut rng = fresh_rng();
        let estimator = |xx: &Array2<f64>, yy: &[f64]| {
            estimate_given_data_sobol(xx, yy)
                .map(|r| r.s1)
                .map_err(boxed)
        };
        let ci = bootstrap_given_data(&x, &y, 200, 0.05, &mut rng, estimator).unwrap();

        assert_eq!(ci.ci_low.len(), 3);
        assert_eq!(ci.ci_high.len(), 3);
        // Per-factor sanity: ci_low ≤ ci_high.
        for i in 0..3 {
            assert!(
                ci.ci_low[i] <= ci.ci_high[i],
                "factor {i}: ci_low {} > ci_high {}",
                ci.ci_low[i],
                ci.ci_high[i]
            );
        }
        // X_0 is the strongest factor — its CI low should sit above
        // zero.
        assert!(
            ci.ci_low[0] > 0.0,
            "X_0 ci_low should be > 0, got {}",
            ci.ci_low[0]
        );
        // The strong factor's CI should dominate the weak factor's:
        // ci_high[1] < ci_low[0] (X_1 is weaker than X_0). This is a
        // soft sanity check — at large N + B it holds reliably for
        // the linear-Gaussian construction.
        assert!(
            ci.ci_high[1] < ci.ci_low[0],
            "X_1 ci_high {} should be < X_0 ci_low {}",
            ci.ci_high[1],
            ci.ci_low[0]
        );
    }

    // ── Inert-factor coverage (single-seed sanity, not statistical
    //    coverage — that's a separate bead) ──────────────────────────

    #[test]
    fn inert_factor_ci_brackets_zero() {
        let (x, y) = linear_gaussian(512, 0x42);
        let mut rng = fresh_rng();
        let estimator = |xx: &Array2<f64>, yy: &[f64]| {
            estimate_given_data_sobol(xx, yy)
                .map(|r| r.s1)
                .map_err(boxed)
        };
        let ci = bootstrap_given_data(&x, &y, 200, 0.05, &mut rng, estimator).unwrap();
        // X_2 is inert: a 95% CI should bracket 0 at this single
        // seed. (Statistical-coverage tests over many seeds are an
        // open bead, not landed here.) The given-data Sobol'
        // estimator clamps `s1` to `[0, 1]`, so ci_low is ≥ 0; the
        // weaker assertion is that the CI contains a value
        // indistinguishable from zero.
        assert!(
            ci.ci_low[2] <= 0.05,
            "inert factor ci_low should be near zero, got {}",
            ci.ci_low[2]
        );
        assert!(
            ci.ci_high[2] < 0.5,
            "inert factor ci_high should be small, got {}",
            ci.ci_high[2]
        );
    }

    // ── Input validation ────────────────────────────────────────────

    #[test]
    fn zero_resamples_errors() {
        let (x, y) = linear_gaussian(64, 0x42);
        let mut rng = fresh_rng();
        let estimator = |xx: &Array2<f64>, yy: &[f64]| {
            estimate_given_data_sobol(xx, yy)
                .map(|r| r.s1)
                .map_err(boxed)
        };
        let result = bootstrap_given_data(&x, &y, 0, 0.05, &mut rng, estimator);
        assert_eq!(result.unwrap_err(), BootstrapGivenDataError::ZeroResamples);
    }

    #[test]
    fn alpha_zero_errors() {
        let (x, y) = linear_gaussian(64, 0x42);
        let mut rng = fresh_rng();
        let estimator = |xx: &Array2<f64>, yy: &[f64]| {
            estimate_given_data_sobol(xx, yy)
                .map(|r| r.s1)
                .map_err(boxed)
        };
        let result = bootstrap_given_data(&x, &y, 50, 0.0, &mut rng, estimator);
        assert_eq!(
            result.unwrap_err(),
            BootstrapGivenDataError::OutOfRangeAlpha { alpha: 0.0 }
        );
    }

    #[test]
    fn alpha_one_errors() {
        let (x, y) = linear_gaussian(64, 0x42);
        let mut rng = fresh_rng();
        let estimator = |xx: &Array2<f64>, yy: &[f64]| {
            estimate_given_data_sobol(xx, yy)
                .map(|r| r.s1)
                .map_err(boxed)
        };
        let result = bootstrap_given_data(&x, &y, 50, 1.0, &mut rng, estimator);
        assert_eq!(
            result.unwrap_err(),
            BootstrapGivenDataError::OutOfRangeAlpha { alpha: 1.0 }
        );
    }

    #[test]
    fn alpha_negative_errors() {
        let (x, y) = linear_gaussian(64, 0x42);
        let mut rng = fresh_rng();
        let estimator = |xx: &Array2<f64>, yy: &[f64]| {
            estimate_given_data_sobol(xx, yy)
                .map(|r| r.s1)
                .map_err(boxed)
        };
        let result = bootstrap_given_data(&x, &y, 50, -0.1, &mut rng, estimator);
        assert!(matches!(
            result.unwrap_err(),
            BootstrapGivenDataError::OutOfRangeAlpha { .. }
        ));
    }

    #[test]
    fn shape_mismatch_errors() {
        let (x, mut y) = linear_gaussian(64, 0x42);
        y.pop();
        let mut rng = fresh_rng();
        let estimator = |xx: &Array2<f64>, yy: &[f64]| {
            estimate_given_data_sobol(xx, yy)
                .map(|r| r.s1)
                .map_err(boxed)
        };
        let result = bootstrap_given_data(&x, &y, 50, 0.05, &mut rng, estimator);
        assert_eq!(
            result.unwrap_err(),
            BootstrapGivenDataError::ShapeMismatch {
                x_rows: 64,
                y_len: 63,
            }
        );
    }

    #[test]
    fn empty_sample_errors() {
        let x = Array2::<f64>::zeros((0, 3));
        let y: Vec<f64> = vec![];
        let mut rng = fresh_rng();
        let estimator = |xx: &Array2<f64>, yy: &[f64]| {
            estimate_given_data_sobol(xx, yy)
                .map(|r| r.s1)
                .map_err(boxed)
        };
        let result = bootstrap_given_data(&x, &y, 50, 0.05, &mut rng, estimator);
        assert_eq!(result.unwrap_err(), BootstrapGivenDataError::EmptySample);
    }

    // ── All-skipped edge case ───────────────────────────────────────

    #[derive(Debug, thiserror::Error)]
    #[error("synthetic estimator failure")]
    struct SyntheticErr;

    #[test]
    fn all_resamples_failing_returns_nan_cis_with_full_skip_count() {
        let (x, y) = linear_gaussian(64, 0x42);
        let mut rng = fresh_rng();
        let always_err =
            |_xx: &Array2<f64>, _yy: &[f64]| -> Result<Vec<f64>, BoxedEstimatorError> {
                Err(boxed(SyntheticErr))
            };
        let ci = bootstrap_given_data(&x, &y, 25, 0.05, &mut rng, always_err).unwrap();
        assert_eq!(ci.n_skipped, 25);
        assert_eq!(ci.n_resamples, 25);
        assert_eq!(ci.ci_low.len(), 3);
        assert_eq!(ci.ci_high.len(), 3);
        for i in 0..3 {
            assert!(ci.ci_low[i].is_nan(), "factor {i} ci_low should be NaN");
            assert!(ci.ci_high[i].is_nan(), "factor {i} ci_high should be NaN");
        }
    }

    #[test]
    fn partial_failures_count_correctly() {
        let (x, y) = linear_gaussian(64, 0x42);
        let mut rng = fresh_rng();
        let mut call_count = 0_usize;
        let estimator = |xx: &Array2<f64>, yy: &[f64]| -> Result<Vec<f64>, BoxedEstimatorError> {
            call_count += 1;
            // Fail every third call.
            if call_count.is_multiple_of(3) {
                return Err(boxed(SyntheticErr));
            }
            estimate_given_data_sobol(xx, yy)
                .map(|r| r.s1)
                .map_err(boxed)
        };
        let ci = bootstrap_given_data(&x, &y, 30, 0.05, &mut rng, estimator).unwrap();
        assert_eq!(ci.n_skipped, 10);
        assert_eq!(ci.n_resamples, 30);
        // Surviving 20 resamples is enough to produce non-NaN CIs.
        for i in 0..3 {
            assert!(
                ci.ci_low[i].is_finite(),
                "factor {i} ci_low should be finite"
            );
            assert!(
                ci.ci_high[i].is_finite(),
                "factor {i} ci_high should be finite"
            );
            assert!(ci.ci_low[i] <= ci.ci_high[i]);
        }
    }

    // ── Result shape ────────────────────────────────────────────────

    #[test]
    fn result_carries_alpha_and_resample_count() {
        let (x, y) = linear_gaussian(64, 0x42);
        let mut rng = fresh_rng();
        let estimator = |xx: &Array2<f64>, yy: &[f64]| {
            estimate_given_data_sobol(xx, yy)
                .map(|r| r.s1)
                .map_err(boxed)
        };
        let ci = bootstrap_given_data(&x, &y, 75, 0.10, &mut rng, estimator).unwrap();
        assert_eq!(ci.n_resamples, 75);
        assert_eq!(ci.alpha, 0.10);
    }
}
