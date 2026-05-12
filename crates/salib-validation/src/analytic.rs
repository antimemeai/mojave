//! Closed-form analytic Sobol' indices for canonical test functions.
//!
//! `SobolIndicesAnalytic` is the *ground truth* the reviewer-affordance
//! contract (per `decisions/2026-04-28-saltelli-tck-posture.md`)
//! demands every estimator PR converge to. The convergence-rate test
//! at Layer 4 of the validation strategy compares estimator output
//! against the analytic indices in this struct as N grows.
//!
//! # Why a single shape across functions
//!
//! Ishigami, Sobol' G, Sobol' G\*, Bratley, Oakley-O'Hagan all admit
//! Sobol-decomposition closed forms with the same shape:
//! `(total_variance, first_order, total_order)`. A single
//! `SobolIndicesAnalytic` type lets every estimator PR's
//! convergence-rate test follow the same code path regardless of
//! which test function is being targeted.
//!
//! # What this *isn't*
//!
//! Morris-test (Morris 1991 §4) has analytic ground truth in the form
//! of elementary-effects values (`μ`, `μ*`, `σ`), not Sobol indices.
//! That's a shape-mismatch with `SobolIndicesAnalytic` — Morris-test
//! lands with its own `MorrisEffectsAnalytic` type alongside the
//! Morris estimator (PR 8 of `plans/0002-saltelli-roadmap.md`), per
//! `decisions/2026-04-28-salib-validation-pattern.md` § "What this
//! gates — NOT gated."
//!
//! Second-order indices (`V_ij`) and higher-order interactions are
//! also out of scope today; future fields land non-breaking via
//! `#[non_exhaustive]`.

/// Closed-form Sobol' decomposition for a test function evaluated
/// with known parameters.
///
/// All three fields are over the same factor index space — `dim()`
/// returns `first_order.len()` which equals `total_order.len()` by
/// invariant. Validity invariants:
///
/// - `total_variance > 0` — degenerate (constant-output) functions
///   are not represented; every test function in `salib-validation`
///   has nonzero variance for the canonical parameters.
/// - `first_order[i] >= 0` and `total_order[i] >= 0` for all `i`
///   (Sobol' indices are non-negative by construction).
/// - `total_order[i] >= first_order[i]` for all `i` (total bounds
///   first; equality iff factor `i` has no interactions).
/// - `first_order.iter().sum::<f64>() <= 1.0` (sum of first-order
///   indices ≤ 1 for any well-defined Sobol' decomposition).
///
/// The struct is `#[non_exhaustive]` — `second_order: Vec<Vec<f64>>`,
/// `dummy_floor: Option<f64>` (Sobol' 2007 dummy-parameter floor),
/// and other fields land non-breaking via follow-on ADRs.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SobolIndicesAnalytic {
    /// `D = Var(Y)` — the total output variance.
    pub total_variance: f64,
    /// `S_i = V_i / D` — per-factor first-order indices, indexed by
    /// factor position in the matching `Problem`'s `factors` vector.
    pub first_order: Vec<f64>,
    /// `S_T_i = V_T_i / D` — per-factor total-order indices,
    /// indexed identically to `first_order`.
    pub total_order: Vec<f64>,
}

impl SobolIndicesAnalytic {
    /// Construct from raw values. Pure data-class constructor; no
    /// validation. Test-function modules (`ishigami`, `sobol_g`,
    /// etc.) call this with closed-form-derived values.
    #[must_use]
    pub fn new(total_variance: f64, first_order: Vec<f64>, total_order: Vec<f64>) -> Self {
        Self {
            total_variance,
            first_order,
            total_order,
        }
    }

    /// Number of factors. Equal to `first_order.len()` and
    /// `total_order.len()` by invariant.
    #[must_use]
    pub fn dim(&self) -> usize {
        self.first_order.len()
    }
}

/// Closed-form Morris elementary-effects ground truth for a test
/// function. Mirrors `salib_estimators::MorrisEffects`'s shape
/// (`μ`, `μ*`, `σ` per factor) so convergence-rate tests can
/// compare estimator output directly against analytic values.
///
/// Per `decisions/2026-04-29-saltelli-morris-estimator.md` § "What
/// this gates — Mechanized."
///
/// Validity invariants:
/// - `mu_star[i] ≥ |mu[i]|` for all `i` (per Campolongo 2007;
///   `mean(|x|) ≥ |mean(x)|`).
/// - `sigma[i] ≥ 0`.
///
/// `#[non_exhaustive]` — future fields (`r_target: usize` for
/// per-factor required trajectory count under Morris's significance
/// criterion, etc.) land non-breaking.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MorrisEffectsAnalytic {
    /// `μ_i` per factor — signed mean elementary effect.
    pub mu: Vec<f64>,
    /// `μ*_i` per factor — Campolongo 2007's absolute-value
    /// statistic.
    pub mu_star: Vec<f64>,
    /// `σ_i` per factor — std of elementary effects.
    pub sigma: Vec<f64>,
}

impl MorrisEffectsAnalytic {
    /// Construct from raw values.
    #[must_use]
    pub fn new(mu: Vec<f64>, mu_star: Vec<f64>, sigma: Vec<f64>) -> Self {
        Self { mu, mu_star, sigma }
    }

    /// Number of factors.
    #[must_use]
    pub fn dim(&self) -> usize {
        self.mu.len()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn new_preserves_fields() {
        let s = SobolIndicesAnalytic::new(2.5, vec![0.3, 0.5], vec![0.4, 0.6]);
        assert_eq!(s.total_variance, 2.5);
        assert_eq!(s.first_order, vec![0.3, 0.5]);
        assert_eq!(s.total_order, vec![0.4, 0.6]);
    }

    #[test]
    fn dim_matches_factor_count() {
        let s = SobolIndicesAnalytic::new(1.0, vec![0.1; 7], vec![0.2; 7]);
        assert_eq!(s.dim(), 7);
    }
}
