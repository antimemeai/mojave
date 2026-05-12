//! Sobol' index types — point estimates and bootstrap-CI extensions.
//!
//! Mirrors `salib_validation::SobolIndicesAnalytic`'s shape (per
//! `decisions/2026-04-28-salib-validation-pattern.md`) so that
//! convergence-rate tests can compare estimator output directly
//! against analytic ground truth field-by-field.

/// Sobol' first-order and total-order indices, point-estimate.
/// Output of `estimate_saltelli2010` and friends.
///
/// `#[non_exhaustive]` — future fields (`second_order: Vec<Vec<f64>>`,
/// `dummy_floor: Option<f64>`, etc.) land non-breaking.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SobolIndices {
    /// Sample size used in the estimate (rows in each `SaltelliMatrix`
    /// matrix).
    pub n: usize,
    /// Factor count.
    pub dim: usize,
    /// `D = Var(Y)` — total output variance, sample-estimated.
    pub total_variance: f64,
    /// `Sᵢ` per factor.
    pub first_order: Vec<f64>,
    /// `S_Tᵢ` per factor.
    pub total_order: Vec<f64>,
}

impl SobolIndices {
    /// Construct from raw values. No validation; estimator code is
    /// the only producer.
    #[must_use]
    pub fn new(
        n: usize,
        dim: usize,
        total_variance: f64,
        first_order: Vec<f64>,
        total_order: Vec<f64>,
    ) -> Self {
        Self {
            n,
            dim,
            total_variance,
            first_order,
            total_order,
        }
    }
}

/// `SobolIndices` plus per-factor bootstrap confidence intervals.
///
/// `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SobolIndicesWithCi {
    /// Point-estimate indices on the original (non-bootstrapped) data.
    pub indices: SobolIndices,
    /// 95% percentile CI per first-order index `(low, high)`.
    pub first_order_ci: Vec<(f64, f64)>,
    /// 95% percentile CI per total-order index `(low, high)`.
    pub total_order_ci: Vec<(f64, f64)>,
    /// Number of bootstrap resamples used.
    pub bootstrap_resamples: usize,
    /// Bootstrap method used.
    pub method: BootstrapMethod,
}

/// Bootstrap-CI estimation method. `#[non_exhaustive]`.
///
/// Today: `Percentile` only — matches `SALib`'s default.
/// `BCa` (bias-corrected accelerated, DiCiccio-Efron 1996) is
/// deferred to a follow-on PR; sky-claude's spec § 5.2 has it as
/// default but the percentile method is sufficient for PR 7's
/// reviewer-affordance contract close.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BootstrapMethod {
    /// Naive percentile bootstrap CI: take `α/2` and `1 - α/2`
    /// percentiles of the bootstrap distribution. `SALib`-compatible.
    Percentile,
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn sobol_indices_new_preserves_fields() {
        let s = SobolIndices::new(64, 3, 12.5, vec![0.3, 0.5, 0.0], vec![0.4, 0.5, 0.2]);
        assert_eq!(s.n, 64);
        assert_eq!(s.dim, 3);
        assert_eq!(s.total_variance, 12.5);
        assert_eq!(s.first_order, vec![0.3, 0.5, 0.0]);
        assert_eq!(s.total_order, vec![0.4, 0.5, 0.2]);
    }

    #[test]
    fn bootstrap_method_percentile_implements_eq() {
        assert_eq!(BootstrapMethod::Percentile, BootstrapMethod::Percentile);
    }
}
