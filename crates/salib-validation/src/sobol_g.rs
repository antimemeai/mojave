//! The Sobol' G function — tunable factor strengths via the
//! parameter vector `a`. Standard ranking-and-apportionment test
//! function in the GSA literature.
//!
//! ```text
//! g(x; a) = Π_{i=1..d}  (|4 x_i - 2| + a_i) / (1 + a_i)
//! ```
//!
//! With `x_i ~ Uniform(0, 1)` independently. Parameters:
//! - `a_i = 0`: factor is "important" (high contribution to V).
//! - `a_i large` (e.g. 99): factor is "unimportant" (low contribution).
//! - Canonical screening cases: `a = (0, 1, 4.5, 9, 99, 99, 99, 99)`
//!   from Saltelli-Sobol 1995.
//!
//! # Closed-form first-order indices
//!
//! Per Saltelli-Sobol 1995:
//!
//! ```text
//! E[g] = 1
//! V_i  = (1/3) / (1 + a_i)²            (per-factor variance contribution)
//! D    = Π_i (1 + V_i) - 1              (total variance, product form)
//! S_i  = V_i / D                        (first-order Sobol' index)
//! ```
//!
//! Total-order indices have a recursive product form (Saltelli-Sobol
//! 1995, Eqs 22-24); deferred to a follow-on PR. Per
//! `decisions/2026-04-28-salib-validation-pattern.md` § "What this
//! gates — NOT gated."
//!
//! # Why the product form for D
//!
//! Each factor `g_i(x_i) = (|4 x_i - 2| + a_i) / (1 + a_i)` has
//! `E[g_i] = 1` and `Var[g_i] = V_i = 1/3 / (1 + a_i)²`. By
//! independence and the product-of-mean-1-factors identity:
//!
//! ```text
//! Var(Π g_i) = Π E[g_i²] - (Π E[g_i])²
//!            = Π (1 + V_i) - 1
//! ```
//!
//! Note `E[g_i²] = Var[g_i] + E[g_i]² = V_i + 1` (since `E[g_i] = 1`).

use salib_core::{Distribution, Problem, ProblemBuilder};

use crate::analytic::SobolIndicesAnalytic;

/// Sobol' G function with parameter vector `a`.
///
/// `a.len()` defines the dimension. Each factor scales as
/// `(|4 x_i - 2| + a_i) / (1 + a_i)` for `x_i ∈ [0, 1]`.
///
/// # Panics
///
/// On `x.len() != a.len()`, or any `a_i < 0` (which would invert
/// the per-factor scaling and is not the standard formulation).
#[must_use]
pub fn sobol_g(x: &[f64], a: &[f64]) -> f64 {
    assert_eq!(x.len(), a.len(), "Sobol' G: x and a must have equal length");
    let mut prod = 1.0_f64;
    for (xi, ai) in x.iter().zip(a.iter()) {
        assert!(*ai >= 0.0, "Sobol' G: a_i must be ≥ 0, got {ai}");
        let factor = ((4.0 * xi - 2.0).abs() + ai) / (1.0 + ai);
        prod *= factor;
    }
    prod
}

/// Closed-form first-order Sobol' indices for Sobol' G with the
/// given `a` vector. Total-order indices are not yet computed; the
/// `total_order` field is populated with `f64::NAN` per factor as
/// a sentinel for "not derived in this PR."
///
/// # Panics
///
/// On any `a_i < 0`.
#[must_use]
pub fn analytic_indices(a: &[f64]) -> SobolIndicesAnalytic {
    let d = a.len();
    assert!(d > 0, "Sobol' G: a must have at least one element");
    for ai in a {
        assert!(*ai >= 0.0, "Sobol' G: a_i must be ≥ 0, got {ai}");
    }

    // V_i = (1/3) / (1 + a_i)².
    let v: Vec<f64> = a
        .iter()
        .map(|ai| {
            let denom = 1.0 + ai;
            (1.0 / 3.0) / (denom * denom)
        })
        .collect();

    // Total variance D = Π_i (1 + V_i) - 1.
    let total_variance: f64 = v.iter().fold(1.0_f64, |acc, vi| acc * (1.0 + vi)) - 1.0;

    // First-order S_i = V_i / D.
    let first_order: Vec<f64> = v.iter().map(|vi| vi / total_variance).collect();

    // Total-order: NOT computed in PR 4. Sentinel NaN.
    let total_order: Vec<f64> = vec![f64::NAN; d];

    SobolIndicesAnalytic::new(total_variance, first_order, total_order, None)
}

