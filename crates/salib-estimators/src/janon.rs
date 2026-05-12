//! Janon 2014 asymptotically-efficient first-order Sobol' estimator
//! (`T_N^X`).
//!
//! Per Janon-Klein-Lagnoux-Nodet-Prieur 2014 (`arXiv:1303.6451`,
//! ESAIM Probability and Statistics), Eq 6 / Eq 8. Same `(A, B, A_Bⁱ)`
//! Saltelli matrix as `saltelli2010`, but with a tighter denominator
//! that uses joint information from both `Y` and `Y^X`. Janon § 2.2
//! Prop 2.5 proves `T_N^X` is **asymptotically efficient** —
//! minimum-variance among regular estimators based on the pick-freeze
//! replications.
//!
//! # Formula (Eq 6, the formal definition)
//!
//! ```text
//! Y       = f(B)                         pick-freeze "Y" series
//! Y^X     = f(A_Bⁱ)                       paired "Y given X frozen"
//! Ȳ      = mean(Y),  Ȳ^X = mean(Y^X),  Ȳ₂ = (Ȳ + Ȳ^X) / 2
//!
//!         (1/N) Σ Y_j Y_j^X  −  Ȳ₂²
//! T_N^X = ─────────────────────────────────────────────────
//!         (1/N) Σ (Y_j² + (Y_j^X)²)/2  −  Ȳ₂²
//! ```
//!
//! Estimates `S^X = Var(E[Y|X]) / Var(Y)`. Note: the paper's Eq 8
//! "rewriting" form `Σ (Y − Ȳ₂)(Y^X − Ȳ₂) / Σ ((Y + Y^X)/2 − Ȳ₂)²`
//! is **not algebraically equivalent** to Eq 6. The numerators *are*
//! equal (algebraic identity from centering), but the denominators
//! differ:
//!
//! ```text
//! Eq 6 denom (popn) ≈ (Var(Y) + Var(Y^X)) / 2
//! Eq 8 denom (popn)  = Var((Y + Y^X)/2) = (Var(Y) + Var(Y^X) + 2·Cov(Y, Y^X)) / 4
//! ```
//!
//! Under the pick-freeze pairing, `Cov(Y, Y^X) = Var(E[Y|Xᵢ]) =
//! Sᵢ · Var(Y)`. Substituting and assuming `Var(Y) = Var(Y^X) = V`:
//!
//! ```text
//! Eq 8 denom / Eq 6 denom = (1 + Sᵢ) / 2
//! Eq 8 estimator         = Eq 6 estimator · 2 / (1 + Sᵢ)
//! ```
//!
//! For Ishigami `S₁ = 0.314`, Eq 8 inflates by `2 / 1.314 ≈ 1.52`,
//! producing `≈ 0.48` instead of the analytic `0.314` — verified
//! empirically during PR-15 implementation. We use Eq 6, the formal
//! definition. (The paper presents Eq 8 as a numerical-stability
//! rewriting; the inequivalence appears to be unintentional.)
//!
//! # Why this alongside Saltelli2010
//!
//! Same model-evaluation budget (`N(d+2)` evals; reuses the existing
//! `SaltelliMatrix`). Drop-in replacement for `estimate_saltelli2010`
//! that strictly improves asymptotic CI width — for any fixed `N`,
//! `T_N^X` has variance `≤` Saltelli's. The improvement is small at
//! large `N` and small `S` (Janon Prop 2.3: equality at `S^X = 0` or
//! `1`); meaningful at moderate `N` and intermediate `S`.
//!
//! # What this module ships
//!
//! - `JanonIndices` — first-order `S_i` per factor (no total-order;
//!   Janon's paper concerns first-order only). Pair with Jansen 1999
//!   from `saltelli2010` for total-order coverage.
//!
//! # Determinism
//!
//! Pure under `(matrix, model)`. All sums route through
//! `tree_sum` / `tree_dot`. Same matrix + model in → bit-identical
//! `JanonIndices` out.

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::expect_used
)]

use ndarray::Array2;
use salib_core::tree_sum;
use salib_samplers::SaltelliMatrix;

/// First-order Sobol' indices via Janon `T_N^X`.
///
/// `#[non_exhaustive]` — future fields (`bootstrap_ci`,
/// `total_variance` for downstream GUM) land non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct JanonIndices {
    /// First-order Sobol' indices, length `d`. Asymptotically
    /// efficient per Janon 2014 Prop 2.5.
    pub first_order: Vec<f64>,
    /// Total variance estimated from the joint `(Y, Y^X)` samples.
    pub total_variance: f64,
}

impl JanonIndices {
    /// Factor count.
    #[must_use]
    pub fn d(&self) -> usize {
        self.first_order.len()
    }
}

