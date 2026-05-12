//! End-to-end reviewer-affordance contract close for the Iman-
//! Conover dependent-input transformation.
//!
//! The headline engineering claim: feeding correlated inputs to a
//! sensitivity estimator that assumes independence biases the
//! resulting indices; **applying Iman-Conover before the estimator
//! recovers the analytic indices**. This file pins that claim on a
//! linear additive model where the closed-form Sobol' indices under
//! correlation are derivable.
//!
//! # Linear-additive closed form (Mara 2015 § 4.1)
//!
//! For `Y = X_0 + X_1 + X_2` with `X_i ~ N(0, 1)` and Pearson
//! correlation matrix `C`, the **full** first-order Sobol' index
//! (counts the correlated contribution of `X_i`) is
//!
//! ```text
//! S_i = Var(E[Y | X_i]) / Var(Y)
//!     = (1 + Σ_{j≠i} ρ_ij)² / Var(Y)
//! ```
//!
//! and `Var(Y) = Σ_i Var(X_i) + 2·Σ_{i<j} Cov(X_i, X_j)
//!            = 3 + 2·(ρ_01 + ρ_02 + ρ_12)`.
//!
//! At `(ρ_01, ρ_02, ρ_12) = (0.6, 0, 0)`:
//! - `Var(Y) = 3 + 1.2 = 4.2`
//! - `S_0 = (1 + 0.6 + 0)² / 4.2 = 1.6² / 4.2 ≈ 0.610`
//! - `S_1 = (1 + 0.6 + 0)² / 4.2 ≈ 0.610` (symmetric to S_0 in the
//!   pair correlation)
//! - `S_2 = (1 + 0 + 0)² / 4.2 = 1 / 4.2 ≈ 0.238`
//! - `Σ S_i = 1.458` exceeds 1 — the Sobol' expansion no longer
//!   sums to 1 under correlation, by design (Song 2016 § 3.2,
//!   Theorem 2).
//!
//! Without IC, feeding *independent* draws into the estimator
//! recovers the un-correlated decomposition `S_i = 1/3` per factor
//! — biased *low* on the correlated pair, biased *high* on the
//! un-correlated factor by relative magnitude. With IC, the
//! correlated structure is induced into the samples and the
//! estimator recovers the correlated-input Sobol' indices to
//! within MC tolerance.

#![allow(
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::similar_names,
    clippy::items_after_statements,
    clippy::doc_markdown,
    clippy::many_single_char_names,
    clippy::cast_precision_loss,
    clippy::needless_range_loop
)]

use ndarray::{array, Array2};
use salib_core::{Distribution, RngState};
use salib_estimators::estimate_given_data_sobol;
use salib_samplers::iman_conover_transform;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn standard_normal_samples(n: usize, d: usize, rng: &mut RngState) -> Array2<f64> {
    let mut chacha = rng.clone().into_chacha();
    use rand::RngCore;
    let normal = Distribution::Normal {
        mu: 0.0,
        sigma: 1.0,
    };
    let u32_norm = 1.0_f64 / (f64::from(u32::MAX) + 1.0);
    let mut x = Array2::<f64>::zeros((n, d));
    for i in 0..n {
        for j in 0..d {
            let u = f64::from(chacha.next_u32()) * u32_norm;
            x[[i, j]] = normal.quantile(u);
        }
    }
    *rng = RngState::snapshot(&chacha, rng);
    x
}

