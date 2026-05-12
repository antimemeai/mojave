//! The Ishigami function — the GSA literature's most-cited
//! analytic test function. Strong nonlinearity; `X_3` has zero
//! first-order Sobol' index but nonzero total-order — the canary
//! that catches "did you implement total-order correctly?" bugs.
//!
//! ```text
//! Y = sin(X_1) + a · sin²(X_2) + b · X_3⁴ · sin(X_1)
//! ```
//!
//! With `X_i ~ Uniform(-π, π)` independently. Canonical parameters:
//! `a = 7`, `b = 0.1` (Ishigami-Homma 1990; reproduced verbatim in
//! Saltelli et al. 2008 *Global Sensitivity Analysis: The Primer*
//! § 5.4).
//!
//! # The closed-form indices
//!
//! Per Saltelli Primer 2008, Eq 5.16-5.18:
//!
//! ```text
//! D    = ½ + a²/8 + b·π⁴/5 + b²·π⁸/18                  (total variance)
//! V_1  = ½ + b·π⁴/5 + b²·π⁸/50
//! V_2  = a²/8
//! V_3  = 0                                              ← the canary
//! V_13 = 8 · b² · π⁸ / 225                              (X_1–X_3 interaction)
//! V_T1 = V_1 + V_13
//! V_T2 = V_2                                            (X_2 has no interactions)
//! V_T3 = V_13
//! ```
//!
//! Then `S_i = V_i / D` and `S_T_i = V_T_i / D`. For `(a, b) = (7, 0.1)`:
//!
//! ```text
//! D    ≈ 13.844
//! S_1  ≈ 0.3139
//! S_2  ≈ 0.4424
//! S_3  = 0
//! S_T1 ≈ 0.5576
//! S_T2 ≈ 0.4424     (= S_2)
//! S_T3 ≈ 0.2436
//! ```
//!
//! These are the values the reviewer-affordance contract expects
//! every Sobol estimator PR to converge to within MC noise (per
//! `decisions/2026-04-28-saltelli-tck-posture.md` Layer 4).
//!
//! # Derivation
//!
//! `E[sin(X)] = 0`, `E[sin²(X)] = ½`, `E[sin⁴(X)] = ⅜` for `X ~ U(-π, π)`.
//! `E[X⁴] = π⁴/5`, `E[X⁸] = π⁸/9` for `X ~ U(-π, π)`.
//! The conditional-mean computations:
//!
//! - `E[Y | X_1] = (1 + b·π⁴/5) · sin(X_1) + a/2`
//!   ⇒ `V_1 = (1 + b·π⁴/5)²/2 = ½ + b·π⁴/5 + b²·π⁸/50`.
//! - `E[Y | X_2] = a · sin²(X_2)`
//!   ⇒ `V_2 = a² · Var(sin²(X)) = a² · (⅜ - ¼) = a²/8`.
//! - `E[Y | X_3] = a/2`
//!   ⇒ `V_3 = 0` — `X_3` enters only through interaction with `X_1`.
//! - `V_13 = D - V_1 - V_2 - V_3 = b²π⁸·(1/18 - 1/50) = 8 b² π⁸ / 225`.

use salib_core::{Distribution, Problem, ProblemBuilder};

use crate::analytic::SobolIndicesAnalytic;

/// Canonical `a` parameter (Ishigami-Homma 1990).
pub const ISHIGAMI_DEFAULT_A: f64 = 7.0;

/// Canonical `b` parameter (Ishigami-Homma 1990).
pub const ISHIGAMI_DEFAULT_B: f64 = 0.1;

/// Ishigami function with canonical parameters `a = 7`, `b = 0.1`.
///
/// # Panics
///
/// On `x.len() != 3`. The Ishigami function is 3-dimensional by
/// definition; passing a wrong-length slice is a programming error.
#[must_use]
pub fn ishigami(x: &[f64]) -> f64 {
    ishigami_with_params(x, ISHIGAMI_DEFAULT_A, ISHIGAMI_DEFAULT_B)
}

/// Ishigami function with explicit `(a, b)` parameters.
///
/// `Y = sin(x[0]) + a · sin²(x[1]) + b · x[2]⁴ · sin(x[0])`.
///
/// # Panics
///
/// On `x.len() != 3`.
#[must_use]
pub fn ishigami_with_params(x: &[f64], a: f64, b: f64) -> f64 {
    assert_eq!(x.len(), 3, "Ishigami: requires exactly 3 inputs");
    let s1 = x[0].sin();
    s1 + a * x[1].sin().powi(2) + b * x[2].powi(4) * s1
}

