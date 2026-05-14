//! Owen 2013 "Correlation 2" first-order Sobol' estimator —
//! variance-optimal in the small-`Sᵢ` regime.
//!
//! Per Owen 2013 ("Better estimation of small Sobol' sensitivity
//! indices", ACM TOMACS 23(2)). Uses **three** independent random
//! vectors `(x, y, z)` (= `A`, `B`, `C` in our notation) instead
//! of the two-vector `(A, B)` design Saltelli 2010 uses. Achieves
//! `O(ε⁴)` variance in the "total insensitivity limit" (where all
//! factor-of-interest variances scale as `ε`); Saltelli 2010 attains
//! only `O(ε²)` in the same limit (Owen 2013 § 6).
//!
//! # Formula (Owen Eq 7, "Correlation 2")
//!
//! For each factor `i` (with `u = {i}`):
//!
//! ```text
//! S_i = (1/N) Σ_j (f(A_j) − f(A_Cⁱ_j)) · (f(B_Aⁱ_j) − f(B_j)) / Var(Y)
//!
//! where:
//!     f(A_j)        — full random sample
//!     f(A_Cⁱ_j)     — A with col i from C  ("z_{i,u}:x_{i,−u}" in Owen's notation)
//!     f(B_Aⁱ_j)     — B with col i from A  ("x_{i,u}:y_{i,−u}")
//!     f(B_j)        — full random sample
//! ```
//!
//! Cost: `n · (3 + 2d)` model evaluations vs Saltelli2010's `n(d + 2)`.
//! Owen pays roughly **2× the model-eval budget** for substantially
//! better MC variance on factors near `S_i = 0`.
//!
//! # When to use
//!
//! Per Owen 2013 Table 1 + § 4:
//!
//! - **Use Owen** when most factors are unimportant (typical for
//!   high-dimensional screening), or when you specifically need
//!   tight CIs on small-`Sᵢ` factors.
//! - **Use Saltelli2010 / Janon** when factor variances are
//!   moderate-to-large; Owen offers no advantage and pays the
//!   doubled-cost penalty.
//! - **Use Janon** when you want the asymptotically efficient
//!   estimator at the standard `n(d + 2)` cost.
//!
//! # What this module ships
//!
//! - `OwenIndices` — first-order `S_i` per factor. No total-order;
//!   Owen 2013 concerns first-order only. Pair with Jansen 1999
//!   (PR 7) for total-order.
//!
//! # Determinism
//!
//! Pure under `(matrix, model)`. All sums route through
//! `tree_sum` / `tree_dot`. Same matrix + model in → bit-identical
//! output.

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::expect_used
)]

use ndarray::Array2;
use salib_core::tree_sum;
use salib_samplers::OwenMatrix;

/// First-order Sobol' indices via Owen Correlation 2.
///
/// `#[non_exhaustive]` — future fields land non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct OwenIndices {
    /// First-order Sobol' indices, length `d`.
    pub first_order: Vec<f64>,
    /// Total variance estimated from the joint `(A, B)` samples.
    pub total_variance: f64,
    /// Second-order indices. Always `None` for Owen — the 3-matrix
    /// `(A, B, C)` design lacks the `A_B` matrices needed for the
    /// Saltelli 2010 Eq d formula.
    pub second_order: Option<Vec<Vec<f64>>>,
}

impl OwenIndices {
    /// Factor count.
    #[must_use]
    pub fn d(&self) -> usize {
        self.first_order.len()
    }
}

