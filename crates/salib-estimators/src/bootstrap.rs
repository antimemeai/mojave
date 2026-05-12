//! Percentile-bootstrap confidence intervals for Sobol' index
//! estimates. Per `SALib`'s default and
//! `decisions/2026-04-28-saltelli-tck-posture.md` Layer 4.
//!
//! # Algorithm
//!
//! Standard percentile bootstrap:
//!
//! 1. For each bootstrap resample `k = 1..=B`:
//!    - Draw `n` row-indices `idx[k]` ~ Uniform{0..n} with replacement.
//!    - Apply `idx[k]` to all matrices — row-aligned resampling
//!      preserves the `(A, B, A_Bⁱ)` relationship per Saltelli 2002 § 5.
//!    - Estimate `S_i^(k)` and `S_T_i^(k)` per Saltelli 2010 formulas.
//! 2. The 95% CI for `S_i` is the `[2.5%, 97.5%]` percentile of
//!    the bootstrap distribution. Same for `S_T_i`.
//!
//! # Caching: model evaluations are NOT re-run per resample
//!
//! Naive bootstrap re-evaluates the model `B × n × (d+2)` times,
//! prohibitive for expensive models. We cache the original
//! evaluations `fa, fb, fab[i]` once and resample the cached
//! values by row-index. Cost drops to `n × (d+2)` original
//! evaluations + `B × O(N·d)` resampling/aggregation work.
//!
//! This is the standard Sobol'-bootstrap optimization (Saltelli
//! 2002 § 5; `SALib`'s `analyze.sobol` uses the same pattern).
//!
//! # RNG
//!
//! Each resample's row-indices are drawn via `ChaCha20Rng` derived
//! from the caller's `RngState`. Same `RngState` in → bit-identical
//! `SobolIndicesWithCi` out.

// `similar_names`: `fa`/`fab`, `s_i`/`s_t_i`, etc. are the standard
// SA notation in Saltelli 2010 — fighting the lint for naming
// purity here makes the code harder to cross-reference against the
// paper. `cast_precision_loss`: `n as f64` shows up in every Sobol'
// formula. `expect_used`: ndarray row.as_slice() never fails on
// row-major Array2 from build_saltelli_matrix.
#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::expect_used
)]

use rand::RngCore;
use salib_core::{tree_sum, RngState};
use salib_samplers::SaltelliMatrix;

use crate::saltelli2010::estimate_saltelli2010;
use crate::sobol_indices::{BootstrapMethod, SobolIndices, SobolIndicesWithCi};

