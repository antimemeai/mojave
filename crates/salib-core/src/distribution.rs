//! `Distribution` — closed enum of factor distributions, with
//! inverse-CDF (`quantile`) and support boundaries as the unified
//! extension point.
//!
//! # Why a closed enum, not a `dyn Distribution` trait
//!
//! Per `decisions/2026-04-28-saltelli-problem-shape.md` — and matching
//! sky-claude's spec in `rust_salib_crate_research.md` § 3.1 verbatim
//! — the distribution set is closed and `Serialize + Deserialize`.
//! The ledger entry is a JSON dump of `Problem`, byte-comparable
//! across runs. A trait-object distribution would break that
//! provenance property.
//!
//! Custom / exotic distributions plug in *on the model side* by
//! composing inverse CDFs over `Uniform { 0, 1 }` — the same trick
//! `SALib` uses. `Empirical { quantiles }` and `Truncated` are
//! `#[non_exhaustive]` extensions that land in follow-on PRs (each
//! has its own design questions: interpolation policy for
//! `Empirical`, truncation discipline for `Truncated`).
//!
//! # The `quantile` contract
//!
//! `quantile(u: f64) -> f64` for `u ∈ [0, 1]`. Out-of-range `u` is
//! saturated to `[0, 1]`, giving well-defined behavior at the
//! support boundaries. This is the only direction samplers consume —
//! they produce uniform `[0, 1)` samples and call `quantile` per
//! factor. The reverse direction (`cdf`) is needed for `Truncated`
//! and for future moment-independent estimators; lands in the
//! follow-on PR that introduces `Truncated`.
//!
//! # Closed-form vs `statrs`
//!
//! - Closed-form (this file): Uniform, Triangular, Weibull,
//!   Exponential, Bernoulli, `DiscreteUniform`.
//! - `statrs` Newton-converged inverse CDF: Normal, `LogNormal`, Beta,
//!   Gamma. These have no useful closed-form quantile.
//!
//! # Determinism
//!
//! Every quantile is a pure function of `(parameters, u)`. No RNG,
//! no clock, no env. Cross-platform-byte-exact under the no-FMA
//! reference build (`cargo xtask reference-ci`) — `statrs`'s
//! Newton iterations are convergence-tolerant, not bit-stable across
//! FMA-on vs FMA-off builds. This is documented in
//! `decisions/2026-04-28-saltelli-problem-shape.md` § "Threat
//! model" as a known property of the inverse-CDF path.

use serde::{Deserialize, Serialize};
use statrs::distribution::{
    Beta as BetaDist, ContinuousCDF, Gamma as GammaDist, LogNormal as LogNormalDist,
    Normal as NormalDist,
};

/// Factor distributions saltelli supports. Closed enum,
/// `#[non_exhaustive]`. Future variants (`Truncated`, `Empirical`,
/// `Categorical`, …) land non-breaking via follow-on ADRs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "kind")]
pub enum Distribution {
    /// Uniform on `[lo, hi]`. Closed-form quantile.
    Uniform { lo: f64, hi: f64 },

    /// Normal `N(mu, sigma²)`. `statrs::Normal::inverse_cdf`.
    Normal { mu: f64, sigma: f64 },

    /// Log-normal — `exp(N(mu_log, sigma_log²))`.
    /// `statrs::LogNormal::inverse_cdf`.
    LogNormal { mu_log: f64, sigma_log: f64 },

    /// Triangular on `[lo, hi]` with mode `mode`. Closed-form
    /// quantile (piecewise sqrt).
    Triangular { lo: f64, mode: f64, hi: f64 },

    /// Beta on `[lo, hi]` with shape parameters `alpha`, `beta`.
    /// `statrs::Beta::inverse_cdf` then affine-mapped to `[lo, hi]`.
    Beta {
        alpha: f64,
        beta: f64,
        lo: f64,
        hi: f64,
    },

    /// Gamma with shape `shape` and **scale** `scale` (so mean =
    /// shape × scale). `statrs::Gamma` parameterizes by rate, so
    /// we pass `1/scale`. Closed-form for shape = 1 collapses to
    /// `Exponential { lambda: 1/scale }`.
    Gamma { shape: f64, scale: f64 },

