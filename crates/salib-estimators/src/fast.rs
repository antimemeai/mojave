//! FAST/eFAST spectral estimator — `Sᵢ` (first-order) and `Sᵀᵢ`
//! (total-order) Sobol' indices via spectral decomposition of the
//! search-curve sample-output series.
//!
//! Per `decisions/2026-04-29-saltelli-fast-estimator.md`. Companion
//! to PR 9a's [`salib_samplers::FastDesign`] sampler.
//!
//! # Algorithm (Saltelli-Tarantola-Chan 1999)
//!
//! For each factor-of-interest `i ∈ 0..d`:
//!
//! 1. Evaluate the model at the `n_per_factor` points of block `i`
//!    in `design.samples` → `y_i ∈ ℝ^N`.
//! 2. Compute the one-sided power spectrum
//!    `Sp[k] = |Y[k]|² / N²` for `k ∈ 1..=⌊N/2⌋`, where `Y[k]` is
//!    the discrete Fourier transform of `y_i` at frequency `k`.
//!    The DC bin (`k = 0`) is omitted; `V` below is variance-about-
//!    mean by construction.
//! 3. Total variance: `V = 2 · Σ_{k=1..=⌊N/2⌋} Sp[k]`.
//! 4. First-order variance:
//!    `V₁ᵢ = 2 · Σ_{p=1..M} Sp[p · ωᵢ]`.
//! 5. Total-effect "complementary" variance (variance carried by
//!    frequencies in the complementary band `[1, ⌊ωᵢ/2⌋]`):
//!    `V_~ᵢ = 2 · Σ_{k=1..⌊ωᵢ/2⌋} Sp[k]`.
//! 6. Indices: `Sᵢ = V₁ᵢ / V`, `Sᵀᵢ = 1 − V_~ᵢ / V`.
//!
//! Matches `SALib`'s `analyze.fast` exactly modulo MC noise (the
//! sampler's random phase shifts produce different realizations
//! that converge to the same population indices).
//!
//! # Determinism
//!
//! Pure under `(design, model)`. The `rustfft` planner is
//! bit-deterministic for a fixed input length; spectrum extraction
//! and accumulation use `salib-core` tree-fold reductions. Same
//! `(design, model)` in → bit-identical `FastIndices` out
//! regardless of rayon thread count.
//!
//! # Cost
//!
//! `n_per_factor · d` model evaluations + `d` FFTs of length
//! `n_per_factor`. The model dominates by orders of magnitude for
//! any non-trivial model (the FFTs are microseconds at typical
//! `N ≤ 1024`).

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::needless_range_loop
)]

use std::sync::Arc;

use rustfft::{num_complex::Complex, Fft, FftPlanner};
use salib_core::tree_sum;
use salib_samplers::FastDesign;

/// First-order and total-order Sobol' index estimates per factor.
///
/// `#[non_exhaustive]` — future fields (`bootstrap_ci`, `harmonic`
/// echo for audit, `total_variance` for downstream GUM contribution)
/// land non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct FastIndices {
    /// First-order Sobol' indices, length `d`. `s[i]` is the
    /// fraction of total variance attributable to factor `i` alone.
    pub s: Vec<f64>,
    /// Total-order Sobol' indices, length `d`. `st[i]` is the
    /// fraction of total variance attributable to factor `i` and
    /// all interactions involving it.
    pub st: Vec<f64>,
}

impl FastIndices {
    /// Constructor with shape validation.
    #[must_use]
    pub fn new(s: Vec<f64>, st: Vec<f64>) -> Self {
        assert_eq!(s.len(), st.len(), "FastIndices: s and st must agree on d");
        Self { s, st }
    }

    /// Factor count.
    #[must_use]
    pub fn d(&self) -> usize {
        self.s.len()
    }
}

/// Errors from [`estimate_fast`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FastEstimatorError {
    /// Total variance is zero (or numerical floor) — model is
    /// constant, no sensitivity to recover.
    #[error("FAST estimator: total variance is zero (model output is constant)")]
    ZeroVariance,
}