/// Estimate Sobol' indices via Saltelli 2010 + percentile bootstrap
/// CIs. The point estimate matches `estimate_saltelli2010`; CIs come
/// from `resamples` row-aligned bootstrap draws.
///
/// Cost: `n × (d+2)` model evaluations (same as the point estimate)
/// + `O(B × n × d)` bookkeeping.
///
/// # Determinism
///
/// Same `RngState` in → bit-identical CIs out. This guarantee assumes
/// the `model` closure is a *pure function* of its argument (or
/// otherwise deterministic across calls). A closure that mutates
/// captured state (counters, accumulators, internal RNGs) defeats
/// the determinism contract — the `RngState` invariant covers only
/// the bootstrap-resample draws, not the model evaluations.
///
/// # Panics
///
/// On `resamples == 0` — at least one resample is required for a CI.
#[allow(clippy::many_single_char_names)]
pub fn estimate_saltelli2010_with_bootstrap<F>(
    matrix: &SaltelliMatrix,
    model: F,
    resamples: usize,
    rng: &mut RngState,
) -> SobolIndicesWithCi
where
    F: Fn(&[f64]) -> f64,
{
    assert!(resamples > 0, "bootstrap: resamples must be ≥ 1");

    let n = matrix.n;
    let d = matrix.dim;

    // Cache original model evaluations once. Resampling reuses these
    // by row-index — no re-evaluation per bootstrap draw.
    let fa = evaluate_rows(&matrix.a, &model);
    let fb = evaluate_rows(&matrix.b, &model);
    let fab: Vec<Vec<f64>> = matrix
        .a_b
        .iter()
        .map(|m| evaluate_rows(m, &model))
        .collect();

    // Point estimate on the original (non-resampled) data — same as
    // `estimate_saltelli2010` would produce.
    let point = estimate_saltelli2010(matrix, &model);

    // Bootstrap. Per resample: draw n row-indices, recompute Sobol'
    // formulas on the resampled cached evaluations.
    let mut chacha = rng.clone().into_chacha();
    let mut s_resamples: Vec<Vec<f64>> = vec![Vec::with_capacity(resamples); d];
    let mut st_resamples: Vec<Vec<f64>> = vec![Vec::with_capacity(resamples); d];

    let mut idx = vec![0usize; n];
    for _ in 0..resamples {
        // Draw row indices uniformly at random with replacement.
        for slot in &mut idx {
            *slot = (chacha.next_u32() as usize) % n;
        }

        // Resampled Sobol' indices on cached fa, fb, fab.
        let resampled = compute_indices_from_cached(&fa, &fb, &fab, &idx, d);
        for ((s_acc, st_acc), (s_v, st_v)) in
            s_resamples.iter_mut().zip(st_resamples.iter_mut()).zip(
                resampled
                    .first_order
                    .iter()
                    .zip(resampled.total_order.iter()),
            )
        {
            s_acc.push(*s_v);
            st_acc.push(*st_v);
        }
    }

    *rng = RngState::snapshot(&chacha, rng);

    // 2.5% / 97.5% percentile per factor.
    let first_order_ci: Vec<(f64, f64)> = s_resamples
        .iter()
        .map(|samples| percentile_ci(samples, 0.025, 0.975))
        .collect();
    let total_order_ci: Vec<(f64, f64)> = st_resamples
        .iter()
        .map(|samples| percentile_ci(samples, 0.025, 0.975))
        .collect();

    SobolIndicesWithCi {
        indices: point,
        first_order_ci,
        total_order_ci,
        bootstrap_resamples: resamples,
        method: BootstrapMethod::Percentile,
    }
}

/// Compute Sobol' indices on cached `fa, fb, fab` values, resampled
/// by row-index `idx`. Internal — used by both the point estimate
/// (`idx = 0..n`) and bootstrap (random `idx`).
#[allow(clippy::many_single_char_names)]
fn compute_indices_from_cached(
    fa: &[f64],
    fb: &[f64],
    fab: &[Vec<f64>],
    idx: &[usize],
    d: usize,
) -> SobolIndices {
    let n = idx.len();
    #[allow(clippy::cast_precision_loss)]
    let n_f = n as f64;

    // Resample fa, fb by idx.
    let fa_re: Vec<f64> = idx.iter().map(|&j| fa[j]).collect();
    let fb_re: Vec<f64> = idx.iter().map(|&j| fb[j]).collect();

    // f_0 and total variance from resampled fa.
    let f0 = tree_sum(&fa_re) / n_f;
    let fa_sq: Vec<f64> = fa_re.iter().map(|x| x * x).collect();
    let d_var = tree_sum(&fa_sq) / n_f - f0 * f0;

    let mut first_order = Vec::with_capacity(d);
    let mut total_order = Vec::with_capacity(d);

    for fab_i in fab {
        let fab_re: Vec<f64> = idx.iter().map(|&j| fab_i[j]).collect();
        // S_i numerator: (1/N) Σ fb · (fab - fa).
        let prod: Vec<f64> = fb_re
            .iter()
            .zip(fab_re.iter().zip(fa_re.iter()))
            .map(|(b, (ab, a))| b * (ab - a))
            .collect();
        let s_i = tree_sum(&prod) / n_f / d_var;
        first_order.push(s_i);

        // S_T_i numerator: (1/(2N)) Σ (fa - fab)².
        let sq: Vec<f64> = fa_re
            .iter()
            .zip(fab_re.iter())
            .map(|(a, ab)| (a - ab).powi(2))
            .collect();
        let s_t_i = tree_sum(&sq) / (2.0 * n_f) / d_var;
        total_order.push(s_t_i);
    }

    SobolIndices::new(n, d, d_var, first_order, total_order)
}

