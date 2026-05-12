//! End-to-end reviewer-affordance contract close for `fit_full_pce`
//! + `sobol_indices_from_pce` against Ishigami at canonical
//!   `(a=7, b=0.1)`.
//!
//! Pattern mirrors `salib-estimators/tests/ishigami_e2e.rs` (the
//! Saltelli2010 PR 7 close), adjusted for surrogate-flow shape:
//! samples → PCE → analytic Sobol' (Sudret 2008 Eq 36-39).
//!
//! Ishigami's analytic indices at `(a=7, b=0.1)`:
//!
//! ```text
//! S_1  ≈ 0.3139
//! S_2  ≈ 0.4424
//! S_3  = 0
//! S_T1 ≈ 0.5576
//! S_T2 ≈ 0.4424
//! S_T3 ≈ 0.2436
//! ```
//!
//! Inputs `X_i ~ Uniform(-π, π)` mapped to Legendre canonical
//! `ξ_i ∈ [-1, 1]` via `ξ = X / π` (equivalently `ξ = 2u - 1` for
//! unit-cube `u ∈ [0, 1)`). PCE is fit with the Legendre basis at
//! `p = 10` (basis size `P = 13!/(3!·10!) = 286`); we use `N = 4096`
//! samples — comfortable `2P` budget for stable OLS.

#![allow(
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal,
    clippy::expect_used,
    clippy::similar_names,
    clippy::items_after_statements
)]

use std::f64::consts::PI;

use ndarray::Array2;
use salib_core::RngState;
use salib_samplers::{Sampler, SobolSampler};
use salib_surrogate::{fit_full_pce, sobol_indices_from_pce, PolynomialFamily};
use salib_validation::{ishigami, SobolIndicesAnalytic};

const FIXTURE_SEED: [u8; 32] = [0; 32];
const N: usize = 4096;
const MAX_DEGREE: usize = 10;

/// Sample N×3 from Sobol' on `[0, 1)³`, map to `ξ ∈ [-1, 1]³`
/// (Legendre canonical), and return both the canonical sample matrix
/// and the corresponding Ishigami outputs (with input `X = π·ξ`).
fn build_pce_inputs() -> (Array2<f64>, Vec<f64>) {
    let sampler = SobolSampler::standard(3).with_skip_first(false);
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let unit = sampler.unit_sample(N, &mut rng);

    let mut canonical = Array2::<f64>::zeros((N, 3));
    let mut y = Vec::with_capacity(N);
    for i in 0..N {
        let xi = [
            2.0 * unit[[i, 0]] - 1.0,
            2.0 * unit[[i, 1]] - 1.0,
            2.0 * unit[[i, 2]] - 1.0,
        ];
        for k in 0..3 {
            canonical[[i, k]] = xi[k];
        }
        let x = [PI * xi[0], PI * xi[1], PI * xi[2]];
        y.push(ishigami::ishigami(&x));
    }
    (canonical, y)
}

// ── Artifact 1+2: canonical Ishigami + model-free identity ──────────

#[test]
fn pce_ishigami_canonical_recovers_published_indices_within_pce_tolerance() {
    let (x, y) = build_pce_inputs();
    let pce = fit_full_pce(&x, &y, &[PolynomialFamily::Legendre; 3], MAX_DEGREE).expect("PCE fit");
    let sobol = sobol_indices_from_pce(&pce).expect("Sobol from PCE");

    let analytic: SobolIndicesAnalytic = ishigami::analytic_indices(7.0, 0.1);

    // PCE at p=10 with Legendre captures Ishigami's `sin(X)` terms via
    // their Legendre expansion; truncation error sets the floor.
    // Empirically (verified at run): all six indices within 0.02 of
    // analytic. Use 0.05 for headroom.
    const TOL: f64 = 0.05;

    for (i, &want) in analytic.first_order.iter().enumerate() {
        let got = sobol.first_order[i];
        assert!(
            (got - want).abs() < TOL,
            "S_{i}: got {got:.4}, want {want:.4} (analytic) within {TOL}"
        );
    }
    for (i, &want) in analytic.total_order.iter().enumerate() {
        let got = sobol.total_order[i];
        assert!(
            (got - want).abs() < TOL,
            "S_T_{i}: got {got:.4}, want {want:.4} (analytic) within {TOL}"
        );
    }

    // The Ishigami canary: S_3 ≈ 0 (analytic = 0).
    assert!(
        sobol.first_order[2].abs() < TOL,
        "X_3 first-order canary: got {} (analytic = 0)",
        sobol.first_order[2]
    );

    // X_2 has no interactions: S_T_2 ≈ S_2.
    assert!(
        (sobol.first_order[1] - sobol.total_order[1]).abs() < TOL,
        "S_2 = {}, S_T_2 = {} should agree (analytic identity)",
        sobol.first_order[1],
        sobol.total_order[1]
    );
}

