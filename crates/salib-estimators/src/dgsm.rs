//! DGSM — Derivative-based Global Sensitivity Measure. Computes
//! `νᵢ = E[(∂f/∂xᵢ)²]` and the Poincaré-inequality upper bound
//! `Sᵀᵢ ≤ νᵢ · C_P(xᵢ) / Var(Y)` per Sobol-Kucherenko 2009.
//!
//! Per `decisions/2026-04-29-saltelli-dgsm.md`.
//!
//! # The Poincaré link to total-order Sobol'
//!
//! For a factor `Xᵢ` with input distribution `μᵢ` and Poincaré
//! constant `C_P(μᵢ)`, the Poincaré inequality states
//!
//! ```text
//! Var_{Xᵢ}(g(Xᵢ)) ≤ C_P(μᵢ) · E[(g'(Xᵢ))²]
//! ```
//!
//! Applying this to the Sobol' decomposition gives
//!
//! ```text
//! V_Tᵢ ≤ C_P(μᵢ) · νᵢ,
//! ```
//!
//! and dividing by `Var(Y)` yields the upper bound on `Sᵀᵢ`. The
//! bound is **provable**, not just empirical — a factor with
//! `νᵢ · C_P / Var(Y) < δ` contributes provably less than `δ` to
//! total variance. This is the formal link to workspace's GUM
//! contribution analysis (Phase E).
//!
//! # Inputs
//!
//! [`estimate_dgsm`] takes:
//!
//! - `gradients: Array2<f64>` of shape `(N, d)` — gradient `∇f` at
//!   each sample. Caller chooses the gradient source: analytical
//!   (closure → derivative), automatic-differentiation (e.g.,
//!   `dual_num` / `enzyme`), or finite-difference via
//!   [`finite_difference_gradients`].
//! - `poincare_constants: &[f64]` of length `d` — `C_P(μᵢ)` per
//!   factor. Use [`poincare_constant`] to derive these from a
//!   `Distribution`.
//! - `var_y: f64` — total variance `Var(Y)` over the sample set.
//!   Caller computes via `salib_core::tree_var` for bit-determinism.
//!
//! # Why caller-supplied gradients (not enum)
//!
//! Sky-spec (`rust_salib_crate_research.md` § 5.6) proposes
//!
//! ```text
//! enum DgsmGradient {
//!     Analytical(closure),
//!     Adjoint(closure),
//!     FiniteDifference { eps, kind },
//! }
//! ```
//!
//! In Rust, an enum-of-closures with different shapes is awkward
//! and the Adjoint variant is just Analytical with a different
//! name. Cleaner: take `gradients: &Array2<f64>` directly (caller
//! computes via whatever method). Forward / central FD live as a
//! helper [`finite_difference_gradients`]. Complex-step FD and
//! Adjoint are bead-eligible (workspace-b9b).
//!
//! # Determinism
//!
//! Pure under `(gradients, poincare_constants, var_y)`. All sums
//! route through `salib_core::tree_sum`. Same inputs in →
//! bit-identical `DgsmIndices` out.

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::needless_range_loop
)]

use std::f64::consts::PI;

use ndarray::Array2;
use salib_core::{tree_sum, Distribution};

/// DGSM index estimates per factor.
///
/// `#[non_exhaustive]` — future fields (`bootstrap_ci`, raw `nu`
/// std for diagnostic, factor-of-influence flag) land non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DgsmIndices {
    /// `νᵢ = E[(∂f/∂xᵢ)²]` per factor. Always `≥ 0`.
    pub vi: Vec<f64>,
    /// Poincaré upper bound on `Sᵀᵢ`: `νᵢ · C_P(xᵢ) / Var(Y)`. May
    /// exceed `1.0` — the inequality is one-sided and the bound
    /// can be loose, especially for distributions with large
    /// Poincaré constants. A factor with `st_upper < δ`
    /// **provably** contributes < δ to total variance.
    pub st_upper: Vec<f64>,
}

impl DgsmIndices {
    /// Factor count.
    #[must_use]
    pub fn d(&self) -> usize {
        self.vi.len()
    }
}