/// Estimate `Sᵢ` and `Sᵀᵢ` from a [`FastDesign`] and a model.
///
/// `model` is called `n_per_factor · d` times, once per row of
/// `design.samples`. The closure must be pure (deterministic given
/// input).
///
/// # Errors
///
/// - [`FastEstimatorError::ZeroVariance`] if the model is constant
///   over the design samples (total variance below `1e-15`, well
///   above the FFT noise floor for `O(1)`-scale outputs).
///
/// `FastDesign` is `#[non_exhaustive]` and constructible only via
/// [`salib_samplers::build_fast_design`], which enforces
/// `n_per_factor ≥ 4·M² + 1`. The bandwidth precondition therefore
/// holds at this seam without a runtime check.
pub fn estimate_fast<F>(
    design: &FastDesign,
    mut model: F,
) -> Result<FastIndices, FastEstimatorError>
where
    F: FnMut(&[f64]) -> f64,
{
    let n = design.n_per_factor;
    let d = design.d;
    let m = design.harmonic;

    let fft = build_fft_planner(n);

    let mut s = vec![0.0_f64; d];
    let mut st = vec![0.0_f64; d];

    // Reusable buffers across blocks.
    let mut row_buf = vec![0.0_f64; d];

    for i in 0..d {
        // Evaluate model at the N samples for factor-of-interest i.
        let mut y: Vec<f64> = Vec::with_capacity(n);
        for n_idx in 0..n {
            let row = i * n + n_idx;
            for j in 0..d {
                row_buf[j] = design.samples[[row, j]];
            }
            y.push(model(&row_buf));
        }

        // One-sided power spectrum: Sp[k] = |Y[k]|² / N² for k ∈ [1, n/2].
        let spectrum = power_spectrum_one_sided(&y, fft.as_ref());

        // Total variance: V = 2 · Σ Sp[k] for k = 1..len(Sp).
        // For a constant signal, the FFT of `[c, c, ..., c]` is
        // `[N·c, 0, 0, ..., 0]` in exact arithmetic; FP rounding
        // leaves residual `~|c|² · N · ε²` per bin (`ε ≈ 1e−16`),
        // accumulating to `~1e−28` at typical `N`. Threshold `1e−15`
        // sits well above that noise floor and well below any
        // legitimate variance signal. Rejects truly-constant-model
        // calls; pass-through otherwise.
        let v_total = 2.0 * tree_sum(&spectrum);
        if !v_total.is_finite() || v_total < 1e-15 {
            return Err(FastEstimatorError::ZeroVariance);
        }

        // First-order: V₁ᵢ = 2 · Σ_{p=1..=M} Sp[p · ωᵢ - 1].
        // (Sp is indexed from 0 corresponding to frequency 1, hence
        // the −1 shift from the math notation.) The bandwidth
        // precondition `n_per_factor ≥ 4·M² + 1` upstream guarantees
        // `M · ωᵢ ≤ ⌊N/2⌋`, so every bin is in range.
        let omega_i = design.omegas[[i, i]] as usize;
        let mut harmonic_bins = [0.0_f64; 32];
        let m_usize = m as usize;
        debug_assert!(m_usize <= harmonic_bins.len(), "harmonic order too large");
        for p in 1..=m_usize {
            let bin = p * omega_i;
            debug_assert!(bin >= 1 && bin - 1 < spectrum.len());
            harmonic_bins[p - 1] = spectrum[bin - 1];
        }
        let v1 = 2.0 * tree_sum(&harmonic_bins[..m_usize]);

        // Total-effect: V_~ᵢ = 2 · Σ_{k=1..=⌊ωᵢ/2⌋} Sp[k].
        // The complementary band sits at `[1, ⌊ωᵢ/2⌋]`; per Saltelli
        // 1999 this is the bandwidth that excludes ωᵢ's harmonics
        // and hence captures only "non-i" variance. The `.max(1)`
        // is defensive — `omegas[[i, i]] = ω_max ≥ 4·M ≥ 4` is
        // guaranteed by `build_fast_design`'s precondition, so
        // `half ≥ 2` always holds.
        let half = (omega_i / 2).max(1).min(spectrum.len());
        let v_comp = 2.0 * tree_sum(&spectrum[..half]);

        s[i] = (v1 / v_total).clamp(0.0, 1.0);
        st[i] = (1.0 - v_comp / v_total).clamp(0.0, 1.0);
    }

    Ok(FastIndices::new(s, st))
}

/// Build a forward FFT plan for length `n`. `rustfft` is
/// deterministic for fixed input.
fn build_fft_planner(n: usize) -> Arc<dyn Fft<f64>> {
    let mut planner: FftPlanner<f64> = FftPlanner::new();
    planner.plan_fft_forward(n)
}

/// Compute the one-sided power spectrum `Sp[k] = |Y[k]|² / N²`
/// for `k ∈ [1, ⌊N/2⌋]`. Output length is `⌊N/2⌋`. Index `k − 1`
/// of the output corresponds to frequency `k` (1-indexed).
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
#[allow(
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::approx_constant
)]
mod tests {
    use super::*;
    use salib_core::RngState;
    use salib_samplers::build_fast_design;

    const SEED: [u8; 32] = [0x42; 32];

    fn build(d: usize, n: usize) -> FastDesign {
        let mut rng = RngState::from_seed(SEED);
        build_fast_design(d, n, 4, &mut rng).expect("valid")
    }