#[test]
fn pce_ishigami_first_order_at_most_total_order() {
    // Model-free identity: S_i ≤ S_T_i. This is exact for PCE
    // (sub-sum vs full-sum over multi-indices); no MC tolerance
    // needed beyond clamp-induced rounding.
    let (x, y) = build_pce_inputs();
    let pce = fit_full_pce(&x, &y, &[PolynomialFamily::Legendre; 3], MAX_DEGREE).expect("PCE fit");
    let sobol = sobol_indices_from_pce(&pce).expect("Sobol from PCE");
    for i in 0..3 {
        assert!(
            sobol.first_order[i] <= sobol.total_order[i] + 1e-12,
            "S_{i} = {} > S_T_{i} = {}",
            sobol.first_order[i],
            sobol.total_order[i]
        );
    }
}

#[test]
fn pce_ishigami_first_order_sum_at_most_one() {
    // Σ S_i ≤ 1 by Sobol' decomposition — exact for PCE.
    let (x, y) = build_pce_inputs();
    let pce = fit_full_pce(&x, &y, &[PolynomialFamily::Legendre; 3], MAX_DEGREE).expect("PCE fit");
    let sobol = sobol_indices_from_pce(&pce).expect("Sobol from PCE");
    let sum: f64 = sobol.first_order.iter().sum();
    assert!(sum <= 1.0 + 1e-12, "Σ S_i = {sum}");
}

// ── Artifact 4: degree-convergence trend ─────────────────────────────

#[test]
fn pce_ishigami_error_decreases_with_degree() {
    // PCE converges in `p`, not `N` (assuming `N ≥ 2P`). At p=4,
    // the Legendre expansion of `sin(πξ)` is poor; at p=10, much
    // better. Check error at p=10 < error at p=4 for S_1 (largest
    // analytic signal involving `sin(X_1)`).
    let analytic = ishigami::analytic_indices(7.0, 0.1);

    let n_for_p4 = 256usize; // P(d=3, p=4) = 35; 256 ≫ 2P.
    let n_for_p10 = N; // P(d=3, p=10) = 286; 4096 ≫ 2P.

    fn fit_at(n: usize, p: usize) -> f64 {
        let sampler = SobolSampler::standard(3).with_skip_first(false);
        let mut rng = RngState::from_seed(FIXTURE_SEED);
        let unit = sampler.unit_sample(n, &mut rng);
        let mut x = Array2::<f64>::zeros((n, 3));
        let mut y = Vec::with_capacity(n);
        for i in 0..n {
            let xi = [
                2.0 * unit[[i, 0]] - 1.0,
                2.0 * unit[[i, 1]] - 1.0,
                2.0 * unit[[i, 2]] - 1.0,
            ];
            for k in 0..3 {
                x[[i, k]] = xi[k];
            }
            let x_real = [PI * xi[0], PI * xi[1], PI * xi[2]];
            y.push(ishigami::ishigami(&x_real));
        }
        let pce = fit_full_pce(&x, &y, &[PolynomialFamily::Legendre; 3], p).expect("PCE fit");
        let sobol = sobol_indices_from_pce(&pce).expect("Sobol from PCE");
        sobol.first_order[0]
    }

    let s1_p4 = fit_at(n_for_p4, 4);
    let s1_p10 = fit_at(n_for_p10, 10);

    let err_p4 = (s1_p4 - analytic.first_order[0]).abs();
    let err_p10 = (s1_p10 - analytic.first_order[0]).abs();

    assert!(
        err_p10 < err_p4,
        "convergence in p: err(p=10) = {err_p10:.4} should be < err(p=4) = {err_p4:.4}"
    );
    assert!(
        err_p10 < 0.05,
        "S_1 at p=10: err = {err_p10:.4}, expected < 0.05"
    );
}

// ── Determinism ──────────────────────────────────────────────────────

#[test]
fn pce_ishigami_is_deterministic() {
    let (x, y) = build_pce_inputs();
    let a = fit_full_pce(&x, &y, &[PolynomialFamily::Legendre; 3], MAX_DEGREE).expect("PCE fit a");
    let b = fit_full_pce(&x, &y, &[PolynomialFamily::Legendre; 3], MAX_DEGREE).expect("PCE fit b");
    assert_eq!(a.coefficients, b.coefficients);
    let sa = sobol_indices_from_pce(&a).expect("Sobol a");
    let sb = sobol_indices_from_pce(&b).expect("Sobol b");
    assert_eq!(sa.first_order, sb.first_order);
    assert_eq!(sa.total_order, sb.total_order);
}
