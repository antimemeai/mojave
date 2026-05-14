//! Saltelli's 2010 first-order + total-order Sobol' index estimator.
//!
//! Per Saltelli et al. (2010), "Variance based sensitivity analysis
//! of model output. Design and estimator for the total sensitivity
//! index." The first-order index uses Eq c (Saltelli's preferred
//! form for moderate-N regimes); the total-order index uses
//! Jansen 1999's form (Eq f), which Saltelli 2010 § 4 recommends
//! as the universal best.
//!
//! # Formulas
//!
//! Given a `SaltelliMatrix` `(A, B, A_Bⁱ)` (radial design) and
//! a model `f`:
//!
//! ```text
//! fa[j]      = f(A.row(j))                 // n evals
//! fb[j]      = f(B.row(j))                 // n evals
//! fab[i][j]  = f(A_Bⁱ.row(j))              // n × d evals
//! Total = N(d+2) model evaluations.
//!
//! f_0  = (1/N) Σⱼ fa[j]
//! D    = Var(Y) = (1/N) Σⱼ fa[j]² - f_0²    // sample variance, biased form
//!
//! S_i   = (1/N) Σⱼ fb[j] · (fab[i][j] - fa[j]) / D     (Saltelli 2010 Eq c)
//! S_T_i = (1/(2N)) Σⱼ (fa[j] - fab[i][j])² / D         (Jansen 1999, Eq f)
//! ```
//!
//! # Determinism
//!
//! Pure function of `(matrix, model)`. All sums route through
//! `salib_core::reduce::tree_sum` / `tree_dot` / `tree_var` —
//! no `f64`-associativity drift under rayon partitioning (per
//! `decisions/2026-04-28-saltelli-rng-determinism.md`).
//!
//! Model evaluations are CPU-bound and synchronous; there's no RNG
//! draw inside the estimator — the `RngState` parameter belongs to
//! the bootstrap wrapper, not the point estimator.

// SA notation per Saltelli 2010: `fa`/`fab`, `s_i`/`s_t_i`. Naming
// purity fights the paper cross-reference. `cast_precision_loss`:
// `n as f64` is everywhere. `expect_used`: ndarray row.as_slice()
// is infallible on row-major Array2 from build_saltelli_matrix.
#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::expect_used
)]

use ndarray::Array2;
use salib_core::{tree_dot, tree_sum, tree_var};
use salib_samplers::SaltelliMatrix;

use crate::sobol_indices::SobolIndices;