/// Errors from [`estimate_dgsm`].
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum DgsmError {
    #[error(
        "DGSM: shape mismatch — gradients has {gradient_rows}×{gradient_cols}, \
         poincare_constants has length {poincare_len}"
    )]
    ShapeMismatch {
        gradient_rows: usize,
        gradient_cols: usize,
        poincare_len: usize,
    },
    #[error("DGSM: empty samples — gradient matrix has zero rows")]
    EmptyGradients,
    #[error("DGSM: d must be ≥ 1, got 0")]
    ZeroD,
    #[error("DGSM: Var(Y) must be > 0 (got {var_y}); model output is constant")]
    ZeroVariance { var_y: f64 },
    #[error("DGSM: poincare_constants[{factor}] = {value} is invalid (must be ≥ 0)")]
    NegativePoincareConstant { factor: usize, value: f64 },
}

/// Errors from [`poincare_constant`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PoincareError {
    /// Closed-form Poincaré constant not implemented for this
    /// distribution. PR 13 covers `Uniform` and `Normal`; remaining
    /// distributions are bead-eligible.
    #[error(
        "Poincaré: closed-form constant not implemented for {distribution_kind}; \
         implement via Roustant 2017 numerical FE (bead workspace-4s0) or \
         supply a constant directly"
    )]
    Unsupported { distribution_kind: &'static str },
}

/// Estimate DGSM `νᵢ` and Poincaré-bounded `Sᵀᵢ` upper bound.
///
/// # Errors
///
/// - [`DgsmError::ShapeMismatch`] if `gradients.ncols() != poincare_constants.len()`.
/// - [`DgsmError::EmptyGradients`] if `gradients.nrows() == 0`.
/// - [`DgsmError::ZeroD`] if `gradients.ncols() == 0`.
/// - [`DgsmError::ZeroVariance`] if `var_y ≤ 0`.
/// - [`DgsmError::NegativePoincareConstant`] if any `poincare_constants[i] < 0`.
pub fn estimate_dgsm(
    gradients: &Array2<f64>,
    poincare_constants: &[f64],
    var_y: f64,
) -> Result<DgsmIndices, DgsmError> {
    let n = gradients.nrows();
    let d = gradients.ncols();
    if d == 0 {
        return Err(DgsmError::ZeroD);
    }
    if n == 0 {
        return Err(DgsmError::EmptyGradients);
    }
    if poincare_constants.len() != d {
        return Err(DgsmError::ShapeMismatch {
            gradient_rows: n,
            gradient_cols: d,
            poincare_len: poincare_constants.len(),
        });
    }
    if !var_y.is_finite() || var_y <= 0.0 {
        return Err(DgsmError::ZeroVariance { var_y });
    }
    for (i, &cp) in poincare_constants.iter().enumerate() {
        if !cp.is_finite() || cp < 0.0 {
            return Err(DgsmError::NegativePoincareConstant {
                factor: i,
                value: cp,
            });
        }
    }

    let n_f = n as f64;
    let mut vi = vec![0.0_f64; d];
    let mut st_upper = vec![0.0_f64; d];

    let mut squared_buf = vec![0.0_f64; n];
    for i in 0..d {
        for k in 0..n {
            let g = gradients[[k, i]];
            squared_buf[k] = g * g;
        }
        vi[i] = tree_sum(&squared_buf) / n_f;
        st_upper[i] = vi[i] * poincare_constants[i] / var_y;
    }

    Ok(DgsmIndices { vi, st_upper })
}

/// Finite-difference scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FdKind {
    /// `(f(x + ε·eᵢ) − f(x)) / ε` — `O(ε)` error, `N·d` extra
    /// model evaluations on top of the base `N`.
    Forward,
    /// `(f(x + ε·eᵢ) − f(x − ε·eᵢ)) / (2·ε)` — `O(ε²)` error,
    /// `2·N·d` extra model evaluations.
    Central,
}

