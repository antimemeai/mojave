//! Morris additive-linear test function — clean closed-form Morris
//! elementary effects.
//!
//! ```text
//! Y(x) = Σ_{i=1..d} i · x_{i-1}              for x ∈ [0, 1]^d
//! ```
//!
//! Factor `i` (1-indexed; index `i-1` in the slice) has coefficient
//! `i`. Per Morris's elementary-effects derivation:
//!
//! - `EE_i = (Y(x + Δ·e_i) - Y(x)) / Δ = i` (constant, independent
//!   of `x`).
//! - `μ_i = mean over R of EE_i = i`.
//! - `μ*_i = mean over R of |EE_i| = i`.
//! - `σ_i = 0` (purely linear; no variation in elementary effects).
//!
//! For `d = 8` (the canonical Morris-test in PR 8): `μ = μ* = [1,
//! 2, 3, 4, 5, 6, 7, 8]`, `σ = [0; 8]`.
//!
//! # Why additive-linear, not Morris 1991 §4
//!
//! Morris 1991 §4 specifies a 20-factor function with mostly fixed
//! and a few `N(0, 1)`-sampled β coefficients. Closed-form `μ` and
//! `σ` for the canonical version require fixing the random β's at a
//! specific seed — well-defined but seed-dependent and complex to
//! cross-reference. Per
//! `decisions/2026-04-29-saltelli-morris-estimator.md` § "Scope
//! refinement," the Morris 1991 §4 function is deferred to a follow-
//! on PR.
//!
//! Additive-linear is sufficient for PR 8's reviewer-affordance
//! contract close: `μ_i = i` is bit-exactly recoverable (modulo FP
//! arithmetic on Δ); ranking is trivially correct; `σ_i = 0`
//! provides a clean noise-floor baseline.
//!
//! # Special-case identities
//!
//! - **Linear** ⇒ `EE_i` constant ⇒ `σ = 0` always.
//! - **Per-factor coefficient ranking** = `μ*_i` ranking. With
//!   `b = [1, 2, 3, …, d]`, factor 1 has the smallest effect and
//!   factor `d` the largest — Morris should recover this ordering
//!   exactly.

use salib_core::{Distribution, Problem, ProblemBuilder};

use crate::analytic::MorrisEffectsAnalytic;

/// Default factor count for the canonical Morris-test in PR 8.
pub const MORRIS_TEST_DEFAULT_DIM: usize = 8;

/// Morris additive-linear test function with default `d = 8`
/// factors.
///
/// `Y(x) = Σ_{i=1..d} i · x_{i-1}` for `x ∈ [0, 1]^d`.
///
/// # Panics
///
/// On `x.len() != MORRIS_TEST_DEFAULT_DIM`. The default canonical
/// is 8-dimensional; use [`morris_additive_linear_with_dim`] for
/// other dimensions.
#[must_use]
pub fn morris_additive_linear(x: &[f64]) -> f64 {
    morris_additive_linear_with_dim(x, MORRIS_TEST_DEFAULT_DIM)
}

/// Morris additive-linear test function with explicit dimension.
///
/// # Panics
///
/// On `x.len() != d`.
#[must_use]
pub fn morris_additive_linear_with_dim(x: &[f64], d: usize) -> f64 {
    assert_eq!(x.len(), d, "Morris additive-linear: expected dim {d}");
    let mut sum = 0.0;
    for (i, &xi) in x.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let coeff = (i + 1) as f64;
        sum += coeff * xi;
    }
    sum
}

/// Closed-form Morris elementary effects for the additive-linear
/// test function with `d` factors. `μ_i = μ*_i = i` (1-indexed),
/// `σ_i = 0`.
#[must_use]
pub fn analytic_effects(d: usize) -> MorrisEffectsAnalytic {
    #[allow(clippy::cast_precision_loss)]
    let mu: Vec<f64> = (0..d).map(|i| (i + 1) as f64).collect();
    let mu_star = mu.clone();
    let sigma = vec![0.0; d];
    MorrisEffectsAnalytic::new(mu, mu_star, sigma)
}

// ── Quadratic-additive: non-linear, σ > 0, contract-substantive ─────

/// Morris quadratic-additive test function with default `d = 8`
/// factors.
///
/// ```text
/// Y(x) = Σ_{i=1..d} bᵢ · xᵢ + cᵢ · xᵢ²
/// ```
///
/// where `bᵢ = cᵢ = i` (1-indexed). Compared to additive-linear,
/// this function has:
///
/// - **Real MC noise on EE.** `EE_i(x) = bᵢ + cᵢ·(2·xᵢ + Δ)` varies
///   with the base point `xᵢ`, so different trajectories sample
///   different EE values for the same factor.
/// - **Non-zero σ.** Substantive convergence-rate target.
/// - **Non-trivial `SALib` differential.** Both implementations
///   produce different MC samples that converge to the same μ
///   and σ as R → ∞.
///
/// This is the test function PR 8.6 uses for the Morris reviewer-
/// affordance contract's *non-degenerate* close.
///
/// # Panics
///
/// On `x.len() != MORRIS_TEST_DEFAULT_DIM`.
#[must_use]
pub fn morris_quadratic_additive(x: &[f64]) -> f64 {
    morris_quadratic_additive_with_dim(x, MORRIS_TEST_DEFAULT_DIM)
}