    /// Weibull with shape `shape`, scale `scale`. Closed-form
    /// quantile: `scale * (-ln(1 - u))^(1/shape)`.
    Weibull { shape: f64, scale: f64 },

    /// Exponential with rate `lambda`. Closed-form quantile:
    /// `-ln(1 - u) / lambda`.
    Exponential { lambda: f64 },

    /// Bernoulli with success probability `p`. Quantile: 0 if
    /// `u < 1 - p`, else 1.
    Bernoulli { p: f64 },

    /// Discrete uniform on the inclusive integer range `[lo, hi]`.
    DiscreteUniform { lo: i64, hi: i64 },
}

// `statrs`'s `Beta::new` etc. return `Result`; the `quantile` impls
// below panic on invalid params via debug-style assertions. This is
// safe because `ProblemBuilder::build` validates parameters at
// `Problem` construction time (per `decisions/2026-04-28-saltelli-problem-shape.md`),
// so a `Distribution` value reachable from a built `Problem` cannot
// have bad params. A future fallible `Distribution::checked_quantile`
// lands when a public `Distribution` constructor surface is needed
// (none today).

impl Distribution {
    /// Inverse CDF. `u ∈ [0, 1]` (saturated; out-of-range inputs
    /// clamp to the support boundary). Pure function of
    /// `(parameters, u)`.
    ///
    /// # Panics
    ///
    /// On invalid distribution parameters (`sigma ≤ 0`, `alpha ≤ 0`,
    /// `Beta` with `lo ≥ hi`, etc.). `Problem` construction validates
    /// parameters at build time so this never fires for a Problem
    /// produced by `ProblemBuilder::build`.
    #[must_use]
    pub fn quantile(&self, u: f64) -> f64 {
        let u = u.clamp(0.0, 1.0);
        match *self {
            Self::Uniform { lo, hi } => uniform_quantile(lo, hi, u),
            Self::Normal { mu, sigma } => normal_quantile(mu, sigma, u),
            Self::LogNormal { mu_log, sigma_log } => lognormal_quantile(mu_log, sigma_log, u),
            Self::Triangular { lo, mode, hi } => triangular_quantile(lo, mode, hi, u),
            Self::Beta {
                alpha,
                beta,
                lo,
                hi,
            } => beta_quantile(alpha, beta, lo, hi, u),
            Self::Gamma { shape, scale } => gamma_quantile(shape, scale, u),
            Self::Weibull { shape, scale } => weibull_quantile(shape, scale, u),
            Self::Exponential { lambda } => exponential_quantile(lambda, u),
            Self::Bernoulli { p } => bernoulli_quantile(p, u),
            Self::DiscreteUniform { lo, hi } => discrete_uniform_quantile(lo, hi, u),
        }
    }

    /// Support of the distribution as `(lower, upper)`. For
    /// distributions with infinite support, returns `±f64::INFINITY`.
    /// `quantile(0.0)` returns the lower support and `quantile(1.0)`
    /// the upper support.
    #[must_use]
    pub fn support(&self) -> (f64, f64) {
        match *self {
            // Distributions with explicit `(lo, hi)` bounds (Uniform,
            // Triangular, Beta) share an arm body. LogNormal /
            // Gamma / Exponential / Weibull share `(0, +∞)`.
            Self::Uniform { lo, hi }
            | Self::Triangular { lo, hi, .. }
            | Self::Beta { lo, hi, .. } => (lo, hi),
            Self::Normal { .. } => (f64::NEG_INFINITY, f64::INFINITY),
            Self::LogNormal { .. }
            | Self::Gamma { .. }
            | Self::Exponential { .. }
            | Self::Weibull { .. } => (0.0, f64::INFINITY),
            Self::Bernoulli { .. } => (0.0, 1.0),
            #[allow(clippy::cast_precision_loss)]
            Self::DiscreteUniform { lo, hi } => (lo as f64, hi as f64),
        }
    }
}

// ── Closed-form quantiles ────────────────────────────────────────────

fn uniform_quantile(lo: f64, hi: f64, u: f64) -> f64 {
    lo + u * (hi - lo)
}