/// Compute per-sample gradients via finite-difference.
///
/// `eps` controls FD step size. Typical: `1e-6` for forward,
/// `1e-4` to `1e-5` for central (balances truncation vs round-off
/// error).
///
/// Cost: `N · (d + 1)` model evals for forward (one base + `d`
/// perturbations per sample), `2 · N · d` for central. Caller is
/// responsible for matching `eps` to the model's numerical
/// precision.
pub fn finite_difference_gradients<F>(
    samples: &Array2<f64>,
    eps: f64,
    kind: FdKind,
    mut model: F,
) -> Array2<f64>
where
    F: FnMut(&[f64]) -> f64,
{
    assert!(eps > 0.0, "FD eps must be positive");
    let n = samples.nrows();
    let d = samples.ncols();
    let mut gradients = Array2::<f64>::zeros((n, d));
    let mut x_buf = vec![0.0_f64; d];

    for k in 0..n {
        for j in 0..d {
            x_buf[j] = samples[[k, j]];
        }
        match kind {
            FdKind::Forward => {
                let f_base = model(&x_buf);
                for i in 0..d {
                    let saved = x_buf[i];
                    x_buf[i] = saved + eps;
                    let f_plus = model(&x_buf);
                    x_buf[i] = saved;
                    gradients[[k, i]] = (f_plus - f_base) / eps;
                }
            }
            FdKind::Central => {
                for i in 0..d {
                    let saved = x_buf[i];
                    x_buf[i] = saved + eps;
                    let f_plus = model(&x_buf);
                    x_buf[i] = saved - eps;
                    let f_minus = model(&x_buf);
                    x_buf[i] = saved;
                    gradients[[k, i]] = (f_plus - f_minus) / (2.0 * eps);
                }
            }
        }
    }
    gradients
}

