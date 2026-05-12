//! Univariate orthogonal polynomial families used as PCE bases.
//!
//! Per Sudret 2006 / Sudret 2008 / Blatman-Sudret 2011: every input
//! distribution admits a "natural" orthogonal polynomial family
//! with respect to its measure. The PCE coefficient extraction
//! (Sudret 2008 Eq 39) relies on orthogonality `⟨Ψₘ, Ψₙ⟩ = 0` for
//! `m ≠ n`.
//!
//! | Family | Distribution | Domain | Weight `w(x)` |
//! |---|---|---|---|
//! | Legendre `Pₙ` | Uniform `[-1, 1]` | `[-1, 1]` | `1/2` |
//! | Hermite `Heₙ` (probabilist) | Normal `N(0, 1)` | `ℝ` | `(1/√(2π)) e^(-x²/2)` |
//! | Laguerre `Lₙ` | Exponential `λ=1` | `[0, ∞)` | `e^(-x)` |
//! | Jacobi `Pₙ^(α,β)` | Beta `Beta(α+1, β+1)` on `[-1, 1]` | `[-1, 1]` | `(1-x)^α (1+x)^β / Z` |
//!
//! All four are computed via three-term recurrences for numerical
//! stability and `O(n)` evaluation.
//!
//! # Norm conventions
//!
//! `norm_squared(family, n) = ⟨Ψₙ, Ψₙ⟩` with respect to the family's
//! reference probability weight (so the weight integrates to 1).
//! This is what Sudret 2008 Eq 36 uses for variance decomposition:
//!
//! ```text
//! D_PC = Σⱼ fⱼ² · ⟨Ψⱼ, Ψⱼ⟩
//! ```
//!
//! # Mapping non-canonical inputs
//!
//! Caller is responsible for mapping their `Distribution` samples
//! to the polynomial family's natural domain:
//!
//! - `Uniform { lo, hi }` → Legendre on `[-1, 1]`: `ξ = 2(x − lo) / (hi − lo) − 1`.
//! - `Normal { mu, sigma }` → Hermite on `ℝ`: `ξ = (x − mu) / sigma`.
//! - `Exponential { lambda }` → Laguerre on `[0, ∞)`: `ξ = λ · x`.
//! - `Beta { alpha, beta, lo, hi }` → Jacobi on `[-1, 1]`: rescale to canonical Beta domain.
//!
//! PR 16b's `fit_full_pce` will handle this mapping internally;
//! this module only provides the polynomial primitives.

#![allow(clippy::similar_names, clippy::cast_precision_loss)]

/// The four orthogonal polynomial families supported by saltelli-PCE.
///
/// `#[non_exhaustive]` — future families (Charlier for Poisson,
/// Krawtchouk for Binomial) land non-breaking via follow-on PRs
/// when discrete-input PCE is needed.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum PolynomialFamily {
    /// Legendre `Pₙ(x)` on `[-1, 1]`. Orthogonal w.r.t. uniform
    /// probability measure `1/2 dx`.
    Legendre,
    /// Probabilist's Hermite `Heₙ(x)` on `ℝ`. Orthogonal w.r.t.
    /// standard Normal measure.
    Hermite,
    /// Laguerre `Lₙ(x)` on `[0, ∞)`. Orthogonal w.r.t. Exponential
    /// (rate-1) measure.
    Laguerre,
    /// Jacobi `Pₙ^(α,β)(x)` on `[-1, 1]`. Orthogonal w.r.t. Beta
    /// measure with shape parameters `(α+1, β+1)`.
    Jacobi { alpha: f64, beta: f64 },
}