/// Estimate first-order Sobol' indices via Owen 2013 Correlation 2.
///
/// Pure function; no RNG.
pub fn estimate_owen<F>(matrix: &OwenMatrix, model: F) -> OwenIndices
where
    F: Fn(&[f64]) -> f64,
{
    let n = matrix.n;
    let d = matrix.dim;
    let n_f = n as f64;

    // Base evaluations.
    let fa = evaluate_rows(&matrix.a, &model);
    let fb = evaluate_rows(&matrix.b, &model);
    // Hybrid evaluations per factor.
    let fac: Vec<Vec<f64>> = matrix
        .a_c
        .iter()
        .map(|m| evaluate_rows(m, &model))
        .collect();
    let fba: Vec<Vec<f64>> = matrix
        .b_a
        .iter()
        .map(|m| evaluate_rows(m, &model))
        .collect();

    // Total variance via the pooled `A ∪ B` sample (2N values
    // total). Owen 2013 Tables 2-3 use the analytic σ² as
    // denominator; the paper does not pin a specific empirical
    // pool. Pooling A and B is lower-variance than using either
    // alone and consistent with the paper's spirit.
    let mut combined = Vec::with_capacity(2 * n);
    combined.extend_from_slice(&fa);
    combined.extend_from_slice(&fb);
    let mean_combined = tree_sum(&combined) / (2.0 * n_f);
    let sq: Vec<f64> = combined
        .iter()
        .map(|v| (v - mean_combined).powi(2))
        .collect();
    let total_variance = tree_sum(&sq) / (2.0 * n_f);

    let mut first_order = Vec::with_capacity(d);
    for i in 0..d {
        // Owen Eq 7 / Correlation 2:
        //   S_i ∝ (1/N) Σ (f(A) − f(A_Cⁱ)) · (f(B_Aⁱ) − f(B)).
        let terms: Vec<f64> = (0..n)
            .map(|j| (fa[j] - fac[i][j]) * (fba[i][j] - fb[j]))
            .collect();
        let num = tree_sum(&terms) / n_f;
        let s_i = if total_variance > 1e-15 {
            num / total_variance
        } else {
            0.0
        };
        first_order.push(s_i);
    }

    OwenIndices {
        first_order,
        total_variance,
        second_order: None,
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
    use salib_samplers::{build_owen_matrix, LhsSampler};

    #[test]
    fn output_length_matches_d() {
        let s = LhsSampler::classic(9); // 3d = 9, d = 3
        let mut rng = RngState::from_seed([0x42; 32]);
        let m = build_owen_matrix(&s, 64, &mut rng).unwrap();
        let est = estimate_owen(&m, |x| x[0]);
        assert_eq!(est.d(), 3);
    }

    #[test]
    fn constant_model_yields_zero_indices() {
        let s = LhsSampler::classic(6);
        let mut rng = RngState::from_seed([0x42; 32]);
        let m = build_owen_matrix(&s, 64, &mut rng).unwrap();
        let est = estimate_owen(&m, |_| 7.0);
        for &v in &est.first_order {
            assert_eq!(v, 0.0);
        }
    }

    #[test]
    fn linear_single_factor_concentrates_first_order() {
        let s = LhsSampler::classic(9);
        let mut rng = RngState::from_seed([0x42; 32]);
        let m = build_owen_matrix(&s, 4096, &mut rng).unwrap();
        let est = estimate_owen(&m, |x| x[0]);
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

    #[test]
    fn small_factor_index_is_well_bounded() {
        // Y = 2·X[0] + 0.001·X[1]: factor 1 has tiny effect (S_1 ≈ 1e-6).
        // Owen should give near-zero S_1 with low variance even at small N.
        let s = LhsSampler::classic(9);
        let mut rng = RngState::from_seed([0x42; 32]);
        let m = build_owen_matrix(&s, 1024, &mut rng).unwrap();
        let est = estimate_owen(&m, |x| 2.0 * x[0] + 0.001 * x[1]);
        // S_0 dominates.
        assert!(est.first_order[0] > 0.9);
        // S_1 is "Owen-suppressed near zero" (~1e-6 analytic).
        assert!(
            est.first_order[1].abs() < 0.01,
            "S_1 = {} should be near 0 (analytic ~1e-6)",
            est.first_order[1]
        );
    }

    #[test]
    fn same_matrix_yields_identical_estimates() {
        let s = LhsSampler::classic(9);
        let mut rng = RngState::from_seed([0x42; 32]);
        let m = build_owen_matrix(&s, 256, &mut rng).unwrap();
        let model = |x: &[f64]| x[0] + x[1] * x[2];
        let a = estimate_owen(&m, model);
        let b = estimate_owen(&m, model);
        assert_eq!(a.first_order, b.first_order);
    }
}