/// Internal: compute `(low_p, high_p)` percentiles of a sample.
/// Linear interpolation between adjacent order statistics — matches
/// numpy's default `percentile` behavior, which is what `SALib` uses.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn percentile_ci(values: &[f64], low_p: f64, high_p: f64) -> (f64, f64) {
    if values.is_empty() {
        return (f64::NAN, f64::NAN);
    }
    let mut sorted: Vec<f64> = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let lo = percentile_value(&sorted, low_p);
    let hi = percentile_value(&sorted, high_p);
    (lo, hi)
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn percentile_value(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }
    let pos = p * (n - 1) as f64;
    let pos_floor = pos.floor();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let lower_idx = pos_floor as usize;
    let frac = pos - pos_floor;
    if lower_idx + 1 >= n {
        return sorted[n - 1];
    }
    sorted[lower_idx] * (1.0 - frac) + sorted[lower_idx + 1] * frac
}

/// Internal: row-major model evaluation. Mirrors `saltelli2010`
/// pattern.
fn evaluate_rows<F>(matrix: &ndarray::Array2<f64>, model: &F) -> Vec<f64>
where
    F: Fn(&[f64]) -> f64,
{
    let n = matrix.shape()[0];
    let mut out = Vec::with_capacity(n);
    for row in matrix.rows() {
        let slice = row
            .as_slice()
            .expect("Array2 row should be contiguous (row-major default)");
        out.push(model(slice));
    }
    out
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use salib_samplers::{build_saltelli_matrix, LhsSampler};

    fn fresh_rng() -> RngState {
        RngState::from_seed([0x42; 32])
    }

    // ── Bootstrap output shape ──────────────────────────────────────

    #[test]
    fn ci_arrays_match_dim() {
        let s = LhsSampler::classic(4);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 256, false, &mut rng).unwrap();
        let mut bootstrap_rng = RngState::from_seed([0xab; 32]);
        let result = estimate_saltelli2010_with_bootstrap(
            &m,
            |x| x[0] + 2.0 * x[1],
            100,
            &mut bootstrap_rng,
        );
        assert_eq!(result.indices.dim, 2);
        assert_eq!(result.first_order_ci.len(), 2);
        assert_eq!(result.total_order_ci.len(), 2);
        assert_eq!(result.bootstrap_resamples, 100);
        assert_eq!(result.method, BootstrapMethod::Percentile);
    }

    #[test]
    fn ci_low_le_high_per_factor() {
        let s = LhsSampler::classic(4);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 256, false, &mut rng).unwrap();
        let mut bootstrap_rng = RngState::from_seed([0xab; 32]);
        let result = estimate_saltelli2010_with_bootstrap(
            &m,
            |x| x[0] + x[1].powi(2),
            200,
            &mut bootstrap_rng,
        );
        for (lo, hi) in &result.first_order_ci {
            assert!(lo <= hi, "S CI: {lo} > {hi}");
        }
        for (lo, hi) in &result.total_order_ci {
            assert!(lo <= hi, "S_T CI: {lo} > {hi}");
        }
    }

    // ── Determinism ─────────────────────────────────────────────────

    #[test]
    fn same_inputs_produce_identical_output() {
        let s = LhsSampler::classic(4);
        let mut rng_m = fresh_rng();
        let m = build_saltelli_matrix(&s, 128, false, &mut rng_m).unwrap();

        let mut r1 = RngState::from_seed([0xab; 32]);
        let mut r2 = RngState::from_seed([0xab; 32]);
        let r1_result = estimate_saltelli2010_with_bootstrap(&m, |x| x[0] + x[1], 50, &mut r1);
        let r2_result = estimate_saltelli2010_with_bootstrap(&m, |x| x[0] + x[1], 50, &mut r2);
        assert_eq!(r1_result, r2_result);
    }

    // ── Bootstrap CIs widen at smaller N ────────────────────────────

    #[test]
    fn ci_width_shrinks_with_more_samples() {
        // Rough sanity: bootstrap CI on n=64 is wider than n=1024.
        // (Per-factor, on average; not strict per-factor — MC noise
        // can flip individual factors.)
        let s = LhsSampler::classic(4);
        let mut rng_small = RngState::from_seed([0x42; 32]);
        let m_small = build_saltelli_matrix(&s, 64, false, &mut rng_small).unwrap();
        let mut rng_large = RngState::from_seed([0x42; 32]);
        let m_large = build_saltelli_matrix(&s, 1024, false, &mut rng_large).unwrap();
        let mut br = RngState::from_seed([0xab; 32]);
        let small =
            estimate_saltelli2010_with_bootstrap(&m_small, |x| x[0] + 2.0 * x[1], 100, &mut br);
        let mut br2 = RngState::from_seed([0xab; 32]);
        let large =
            estimate_saltelli2010_with_bootstrap(&m_large, |x| x[0] + 2.0 * x[1], 100, &mut br2);
        // Average CI width over factors.
        let small_width: f64 = small
            .first_order_ci
            .iter()
            .map(|(lo, hi)| hi - lo)
            .sum::<f64>()
            / small.indices.dim as f64;
        let large_width: f64 = large
            .first_order_ci
            .iter()
            .map(|(lo, hi)| hi - lo)
            .sum::<f64>()
            / large.indices.dim as f64;
        assert!(
            small_width > large_width,
            "n=64 width {small_width} should exceed n=1024 width {large_width}"
        );
    }

    // ── Point estimate inside CI ────────────────────────────────────

    #[test]
    fn point_estimate_is_inside_or_near_ci() {
        // The percentile bootstrap CI should usually contain the
        // point estimate (within reasonable tolerance — at small B,
        // the point and median may differ slightly).
        let s = LhsSampler::classic(4);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 256, false, &mut rng).unwrap();
        let mut br = RngState::from_seed([0xab; 32]);
        let result = estimate_saltelli2010_with_bootstrap(&m, |x| x[0] + 2.0 * x[1], 500, &mut br);
        for i in 0..result.indices.dim {
            let s_i = result.indices.first_order[i];
            let (lo, hi) = result.first_order_ci[i];
            // Allow modest tolerance — point can sit slightly outside
            // CI due to bootstrap-distribution skew.
            assert!(
                s_i >= lo - 0.1 && s_i <= hi + 0.1,
                "S_{i} = {s_i} far outside CI ({lo}, {hi})"
            );
        }
    }

    // ── percentile helper ───────────────────────────────────────────

    #[test]
    fn percentile_value_at_zero_is_min() {
        let v = vec![3.0, 1.0, 2.0, 5.0, 4.0];
        let mut sorted = v.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(percentile_value(&sorted, 0.0), 1.0);
    }

    #[test]
    fn percentile_value_at_one_is_max() {
        let v = vec![3.0, 1.0, 2.0, 5.0, 4.0];
        let mut sorted = v.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(percentile_value(&sorted, 1.0), 5.0);
    }

    #[test]
    fn percentile_value_at_half_interpolates() {
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        // pos = 0.5 * 4 = 2, so sorted[2] = 3.0.
        assert_eq!(percentile_value(&v, 0.5), 3.0);
    }

    #[test]
    fn percentile_value_with_one_element() {
        assert_eq!(percentile_value(&[42.0], 0.5), 42.0);
        assert_eq!(percentile_value(&[42.0], 0.0), 42.0);
        assert_eq!(percentile_value(&[42.0], 1.0), 42.0);
    }

    #[test]
    fn percentile_ci_returns_low_to_high() {
        let v = (0..100).map(f64::from).collect::<Vec<_>>();
        let (lo, hi) = percentile_ci(&v, 0.025, 0.975);
        // Roughly 2.5%–97.5%: indices ~2.475–96.525, so 2.475 / 96.525.
        assert!(lo < hi);
        assert!(lo > 0.0 && lo < 5.0);
        assert!(hi > 95.0 && hi < 100.0);
    }
}