/// Quadratic-additive with explicit dimension.
///
/// # Panics
///
/// On `x.len() != d`.
#[must_use]
pub fn morris_quadratic_additive_with_dim(x: &[f64], d: usize) -> f64 {
    assert_eq!(x.len(), d, "Morris quadratic-additive: expected dim {d}");
    let mut sum = 0.0;
    for (i, &xi) in x.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let coeff = (i + 1) as f64;
        // bᵢ · xᵢ + cᵢ · xᵢ² with bᵢ = cᵢ = i+1.
        sum += coeff * xi + coeff * xi * xi;
    }
    sum
}

/// Closed-form Morris elementary effects for the quadratic-additive
/// test function with `d` factors at Morris-trajectory grid `p = 4`
/// (Δ = 2/3, base ∈ {0, 1/3}).
///
/// Derivation (per factor `i`, 1-indexed):
///
/// ```text
/// EE_i(x) = (Y(x + Δ·eᵢ) - Y(x)) / Δ
///         = bᵢ + cᵢ · (2·xᵢ + Δ)
///
/// At x_i = 0:    EE_i = bᵢ + cᵢ · (0 + Δ)         = bᵢ + 2cᵢ/3
/// At x_i = 1/3:  EE_i = bᵢ + cᵢ · (2·(1/3) + Δ)   = bᵢ + 4cᵢ/3
///
/// Population mean (uniform over the two base levels):
///   μᵢ = (bᵢ + 2cᵢ/3 + bᵢ + 4cᵢ/3) / 2 = bᵢ + cᵢ
///
/// Population std:
///   σᵢ = |cᵢ| / 3
/// ```
///
/// With `bᵢ = cᵢ = i+1`, this gives:
/// - `μᵢ = 2(i+1)`: `[2, 4, 6, 8, 10, 12, 14, 16]` for d=8.
/// - `μ*ᵢ = |μᵢ| = μᵢ` (no sign flips).
/// - `σᵢ = (i+1)/3`: `[0.333, 0.667, 1.0, 1.333, 1.667, 2.0, 2.333, 2.667]`.
///
/// Note: the analytic σ is the **population** std over the two base
/// levels, weighted equally. At finite R, the **sample** std (with
/// Bessel correction `R-1`) deviates by `O(1/√R)` MC noise.
#[must_use]
pub fn analytic_quadratic_effects(d: usize) -> MorrisEffectsAnalytic {
    #[allow(clippy::cast_precision_loss)]
    let mu: Vec<f64> = (0..d).map(|i| 2.0 * (i + 1) as f64).collect();
    let mu_star = mu.clone();
    #[allow(clippy::cast_precision_loss)]
    let sigma: Vec<f64> = (0..d).map(|i| (i + 1) as f64 / 3.0).collect();
    MorrisEffectsAnalytic::new(mu, mu_star, sigma)
}

/// `Problem` of `d` factors, each `Uniform(0, 1)`. Factor names are
/// `"x1"`, `"x2"`, …, `"x{d}"`.
///
/// # Panics
///
/// On `d == 0`.
#[must_use]
pub fn input_distribution(d: usize) -> Problem {
    assert!(d > 0, "Morris-test: dim must be ≥ 1");
    let mut builder = ProblemBuilder::new();
    for i in 1..=d {
        let name = format!("x{i}");
        builder = builder.factor(&name, Distribution::Uniform { lo: 0.0, hi: 1.0 });
    }
    #[allow(clippy::expect_used)]
    builder
        .build()
        .expect("Morris-test's canonical Uniform(0, 1) factors are valid")
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::cast_precision_loss)]
mod tests {
    use super::*;

    // ── Function values at known inputs ──────────────────────────────

    #[test]
    fn at_origin_is_zero() {
        let zeros = vec![0.0; 8];
        assert_eq!(morris_additive_linear(&zeros), 0.0);
    }

    #[test]
    fn at_all_ones_is_sum_of_coefficients() {
        // Σ i for i in 1..=8 = 8·9/2 = 36.
        let ones = vec![1.0; 8];
        assert_eq!(morris_additive_linear(&ones), 36.0);
    }

    #[test]
    fn at_unit_basis_recovers_coefficient() {
        // x_i = e_k → Y = k.
        for k in 0..8 {
            let mut x = vec![0.0; 8];
            x[k] = 1.0;
            #[allow(clippy::cast_precision_loss)]
            let want = (k + 1) as f64;
            assert_eq!(morris_additive_linear(&x), want);
        }
    }