/// Evaluate the orthogonal polynomial `Ψₙ(x)` of the given family
/// at degree `n`, point `x`.
///
/// Uses the family's three-term recurrence for `O(n)` stable
/// evaluation. For `n = 0` returns the family's constant
/// `Ψ₀(x) = 1`.
///
/// # Panics
///
/// On Jacobi with `alpha ≤ -1` or `beta ≤ -1` (the weight integral
/// diverges; orthogonality not defined).
#[must_use]
pub fn evaluate(family: PolynomialFamily, n: usize, x: f64) -> f64 {
    match family {
        PolynomialFamily::Legendre => legendre(n, x),
        PolynomialFamily::Hermite => hermite_probabilist(n, x),
        PolynomialFamily::Laguerre => laguerre(n, x),
        PolynomialFamily::Jacobi { alpha, beta } => {
            assert!(
                alpha > -1.0 && beta > -1.0,
                "Jacobi: alpha and beta must be > -1 for orthogonality"
            );
            jacobi(n, alpha, beta, x)
        }
    }
}

/// `⟨Ψₙ, Ψₙ⟩` under the family's reference probability measure
/// (so the weight integrates to 1). Used as the per-coefficient
/// variance contribution in Sudret 2008 Eq 36:
/// `Var(Ψₙ(X)) = norm_squared(family, n)`.
///
/// # Panics
///
/// Same as `evaluate`.
#[must_use]
pub fn norm_squared(family: PolynomialFamily, n: usize) -> f64 {
    match family {
        // Legendre on [-1, 1] with uniform probability weight 1/2:
        // ⟨Pₙ, Pₙ⟩ = 1 / (2n + 1).
        PolynomialFamily::Legendre => 1.0 / (2.0 * n as f64 + 1.0),

        // Probabilist Hermite under N(0, 1):
        // ⟨Heₙ, Heₙ⟩ = n!.
        PolynomialFamily::Hermite => factorial(n),

        // Laguerre under Exp(1):
        // ⟨Lₙ, Lₙ⟩ = 1 (standard normalization).
        PolynomialFamily::Laguerre => 1.0,

        // Jacobi under Beta(α+1, β+1) on [-1, 1]:
        // ⟨Pₙ^(α,β), Pₙ^(α,β)⟩ =
        //     [2^(α+β+1) / (2n+α+β+1)] · Γ(n+α+1)Γ(n+β+1) / [n! · Γ(n+α+β+1)]
        // divided by the normalizing constant of the Beta measure
        // 2^(α+β+1) · B(α+1, β+1).
        // After cancellation, with the probability-weighted convention:
        //   ⟨Pₙ^(α,β), Pₙ^(α,β)⟩ = Γ(n+α+1) Γ(n+β+1) Γ(α+β+2)
        //                          / [(2n+α+β+1) · n! · Γ(n+α+β+1) · Γ(α+1) · Γ(β+1)]
        PolynomialFamily::Jacobi { alpha, beta } => {
            assert!(alpha > -1.0 && beta > -1.0);
            jacobi_norm_squared(n, alpha, beta)
        }
    }
}

/// Predicate for a value being in the family's polynomial-canonical
/// domain. Used by debug-only callers (`fit_full_pce`,
/// `fit_sparse_pce`, `PolynomialChaos::evaluate`) to catch the
/// silent-garbage footgun where out-of-domain inputs evaluate
/// cleanly through the recurrences but produce nonsense Sobol'
/// indices. Crate-internal — release builds elide the check.
#[must_use]
pub(crate) fn is_in_canonical_domain(family: PolynomialFamily, x: f64) -> bool {
    match family {
        PolynomialFamily::Legendre | PolynomialFamily::Jacobi { .. } => (-1.0..=1.0).contains(&x),
        PolynomialFamily::Hermite => x.is_finite(),
        PolynomialFamily::Laguerre => x >= 0.0 && x.is_finite(),
    }
}

// ── Three-term recurrences ──────────────────────────────────────

/// Legendre `Pₙ(x)` via the recurrence
/// `(n+1) P_{n+1}(x) = (2n+1) x Pₙ(x) − n P_{n−1}(x)`.
///
/// `P₀(x) = 1`, `P₁(x) = x`.
fn legendre(n: usize, x: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let mut p_prev = 1.0; // P_0
    let mut p_curr = x; // P_1
    for k in 1..n {
        let k_f = k as f64;
        let p_next = ((2.0 * k_f + 1.0) * x * p_curr - k_f * p_prev) / (k_f + 1.0);
        p_prev = p_curr;
        p_curr = p_next;
    }
    p_curr
}