/// Closed-form gradient `∇f` of Ishigami at `x` with the given
/// `(a, b)` parameters. Returns `[∂f/∂x_1, ∂f/∂x_2, ∂f/∂x_3]`:
///
/// ```text
/// ∂f/∂x_1 = cos(x_1) · (1 + b · x_3⁴)
/// ∂f/∂x_2 = a · sin(2·x_2)                     (= 2a·sin(x_2)·cos(x_2))
/// ∂f/∂x_3 = 4 · b · x_3³ · sin(x_1)
/// ```
///
/// Useful for DGSM (`salib_estimators::dgsm`) — analytical
/// gradient input avoids finite-difference approximation error.
///
/// # Panics
///
/// On `x.len() != 3`.
#[must_use]
pub fn ishigami_gradient_with_params(x: &[f64], a: f64, b: f64) -> [f64; 3] {
    assert_eq!(x.len(), 3, "Ishigami gradient: requires exactly 3 inputs");
    let cos_x1 = x[0].cos();
    let sin_x1 = x[0].sin();
    let x3_to_4 = x[2].powi(4);
    let x3_to_3 = x[2].powi(3);
    [
        cos_x1 * (1.0 + b * x3_to_4),
        a * (2.0 * x[1]).sin(),
        4.0 * b * x3_to_3 * sin_x1,
    ]
}

/// Closed-form gradient at canonical `(a = 7, b = 0.1)`.
///
/// # Panics
///
/// On `x.len() != 3`.
#[must_use]
pub fn ishigami_gradient(x: &[f64]) -> [f64; 3] {
    ishigami_gradient_with_params(x, ISHIGAMI_DEFAULT_A, ISHIGAMI_DEFAULT_B)
}

/// Closed-form analytic Sobol' indices for Ishigami at the given
/// `(a, b)` parameters. Per Saltelli Primer 2008 Eq 5.16-5.18.
#[must_use]
pub fn analytic_indices(a: f64, b: f64) -> SobolIndicesAnalytic {
    use std::f64::consts::PI;
    let pi4 = PI.powi(4);
    let pi8 = PI.powi(8);

    // Total variance D = ½ + a²/8 + b·π⁴/5 + b²·π⁸/18.
    let total_variance = 0.5 + a * a / 8.0 + b * pi4 / 5.0 + b * b * pi8 / 18.0;

    // First-order variances.
    let v1 = 0.5 + b * pi4 / 5.0 + b * b * pi8 / 50.0;
    let v2 = a * a / 8.0;
    let v3 = 0.0;

    // X_1–X_3 interaction.
    let v13 = 8.0 * b * b * pi8 / 225.0;

    // Total-order variances.
    let vt1 = v1 + v13;
    let vt2 = v2; // X_2 has no interactions
    let vt3 = v13;

    SobolIndicesAnalytic::new(
        total_variance,
        vec![
            v1 / total_variance,
            v2 / total_variance,
            v3 / total_variance,
        ],
        vec![
            vt1 / total_variance,
            vt2 / total_variance,
            vt3 / total_variance,
        ],
    )
}