    #[test]
    fn explicit_dim_changes_factor_count() {
        // d=3: Y = x_0 + 2·x_1 + 3·x_2. At x=[1,1,1]: Y=6.
        let x = vec![1.0; 3];
        assert_eq!(morris_additive_linear_with_dim(&x, 3), 6.0);
    }

    #[test]
    #[should_panic(expected = "expected dim")]
    fn wrong_length_panics() {
        let _ = morris_additive_linear(&[1.0, 2.0]);
    }

    // ── Analytic effects ────────────────────────────────────────────

    #[test]
    fn analytic_effects_for_default_dim() {
        let e = analytic_effects(8);
        assert_eq!(e.dim(), 8);
        for i in 0..8 {
            #[allow(clippy::cast_precision_loss)]
            let want = (i + 1) as f64;
            assert_eq!(e.mu[i], want);
            assert_eq!(e.mu_star[i], want);
            assert_eq!(e.sigma[i], 0.0);
        }
    }

    #[test]
    fn analytic_mu_star_at_least_absolute_mu() {
        // Identity check (μ* ≥ |μ|).
        let e = analytic_effects(5);
        for i in 0..5 {
            assert!(e.mu_star[i] >= e.mu[i].abs());
        }
    }

    #[test]
    fn analytic_sigma_is_zero_for_linear() {
        let e = analytic_effects(10);
        for s in &e.sigma {
            assert_eq!(*s, 0.0);
        }
    }

    #[test]
    fn analytic_factors_strictly_ordered_by_coefficient() {
        // μ_i strictly increases with i (b = [1, 2, 3, …]).
        let e = analytic_effects(8);
        for i in 1..8 {
            assert!(e.mu[i] > e.mu[i - 1]);
        }
    }

    // ── Input distribution ──────────────────────────────────────────

    #[test]
    fn input_distribution_has_d_factors() {
        let p = input_distribution(8);
        assert_eq!(p.dim(), 8);
    }

    #[test]
    fn input_distribution_factor_names_are_x1_through_xd() {
        let p = input_distribution(4);
        let names: Vec<&str> = p.factors().iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["x1", "x2", "x3", "x4"]);
    }

    #[test]
    fn input_distribution_factors_are_uniform_zero_one() {
        let p = input_distribution(3);
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

    // ── Quadratic-additive ──────────────────────────────────────────

    #[test]
    fn quadratic_at_origin_is_zero() {
        let zeros = vec![0.0; 8];
        assert_eq!(morris_quadratic_additive(&zeros), 0.0);
    }

    #[test]
    fn quadratic_at_unit_basis_recovers_b_plus_c() {
        // x = e_k → Y = b_k · 1 + c_k · 1 = (k+1) + (k+1) = 2(k+1).
        for k in 0..8 {
            let mut x = vec![0.0; 8];
            x[k] = 1.0;
            #[allow(clippy::cast_precision_loss)]
            let want = 2.0 * (k + 1) as f64;
            assert_eq!(morris_quadratic_additive(&x), want);
        }
    }

    #[test]
    fn quadratic_explicit_dim() {
        // d=3: Y = (1·x_0 + 1·x_0²) + (2·x_1 + 2·x_1²) + (3·x_2 + 3·x_2²).
        // At [1,1,1]: Y = 2 + 4 + 6 = 12.
        let x = vec![1.0; 3];
        assert_eq!(morris_quadratic_additive_with_dim(&x, 3), 12.0);
    }

    #[test]
    #[should_panic(expected = "expected dim")]
    fn quadratic_wrong_length_panics() {
        let _ = morris_quadratic_additive(&[1.0, 2.0]);
    }

    #[test]
    fn analytic_quadratic_effects_match_closed_form() {
        let e = analytic_quadratic_effects(8);
        assert_eq!(e.dim(), 8);
        for i in 0..8 {
            #[allow(clippy::cast_precision_loss)]
            let want_mu = 2.0 * (i + 1) as f64;
            #[allow(clippy::cast_precision_loss)]
            let want_sigma = (i + 1) as f64 / 3.0;
            assert!(
                (e.mu[i] - want_mu).abs() < 1e-12,
                "μ_{i}: got {} want {want_mu}",
                e.mu[i]
            );
            assert_eq!(e.mu_star[i], want_mu);
            assert!(
                (e.sigma[i] - want_sigma).abs() < 1e-12,
                "σ_{i}: got {} want {want_sigma}",
                e.sigma[i]
            );
        }
    }

    #[test]
    fn analytic_quadratic_mu_star_at_least_absolute_mu() {
        let e = analytic_quadratic_effects(8);
        for i in 0..8 {
            assert!(e.mu_star[i] >= e.mu[i].abs());
        }
    }

    #[test]
    fn analytic_quadratic_sigma_strictly_positive() {
        // c_i = i+1 > 0 always ⇒ σ_i = (i+1)/3 > 0.
        let e = analytic_quadratic_effects(8);
        for s in &e.sigma {
            assert!(*s > 0.0);
        }
    }
}