/// Probabilist's Hermite `Heₙ(x)` via the recurrence
/// `He_{n+1}(x) = x · Heₙ(x) − n · He_{n−1}(x)`.
///
/// `He₀(x) = 1`, `He₁(x) = x`. This is the "probabilist's"
/// convention orthogonal under the standard Normal measure
/// `(1/√(2π)) e^(-x²/2)`. Distinct from "physicist's" Hermite
/// `Hₙ(x)` (orthogonal under `e^(-x²)`); the probabilist form is
/// standard for PCE per Sudret 2006 § 3.1.
fn hermite_probabilist(n: usize, x: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let mut h_prev = 1.0;
    let mut h_curr = x;
    for k in 1..n {
        let k_f = k as f64;
        let h_next = x * h_curr - k_f * h_prev;
        h_prev = h_curr;
        h_curr = h_next;
    }
    h_curr
}

/// Laguerre `Lₙ(x)` via the recurrence
/// `(n+1) L_{n+1}(x) = (2n + 1 − x) Lₙ(x) − n L_{n−1}(x)`.
///
/// `L₀(x) = 1`, `L₁(x) = 1 − x`. Orthogonal under Exp(1).
fn laguerre(n: usize, x: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let mut l_prev = 1.0;
    let mut l_curr = 1.0 - x;
    for k in 1..n {
        let k_f = k as f64;
        let l_next = ((2.0 * k_f + 1.0 - x) * l_curr - k_f * l_prev) / (k_f + 1.0);
        l_prev = l_curr;
        l_curr = l_next;
    }
    l_curr
}

/// Jacobi `Pₙ^(α,β)(x)` via the standard recurrence (NIST DLMF
/// 18.9.2). `P₀ = 1`. `P₁ = (α + 1) + (α + β + 2)·(x − 1)/2`.
///
/// Recurrence:
///
/// ```text
/// 2(n+1)(n+α+β+1)(2n+α+β) · P_{n+1}
///   = (2n+α+β+1)·[(2n+α+β)(2n+α+β+2)·x + α² − β²] · Pₙ
///     − 2(n+α)(n+β)(2n+α+β+2) · P_{n−1}
/// ```
///
/// Numerically stable for moderate `n` (< 50) in the canonical
/// `α, β > -1` regime; degrades for very high `n` or near-zero
/// `2n+α+β` denominators (avoidable in practice for the PCE
/// truncation orders we use, `p ≤ 10`).
fn jacobi(n: usize, alpha: f64, beta: f64, x: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    if n == 1 {
        return (alpha + 1.0) + (alpha + beta + 2.0) * (x - 1.0) / 2.0;
    }
    let mut p_prev = 1.0_f64;
    let mut p_curr = (alpha + 1.0) + (alpha + beta + 2.0) * (x - 1.0) / 2.0;
    for k in 1..n {
        let k_f = k as f64;
        let two_k_ab = 2.0 * k_f + alpha + beta;
        let coeff_lhs = 2.0 * (k_f + 1.0) * (k_f + alpha + beta + 1.0) * two_k_ab;
        let coeff_curr_x = (two_k_ab + 1.0) * two_k_ab * (two_k_ab + 2.0);
        let coeff_curr_const = (two_k_ab + 1.0) * (alpha * alpha - beta * beta);
        let coeff_prev = 2.0 * (k_f + alpha) * (k_f + beta) * (two_k_ab + 2.0);
        let p_next =
            ((coeff_curr_x * x + coeff_curr_const) * p_curr - coeff_prev * p_prev) / coeff_lhs;
        p_prev = p_curr;
        p_curr = p_next;
    }
    p_curr
}

