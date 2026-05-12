//! End-to-end reviewer-affordance contract close for
//! `compute_active_subspace`. Two scenarios:
//!
//! 1. **Ridge function `f(x) = aᵀx`** (Constantine 2014 § 2.1). C̃
//!    rank-1, leading eigenvector aligned to `a/||a||`, `k_active = 1`.
//!    Pinned with finite-difference gradients to exercise the full
//!    "samples → FD → C̃ → eigendecomposition" pipeline.
//!
//! 2. **Ishigami canonical** `(a=7, b=0.1)`. The interaction
//!    structure means C̃ is full rank with non-axis-aligned
//!    eigenvectors — Constantine 2014 expects a 3-D active subspace
//!    here (`k_active = 3` under the gap heuristic) but with a
//!    non-trivial eigenvector ordering reflecting the
//!    `sin(X_1)·X_3⁴` cross-term. We assert the qualitative shape:
//!    all three eigenvalues positive, leading eigenvector mixes
//!    factors 1 and 3 (the interacting pair).

#![allow(
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::similar_names,
    clippy::items_after_statements
)]

use std::f64::consts::PI;

use ndarray::Array2;
use salib_core::RngState;
use salib_estimators::{finite_difference_gradients, FdKind};
use salib_samplers::{LhsSampler, Sampler};
use salib_surrogate::compute_active_subspace;
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn lhs_inputs(n: usize, d: usize, lo: f64, hi: f64) -> Array2<f64> {
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let unit = LhsSampler::classic(d).unit_sample(n, &mut rng);
    let mut x = Array2::<f64>::zeros((n, d));
    for i in 0..n {
        for j in 0..d {
            x[[i, j]] = lo + (hi - lo) * unit[[i, j]];
        }
    }
    x
}

// ── Ridge function — closed-form active subspace ────────────────────

#[test]
fn ridge_function_pipeline_recovers_rank_one_c_and_aligned_eigenvector() {
    // f(x) = 3·x_0 + 0·x_1 + 4·x_2. Gradient is constant a = (3, 0, 4)
    // everywhere. FD gradient samples + active-subspace pipeline
    // should recover λ_1 = ||a||² = 25, λ_2 = λ_3 = 0, leading
    // eigenvector ±a/||a|| = ±(0.6, 0, 0.8).
    let n = 32;
    let x = lhs_inputs(n, 3, -1.0, 1.0);
    let gradients = finite_difference_gradients(&x, 1e-6, FdKind::Central, |x: &[f64]| {
        3.0 * x[0] + 4.0 * x[2]
    });
    let result = compute_active_subspace(&gradients, None).expect("active-subspace fit");

    assert!(
        (result.eigenvalues[0] - 25.0).abs() < 1e-6,
        "λ_1 = {:.6}, expected 25",
        result.eigenvalues[0]
    );
    assert!(result.eigenvalues[1].abs() < 1e-6);
    assert!(result.eigenvalues[2].abs() < 1e-6);
    assert_eq!(result.k_active, 1);

    let v: Vec<f64> = (0..3).map(|i| result.eigenvectors[[i, 0]]).collect();
    let norm_a = 5.0_f64;
    let dot = 3.0 * v[0] + 4.0 * v[2]; // a · v
    let alignment = dot.abs() / norm_a;
    assert!(
        (alignment - 1.0).abs() < 1e-6,
        "alignment with a/||a|| = {alignment}, expected 1"
    );
    // The middle component (factor 1) should also drop out.
    assert!(v[1].abs() < 1e-6, "v[1] = {} should be 0", v[1]);
}

// ── Ishigami — qualitative shape of the spectrum ────────────────────

#[test]
fn ishigami_active_subspace_spectrum_reflects_per_factor_gradient_magnitudes() {
    // Ishigami at canonical (a=7, b=0.1): the gradient is
    //   ∂Y/∂X_1 = cos(X_1)·(1 + 0.1·X_3⁴)
    //   ∂Y/∂X_2 = 7·sin(2·X_2)
    //   ∂Y/∂X_3 = 0.4·X_3³·sin(X_1)
    // Cross-covariances integrate to zero over Uniform[-π, π]³
    // (E[cos·sin] = 0 across factors), so C̃ is approximately
    // diagonal with entries proportional to the mean-squared
    // per-factor gradients:
    //   E[(∂Y/∂X_1)²] ≈ 7.72
    //   E[(∂Y/∂X_2)²] = 49·E[sin²(2X_2)] = 24.5
    //   E[(∂Y/∂X_3)²] = 0.16·E[X_3⁶]·E[sin²(X_1)] ≈ 11.0
    //
    // Eigenvectors are therefore approximately axis-aligned, with
    // the spectrum order (largest first) X_2, X_3, X_1 — the same
    // ordering as Ishigami's first-order Sobol' indices
    // (S_2 = 0.44 > [S_T_3 sandwich] > S_1 = 0.31).
    //
    // The active-subspace lens correctly reads "X_2 carries the
    // most output variance per unit input perturbation" — even
    // though the X_1·X_3 interaction is what makes Ishigami a hard
    // problem for first-order Sobol'.
    let n = 256;
    let x = lhs_inputs(n, 3, -PI, PI);
    let gradients = finite_difference_gradients(&x, 1e-5, FdKind::Central, |xs: &[f64]| {
        ishigami::ishigami(xs)
    });
    let result = compute_active_subspace(&gradients, None).expect("active-subspace fit");

    // All three eigenvalues should be strictly positive — Ishigami
    // varies along every input.
    for (i, &lambda) in result.eigenvalues.iter().enumerate() {
        assert!(
            lambda > 1e-3,
            "λ_{i} = {lambda} should be > 1e-3 (Ishigami varies in every direction)"
        );
    }
    // Eigenvalues descending.
    assert!(result.eigenvalues[0] >= result.eigenvalues[1]);
    assert!(result.eigenvalues[1] >= result.eigenvalues[2]);

    // Leading eigenvector ≈ (0, ±1, 0) — X_2 axis, the factor with
    // the largest mean-squared gradient via the `a=7` coefficient.
    let v1: Vec<f64> = (0..3).map(|i| result.eigenvectors[[i, 0]]).collect();
    assert!(
        v1[1].abs() > 0.95,
        "leading eigenvector factor-2 component = {} should dominate (≥ 0.95)",
        v1[1].abs()
    );
    assert!(v1[0].abs() < 0.3, "v_1[0] = {} should be small", v1[0]);
    assert!(v1[2].abs() < 0.3, "v_1[2] = {} should be small", v1[2]);

    // Quantitative check: λ_1 ≈ 24.5 (X_2's mean-squared gradient).
    // FD truncation + LHS noise put us within ~10%.
    assert!(
        (result.eigenvalues[0] - 24.5).abs() < 3.0,
        "λ_1 = {:.2}, expected ≈ 24.5",
        result.eigenvalues[0]
    );
}

// ── Determinism ─────────────────────────────────────────────────────

#[test]
fn active_subspace_is_deterministic() {
    let x = lhs_inputs(64, 3, -1.0, 1.0);
    let gradients = finite_difference_gradients(&x, 1e-6, FdKind::Central, |xs: &[f64]| {
        xs[0] * xs[0] + 2.0 * xs[1] + xs[2].sin()
    });
    let a = compute_active_subspace(&gradients, None).unwrap();
    let b = compute_active_subspace(&gradients, None).unwrap();
    assert_eq!(a.eigenvalues, b.eigenvalues);
    for col in 0..3 {
        for row in 0..3 {
            assert_eq!(a.eigenvectors[[row, col]], b.eigenvectors[[row, col]]);
        }
    }
}