fn pearson(x: &Array2<f64>, i: usize, j: usize) -> f64 {
    let n = x.nrows() as f64;
    let mean_i: f64 = (0..x.nrows()).map(|k| x[[k, i]]).sum::<f64>() / n;
    let mean_j: f64 = (0..x.nrows()).map(|k| x[[k, j]]).sum::<f64>() / n;
    let mut num = 0.0;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    for k in 0..x.nrows() {
        let dx = x[[k, i]] - mean_i;
        let dy = x[[k, j]] - mean_j;
        num += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    num / (sxx * syy).sqrt()
}

// ── Marginal preservation under the IC procedure ───────────────────

#[test]
fn ic_preserves_each_input_marginal_value_set() {
    let n = 1024;
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let x = standard_normal_samples(n, 3, &mut rng);

    let r = array![[1.0, 0.5, 0.3], [0.5, 1.0, 0.2], [0.3, 0.2, 1.0]];
    let mut rng2 = RngState::from_seed([1; 32]);
    let out = iman_conover_transform(&x, &r, &mut rng2).unwrap();

    for j in 0..3 {
        let mut input_col: Vec<f64> = (0..n).map(|i| x[[i, j]]).collect();
        let mut output_col: Vec<f64> = (0..n).map(|i| out[[i, j]]).collect();
        input_col.sort_by(|a, b| a.partial_cmp(b).unwrap());
        output_col.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for k in 0..n {
            assert!((input_col[k] - output_col[k]).abs() < 1e-12);
        }
    }
}

// ── Pearson correlation recovery on Gaussian marginals ─────────────

#[test]
fn ic_recovers_target_pearson_correlation_on_gaussian_marginals() {
    let n = 4096;
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let x = standard_normal_samples(n, 3, &mut rng);

    let target = 0.6;
    let r = array![[1.0, target, 0.0], [target, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mut rng2 = RngState::from_seed([2; 32]);
    let out = iman_conover_transform(&x, &r, &mut rng2).unwrap();

    let realized = pearson(&out, 0, 1);
    assert!(
        (realized - target).abs() < 0.05,
        "realized ρ = {realized:.3}, target = {target}"
    );
}

// ── Engineering pay-off: dependent-input Sobol' via IC ─────────────

#[test]
fn ic_transformed_sobol_recovers_correlated_first_order_indices() {
    // Y = X_0 + X_1 + X_2, X_i ~ N(0, 1), ρ_01 = 0.6.
    // Closed form (Mara 2015 § 4.1 + factor-symmetry):
    //   Var(Y) = 4.2
    //   S_0 = S_1 = (1.6)² / 4.2 ≈ 0.610  (counts the correlated lift)
    //   S_2 = 1 / 4.2 ≈ 0.238
    //   Σ S_i ≈ 1.458 (> 1 under correlation, by design).
    let n = 8192;
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let x_indep = standard_normal_samples(n, 3, &mut rng);
    let r = array![[1.0, 0.6, 0.0], [0.6, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mut rng2 = RngState::from_seed([3; 32]);
    let x = iman_conover_transform(&x_indep, &r, &mut rng2).unwrap();
    let y: Vec<f64> = (0..n).map(|k| x[[k, 0]] + x[[k, 1]] + x[[k, 2]]).collect();

    let result = estimate_given_data_sobol(&x, &y).expect("Sobol fit");

    // Tolerance: partition-based given-data Sobol' has bias from
    // the K-class smoothing; at N=8192 with K=class_count(8192)=22
    // classes, realized accuracy is ~0.05 absolute on this problem.
    const TOL: f64 = 0.10;
    let s = &result.s1;
    assert!(
        (s[0] - 0.610).abs() < TOL,
        "S_0 = {:.3}, want 0.610 (correlated pair, full Sobol' index)",
        s[0]
    );
    assert!(
        (s[1] - 0.610).abs() < TOL,
        "S_1 = {:.3}, want 0.610 (correlated pair, full Sobol' index)",
        s[1]
    );
    assert!(
        (s[2] - 0.238).abs() < TOL,
        "S_2 = {:.3}, want 0.238 (uncorrelated factor)",
        s[2]
    );
    let sum: f64 = s.iter().sum();
    // Σ S_i > 1 under correlation — the Sobol' decomposition's
    // sum-to-one identity holds only under independence (Song 2016
    // Theorem 2). At correlation 0.6 between two of three factors,
    // Σ S_i ≈ 1.46 in the population limit.
    assert!(
        sum > 1.0,
        "Σ S_i = {sum:.3} should exceed 1 under correlation (Song Theorem 2)"
    );
}

#[test]
fn independent_input_sobol_biases_correlated_first_order_indices() {
    // Same model, but feed *un*-correlated samples through the
    // estimator. Confirms the bias the IC procedure is correcting:
    // independent S_i ≈ 1/3 = 0.333 each, far from the correlated
    // analytic (S_0, S_1) = 0.610, S_2 = 0.238.
    let n = 8192;
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let x = standard_normal_samples(n, 3, &mut rng);
    let y: Vec<f64> = (0..n).map(|k| x[[k, 0]] + x[[k, 1]] + x[[k, 2]]).collect();

    let result = estimate_given_data_sobol(&x, &y).expect("Sobol fit");
    let s = &result.s1;

    // Independence Sobol' recovers ~1/3 each (loose tolerance:
    // partition smoothing).
    for i in 0..3 {
        assert!(
            (s[i] - 1.0 / 3.0).abs() < 0.10,
            "without IC: S_{i} = {:.3}, expected ≈ 1/3 = 0.333",
            s[i]
        );
    }

    // Σ S_i ≈ 1 under independence — that's the *signal* that the
    // sample doesn't have the correlated structure baked in.
    let sum: f64 = s.iter().sum();
    assert!(
        (sum - 1.0).abs() < 0.10,
        "without IC: Σ S_i = {sum:.3}, expected ≈ 1 under independence"
    );
}

// ── Determinism through the e2e pipeline ────────────────────────────

#[test]
fn ic_e2e_pipeline_is_deterministic() {
    let n = 1024;
    let mut rng_a = RngState::from_seed(FIXTURE_SEED);
    let x_indep_a = standard_normal_samples(n, 3, &mut rng_a);
    let r = array![[1.0, 0.4, 0.2], [0.4, 1.0, 0.1], [0.2, 0.1, 1.0]];
    let mut ic_rng_a = RngState::from_seed([99; 32]);
    let xa = iman_conover_transform(&x_indep_a, &r, &mut ic_rng_a).unwrap();

    let mut rng_b = RngState::from_seed(FIXTURE_SEED);
    let x_indep_b = standard_normal_samples(n, 3, &mut rng_b);
    let mut ic_rng_b = RngState::from_seed([99; 32]);
    let xb = iman_conover_transform(&x_indep_b, &r, &mut ic_rng_b).unwrap();

    for i in 0..n {
        for j in 0..3 {
            assert_eq!(xa[[i, j]], xb[[i, j]]);
        }
    }
}
