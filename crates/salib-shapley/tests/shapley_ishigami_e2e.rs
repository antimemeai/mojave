//! End-to-end reviewer-affordance contract close for `estimate_shapley`
//! on Ishigami canonical `(a=7, b=0.1)`.
//!
//! Closed-form Shapley values under independence — derived from the
//! analytic Sobol' decomposition `V_1 = 4.345, V_2 = 6.124, V_3 = 0,
//! V_13 = 3.373` (the only non-trivial interaction):
//!
//! ```text
//! Sh_1 = V_1 + ½·V_13 ≈ 6.032
//! Sh_2 = V_2          ≈ 6.124   (X_2 enters no interaction)
//! Sh_3 = ½·V_13       ≈ 1.687
//! ```
//!
//! Sum = 13.844 = Var(Y). Under Song 2016 Theorem 2 (independence),
//! `V_i ≤ Sh_i ≤ V_T_i`:
//!
//! ```text
//! 4.345 ≤ Sh_1 ≤ 7.720
//! 6.124 = Sh_2 = 6.124   (V_2 = V_T_2)
//! 0     ≤ Sh_3 ≤ 3.373
//! ```

#![allow(
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::expect_used,
    clippy::similar_names,
    clippy::items_after_statements
)]

use std::f64::consts::PI;

use salib_core::{Distribution, RngState};
use salib_shapley::estimate_shapley;
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn ishigami_distributions() -> Vec<Distribution> {
    (0..3)
        .map(|_| Distribution::Uniform { lo: -PI, hi: PI })
        .collect()
}

fn run_at_budget(n_perm: usize, n_var: usize) -> salib_shapley::ShapleyIndices {
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    estimate_shapley(
        &ishigami_distributions(),
        |x: &[f64]| ishigami::ishigami(x),
        n_perm,
        1, // n_outer (Song 2016 Appendix B recommendation)
        3, // n_inner (Song 2016 Appendix B recommendation)
        n_var,
        &mut rng,
    )
    .expect("shapley fit")
}

// ── Var(Y) recovery ─────────────────────────────────────────────────

#[test]
fn ishigami_var_y_recovers_analytic_within_mc_tolerance() {
    let result = run_at_budget(4000, 8000);
    let analytic_var = ishigami::analytic_indices(7.0, 0.1).total_variance;
    // Var(Y) ≈ 13.844; MC noise at N_V = 8000 is ~3% relative.
    assert!(
        (result.var_y - analytic_var).abs() < 0.7,
        "Var(Y) = {:.3}, analytic = {:.3}",
        result.var_y,
        analytic_var
    );
}

// ── Shapley closed form ─────────────────────────────────────────────

#[test]
fn ishigami_shapley_recovers_closed_form_within_mc_tolerance() {
    let result = run_at_budget(4000, 8000);
    let analytic = ishigami::analytic_indices(7.0, 0.1);
    // V_1 = S_1·D, V_T_1 = S_T_1·D, V_13 = (S_T_1 - S_1)·D.
    let d = analytic.total_variance;
    let v_1 = analytic.first_order[0] * d;
    let v_2 = analytic.first_order[1] * d;
    let v_t1 = analytic.total_order[0] * d;
    let v_13 = v_t1 - v_1;

    let expected = [
        v_1 + 0.5 * v_13, // Sh_1 ≈ 6.032
        v_2,              // Sh_2 ≈ 6.124
        0.5 * v_13,       // Sh_3 ≈ 1.687
    ];

    // MC tolerance at m = 4000, N_O = 1, N_I = 3 is ~10% relative
    // for Ishigami (large variance, moderate budget). Use absolute
    // 1.0 — comfortable headroom over realized error.
    const TOL: f64 = 1.0;
    for (i, &want) in expected.iter().enumerate() {
        assert!(
            (result.sh[i] - want).abs() < TOL,
            "Sh_{i}: got {:.3}, want {:.3} (closed-form ±{TOL})",
            result.sh[i],
            want
        );
    }
}

// ── Theorem 2 sandwich (independence) ───────────────────────────────

#[test]
fn ishigami_shapley_sandwiches_first_and_total_order() {
    // Song 2016 Theorem 2 (independence): V_i ≤ Sh_i ≤ V_T_i.
    let result = run_at_budget(4000, 8000);
    let analytic = ishigami::analytic_indices(7.0, 0.1);
    let d = analytic.total_variance;
    for i in 0..3 {
        let v_i = analytic.first_order[i] * d;
        let v_t_i = analytic.total_order[i] * d;
        // MC slack ~10% on each side.
        let slack = 0.5;
        assert!(
            result.sh[i] >= v_i - slack,
            "Sh_{i} = {:.3} should be ≥ V_{i} = {v_i:.3} (Theorem 2 lower)",
            result.sh[i]
        );
        assert!(
            result.sh[i] <= v_t_i + slack,
            "Sh_{i} = {:.3} should be ≤ V_T_{i} = {v_t_i:.3} (Theorem 2 upper)",
            result.sh[i]
        );
    }
}

// ── Eq 10 — Σ Sh_i = Var(Y) ─────────────────────────────────────────

#[test]
fn shapley_indices_sum_to_var_y() {
    let result = run_at_budget(4000, 8000);
    let sum: f64 = result.sh.iter().sum();
    // Per-permutation telescoping is exact: Σ_j Δ_j = c(K) - c(∅) =
    // Var(Y) - 0. The deviation is purely MC noise on the cost
    // estimates within each permutation.
    assert!(
        (sum - result.var_y).abs() < 0.5,
        "Σ Sh_i = {sum:.3}, Var(Y) = {:.3} (Song Eq 10)",
        result.var_y
    );
}

// ── Determinism ─────────────────────────────────────────────────────

#[test]
fn ishigami_shapley_is_deterministic() {
    let a = run_at_budget(64, 256);
    let b = run_at_budget(64, 256);
    assert_eq!(a.sh, b.sh);
    assert_eq!(a.var_y, b.var_y);
}