fn triangular_quantile(lo: f64, mode: f64, hi: f64, u: f64) -> f64 {
    assert!(lo < hi, "Triangular: lo must be < hi");
    assert!(
        lo <= mode && mode <= hi,
        "Triangular: mode must be in [lo, hi]"
    );
    let f_mode = (mode - lo) / (hi - lo);
    if u <= f_mode {
        lo + (u * (hi - lo) * (mode - lo)).sqrt()
    } else {
        hi - ((1.0 - u) * (hi - lo) * (hi - mode)).sqrt()
    }
}

fn weibull_quantile(shape: f64, scale: f64, u: f64) -> f64 {
    assert!(shape > 0.0, "Weibull: shape must be > 0");
    assert!(scale > 0.0, "Weibull: scale must be > 0");
    if u >= 1.0 {
        return f64::INFINITY;
    }
    scale * (-(1.0 - u).ln()).powf(1.0 / shape)
}

fn exponential_quantile(lambda: f64, u: f64) -> f64 {
    assert!(lambda > 0.0, "Exponential: lambda must be > 0");
    if u >= 1.0 {
        return f64::INFINITY;
    }
    -((1.0 - u).ln()) / lambda
}

fn bernoulli_quantile(p: f64, u: f64) -> f64 {
    assert!((0.0..=1.0).contains(&p), "Bernoulli: p must be in [0, 1]");
    // Standard inverse CDF: F⁻¹(u) = inf{x : F(x) ≥ u}. For Bernoulli,
    // F(0) = 1-p, F(1) = 1, so the boundary at u = 1-p maps to 0.
    // u ∈ [0, 1-p] → 0; u ∈ (1-p, 1] → 1.
    // Marginal probability of returning 1 is the measure of (1-p, 1],
    // which is 1 - (1-p) = p — as required.
    if u <= 1.0 - p {
        0.0
    } else {
        1.0
    }
}

fn discrete_uniform_quantile(lo: i64, hi: i64, u: f64) -> f64 {
    assert!(lo <= hi, "DiscreteUniform: lo must be <= hi");
    let n = hi - lo + 1;
    #[allow(clippy::cast_precision_loss)]
    let scaled = u * (n as f64);
    // Floor and clamp the upper edge: at u = 1.0, `scaled == n`,
    // which would index past `hi`. Clamp to `n - 1`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let idx = (scaled.floor() as i64).min(n - 1);
    #[allow(clippy::cast_precision_loss)]
    let result = (lo + idx) as f64;
    result
}

// ── statrs-backed quantiles ──────────────────────────────────────────
//
// `expect()` on the statrs constructors is sound because
// `validate_distribution` in `problem.rs` rejects bad params at
// `ProblemBuilder::build` time; a `Distribution` value reachable from
// a built `Problem` cannot have parameters that fail statrs's own
// validity check. The local `assert!`s below are belt-and-suspenders
// for the case where these helpers are called directly (only inside
// this crate, and only from `Distribution::quantile` which receives
// the already-validated `Distribution`).

#[allow(clippy::expect_used)]
fn normal_quantile(mu: f64, sigma: f64, u: f64) -> f64 {
    assert!(sigma > 0.0, "Normal: sigma must be > 0");
    let dist = NormalDist::new(mu, sigma).expect("Normal::new param check");
    dist.inverse_cdf(u)
}

#[allow(clippy::expect_used)]
fn lognormal_quantile(mu_log: f64, sigma_log: f64, u: f64) -> f64 {
    assert!(sigma_log > 0.0, "LogNormal: sigma_log must be > 0");
    let dist = LogNormalDist::new(mu_log, sigma_log).expect("LogNormal::new param check");
    dist.inverse_cdf(u)
}

#[allow(clippy::expect_used)]
fn beta_quantile(alpha: f64, beta: f64, lo: f64, hi: f64, u: f64) -> f64 {
    assert!(alpha > 0.0, "Beta: alpha must be > 0");
    assert!(beta > 0.0, "Beta: beta must be > 0");
    assert!(lo < hi, "Beta: lo must be < hi");
    let dist = BetaDist::new(alpha, beta).expect("Beta::new param check");
    let v = dist.inverse_cdf(u);
    lo + (hi - lo) * v
}