/// Estimate first-order and total-order Sobol' indices via Saltelli
/// 2010 (first-order) + Jansen 1999 (total-order). Pure function;
/// no RNG.
///
/// `model` is called `n × (d + 2)` times. For a typical SA campaign
/// with N=8192 and d=3, that's 40,960 evaluations.
///
/// # Panics
///
/// Never panics under valid inputs. If `matrix` was produced by
/// `build_saltelli_matrix`, all internal invariants hold:
/// - `matrix.a.shape() == matrix.b.shape() == [n, d]`.
/// - `matrix.a_b.len() == d`, each `n × d`.
/// - `n ≥ 1`, `d ≥ 1`.
pub fn estimate_saltelli2010<F>(matrix: &SaltelliMatrix, model: F) -> SobolIndices
where
    F: Fn(&[f64]) -> f64,
{
    let n = matrix.n;
    let d = matrix.dim;

    // Evaluate model on every row of A, B, and each A_Bⁱ. Output is
    // a flat Vec<f64> per matrix.
    let fa = evaluate_rows(&matrix.a, &model);
    let fb = evaluate_rows(&matrix.b, &model);
    let fab: Vec<Vec<f64>> = matrix
        .a_b
        .iter()
        .map(|m| evaluate_rows(m, &model))
        .collect();

    // Total variance D = Var(Y), sample-estimated from fa.
    // `tree_var` returns the *unbiased* (Bessel-corrected) variance.
    // Saltelli's formulas typically use the biased estimator
    // `(1/N) Σ (fa - mean)²`. The difference is `(N-1)/N`, which
    // washes out in MC noise at N ≥ 1024 but matters for byte-exact
    // SALib differential. SALib uses `np.var` which defaults to
    // biased — match it.
    #[allow(clippy::cast_precision_loss)]
    let n_f = n as f64;
    let f0 = tree_sum(&fa) / n_f;
    let fa_sq: Vec<f64> = fa.iter().map(|x| x * x).collect();
    let d_var = tree_sum(&fa_sq) / n_f - f0 * f0;

    // Per-factor first-order and total-order.
    let mut first_order = Vec::with_capacity(d);
    let mut total_order = Vec::with_capacity(d);

    for fab_i in &fab {
        // Saltelli 2010 Eq c:
        //   S_i = (1/N) Σⱼ fb[j] · (fab[i][j] - fa[j]) / D
        let diff: Vec<f64> = fab_i.iter().zip(fa.iter()).map(|(ab, a)| ab - a).collect();
        let s_i_num = tree_dot(&fb, &diff) / n_f;
        first_order.push(s_i_num / d_var);

        // Jansen 1999 (Saltelli 2010 Eq f):
        //   S_T_i = (1/(2N)) Σⱼ (fa[j] - fab[i][j])² / D
        let sq_diff: Vec<f64> = fa
            .iter()
            .zip(fab_i.iter())
            .map(|(a, ab)| (a - ab).powi(2))
            .collect();
        let s_t_i_num = tree_sum(&sq_diff) / (2.0 * n_f);
        total_order.push(s_t_i_num / d_var);
    }

    // Touch tree_var to surface a use; convergence-rate tests
    // compare against this for diagnostic purposes.
    let _diagnostic_var = tree_var(&fa);

    SobolIndices::new(n, d, d_var, first_order, total_order, None)
}