/// `⟨Pₙ^(α,β), Pₙ^(α,β)⟩` under the Beta probability measure on
/// `[-1, 1]`. Closed form via Γ functions; routed through `lgamma`
/// (computing logs and exponentiating) to avoid overflow at
/// moderate `n + α + β`.
fn jacobi_norm_squared(n: usize, alpha: f64, beta: f64) -> f64 {
    let n_f = n as f64;
    // log of:
    //   Γ(n+α+1) · Γ(n+β+1) · Γ(α+β+2)
    //   / [(2n+α+β+1) · n! · Γ(n+α+β+1) · Γ(α+1) · Γ(β+1)]
    let log_num = lgamma(n_f + alpha + 1.0) + lgamma(n_f + beta + 1.0) + lgamma(alpha + beta + 2.0);
    let log_denom = ((2.0 * n_f + alpha + beta + 1.0).ln())
        + lgamma(n_f + 1.0)
        + lgamma(n_f + alpha + beta + 1.0)
        + lgamma(alpha + 1.0)
        + lgamma(beta + 1.0);
    (log_num - log_denom).exp()
}

/// `factorial(n)` as `f64` — direct multiplicative form. Sufficient
/// for PCE truncation orders `p ≤ 20` where overflow stays below
/// `f64::MAX`.
fn factorial(n: usize) -> f64 {
    let mut f = 1.0_f64;
    for k in 1..=n {
        f *= k as f64;
    }
    f
}