/// Estimate first-order Sobol' indices via Janon 2014 `T_N^X`.
///
/// Consumes the same `SaltelliMatrix` as
/// `saltelli2010::estimate_saltelli2010`. Pure function; no RNG.
pub fn estimate_janon<F>(matrix: &SaltelliMatrix, model: F) -> JanonIndices
where
    F: Fn(&[f64]) -> f64,
{
    let n = matrix.n;
    let d = matrix.dim;
    let n_f = n as f64;

    // Y = f(B), Y^X_i = f(A_B^i). The pair (Y, Y^X_i) shares column i
    // (both come from B's col i) and differs in everything else.
    let y = evaluate_rows(&matrix.b, &model);
    let y_x: Vec<Vec<f64>> = matrix
        .a_b
        .iter()
        .map(|m| evaluate_rows(m, &model))
        .collect();

    // Total variance from the Y series alone (same posture as
    // saltelli2010's diagnostic `total_variance`). The denominator
    // inside the per-factor T_N^X formula uses joint variance and
    // is computed below.
    let mean_y = tree_sum(&y) / n_f;
    let y_sq: Vec<f64> = y.iter().map(|v| v * v).collect();
    let total_variance = tree_sum(&y_sq) / n_f - mean_y * mean_y;

    let mut first_order = Vec::with_capacity(d);
    for y_xi in &y_x {
        let mean_yx = tree_sum(y_xi) / n_f;
        let mean_joint = 0.5 * (mean_y + mean_yx);

        // Numerator (Janon Eq 6): (1/N) Σ Y·Y^X − Ȳ₂².
        let yy_xi: Vec<f64> = y
            .iter()
            .zip(y_xi.iter())
            .map(|(yj, yxj)| yj * yxj)
            .collect();
        let mean_y_yx = tree_sum(&yy_xi) / n_f;
        let num = mean_y_yx - mean_joint * mean_joint;

        // Denominator (Janon Eq 6): (1/N) Σ (Y² + Y^X²)/2 − Ȳ₂².
        // This is the joint second-moment estimator that gives
        // Janon's asymptotic-efficiency property.
        let half_sq_sum: Vec<f64> = y
            .iter()
            .zip(y_xi.iter())
            .map(|(yj, yxj)| 0.5 * (yj * yj + yxj * yxj))
            .collect();
        let mean_half_sq = tree_sum(&half_sq_sum) / n_f;
        let denom = mean_half_sq - mean_joint * mean_joint;

        let s_i = if denom > 1e-15 { num / denom } else { 0.0 };
        first_order.push(s_i);
    }

    JanonIndices {
        first_order,
        total_variance,
    }
}

fn evaluate_rows<F: Fn(&[f64]) -> f64>(matrix: &Array2<f64>, model: &F) -> Vec<f64> {
    let n = matrix.shape()[0];
    let mut out = Vec::with_capacity(n);
    for row in matrix.rows() {
        let slice = row
            .as_slice()
            .expect("Array2 row should be contiguous (row-major)");
        out.push(model(slice));
    }
    out
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use super::*;
    use salib_core::RngState;
    use salib_samplers::{build_saltelli_matrix, LhsSampler};

    // ── Output shape ──────────────────────────────────────────────

    #[test]
    fn output_length_matches_d() {
        let s = LhsSampler::classic(6); // d=3 (Saltelli matrix takes 2d-dim sampler)
        let mut rng = RngState::from_seed([0x42; 32]);
        let m = build_saltelli_matrix(&s, 64, false, &mut rng).unwrap();
        let est = estimate_janon(&m, |x| x[0] + x[1]);
        assert_eq!(est.d(), 3);
    }

    // ── Constant model: zero variance, indices clamp to zero ──────

    #[test]
    fn constant_model_yields_zero_indices() {
        let s = LhsSampler::classic(4);
        let mut rng = RngState::from_seed([0x42; 32]);
        let m = build_saltelli_matrix(&s, 64, false, &mut rng).unwrap();
        let est = estimate_janon(&m, |_| 7.0);
        for &v in &est.first_order {
            assert_eq!(v, 0.0, "constant model should give S_i = 0");
        }
    }

    // ── Linear single-factor: factor 0 dominant ───────────────────

    #[test]
    fn linear_single_factor_concentrates_first_order() {
        // Y = X[0]; S_0 ≈ 1, others ≈ 0.
        let s = LhsSampler::classic(6);
        let mut rng = RngState::from_seed([0x42; 32]);
        let m = build_saltelli_matrix(&s, 4096, false, &mut rng).unwrap();
        let est = estimate_janon(&m, |x| x[0]);
        assert!(
            est.first_order[0] > 0.85,
            "S_0 = {} should be near 1",
            est.first_order[0]
        );
        assert!(
            est.first_order[1].abs() < 0.1,
            "S_1 = {} should be near 0",
            est.first_order[1]
        );
        assert!(
            est.first_order[2].abs() < 0.1,
            "S_2 = {} should be near 0",
            est.first_order[2]
        );
    }

    // ── Determinism ───────────────────────────────────────────────

    #[test]
    fn same_matrix_yields_identical_estimates() {
        let s = LhsSampler::classic(6);
        let mut rng = RngState::from_seed([0x42; 32]);
        let m = build_saltelli_matrix(&s, 256, false, &mut rng).unwrap();
        let model = |x: &[f64]| x[0] + 0.5 * x[1] * x[2];
        let a = estimate_janon(&m, model);
        let b = estimate_janon(&m, model);
        assert_eq!(a.first_order, b.first_order);
        assert_eq!(a.total_variance, b.total_variance);
    }
}