/// `Problem` of `dim` factors, each `Uniform(0, 1)`. Factor names are
/// `"x1"`, `"x2"`, …, `"x{dim}"`.
///
/// # Panics
///
/// On `dim == 0`. The Sobol' G function is undefined at zero
/// dimensions.
#[must_use]
pub fn input_distribution(dim: usize) -> Problem {
    assert!(dim > 0, "Sobol' G: dim must be ≥ 1");
    let mut builder = ProblemBuilder::new();
    for i in 1..=dim {
        let name = format!("x{i}");
        builder = builder.factor(&name, Distribution::Uniform { lo: 0.0, hi: 1.0 });
    }
    #[allow(clippy::expect_used)]
    builder
        .build()
        .expect("Sobol' G's canonical Uniform(0, 1) factors are valid")
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use super::*;

    fn assert_close(got: f64, want: f64, tol: f64, ctx: &str) {
        assert!(
            (got - want).abs() <= tol,
            "{ctx}: got {got}, want {want}, |Δ|={}",
            (got - want).abs()
        );
    }

    // ── Function values at known inputs ──────────────────────────────

    #[test]
    fn sobol_g_at_x_equals_half_each_factor_is_a_over_one_plus_a() {
        // |4·0.5 - 2| = 0, so each factor = a_i / (1 + a_i).
        let a = vec![1.0, 4.0, 9.0];
        let x = vec![0.5; 3];
        let want = a.iter().map(|ai| ai / (1.0 + ai)).product::<f64>();
        assert_close(sobol_g(&x, &a), want, 1e-12, "x=½");
    }

    #[test]
    fn sobol_g_at_x_equals_zero_each_factor_is_two_plus_a_over_one_plus_a() {
        // |4·0 - 2| = 2, so each factor = (2 + a_i) / (1 + a_i).
        let a = vec![0.0, 1.0, 9.0];
        let x = vec![0.0; 3];
        let want = a.iter().map(|ai| (2.0 + ai) / (1.0 + ai)).product::<f64>();
        assert_close(sobol_g(&x, &a), want, 1e-12, "x=0");
    }

    #[test]
    fn sobol_g_at_x_equals_one_each_factor_is_two_plus_a_over_one_plus_a() {
        // |4·1 - 2| = 2; same as x=0.
        let a = vec![0.0, 1.0, 9.0];
        let x = vec![1.0; 3];
        let want = a.iter().map(|ai| (2.0 + ai) / (1.0 + ai)).product::<f64>();
        assert_close(sobol_g(&x, &a), want, 1e-12, "x=1");
    }

    #[test]
    fn sobol_g_with_zero_a_at_x_equals_half_is_zero() {
        // a_i = 0 ⇒ factor = |4·0.5 - 2| / 1 = 0. Product = 0.
        let a = vec![0.0; 4];
        let x = vec![0.5; 4];
        assert_eq!(sobol_g(&x, &a), 0.0);
    }

    #[test]
    fn sobol_g_with_large_a_factor_is_close_to_one() {
        // a_i = 99 ⇒ factor ≈ 1 + (small variation around mean 1).
        let a = vec![99.0];
        let g_low = sobol_g(&[0.0], &a);
        let g_mid = sobol_g(&[0.5], &a);
        let g_high = sobol_g(&[1.0], &a);
        // Range of factor: [a/(1+a), (2+a)/(1+a)] = [99/100, 101/100].
        assert!((g_low - 1.01).abs() < 1e-12);
        assert!((g_mid - 0.99).abs() < 1e-12);
        assert!((g_high - 1.01).abs() < 1e-12);
    }

    #[test]
    fn sobol_g_one_dim_evaluates_correctly() {
        // d=1, a=[0]: g(x; [0]) = |4x - 2|.
        let a = vec![0.0];
        for (x, want) in [
            (0.0_f64, 2.0),
            (0.25, 1.0),
            (0.5, 0.0),
            (0.75, 1.0),
            (1.0, 2.0),
        ] {
            assert_close(sobol_g(&[x], &a), want, 1e-12, &format!("x={x}"));
        }
    }

    #[test]
    fn sobol_g_separability_via_independence() {
        // g(x; a) = Π g_i(x_i; a_i). Verify by computing each factor
        // independently and multiplying.
        let a = vec![0.0, 1.0, 4.5, 9.0];
        let x = vec![0.1_f64, 0.3, 0.7, 0.9];
        let mut prod = 1.0_f64;
        for (xi, ai) in x.iter().zip(a.iter()) {
            prod *= (xi.mul_add(4.0, -2.0).abs() + ai) / (1.0 + ai);
        }
        assert_close(sobol_g(&x, &a), prod, 1e-12, "separability");
    }

    #[test]
    #[should_panic(expected = "x and a must have equal length")]
    fn sobol_g_length_mismatch_panics() {
        let _ = sobol_g(&[0.5, 0.5], &[1.0]);
    }

    #[test]
    #[should_panic(expected = "a_i must be ≥ 0")]
    fn sobol_g_negative_a_panics() {
        let _ = sobol_g(&[0.5], &[-0.1]);
    }

    // ── Analytic indices ─────────────────────────────────────────────

    #[test]
    fn analytic_indices_dim_matches_a_length() {
        let s = analytic_indices(&[0.0, 1.0, 9.0, 99.0]);
        assert_eq!(s.dim(), 4);
        assert_eq!(s.first_order.len(), 4);
        assert_eq!(s.total_order.len(), 4);
    }

    #[test]
    fn analytic_indices_total_order_is_nan_sentinel() {
        // PR 4 doesn't compute total-order; sentinel is NaN.
        let s = analytic_indices(&[1.0, 2.0]);
        for v in &s.total_order {
            assert!(v.is_nan(), "expected NaN sentinel, got {v}");
        }
    }

    #[test]
    fn analytic_v_i_formula_matches_one_third_over_one_plus_a_squared() {
        // Per-factor variance V_i = (1/3) / (1 + a_i)².
        // Recover V_i from S_i: V_i = S_i · D.
        let a = vec![0.0, 1.0, 9.0];
        let s = analytic_indices(&a);
        for (i, ai) in a.iter().enumerate() {
            let v_i = s.first_order[i] * s.total_variance;
            let want = (1.0 / 3.0) / (1.0 + ai).powi(2);
            assert_close(v_i, want, 1e-9, &format!("V_{i} for a_i={ai}"));
        }
    }

    #[test]
    fn analytic_total_variance_uses_product_form() {
        // D = Π (1 + V_i) - 1. With a = (0, 1, 9):
        //   V_1 = 1/3, V_2 = 1/12, V_3 = 1/300.
        //   D = (1 + 1/3)(1 + 1/12)(1 + 1/300) - 1.
        let a = vec![0.0, 1.0, 9.0];
        let s = analytic_indices(&a);
        let v1 = 1.0 / 3.0;
        let v2 = 1.0 / 12.0;
        let v3 = 1.0 / 300.0;
        let want = (1.0 + v1) * (1.0 + v2) * (1.0 + v3) - 1.0;
        assert_close(s.total_variance, want, 1e-12, "D product form");
    }

    #[test]
    fn analytic_high_a_factor_has_low_first_order() {
        // a_i = 99 contributes V_i ≈ 1/30000. With d small, S_i ≈ 0.
        let s = analytic_indices(&[0.0, 99.0]);
        // S_2 should be far smaller than S_1.
        assert!(s.first_order[1] < 0.01);
        assert!(s.first_order[0] > 0.5);
    }

    #[test]
    fn analytic_first_order_strictly_decreasing_with_increasing_a() {
        // Single-factor families: smaller a_i ⇒ larger relative
        // contribution. Build a 2-factor problem and verify.
        let a_small = vec![0.0, 1.0];
        let a_big = vec![0.0, 99.0];
        let s_small = analytic_indices(&a_small);
        let s_big = analytic_indices(&a_big);
        // X_1 has the same a_1=0 in both, but X_2's contribution
        // differs; since the total variance changes, S_1 differs.
        // Sanity: S_2 in the "big" case should be much smaller.
        assert!(s_big.first_order[1] < s_small.first_order[1]);
    }

    #[test]
    fn analytic_total_variance_is_positive() {
        let s = analytic_indices(&[0.0, 1.0, 9.0]);
        assert!(s.total_variance > 0.0);
    }

    #[test]
    fn analytic_first_order_sum_at_most_one() {
        let s = analytic_indices(&[0.0, 1.0, 4.5, 9.0, 99.0]);
        let sum: f64 = s.first_order.iter().sum();
        assert!(sum <= 1.0 + 1e-12, "Σ S_i = {sum} > 1");
    }

    #[test]
    fn analytic_first_order_indices_are_positive_for_finite_a() {
        let s = analytic_indices(&[0.0, 1.0, 4.5, 9.0, 99.0]);
        for v in &s.first_order {
            assert!(*v > 0.0, "S_i = {v} should be positive");
        }
    }

    #[test]
    fn analytic_canonical_screening_case_matches_published() {
        // Canonical Saltelli-Sobol 1995 test:
        // a = (0, 1, 4.5, 9, 99, 99, 99, 99).
        let a = vec![0.0, 1.0, 4.5, 9.0, 99.0, 99.0, 99.0, 99.0];
        let s = analytic_indices(&a);
        // First two factors dominate; last four are negligible.
        assert!(s.first_order[0] > s.first_order[1]);
        assert!(s.first_order[1] > s.first_order[2]);
        assert!(s.first_order[2] > s.first_order[3]);
        for i in 4..8 {
            assert!(
                s.first_order[i] < 1e-3,
                "S_{i} should be ~0 for a_i=99, got {}",
                s.first_order[i]
            );
        }
    }

    #[test]
    #[should_panic(expected = "a must have at least one element")]
    fn analytic_empty_a_panics() {
        let _ = analytic_indices(&[]);
    }

    // ── Input distribution ───────────────────────────────────────────

    #[test]
    fn input_distribution_dim_matches() {
        let p = input_distribution(5);
        assert_eq!(p.dim(), 5);
    }

    #[test]
    fn input_distribution_factor_names_are_x1_through_xn() {
        let p = input_distribution(3);
        let names: Vec<&str> = p.factors().iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["x1", "x2", "x3"]);
    }

    #[test]
    fn input_distribution_factors_are_uniform_zero_one() {
        let p = input_distribution(4);
        for f in p.factors() {
            match &f.distribution {
                Distribution::Uniform { lo, hi } => {
                    assert_eq!(*lo, 0.0);
                    assert_eq!(*hi, 1.0);
                }
                other => panic!("expected Uniform, got {other:?}"),
            }
        }
    }

    #[test]
    #[should_panic(expected = "dim must be ≥ 1")]
    fn input_distribution_zero_dim_panics() {
        let _ = input_distribution(0);
    }
}