#[allow(clippy::expect_used)]
fn gamma_quantile(shape: f64, scale: f64, u: f64) -> f64 {
    assert!(shape > 0.0, "Gamma: shape must be > 0");
    assert!(scale > 0.0, "Gamma: scale must be > 0");
    // statrs `Gamma::new(shape, rate)` parameterizes by rate = 1/scale.
    let dist = GammaDist::new(shape, 1.0 / scale).expect("Gamma::new param check");
    dist.inverse_cdf(u)
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::cast_precision_loss
)]
mod tests {
    use super::*;

    // Numerical-tolerance helper for floating-point assertions.
    fn assert_close(got: f64, want: f64, tol: f64, ctx: &str) {
        assert!(
            (got - want).abs() <= tol,
            "{ctx}: got {got}, want {want}, |Δ|={}, tol={tol}",
            (got - want).abs()
        );
    }

    fn assert_monotone_non_decreasing(d: &Distribution) {
        // 17 sample u values; check that quantile is monotone
        // non-decreasing across the [0, 1] range.
        let us = [
            0.0, 0.001, 0.01, 0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95, 0.99, 0.999,
            1.0,
        ];
        let mut prev = f64::NEG_INFINITY;
        for &u in &us {
            let q = d.quantile(u);
            assert!(
                q >= prev || (q.is_nan() && prev.is_nan()),
                "monotonicity violated for {d:?}: q({u}) = {q} < prev {prev}"
            );
            prev = q;
        }
    }

    // ── Uniform ─────────────────────────────────────────────────────