/// The 3-factor `Uniform(-π, π)` `Problem` matching Ishigami's
/// canonical input space. Factor names are `"x1"`, `"x2"`, `"x3"`.
///
/// # Panics
///
/// Never. Construction goes through `ProblemBuilder::build` with
/// validated `Uniform` parameters.
#[must_use]
pub fn input_distribution() -> Problem {
    use std::f64::consts::PI;
    #[allow(clippy::expect_used)]
    ProblemBuilder::new()
        .factor("x1", Distribution::Uniform { lo: -PI, hi: PI })
        .factor("x2", Distribution::Uniform { lo: -PI, hi: PI })
        .factor("x3", Distribution::Uniform { lo: -PI, hi: PI })
        .build()
        .expect("Ishigami's canonical Uniform(-π, π) factors are valid")
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn assert_close(got: f64, want: f64, tol: f64, ctx: &str) {
        assert!(
            (got - want).abs() <= tol,
            "{ctx}: got {got}, want {want}, |Δ|={}, tol={tol}",
            (got - want).abs()
        );
    }

    // ── Function values at known inputs ──────────────────────────────

    #[test]
    fn ishigami_at_origin_with_default_params() {
        // sin(0) + 7·sin²(0) + 0.1·0⁴·sin(0) = 0
        assert_eq!(ishigami(&[0.0, 0.0, 0.0]), 0.0);
    }

    #[test]
    fn ishigami_x2_only_at_pi_over_two() {
        // sin(0) + 7·sin²(π/2) + 0.1·0⁴·0 = 0 + 7·1 + 0 = 7
        let y = ishigami(&[0.0, PI / 2.0, 0.0]);
        assert_close(y, 7.0, 1e-12, "Ishigami x2=π/2");
    }

    #[test]
    fn ishigami_zero_first_input_zeros_first_and_third_terms() {
        // X_1 = 0 ⇒ sin(X_1) = 0 ⇒ first and third terms vanish.
        // Y = 0 + a·sin²(X_2) + 0 = a/2 + (a/2)·cos(2 X_2) (any X_3 OK).
        let y = ishigami_with_params(&[0.0, PI / 4.0, 99.0], 7.0, 0.1);
        // sin²(π/4) = 1/2, so Y = 7·0.5 = 3.5.
        assert_close(y, 3.5, 1e-12, "Ishigami sin(X1)=0");
    }

    #[test]
    fn ishigami_with_params_decouples_a_and_b() {
        // a=0, b=0: Y = sin(X_1).
        for x1 in [-PI, -PI / 4.0, 0.0, PI / 4.0, PI] {
            assert_close(
                ishigami_with_params(&[x1, 0.5, 1.0], 0.0, 0.0),
                x1.sin(),
                1e-12,
                "a=b=0",
            );
        }
    }

    #[test]
    fn ishigami_with_zero_b_drops_x3_dependence() {
        // b=0 ⇒ Y is independent of X_3.
        let a = 7.0_f64;
        let y_low = ishigami_with_params(&[1.0, 1.0, 0.0], a, 0.0);
        let y_high = ishigami_with_params(&[1.0, 1.0, 100.0], a, 0.0);
        assert_close(y_low, y_high, 1e-12, "b=0 X_3 invariance");
    }

    #[test]
    fn ishigami_default_matches_explicit_canonical_params() {
        let x = [0.7, -1.3, 2.1];
        let default = ishigami(&x);
        let explicit = ishigami_with_params(&x, ISHIGAMI_DEFAULT_A, ISHIGAMI_DEFAULT_B);
        assert_eq!(default, explicit);
    }

    #[test]
    #[should_panic(expected = "requires exactly 3 inputs")]
    fn ishigami_wrong_length_panics() {
        let _ = ishigami(&[1.0, 2.0]);
    }

    // ── Analytic indices ─────────────────────────────────────────────

    #[test]
    fn analytic_indices_returns_three_dimensional() {
        let s = analytic_indices(7.0, 0.1);
        assert_eq!(s.dim(), 3);
        assert_eq!(s.first_order.len(), 3);
        assert_eq!(s.total_order.len(), 3);
    }

    #[test]
    fn analytic_x3_first_order_is_exactly_zero() {
        // The Ishigami canary: V_3 = 0 by closed form.
        let s = analytic_indices(7.0, 0.1);
        assert_eq!(s.first_order[2], 0.0);
    }

    #[test]
    fn analytic_x2_total_equals_x2_first() {
        // X_2 has no interactions, so S_T2 == S_2.
        let s = analytic_indices(7.0, 0.1);
        assert_close(s.total_order[1], s.first_order[1], 1e-12, "S_T2 = S_2");
    }

    #[test]
    fn analytic_x3_total_equals_v13_over_d() {
        // V_3 = 0, so V_T3 = V_13. Numerically: V_13 = 8·b²π⁸/225
        // and D = 1/2 + a²/8 + bπ⁴/5 + b²π⁸/18.
        let a = 7.0_f64;
        let b = 0.1_f64;
        let pi8 = PI.powi(8);
        let pi4 = PI.powi(4);
        let v13 = 8.0 * b * b * pi8 / 225.0;
        let d = 0.5 + a * a / 8.0 + b * pi4 / 5.0 + b * b * pi8 / 18.0;
        let s = analytic_indices(a, b);
        assert_close(s.total_order[2], v13 / d, 1e-12, "S_T3 = V_13/D");
    }

    #[test]
    fn analytic_canonical_first_order_matches_published_values() {
        // Saltelli Primer 2008 §5.4: with a=7, b=0.1, S_1 ≈ 0.3139.
        let s = analytic_indices(7.0, 0.1);
        assert_close(s.first_order[0], 0.3139, 5e-4, "S_1 published");
        assert_close(s.first_order[1], 0.4424, 5e-4, "S_2 published");
        assert_eq!(s.first_order[2], 0.0);
    }

    #[test]
    fn analytic_canonical_total_order_matches_published_values() {
        // Saltelli Primer 2008 §5.4: S_T1 ≈ 0.5576, S_T2 ≈ 0.4424, S_T3 ≈ 0.2436.
        let s = analytic_indices(7.0, 0.1);
        assert_close(s.total_order[0], 0.5576, 5e-4, "S_T1 published");
        assert_close(s.total_order[1], 0.4424, 5e-4, "S_T2 published");
        assert_close(s.total_order[2], 0.2436, 5e-4, "S_T3 published");
    }

    #[test]
    fn analytic_total_variance_is_positive() {
        let s = analytic_indices(7.0, 0.1);
        assert!(s.total_variance > 0.0);
    }

    #[test]
    fn analytic_first_order_sum_at_most_one() {
        let s = analytic_indices(7.0, 0.1);
        let sum: f64 = s.first_order.iter().sum();
        assert!(sum <= 1.0 + 1e-12, "Σ S_i = {sum} > 1");
    }

    #[test]
    fn analytic_total_bounds_first_per_factor() {
        let s = analytic_indices(7.0, 0.1);
        for i in 0..3 {
            assert!(
                s.total_order[i] >= s.first_order[i] - 1e-12,
                "S_T_{i} ({}) < S_{i} ({})",
                s.total_order[i],
                s.first_order[i]
            );
        }
    }

    #[test]
    fn analytic_indices_are_non_negative() {
        let s = analytic_indices(7.0, 0.1);
        for v in s.first_order.iter().chain(s.total_order.iter()) {
            assert!(*v >= 0.0, "negative index {v}");
        }
    }

    #[test]
    fn analytic_zero_a_gives_zero_x2_first_order() {
        // a=0 ⇒ V_2 = 0 ⇒ S_2 = 0.
        let s = analytic_indices(0.0, 0.1);
        assert_eq!(s.first_order[1], 0.0);
    }

    #[test]
    fn analytic_zero_b_gives_zero_x3_total_order() {
        // b=0 ⇒ V_13 = 0 ⇒ V_T3 = 0 ⇒ S_T3 = 0.
        let s = analytic_indices(7.0, 0.0);
        assert_eq!(s.total_order[2], 0.0);
    }

    #[test]
    fn analytic_zero_a_b_gives_pure_x1_dependence() {
        // a=b=0 ⇒ Y = sin(X_1) ⇒ S_1 = 1, S_2 = S_3 = 0.
        let s = analytic_indices(0.0, 0.0);
        assert_close(s.first_order[0], 1.0, 1e-12, "S_1 with a=b=0");
        assert_eq!(s.first_order[1], 0.0);
        assert_eq!(s.first_order[2], 0.0);
        assert_close(s.total_order[0], 1.0, 1e-12, "S_T1 with a=b=0");
    }

    // ── Input distribution ───────────────────────────────────────────

    #[test]
    fn input_distribution_has_three_factors() {
        let p = input_distribution();
        assert_eq!(p.dim(), 3);
    }

    #[test]
    fn input_distribution_factor_names_are_x1_x2_x3() {
        let p = input_distribution();
        let names: Vec<&str> = p.factors().iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["x1", "x2", "x3"]);
    }

    #[test]
    fn input_distribution_factors_are_uniform_minus_pi_to_pi() {
        let p = input_distribution();
        for f in p.factors() {
            match &f.distribution {
                Distribution::Uniform { lo, hi } => {
                    assert_close(*lo, -PI, 1e-12, "lo");
                    assert_close(*hi, PI, 1e-12, "hi");
                }
                other => panic!("expected Uniform, got {other:?}"),
            }
        }
    }
}