/// `ln Γ(x)` via Stirling-series approximation. Used for Jacobi
/// `norm_squared`; precision adequate for PCE workloads.
///
/// Implementation: Lanczos series with Spouge-style coefficients.
/// Adapted from Numerical Recipes §6.1; returns within ~1e-15
/// relative error for `x > 0`.
#[allow(clippy::excessive_precision)]
fn lgamma(x: f64) -> f64 {
    // Standard Lanczos coefficients (g=7, n=9). Coefficient
    // precision deliberately exceeds f64 mantissa to round-trip
    // through `f64` correctly via Lanczos's standard table.
    const G: f64 = 7.0;
    const COEFF: [f64; 9] = [
        0.999_999_999_999_809_93,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_13,
        -176.615_029_162_140_59,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_571_6e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        // Reflection formula: ln Γ(x) = ln(π / sin(πx)) − ln Γ(1−x).
        let sin_pi_x = (std::f64::consts::PI * x).sin();
        std::f64::consts::PI.ln() - sin_pi_x.abs().ln() - lgamma(1.0 - x)
    } else {
        let xm = x - 1.0;
        let mut series = COEFF[0];
        for (i, c) in COEFF.iter().enumerate().skip(1) {
            #[allow(clippy::cast_precision_loss)]
            let denom = xm + i as f64;
            series += c / denom;
        }
        let t = xm + G + 0.5;
        0.5 * (2.0 * std::f64::consts::PI).ln() + (xm + 0.5) * t.ln() - t + series.ln()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use super::*;

    // ── Legendre — closed-form values at known points ────────────

    #[test]
    fn legendre_at_zero_matches_table() {
        // P_0 = 1, P_1 = 0, P_2 = -1/2, P_3 = 0, P_4 = 3/8.
        assert_eq!(legendre(0, 0.0), 1.0);
        assert_eq!(legendre(1, 0.0), 0.0);
        assert!((legendre(2, 0.0) - (-0.5)).abs() < 1e-12);
        assert!((legendre(3, 0.0) - 0.0).abs() < 1e-12);
        assert!((legendre(4, 0.0) - 0.375).abs() < 1e-12);
    }

    #[test]
    fn legendre_at_one_is_unity() {
        // P_n(1) = 1 for all n.
        for n in 0..=10 {
            assert!(
                (legendre(n, 1.0) - 1.0).abs() < 1e-10,
                "P_{n}(1) = {}, want 1",
                legendre(n, 1.0)
            );
        }
    }

    #[test]
    fn legendre_at_half_p3_matches_closed_form() {
        // P_3(x) = (5x³ − 3x)/2. At x = 0.5: (5·0.125 − 1.5)/2 = -0.4375.
        assert!((legendre(3, 0.5) - (-0.4375)).abs() < 1e-12);
    }

    // ── Hermite — closed-form values ─────────────────────────────

    #[test]
    fn hermite_at_zero_matches_table() {
        // He_0 = 1, He_1 = 0, He_2 = -1, He_3 = 0, He_4 = 3.
        assert_eq!(hermite_probabilist(0, 0.0), 1.0);
        assert_eq!(hermite_probabilist(1, 0.0), 0.0);
        assert_eq!(hermite_probabilist(2, 0.0), -1.0);
        assert_eq!(hermite_probabilist(3, 0.0), 0.0);
        assert_eq!(hermite_probabilist(4, 0.0), 3.0);
    }

    #[test]
    fn hermite_h2_x_squared_minus_one() {
        // He_2(x) = x² − 1. Spot-check at x = 2: 4 − 1 = 3.
        assert_eq!(hermite_probabilist(2, 2.0), 3.0);
    }

    #[test]
    fn hermite_h3_at_two() {
        // He_3(x) = x³ − 3x. At x = 2: 8 − 6 = 2.
        assert!((hermite_probabilist(3, 2.0) - 2.0).abs() < 1e-12);
    }

    // ── Laguerre — closed-form values ────────────────────────────

    #[test]
    fn laguerre_at_zero_is_unity_for_all_n() {
        // L_n(0) = 1.
        for n in 0..=8 {
            assert!(
                (laguerre(n, 0.0) - 1.0).abs() < 1e-10,
                "L_{n}(0) = {}",
                laguerre(n, 0.0)
            );
        }
    }

    #[test]
    fn laguerre_l2_closed_form() {
        // L_2(x) = (x² − 4x + 2) / 2. At x = 1: (1 − 4 + 2)/2 = -0.5.
        assert!((laguerre(2, 1.0) - (-0.5)).abs() < 1e-12);
    }

    #[test]
    fn laguerre_l3_closed_form() {
        // L_3(x) = (-x³ + 9x² − 18x + 6) / 6.
        // At x = 1: (-1 + 9 - 18 + 6)/6 = -4/6 ≈ -0.6667.
        let want = -4.0 / 6.0;
        assert!((laguerre(3, 1.0) - want).abs() < 1e-12);
    }

    // ── Jacobi — closed-form spot checks ─────────────────────────

    #[test]
    fn jacobi_p0_is_unity() {
        for x in [-0.7_f64, 0.0, 0.5] {
            assert_eq!(jacobi(0, 1.0, 2.0, x), 1.0);
        }
    }

    #[test]
    fn jacobi_p1_matches_formula() {
        // P_1^(α,β)(x) = (α + 1) + (α + β + 2)·(x − 1)/2.
        // α = 0, β = 0: P_1 = 1 + 2·(x−1)/2 = x. (Reduces to Legendre.)
        assert!((jacobi(1, 0.0, 0.0, 0.5) - 0.5).abs() < 1e-12);
        // α = 1, β = 0: P_1 = 2 + 3·(x−1)/2 = (3x + 1)/2. At x=0: 0.5.
        assert!((jacobi(1, 1.0, 0.0, 0.0) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn jacobi_reduces_to_legendre_when_alpha_beta_zero() {
        // P_n^(0,0)(x) = P_n(x).
        for n in 0..=5 {
            for &x in &[-0.7_f64, -0.3, 0.0, 0.4, 0.8] {
                let j = jacobi(n, 0.0, 0.0, x);
                let l = legendre(n, x);
                assert!(
                    (j - l).abs() < 1e-10,
                    "n={n}, x={x}: Jacobi(0,0)={j}, Legendre={l}"
                );
            }
        }
    }

    // ── Norms ────────────────────────────────────────────────────

    #[test]
    fn legendre_norm_squared_matches_formula() {
        // ⟨P_n, P_n⟩ under uniform [−1, 1] = 1/(2n+1).
        for n in 0..=8 {
            let want = 1.0 / (2.0 * n as f64 + 1.0);
            assert!(
                (norm_squared(PolynomialFamily::Legendre, n) - want).abs() < 1e-12,
                "n={n}"
            );
        }
    }

    #[test]
    fn hermite_norm_squared_is_factorial() {
        // ⟨He_n, He_n⟩ = n!.
        let want = [1.0, 1.0, 2.0, 6.0, 24.0, 120.0, 720.0];
        for (n, &expected) in want.iter().enumerate() {
            assert_eq!(norm_squared(PolynomialFamily::Hermite, n), expected);
        }
    }

    #[test]
    fn laguerre_norm_squared_is_unity() {
        // Standard normalization: ⟨L_n, L_n⟩ = 1.
        for n in 0..=5 {
            assert_eq!(norm_squared(PolynomialFamily::Laguerre, n), 1.0);
        }
    }

    #[test]
    fn jacobi_norm_squared_reduces_to_legendre() {
        // Jacobi(0, 0) under Beta(1, 1) (= Uniform on [-1, 1])
        // should give 1/(2n+1). Within FP tolerance from lgamma path.
        for n in 0..=5 {
            let j = norm_squared(
                PolynomialFamily::Jacobi {
                    alpha: 0.0,
                    beta: 0.0,
                },
                n,
            );
            let l = norm_squared(PolynomialFamily::Legendre, n);
            assert!(
                (j - l).abs() < 1e-10,
                "n={n}: Jacobi norm = {j}, Legendre norm = {l}"
            );
        }
    }

    // ── Orthogonality (numerical integration) ────────────────────

    #[test]
    fn legendre_orthogonality_via_quadrature() {
        // ∫_{-1}^{1} P_m(x) P_n(x) (1/2) dx ≈ 0 for m ≠ n,
        // ≈ 1/(2n+1) for m = n. Quadrature: trapezoidal at 1000 points.
        const NPTS: usize = 1000;
        let dx = 2.0 / NPTS as f64;
        let weight = 0.5 * dx; // (1/2) is the uniform-[-1, 1] probability density.
        for m in 0..=4 {
            for n in 0..=4 {
                let mut integral = 0.0;
                for k in 0..=NPTS {
                    let x = -1.0 + (k as f64) * dx;
                    let w = if k == 0 || k == NPTS {
                        weight * 0.5
                    } else {
                        weight
                    };
                    integral += w * legendre(m, x) * legendre(n, x);
                }
                let expected = if m == n {
                    1.0 / (2.0 * n as f64 + 1.0)
                } else {
                    0.0
                };
                assert!(
                    (integral - expected).abs() < 0.01,
                    "⟨P_{m}, P_{n}⟩ = {integral}, want {expected}"
                );
            }
        }
    }

    #[test]
    fn evaluate_dispatches_correctly() {
        // Round-trip via the `evaluate` entrypoint.
        assert_eq!(evaluate(PolynomialFamily::Legendre, 2, 0.0), -0.5);
        assert_eq!(evaluate(PolynomialFamily::Hermite, 2, 0.0), -1.0);
        assert_eq!(evaluate(PolynomialFamily::Laguerre, 0, 5.0), 1.0);
        assert!(
            (evaluate(
                PolynomialFamily::Jacobi {
                    alpha: 0.0,
                    beta: 0.0
                },
                3,
                0.5
            ) - (-0.4375))
                .abs()
                < 1e-10
        );
    }

    #[test]
    #[should_panic(expected = "alpha and beta must be > -1")]
    fn jacobi_panics_on_invalid_alpha() {
        let _ = evaluate(
            PolynomialFamily::Jacobi {
                alpha: -1.5,
                beta: 0.0,
            },
            1,
            0.0,
        );
    }
}