    #[test]
    fn uniform_zero_one_quantile_is_u() {
        let d = Distribution::Uniform { lo: 0.0, hi: 1.0 };
        for u in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert_eq!(d.quantile(u), u);
        }
    }

    #[test]
    fn uniform_general_quantile_linearly_maps() {
        let d = Distribution::Uniform { lo: 10.0, hi: 30.0 };
        assert_eq!(d.quantile(0.0), 10.0);
        assert_eq!(d.quantile(0.5), 20.0);
        assert_eq!(d.quantile(1.0), 30.0);
    }

    #[test]
    fn uniform_negative_range() {
        let d = Distribution::Uniform { lo: -5.0, hi: 5.0 };
        assert_eq!(d.quantile(0.5), 0.0);
        assert_eq!(d.quantile(0.0), -5.0);
        assert_eq!(d.quantile(1.0), 5.0);
    }

    #[test]
    fn uniform_support_matches_params() {
        let d = Distribution::Uniform { lo: 2.5, hi: 7.5 };
        assert_eq!(d.support(), (2.5, 7.5));
    }

    #[test]
    fn uniform_monotone() {
        assert_monotone_non_decreasing(&Distribution::Uniform { lo: 0.0, hi: 1.0 });
        assert_monotone_non_decreasing(&Distribution::Uniform {
            lo: -10.0,
            hi: 100.0,
        });
    }

    #[test]
    fn uniform_saturates_out_of_range_u() {
        let d = Distribution::Uniform { lo: 0.0, hi: 1.0 };
        assert_eq!(d.quantile(-0.5), 0.0);
        assert_eq!(d.quantile(1.5), 1.0);
    }

    // ── Normal ──────────────────────────────────────────────────────

    #[test]
    fn normal_quantile_at_half_is_mean() {
        let d = Distribution::Normal {
            mu: 5.0,
            sigma: 2.0,
        };
        assert_close(d.quantile(0.5), 5.0, 1e-12, "Normal median");
    }

    #[test]
    fn normal_quantile_one_sigma_above_mean() {
        // Φ⁻¹(0.8413447) ≈ 1.0
        let d = Distribution::Normal {
            mu: 0.0,
            sigma: 1.0,
        };
        assert_close(d.quantile(0.841_344_746_068_543), 1.0, 1e-9, "+1σ");
    }

    #[test]
    fn normal_quantile_symmetric_about_mean() {
        let d = Distribution::Normal {
            mu: 7.0,
            sigma: 3.0,
        };
        for u in [0.1, 0.2, 0.3, 0.4] {
            let q_lo = d.quantile(u);
            let q_hi = d.quantile(1.0 - u);
            // Symmetric: lo + hi = 2 * mu
            assert_close(q_lo + q_hi, 2.0 * 7.0, 1e-9, "Normal symmetry");
        }
    }

    #[test]
    fn normal_support_is_unbounded() {
        let d = Distribution::Normal {
            mu: 0.0,
            sigma: 1.0,
        };
        let (lo, hi) = d.support();
        assert_eq!(lo, f64::NEG_INFINITY);
        assert_eq!(hi, f64::INFINITY);
    }

    #[test]
    fn normal_monotone() {
        assert_monotone_non_decreasing(&Distribution::Normal {
            mu: 0.0,
            sigma: 1.0,
        });
    }

    #[test]
    #[should_panic(expected = "sigma must be > 0")]
    fn normal_zero_sigma_panics() {
        let d = Distribution::Normal {
            mu: 0.0,
            sigma: 0.0,
        };
        let _ = d.quantile(0.5);
    }

    // ── LogNormal ───────────────────────────────────────────────────

    #[test]
    fn lognormal_quantile_at_half_is_exp_mu_log() {
        // Median of LogNormal(μ, σ²) is exp(μ).
        let d = Distribution::LogNormal {
            mu_log: 1.0,
            sigma_log: 0.5,
        };
        assert_close(d.quantile(0.5), 1.0_f64.exp(), 1e-9, "LogNormal median");
    }

    #[test]
    fn lognormal_support_is_zero_to_infinity() {
        let d = Distribution::LogNormal {
            mu_log: 0.0,
            sigma_log: 1.0,
        };
        let (lo, hi) = d.support();
        assert_eq!(lo, 0.0);
        assert_eq!(hi, f64::INFINITY);
    }

    #[test]
    fn lognormal_monotone() {
        assert_monotone_non_decreasing(&Distribution::LogNormal {
            mu_log: 0.0,
            sigma_log: 1.0,
        });
    }

    // ── Triangular ──────────────────────────────────────────────────

    #[test]
    fn triangular_quantile_at_zero_is_lo() {
        let d = Distribution::Triangular {
            lo: 0.0,
            mode: 0.5,
            hi: 1.0,
        };
        assert_eq!(d.quantile(0.0), 0.0);
    }

    #[test]
    fn triangular_quantile_at_one_is_hi() {
        let d = Distribution::Triangular {
            lo: 0.0,
            mode: 0.5,
            hi: 1.0,
        };
        assert_eq!(d.quantile(1.0), 1.0);
    }

    #[test]
    fn triangular_at_f_mode_is_mode() {
        // For symmetric triangular [0, 0.5, 1], F(mode) = 0.5.
        let d = Distribution::Triangular {
            lo: 0.0,
            mode: 0.5,
            hi: 1.0,
        };
        assert_close(d.quantile(0.5), 0.5, 1e-12, "Triangular at F(mode)");
    }

    #[test]
    fn triangular_asymmetric_mode() {
        // Triangular [0, 0.25, 1]: F(mode) = 0.25.
        let d = Distribution::Triangular {
            lo: 0.0,
            mode: 0.25,
            hi: 1.0,
        };
        assert_close(d.quantile(0.25), 0.25, 1e-12, "asymmetric mode");
    }

    #[test]
    fn triangular_support_matches_params() {
        let d = Distribution::Triangular {
            lo: -2.0,
            mode: 0.0,
            hi: 5.0,
        };
        assert_eq!(d.support(), (-2.0, 5.0));
    }

    #[test]
    fn triangular_monotone_symmetric() {
        assert_monotone_non_decreasing(&Distribution::Triangular {
            lo: 0.0,
            mode: 0.5,
            hi: 1.0,
        });
    }

    #[test]
    fn triangular_monotone_asymmetric() {
        assert_monotone_non_decreasing(&Distribution::Triangular {
            lo: -10.0,
            mode: -3.0,
            hi: 7.0,
        });
    }

    // ── Beta ────────────────────────────────────────────────────────

    #[test]
    fn beta_quantile_at_half_for_alpha_eq_beta_is_midpoint() {
        // Beta(α, α) is symmetric about 0.5 (in unit space).
        let d = Distribution::Beta {
            alpha: 2.0,
            beta: 2.0,
            lo: 0.0,
            hi: 1.0,
        };
        assert_close(d.quantile(0.5), 0.5, 1e-9, "symmetric Beta median");
    }

    #[test]
    fn beta_affine_to_general_range() {
        // Beta(2, 2) on [10, 30] should have quantile(0.5) = 20.
        let d = Distribution::Beta {
            alpha: 2.0,
            beta: 2.0,
            lo: 10.0,
            hi: 30.0,
        };
        assert_close(d.quantile(0.5), 20.0, 1e-8, "Beta affine median");
    }

    #[test]
    fn beta_quantile_at_zero_is_lo() {
        let d = Distribution::Beta {
            alpha: 2.0,
            beta: 5.0,
            lo: 1.0,
            hi: 7.0,
        };
        assert_close(d.quantile(0.0), 1.0, 1e-12, "Beta lo edge");
    }

    #[test]
    fn beta_quantile_at_one_is_hi() {
        let d = Distribution::Beta {
            alpha: 2.0,
            beta: 5.0,
            lo: 1.0,
            hi: 7.0,
        };
        assert_close(d.quantile(1.0), 7.0, 1e-12, "Beta hi edge");
    }

    #[test]
    fn beta_uniform_special_case() {
        // Beta(1, 1) ≡ Uniform.
        let d = Distribution::Beta {
            alpha: 1.0,
            beta: 1.0,
            lo: 0.0,
            hi: 1.0,
        };
        for u in [0.1, 0.3, 0.5, 0.7, 0.9] {
            assert_close(d.quantile(u), u, 1e-9, "Beta(1,1) ≡ Uniform");
        }
    }

    #[test]
    fn beta_monotone() {
        assert_monotone_non_decreasing(&Distribution::Beta {
            alpha: 2.0,
            beta: 5.0,
            lo: 0.0,
            hi: 1.0,
        });
    }

    // ── Gamma ───────────────────────────────────────────────────────

    #[test]
    fn gamma_shape_one_collapses_to_exponential() {
        // Gamma(shape=1, scale) ≡ Exponential(rate = 1/scale).
        let d_g = Distribution::Gamma {
            shape: 1.0,
            scale: 2.0,
        };
        let d_e = Distribution::Exponential { lambda: 0.5 };
        for u in [0.1, 0.3, 0.5, 0.7, 0.9] {
            assert_close(
                d_g.quantile(u),
                d_e.quantile(u),
                1e-7,
                "Gamma(1) ≡ Exponential",
            );
        }
    }

    #[test]
    fn gamma_quantile_at_zero_is_zero() {
        let d = Distribution::Gamma {
            shape: 2.0,
            scale: 3.0,
        };
        assert_close(d.quantile(0.0), 0.0, 1e-12, "Gamma lo edge");
    }

    #[test]
    fn gamma_support() {
        let d = Distribution::Gamma {
            shape: 2.0,
            scale: 3.0,
        };
        assert_eq!(d.support(), (0.0, f64::INFINITY));
    }

    #[test]
    fn gamma_monotone() {
        assert_monotone_non_decreasing(&Distribution::Gamma {
            shape: 2.0,
            scale: 3.0,
        });
    }

    // ── Weibull ─────────────────────────────────────────────────────

    #[test]
    fn weibull_shape_one_collapses_to_exponential() {
        // Weibull(shape=1, scale) ≡ Exponential(rate = 1/scale).
        let d_w = Distribution::Weibull {
            shape: 1.0,
            scale: 4.0,
        };
        let d_e = Distribution::Exponential { lambda: 0.25 };
        for u in [0.1, 0.3, 0.5, 0.7, 0.9] {
            assert_close(
                d_w.quantile(u),
                d_e.quantile(u),
                1e-12,
                "Weibull(1) ≡ Exponential",
            );
        }
    }

    #[test]
    fn weibull_quantile_at_zero_is_zero() {
        let d = Distribution::Weibull {
            shape: 2.0,
            scale: 1.0,
        };
        assert_close(d.quantile(0.0), 0.0, 1e-12, "Weibull lo edge");
    }

    #[test]
    fn weibull_quantile_at_one_is_infinity() {
        let d = Distribution::Weibull {
            shape: 2.0,
            scale: 1.0,
        };
        assert_eq!(d.quantile(1.0), f64::INFINITY);
    }

    #[test]
    fn weibull_monotone() {
        assert_monotone_non_decreasing(&Distribution::Weibull {
            shape: 2.0,
            scale: 1.0,
        });
    }

    // ── Exponential ─────────────────────────────────────────────────

    #[test]
    fn exponential_quantile_at_zero_is_zero() {
        let d = Distribution::Exponential { lambda: 1.0 };
        assert_eq!(d.quantile(0.0), 0.0);
    }

    #[test]
    fn exponential_quantile_at_one_is_infinity() {
        let d = Distribution::Exponential { lambda: 1.0 };
        assert_eq!(d.quantile(1.0), f64::INFINITY);
    }

    #[test]
    fn exponential_quantile_known_point() {
        // Q(1 - e⁻¹) = 1/λ.
        let lambda = 2.0_f64;
        let d = Distribution::Exponential { lambda };
        let u = 1.0 - (-1.0_f64).exp();
        assert_close(d.quantile(u), 1.0 / lambda, 1e-12, "Exponential @1/λ");
    }

    #[test]
    fn exponential_monotone() {
        assert_monotone_non_decreasing(&Distribution::Exponential { lambda: 1.0 });
    }

    // ── Bernoulli ───────────────────────────────────────────────────

    #[test]
    fn bernoulli_zero_p_is_always_zero() {
        let d = Distribution::Bernoulli { p: 0.0 };
        for u in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert_eq!(d.quantile(u), 0.0);
        }
    }

    #[test]
    fn bernoulli_one_p_returns_one_above_zero() {
        // For p=1, threshold = 1-p = 0. u=0 saturates to lower
        // support (0); u>0 returns 1. At Bernoulli(p=1) the marginal
        // P(X=1) = measure of (0, 1] = 1, so all u ∈ (0, 1] should
        // give 1.
        let d = Distribution::Bernoulli { p: 1.0 };
        assert_eq!(d.quantile(0.0), 0.0); // boundary; F⁻¹(0) = 0.
        for u in [0.000_001, 0.25, 0.5, 0.75, 1.0] {
            assert_eq!(d.quantile(u), 1.0);
        }
    }

    #[test]
    fn bernoulli_threshold_is_inclusive_at_one_minus_p() {
        // P(X = 0) = 1 - p; standard inverse CDF places the boundary
        // u = 1 - p on the X = 0 side (F⁻¹(1 - p) = 0).
        // u ∈ [0, 1-p] → 0; u ∈ (1-p, 1] → 1.
        let d = Distribution::Bernoulli { p: 0.3 };
        // 1 - p = 0.7.
        assert_eq!(d.quantile(0.0), 0.0);
        assert_eq!(d.quantile(0.5), 0.0);
        assert_eq!(d.quantile(0.69), 0.0);
        assert_eq!(d.quantile(0.7), 0.0); // boundary inclusive on 0 side
                                          // Just past threshold:
        assert_eq!(d.quantile(0.700_000_000_001), 1.0);
        assert_eq!(d.quantile(0.99), 1.0);
        assert_eq!(d.quantile(1.0), 1.0);
    }

    #[test]
    fn bernoulli_monotone() {
        assert_monotone_non_decreasing(&Distribution::Bernoulli { p: 0.4 });
    }

    // ── DiscreteUniform ─────────────────────────────────────────────

    #[test]
    fn discrete_uniform_singleton() {
        let d = Distribution::DiscreteUniform { lo: 5, hi: 5 };
        for u in [0.0, 0.5, 1.0] {
            assert_eq!(d.quantile(u), 5.0);
        }
    }

    #[test]
    fn discrete_uniform_two_values() {
        let d = Distribution::DiscreteUniform { lo: 0, hi: 1 };
        // n = 2; u in [0, 0.5) → 0; u in [0.5, 1.0] → 1.
        assert_eq!(d.quantile(0.0), 0.0);
        assert_eq!(d.quantile(0.49), 0.0);
        assert_eq!(d.quantile(0.5), 1.0);
        assert_eq!(d.quantile(0.99), 1.0);
        assert_eq!(d.quantile(1.0), 1.0);
    }

    #[test]
    fn discrete_uniform_six_values() {
        // Roll a six-sided die: lo=1, hi=6, n=6.
        let d = Distribution::DiscreteUniform { lo: 1, hi: 6 };
        assert_eq!(d.quantile(0.0), 1.0);
        assert_eq!(d.quantile(1.0 / 6.0 + 1e-9), 2.0); // edge of bin 1
        assert_eq!(d.quantile(0.5), 4.0); // floor(0.5 * 6) = 3, so lo + 3 = 4
        assert_eq!(d.quantile(1.0), 6.0); // saturated to last bin
    }

    #[test]
    fn discrete_uniform_negative_range() {
        let d = Distribution::DiscreteUniform { lo: -3, hi: 3 };
        assert_eq!(d.quantile(0.0), -3.0);
        assert_eq!(d.quantile(1.0), 3.0);
        let mid = d.quantile(0.5);
        // n = 7, scaled = 3.5, floor = 3, lo + 3 = 0
        assert_eq!(mid, 0.0);
    }

    #[test]
    fn discrete_uniform_monotone() {
        assert_monotone_non_decreasing(&Distribution::DiscreteUniform { lo: 1, hi: 10 });
    }

    // ── Cross-cutting: serde round-trip ─────────────────────────────

    #[test]
    fn distribution_serde_round_trip_for_all_variants() {
        let cases = vec![
            Distribution::Uniform { lo: 1.0, hi: 5.0 },
            Distribution::Normal {
                mu: 0.0,
                sigma: 2.0,
            },
            Distribution::LogNormal {
                mu_log: 1.0,
                sigma_log: 0.5,
            },
            Distribution::Triangular {
                lo: 0.0,
                mode: 0.3,
                hi: 1.0,
            },
            Distribution::Beta {
                alpha: 2.0,
                beta: 5.0,
                lo: 0.0,
                hi: 1.0,
            },
            Distribution::Gamma {
                shape: 2.0,
                scale: 1.0,
            },
            Distribution::Weibull {
                shape: 1.5,
                scale: 2.0,
            },
            Distribution::Exponential { lambda: 0.7 },
            Distribution::Bernoulli { p: 0.3 },
            Distribution::DiscreteUniform { lo: 1, hi: 6 },
        ];
        for d in cases {
            let json = serde_json::to_string(&d).expect("serialize");
            let back: Distribution = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, d, "round-trip {d:?} → {json} → {back:?}");
        }
    }

    #[test]
    fn quantile_at_zero_returns_lower_support_for_finite_distributions() {
        let cases = vec![
            (Distribution::Uniform { lo: 2.0, hi: 5.0 }, 2.0),
            (
                Distribution::Triangular {
                    lo: -1.0,
                    mode: 0.0,
                    hi: 1.0,
                },
                -1.0,
            ),
            (
                Distribution::Beta {
                    alpha: 2.0,
                    beta: 3.0,
                    lo: 0.5,
                    hi: 1.5,
                },
                0.5,
            ),
            (Distribution::Bernoulli { p: 0.4 }, 0.0),
            (Distribution::DiscreteUniform { lo: -2, hi: 2 }, -2.0),
        ];
        for (d, lo) in cases {
            assert_close(d.quantile(0.0), lo, 1e-9, "lo edge");
        }
    }

    #[test]
    fn quantile_at_one_returns_upper_support_for_finite_distributions() {
        let cases = vec![
            (Distribution::Uniform { lo: 2.0, hi: 5.0 }, 5.0),
            (
                Distribution::Triangular {
                    lo: -1.0,
                    mode: 0.0,
                    hi: 1.0,
                },
                1.0,
            ),
            (
                Distribution::Beta {
                    alpha: 2.0,
                    beta: 3.0,
                    lo: 0.5,
                    hi: 1.5,
                },
                1.5,
            ),
            (Distribution::Bernoulli { p: 0.4 }, 1.0),
            (Distribution::DiscreteUniform { lo: -2, hi: 2 }, 2.0),
        ];
        for (d, hi) in cases {
            assert_close(d.quantile(1.0), hi, 1e-9, "hi edge");
        }
    }
}
