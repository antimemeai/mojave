//! Jansen 1999 squared-difference first-order Sobol' estimator.
//!
//! Per Jansen 1999 ("Analysis of variance designs for model output").
//! The "complementary" first-order form to PR 7's total-order form
//! (Saltelli 2010 Eq f). Both use the squared-difference structure;
//! they differ only in which pair of evaluations is paired.
//!
//! # Formula
//!
//! ```text
//! Y       = f(B)                         pick-freeze "Y" series
//! Y^X     = f(A_Bⁱ)                       pair sharing column i
//!
//! S_i^Jansen = 1 − (1/(2N)) Σ (Y_j − Y^X_j)² / Var(Y)
//! ```
//!
//! Derivation: under the pick-freeze design, `Var(Y − Y^X) = 2 ·
//! Var(Y) · (1 − S_i)`, and `Var(Y − Y^X) = E[(Y − Y^X)²]` when
//! `E[Y] = E[Y^X]` (which holds in expectation for our design).
//! Rearranging gives the formula.
//!
//! # Why this alongside Saltelli2010 + Janon
//!
//! Three first-order estimators ship now from the same `(A, B, A_Bⁱ)`
//! matrix; they differ in finite-sample bias / variance tradeoffs:
//!
//! | Estimator | Form | Best for |
//! |---|---|---|
//! | Saltelli 2010 (Eq c) | Covariance with biased denominator | General default; widely cited |
//! | Janon 2014 (`T_N^X`) | Covariance with efficient joint denominator | Asymptotically optimal CI |
//! | Jansen 1999 (this module) | Squared-difference, complementary form | Numerical stability when `S_i` is close to 1 |
//!
//! Jansen's form has the property that `1 − S_i` is computed as
//! a sum of squares, which is non-negative by construction. For
//! near-saturation factors (`S_i → 1`), the squared-difference path
//! avoids the cancellation noise that Saltelli's Eq c can exhibit.
//!
//! # Determinism
//!
//! Pure under `(matrix, model)`. All sums route through `tree_sum`.
//! Same matrix + model in → bit-identical output.

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::expect_used
)]

use ndarray::Array2;
use salib_core::tree_sum;
use salib_samplers::SaltelliMatrix;

/// First-order Sobol' indices via Jansen 1999 squared-difference form.
///
/// `#[non_exhaustive]` — future fields land non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct JansenIndices {
    /// First-order Sobol' indices, length `d`. Computed via the
    /// squared-difference identity `S_i = 1 − (1/(2N)) Σ (Y − Y^X)² /
    /// Var(Y)` (Jansen 1999).
    pub first_order: Vec<f64>,
    /// Total variance estimated from the joint `(Y, Y^X)` samples.
    pub total_variance: f64,
}

impl JansenIndices {
    /// Factor count.
    #[must_use]
    pub fn d(&self) -> usize {
        self.first_order.len()
    }
}

/// Estimate first-order Sobol' indices via Jansen 1999.
pub fn estimate_jansen<F>(matrix: &SaltelliMatrix, model: F) -> JansenIndices
where
    F: Fn(&[f64]) -> f64,
{
    let n = matrix.n;
    let d = matrix.dim;
    let n_f = n as f64;

    let y = evaluate_rows(&matrix.b, &model);
    let y_x: Vec<Vec<f64>> = matrix
        .a_b
        .iter()
        .map(|m| evaluate_rows(m, &model))
        .collect();

    // Total variance from Y (population variance, 1/n).
    let mean_y = tree_sum(&y) / n_f;
    let y_sq: Vec<f64> = y.iter().map(|v| v * v).collect();
    let total_variance = tree_sum(&y_sq) / n_f - mean_y * mean_y;

    let mut first_order = Vec::with_capacity(d);
    for y_xi in &y_x {
        // (1/(2N)) Σ (Y − Y^X_i)².
        let sq_diffs: Vec<f64> = y
            .iter()
            .zip(y_xi.iter())
            .map(|(yj, yxj)| (yj - yxj).powi(2))
            .collect();
        let half_avg = tree_sum(&sq_diffs) / (2.0 * n_f);

        // S_i = 1 − half_avg / Var(Y).
        let s_i = if total_variance > 1e-15 {
            1.0 - half_avg / total_variance
        } else {
            0.0
        };
        first_order.push(s_i);
    }

    JansenIndices {
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

    #[test]
    fn output_length_matches_d() {
        let s = LhsSampler::classic(6);
        let mut rng = RngState::from_seed([0x42; 32]);
        let m = build_saltelli_matrix(&s, 64, false, &mut rng).unwrap();
        let est = estimate_jansen(&m, |x| x[0]);
        assert_eq!(est.d(), 3);
    }

    #[test]
    fn constant_model_yields_zero_indices() {
        let s = LhsSampler::classic(4);
        let mut rng = RngState::from_seed([0x42; 32]);
        let m = build_saltelli_matrix(&s, 64, false, &mut rng).unwrap();
        let est = estimate_jansen(&m, |_| 7.0);
        for &v in &est.first_order {
            assert_eq!(v, 0.0, "constant model should give S_i = 0");
        }
    }

    #[test]
    fn linear_single_factor_concentrates_first_order() {
        let s = LhsSampler::classic(6);
        let mut rng = RngState::from_seed([0x42; 32]);
        let m = build_saltelli_matrix(&s, 4096, false, &mut rng).unwrap();
        let est = estimate_jansen(&m, |x| x[0]);
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
    }

    #[test]
    fn same_matrix_yields_identical_estimates() {
        let s = LhsSampler::classic(6);
        let mut rng = RngState::from_seed([0x42; 32]);
        let m = build_saltelli_matrix(&s, 256, false, &mut rng).unwrap();
        let model = |x: &[f64]| x[0] + 0.5 * x[1] * x[2];
        let a = estimate_jansen(&m, model);
        let b = estimate_jansen(&m, model);
        assert_eq!(a.first_order, b.first_order);
    }
}