/// Internal: call `model` on every row of an ndarray matrix and
/// return the values as a flat `Vec<f64>`. Row-major iteration; same
/// order downstream sums consume.
fn evaluate_rows<F>(matrix: &Array2<f64>, model: &F) -> Vec<f64>
where
    F: Fn(&[f64]) -> f64,
{
    let n = matrix.shape()[0];
    let mut out = Vec::with_capacity(n);
    for row in matrix.rows() {
        // ndarray row views in row-major Array2 are contiguous —
        // .as_slice() succeeds. (build_saltelli_matrix uses
        // Array2::zeros which is row-major-contiguous.)
        let slice = row
            .as_slice()
            .expect("Array2 row should be contiguous (row-major default)");
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

    fn fresh_rng() -> RngState {
        RngState::from_seed([0x42; 32])
    }

    // ── Trivial models ──────────────────────────────────────────────

    #[test]
    fn constant_model_yields_zero_variance() {
        let s = LhsSampler::classic(4); // d = 2
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 64, false, &mut rng).unwrap();
        let indices = estimate_saltelli2010(&m, |_x| 7.0);
        // Constant model: variance = 0, indices undefined. The math
        // produces NaN or Inf; assert variance is 0 within FP.
        assert!(indices.total_variance.abs() < 1e-12);
        // Indices are 0/0 = NaN; that's expected behavior, not a bug.
        // Estimators surfacing NaN to the caller is the right signal.
    }

    #[test]
    fn purely_linear_first_factor_concentrates_indices() {
        // Y = X_1. All variance attributable to factor 0; factor 1 = 0.
        let s = LhsSampler::classic(4); // d = 2
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 4096, false, &mut rng).unwrap();
        let indices = estimate_saltelli2010(&m, |x| x[0]);
        // S_1 should be near 1, S_2 near 0.
        assert!(
            indices.first_order[0] > 0.9,
            "S_1 = {}",
            indices.first_order[0]
        );
        assert!(
            indices.first_order[1].abs() < 0.1,
            "S_2 = {}",
            indices.first_order[1]
        );
        // Total-order similar: S_T_1 near 1, S_T_2 near 0.
        assert!(indices.total_order[0] > 0.9);
        assert!(indices.total_order[1].abs() < 0.1);
    }

    #[test]
    fn additive_model_indices_are_close_to_the_factor_share() {
        // Y = X_0 + 2*X_1. Var(Y) = Var(X_0) + 4*Var(X_1) = 1/12 + 4/12 = 5/12.
        // S_0 = (1/12) / (5/12) = 1/5 = 0.2.
        // S_1 = (4/12) / (5/12) = 4/5 = 0.8.
        let s = LhsSampler::classic(4); // d = 2
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 4096, false, &mut rng).unwrap();
        let indices = estimate_saltelli2010(&m, |x| x[0] + 2.0 * x[1]);
        // MC noise at N=4096 ~ 1/sqrt(4096) ≈ 0.016 in S_i units.
        // Allow 0.05 tolerance.
        assert!(
            (indices.first_order[0] - 0.2).abs() < 0.05,
            "S_0 = {}",
            indices.first_order[0]
        );
        assert!(
            (indices.first_order[1] - 0.8).abs() < 0.05,
            "S_1 = {}",
            indices.first_order[1]
        );
    }

    // ── Output shape ────────────────────────────────────────────────

    #[test]
    fn indices_have_correct_dim() {
        let s = LhsSampler::classic(6); // d = 3
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 256, false, &mut rng).unwrap();
        let indices = estimate_saltelli2010(&m, |x| x[0] + x[1] + x[2]);
        assert_eq!(indices.dim, 3);
        assert_eq!(indices.first_order.len(), 3);
        assert_eq!(indices.total_order.len(), 3);
        assert_eq!(indices.n, 256);
    }

    // ── Identity properties (model-free) ───────────────────────────

    #[test]
    fn first_order_at_most_total_order_within_mc_noise() {
        // For any model, S_i ≤ S_T_i. MC noise can flip signs at
        // small N; tolerance should accommodate.
        let s = LhsSampler::classic(6);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 4096, false, &mut rng).unwrap();
        let indices = estimate_saltelli2010(&m, |x| x[0].sin() + x[1] * x[2] + x[0] * x[2]);
        for i in 0..3 {
            // MC noise can produce negative S or S_T at small N; the
            // population-level claim holds modulo MC noise.
            assert!(
                indices.total_order[i] + 0.05 >= indices.first_order[i],
                "S_T[{i}] = {} < S[{i}] = {} (more than 0.05 below)",
                indices.total_order[i],
                indices.first_order[i]
            );
        }
    }

    // ── Determinism ─────────────────────────────────────────────────

    #[test]
    fn same_matrix_same_model_produces_identical_indices() {
        let s = LhsSampler::classic(4);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 1024, false, &mut rng).unwrap();
        let i1 = estimate_saltelli2010(&m, |x| x[0].powi(2) + x[1]);
        let i2 = estimate_saltelli2010(&m, |x| x[0].powi(2) + x[1]);
        assert_eq!(i1, i2);
    }

    // ── total_variance reflects the model ──────────────────────────

    #[test]
    fn total_variance_increases_with_model_amplitude() {
        let s = LhsSampler::classic(4);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 1024, false, &mut rng).unwrap();
        let small = estimate_saltelli2010(&m, |x| x[0] * 0.1);
        let large = estimate_saltelli2010(&m, |x| x[0] * 10.0);
        // Var(c * X) = c² * Var(X), so 100x amplitude → 10000x variance.
        assert!(
            large.total_variance > small.total_variance * 1000.0,
            "small={} large={}",
            small.total_variance,
            large.total_variance
        );
    }

    // ── Edge cases ──────────────────────────────────────────────────

    #[test]
    fn d_one_is_handled() {
        // Minimum d. sampler.dim() = 2 → d = 1.
        let s = LhsSampler::classic(2);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 256, false, &mut rng).unwrap();
        let indices = estimate_saltelli2010(&m, |x| x[0].powi(3));
        assert_eq!(indices.dim, 1);
        // Single factor explains everything: S_0 ≈ S_T0 ≈ 1.
        assert!(
            indices.first_order[0] > 0.9,
            "S_0 = {}",
            indices.first_order[0]
        );
    }
}