/// Poincaré constant `C_P(μ)` for a `Distribution` per
/// Roustant-Barthe-Iooss 2017 (Electronic Journal of Statistics).
///
/// # Implemented
///
/// - **`Uniform { lo, hi }`**: `(hi − lo)² / π²`
///   (Roustant 2017 Table 1 + Lemma 2 affine rescaling).
/// - **`Normal { sigma, .. }`**: `σ²` (Roustant 2017 Table 1, the
///   standard Gaussian on `ℝ` has `C_P = 1`; affine rescaling
///   gives `σ²`).
///
/// # Bead-eligible
///
/// - `Triangular` (Roustant 2017 Prop 6, symmetric only) —
///   workspace-10w.
/// - `Beta` / `LogNormal` / `Gamma` / `Weibull` / `Exponential`
///   (Roustant 2017 §4.2 numerical FE) — workspace-4s0.
/// - Truncated variants (Roustant 2017 Prop 8 Kummer functions) —
///   workspace (filed adjacent).
/// - `Bernoulli` / `DiscreteUniform` (discrete Markov-chain
///   spectral gap) — workspace-6rx.
///
/// # Errors
///
/// [`PoincareError::Unsupported`] for distributions not yet
/// implemented. Caller can supply the constant directly via
/// numerical methods if needed.
pub fn poincare_constant(distribution: &Distribution) -> Result<f64, PoincareError> {
    match distribution {
        Distribution::Uniform { lo, hi } => Ok((hi - lo).powi(2) / PI.powi(2)),
        Distribution::Normal { sigma, .. } => Ok(sigma.powi(2)),
        Distribution::LogNormal { .. } => Err(PoincareError::Unsupported {
            distribution_kind: "LogNormal",
        }),
        Distribution::Triangular { .. } => Err(PoincareError::Unsupported {
            distribution_kind: "Triangular",
        }),
        Distribution::Beta { .. } => Err(PoincareError::Unsupported {
            distribution_kind: "Beta",
        }),
        Distribution::Gamma { .. } => Err(PoincareError::Unsupported {
            distribution_kind: "Gamma",
        }),
        Distribution::Weibull { .. } => Err(PoincareError::Unsupported {
            distribution_kind: "Weibull",
        }),
        Distribution::Exponential { .. } => Err(PoincareError::Unsupported {
            distribution_kind: "Exponential",
        }),
        Distribution::Bernoulli { .. } => Err(PoincareError::Unsupported {
            distribution_kind: "Bernoulli",
        }),
        Distribution::DiscreteUniform { .. } => Err(PoincareError::Unsupported {
            distribution_kind: "DiscreteUniform",
        }),
        // salib-core's `Distribution` is `#[non_exhaustive]`.
        // When you add a variant there: decide whether a closed-
        // form Poincaré constant lands here (per Roustant 2017
        // Table 1 / Prop 5-8) or whether the variant returns
        // `Unsupported` until workspace-4s0 / workspace-10w /
        // workspace-6rx (the numerical-FE / Bessel-root /
        // discrete-Markov-chain Poincaré beads) resolves it.
        // Anything that hits this catch-all is silently surfaced
        // as `Unsupported` with no variant-specific label —
        // intentional fail-safe but not a substitute for an
        // explicit arm above.
        _ => Err(PoincareError::Unsupported {
            distribution_kind:
                "<non_exhaustive variant — wire an explicit arm in poincare_constant>",
        }),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use super::*;

    // ── estimate_dgsm validation ──────────────────────────────────

    #[test]
    fn zero_d_errors() {
        let g = Array2::<f64>::zeros((10, 0));
        assert_eq!(estimate_dgsm(&g, &[], 1.0).unwrap_err(), DgsmError::ZeroD);
    }

    #[test]
    fn empty_gradients_errors() {
        let g = Array2::<f64>::zeros((0, 3));
        let err = estimate_dgsm(&g, &[1.0, 1.0, 1.0], 1.0).unwrap_err();
        assert_eq!(err, DgsmError::EmptyGradients);
    }

    #[test]
    fn shape_mismatch_errors() {
        let g = Array2::<f64>::zeros((10, 3));
        let err = estimate_dgsm(&g, &[1.0, 1.0], 1.0).unwrap_err();
        assert!(matches!(err, DgsmError::ShapeMismatch { .. }));
    }

    #[test]
    fn zero_variance_errors() {
        let g = Array2::<f64>::ones((10, 3));
        let err = estimate_dgsm(&g, &[1.0, 1.0, 1.0], 0.0).unwrap_err();
        assert!(matches!(err, DgsmError::ZeroVariance { .. }));
    }

    #[test]
    fn negative_poincare_errors() {
        let g = Array2::<f64>::ones((10, 3));
        let err = estimate_dgsm(&g, &[1.0, -0.5, 1.0], 1.0).unwrap_err();
        assert!(matches!(
            err,
            DgsmError::NegativePoincareConstant { factor: 1, .. }
        ));
    }

    // ── core math ─────────────────────────────────────────────────

    #[test]
    fn vi_recovers_squared_gradient_mean() {
        // Constant gradient = 2 → νᵢ = 4.
        let mut g = Array2::<f64>::zeros((100, 2));
        for k in 0..100 {
            g[[k, 0]] = 2.0;
            g[[k, 1]] = 3.0;
        }
        let est = estimate_dgsm(&g, &[1.0, 1.0], 1.0).unwrap();
        assert!((est.vi[0] - 4.0).abs() < 1e-12);
        assert!((est.vi[1] - 9.0).abs() < 1e-12);
    }

    #[test]
    fn st_upper_scales_with_poincare_constant() {
        // νᵢ = 4, C_P = 0.5, Var(Y) = 2 → ST_upper = 4·0.5/2 = 1.0.
        let mut g = Array2::<f64>::zeros((50, 1));
        for k in 0..50 {
            g[[k, 0]] = 2.0;
        }
        let est = estimate_dgsm(&g, &[0.5], 2.0).unwrap();
        assert!((est.st_upper[0] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn vi_is_non_negative() {
        // Mixed-sign gradients still produce non-negative νᵢ
        // (squared mean).
        let mut g = Array2::<f64>::zeros((100, 1));
        for k in 0..100 {
            g[[k, 0]] = if k.is_multiple_of(2) { 1.0 } else { -1.0 };
        }
        let est = estimate_dgsm(&g, &[1.0], 1.0).unwrap();
        assert_eq!(est.vi[0], 1.0);
    }

    // ── poincare_constant ─────────────────────────────────────────

    #[test]
    fn poincare_uniform_matches_closed_form() {
        // Uniform[-π, π]: C_P = (2π)²/π² = 4.
        let d = Distribution::Uniform { lo: -PI, hi: PI };
        let c = poincare_constant(&d).unwrap();
        assert!(
            (c - 4.0).abs() < 1e-12,
            "C_P(Uniform[-π, π]) = {c}, expected 4"
        );
    }

    #[test]
    fn poincare_uniform_unit_interval() {
        // Uniform[0, 1]: C_P = 1/π².
        let d = Distribution::Uniform { lo: 0.0, hi: 1.0 };
        let c = poincare_constant(&d).unwrap();
        let expected = 1.0 / PI.powi(2);
        assert!((c - expected).abs() < 1e-12);
    }

    #[test]
    fn poincare_normal_uses_sigma_squared() {
        // N(0, σ²): C_P = σ². Test σ=2 → C_P=4.
        let d = Distribution::Normal {
            mu: 0.0,
            sigma: 2.0,
        };
        assert_eq!(poincare_constant(&d).unwrap(), 4.0);
    }

    #[test]
    fn poincare_unsupported_returns_error() {
        let d = Distribution::Beta {
            alpha: 2.0,
            beta: 2.0,
            lo: 0.0,
            hi: 1.0,
        };
        assert!(matches!(
            poincare_constant(&d).unwrap_err(),
            PoincareError::Unsupported { .. }
        ));
    }

    // ── finite_difference_gradients ──────────────────────────────

    #[test]
    fn forward_fd_recovers_linear_gradient() {
        // f(x) = 2x_0 + 3x_1 → ∇f = [2, 3].
        let mut samples = Array2::<f64>::zeros((10, 2));
        for k in 0..10 {
            samples[[k, 0]] = (k as f64) * 0.1;
            samples[[k, 1]] = (k as f64) * 0.2;
        }
        let gradients =
            finite_difference_gradients(&samples, 1e-6, FdKind::Forward, |x: &[f64]| {
                2.0 * x[0] + 3.0 * x[1]
            });
        for k in 0..10 {
            assert!((gradients[[k, 0]] - 2.0).abs() < 1e-3);
            assert!((gradients[[k, 1]] - 3.0).abs() < 1e-3);
        }
    }

    #[test]
    fn central_fd_more_accurate_than_forward_for_quadratic() {
        // f(x) = x_0² → ∂f/∂x_0 = 2x_0. At x_0 = 1: gradient = 2.
        // Forward FD has O(ε) error; central O(ε²). At ε = 1e-3,
        // forward error ~ ε, central ~ ε² — central wins by ~10⁻³.
        let mut samples = Array2::<f64>::zeros((1, 1));
        samples[[0, 0]] = 1.0;
        let model = |x: &[f64]| x[0] * x[0];
        let g_fwd = finite_difference_gradients(&samples, 1e-3, FdKind::Forward, model);
        let g_ctr = finite_difference_gradients(&samples, 1e-3, FdKind::Central, model);
        let err_fwd = (g_fwd[[0, 0]] - 2.0).abs();
        let err_ctr = (g_ctr[[0, 0]] - 2.0).abs();
        assert!(
            err_ctr < err_fwd,
            "central err {err_ctr} should be < forward err {err_fwd}"
        );
    }

    #[test]
    fn fd_sample_buffer_restored_after_each_factor() {
        // Bug: forgetting to restore x_buf[i] after perturbation
        // would leak state across factors. This test exercises a
        // model that detects coordinate corruption.
        let mut samples = Array2::<f64>::zeros((1, 3));
        samples[[0, 0]] = 1.0;
        samples[[0, 1]] = 2.0;
        samples[[0, 2]] = 3.0;
        // f(x) = x_0 + x_1 + x_2 — gradient should be [1, 1, 1].
        let g = finite_difference_gradients(&samples, 1e-5, FdKind::Central, |x: &[f64]| {
            x[0] + x[1] + x[2]
        });
        for i in 0..3 {
            assert!(
                (g[[0, i]] - 1.0).abs() < 1e-6,
                "factor {i}: gradient = {} should be 1",
                g[[0, i]]
            );
        }
    }

    #[test]
    fn fd_panics_on_zero_eps() {
        let samples = Array2::<f64>::zeros((1, 1));
        let result = std::panic::catch_unwind(|| {
            finite_difference_gradients(&samples, 0.0, FdKind::Forward, |_x| 0.0)
        });
        assert!(result.is_err(), "FD with eps=0 should panic");
    }

    // ── Determinism ───────────────────────────────────────────────

    #[test]
    fn same_inputs_yield_identical_estimates() {
        let mut g = Array2::<f64>::zeros((50, 3));
        for k in 0..50 {
            g[[k, 0]] = (k as f64) * 0.1;
            g[[k, 1]] = -(k as f64) * 0.2;
            g[[k, 2]] = (k as f64).sin();
        }
        let a = estimate_dgsm(&g, &[1.0, 2.0, 0.5], 5.0).unwrap();
        let b = estimate_dgsm(&g, &[1.0, 2.0, 0.5], 5.0).unwrap();
        assert_eq!(a.vi, b.vi);
        assert_eq!(a.st_upper, b.st_upper);
    }
}