    // ── FastIndices basics ────────────────────────────────────────

    #[test]
    fn fast_indices_d_matches_vec_length() {
        let i = FastIndices::new(vec![0.1, 0.2], vec![0.3, 0.4]);
        assert_eq!(i.d(), 2);
    }

    #[test]
    #[should_panic(expected = "must agree on d")]
    fn fast_indices_mismatch_panics() {
        let _ = FastIndices::new(vec![0.1, 0.2], vec![0.3]);
    }

    // ── Constant model: ZeroVariance ──────────────────────────────

    #[test]
    fn constant_model_returns_zero_variance() {
        let design = build(3, 65);
        let err = estimate_fast(&design, |_| 1.0).unwrap_err();
        assert_eq!(err, FastEstimatorError::ZeroVariance);
    }

    // ── Indices in [0, 1] ─────────────────────────────────────────

    #[test]
    fn indices_clamped_to_unit_interval() {
        let design = build(3, 257);
        // Linear model in factor 0: Y = x_0.
        let est = estimate_fast(&design, |x| x[0]).expect("estimate");
        for &v in &est.s {
            assert!((0.0..=1.0).contains(&v), "S out of range: {v}");
        }
        for &v in &est.st {
            assert!((0.0..=1.0).contains(&v), "ST out of range: {v}");
        }
    }

    // ── Linear single-factor: factor 0 dominant ───────────────────

    #[test]
    fn linear_single_factor_concentrates_variance() {
        // Y = x_0 — all variance attributable to factor 0.
        // S_0 ≈ 1, ST_0 ≈ 1, S_{1,2} ≈ 0, ST_{1,2} ≈ 0.
        let design = build(3, 257);
        let est = estimate_fast(&design, |x| x[0]).expect("estimate");
        assert!(
            est.s[0] > 0.5,
            "S_0 = {} should dominate for Y = x_0",
            est.s[0]
        );
        assert!(est.st[0] > 0.5, "ST_0 = {} should dominate", est.st[0]);
        assert!(
            est.s[1] < 0.2,
            "S_1 = {} should be small (factor 1 not in model)",
            est.s[1]
        );
        assert!(est.s[2] < 0.2, "S_2 = {} should be small", est.s[2]);
    }

    // ── ST ≥ S identity ───────────────────────────────────────────

    #[test]
    fn st_at_least_s_for_additive_model() {
        // Y = x_0 + x_1 + x_2 — additive, no interactions.
        // ST_i ≥ S_i is the universal Sobol' identity.
        let design = build(3, 257);
        let est = estimate_fast(&design, |x| x[0] + x[1] + x[2]).expect("estimate");
        for i in 0..3 {
            assert!(
                est.st[i] + 1e-6 >= est.s[i],
                "factor {i}: ST = {}, S = {} (ST ≥ S violated)",
                est.st[i],
                est.s[i]
            );
        }
    }

    // ── Determinism ───────────────────────────────────────────────

    #[test]
    fn same_design_yields_identical_estimates() {
        let design = build(3, 257);
        let model = |x: &[f64]| x[0] + 0.5 * x[1] * x[2];
        let a = estimate_fast(&design, model).expect("a");
        let b = estimate_fast(&design, model).expect("b");
        assert_eq!(a.s, b.s);
        assert_eq!(a.st, b.st);
    }

    // ── Power spectrum unit ───────────────────────────────────────

    #[test]
    fn power_spectrum_of_zero_signal_is_zero() {
        let n = 16;
        let fft = build_fft_planner(n);
        let zeros = vec![0.0_f64; n];
        let sp = power_spectrum_one_sided(&zeros, fft.as_ref());
        assert_eq!(sp.len(), n / 2);
        for &v in &sp {
            assert_eq!(v, 0.0);
        }
    }

    #[test]
    fn power_spectrum_of_pure_sinusoid_concentrates_at_frequency() {
        // y[n] = sin(2π · k · n / N) — energy concentrates at bin k.
        let n = 64;
        let k = 5_usize;
        let fft = build_fft_planner(n);
        let y: Vec<f64> = (0..n)
            .map(|i| {
                let arg = 2.0 * std::f64::consts::PI * (k as f64) * (i as f64) / (n as f64);
                arg.sin()
            })
            .collect();
        let sp = power_spectrum_one_sided(&y, fft.as_ref());
        // Bin k-1 (0-indexed) corresponds to frequency k. Expect
        // peak there; other bins ≈ 0.
        let peak = sp[k - 1];
        for (idx, &v) in sp.iter().enumerate() {
            if idx != k - 1 {
                assert!(
                    v < peak * 1e-6,
                    "bin {idx}: {v} not negligible vs peak {peak}"
                );
            }
        }
    }
}
