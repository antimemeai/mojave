//! End-to-end reviewer-affordance contract close for the DGSM
//! estimator on Ishigami.
//!
//! Per `decisions/2026-04-29-saltelli-dgsm.md`. Seventh PR
//! exercising the contract pattern (after Saltelli2010, Morris,
//! eFAST, RBD-FAST, Borgonovo δ, PAWN).
//!
//! Contract artifacts:
//!
//! 1. **Canonical analytic recovery** — Ishigami at `(a=7, b=0.1)`
//!    has hand-derivable closed-form `νᵢ`:
//!
//!    ```text
//!    ν_1 = (1/2) · E[(1 + b·x_3⁴)²]
//!        = (1/2) · (1 + 2b · E[x_3⁴] + b² · E[x_3⁸])
//!        = (1/2) · (1 + 2·0.1·π⁴/5 + 0.01·π⁸/9)
//!        ≈ 7.72
//!    ν_2 = a² · E[sin²(2x_2)] = a² / 2 = 24.5             EXACT
//!    ν_3 = 16b² · E[x_3⁶] · E[sin²(x_1)]
//!        = 16 · 0.01 · (π⁶/7) · (1/2)
//!        ≈ 10.99
//!    ```
//!
//! 2. **Poincaré property** — `Sᵀᵢ_analytic ≤ νᵢ · C_P / Var(Y)`
//!    must hold for every factor (the bound's defining
//!    inequality, Sobol-Kucherenko 2009).
//! 3. **Analytical-vs-FD agreement** — for smooth Ishigami,
//!    central FD with `ε = 1e-5` should match analytical gradient
//!    to `~1e-5` per element.
//! 4. **Convergence** — `νᵢ` estimates approach analytic as `N`
//!    grows from 1024 to 4096.
//! 5. **cargo-mutants kill rate** — deferred (workspace-63g).
//!
//! # Realized at FIXTURE_SEED, N=4096
//!
//! ```text
//! Analytical gradient: vi = [7.7721, 24.5001, 10.9184]
//! Central FD (ε=1e-5):  vi = [7.7721, 24.5001, 10.9184]   identical to ~1e-5
//! ST_upper:             [2.182, 6.877, 3.065]
//! ST_analytic:          [0.558, 0.442, 0.244]              ≤ all upper bounds ✓
//! ```
//!
//! The Poincaré bound is **loose** (ST_upper >> ST_analytic)
//! because Uniform[-π, π] has `C_P = 4`, a generous spectral-
//! gap-derived upper bound. Looseness is acceptable: DGSM's
//! screening role uses `ST_upper < δ` as a *proof* of low total
//! contribution, not a tight estimate.

#![allow(
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::similar_names,
    clippy::items_after_statements,
    clippy::needless_range_loop,
    clippy::doc_markdown
)]

use std::f64::consts::PI;

use ndarray::Array2;
use salib_core::{tree_var, Distribution, RngState};
use salib_estimators::{
    estimate_dgsm, finite_difference_gradients, poincare_constant, DgsmIndices, FdKind,
};
use salib_samplers::{LhsSampler, Sampler};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn lhs_ishigami(n: usize) -> (Array2<f64>, Vec<f64>, f64) {
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let unit = LhsSampler::classic(3).unit_sample(n, &mut rng);
    let mut x = Array2::<f64>::zeros((n, 3));
    for i in 0..n {
        for j in 0..3 {
            x[[i, j]] = -PI + 2.0 * PI * unit[[i, j]];
        }
    }
    let y: Vec<f64> = (0..n)
        .map(|i| ishigami::ishigami(&[x[[i, 0]], x[[i, 1]], x[[i, 2]]]))
        .collect();
    let var_y = tree_var(&y);
    (x, y, var_y)
}

fn ishigami_poincare_constants() -> [f64; 3] {
    let cp = poincare_constant(&Distribution::Uniform { lo: -PI, hi: PI }).unwrap();
    [cp, cp, cp]
}

fn estimate_with_analytical_gradient(n: usize) -> DgsmIndices {
    let (x, _y, var_y) = lhs_ishigami(n);
    let mut g = Array2::<f64>::zeros((n, 3));
    for k in 0..n {
        let grad = ishigami::ishigami_gradient(&[x[[k, 0]], x[[k, 1]], x[[k, 2]]]);
        for j in 0..3 {
            g[[k, j]] = grad[j];
        }
    }
    let cp = ishigami_poincare_constants();
    estimate_dgsm(&g, &cp, var_y).expect("estimate")
}

fn estimate_with_central_fd(n: usize, eps: f64) -> DgsmIndices {
    let (x, _y, var_y) = lhs_ishigami(n);
    let g = finite_difference_gradients(&x, eps, FdKind::Central, |xi: &[f64]| {
        ishigami::ishigami(&[xi[0], xi[1], xi[2]])
    });
    let cp = ishigami_poincare_constants();
    estimate_dgsm(&g, &cp, var_y).expect("estimate")
}

// ── Artifact 1: canonical analytic recovery ─────────────────────────

#[test]
fn dgsm_ishigami_vi_recovers_closed_form() {
    // ν_2 has an exact closed form: a²/2 = 49/2 = 24.5.
    // ν_1 ≈ 7.72, ν_3 ≈ 10.99 derived above (within MC tolerance).
    let est = estimate_with_analytical_gradient(4096);
    assert!(
        (est.vi[0] - 7.72).abs() < 0.1,
        "ν_1: got {:.4}, expected ≈ 7.72",
        est.vi[0]
    );
    assert!(
        (est.vi[1] - 24.5).abs() < 0.1,
        "ν_2: got {:.4}, expected = 24.5",
        est.vi[1]
    );
    assert!(
        (est.vi[2] - 10.99).abs() < 0.2,
        "ν_3: got {:.4}, expected ≈ 10.99",
        est.vi[2]
    );
}

// ── Artifact 2: Poincaré property ───────────────────────────────────

#[test]
fn dgsm_ishigami_poincare_bound_holds() {
    // Sᵀᵢ_analytic ≤ νᵢ · C_P / Var(Y) for every factor.
    // This is the bound's defining inequality (Sobol-Kucherenko 2009).
    let est = estimate_with_analytical_gradient(4096);
    let analytic = ishigami::analytic_indices(7.0, 0.1);
    for i in 0..3 {
        assert!(
            analytic.total_order[i] <= est.st_upper[i] + 1e-9,
            "Poincaré bound violated for factor {i}: \
             ST_analytic = {:.4} > ST_upper = {:.4}",
            analytic.total_order[i],
            est.st_upper[i]
        );
    }
}

// ── Artifact 3: analytical vs central FD ────────────────────────────

#[test]
fn dgsm_ishigami_central_fd_matches_analytical() {
    // For smooth Ishigami at ε=1e-5, central FD has O(ε²) ≈ 1e-10
    // truncation error. νᵢ depends on squared gradient, so error
    // squared ≈ 1e-20 — negligible. Tolerance 1e-5 is generous.
    let est_analytical = estimate_with_analytical_gradient(4096);
    let est_fd = estimate_with_central_fd(4096, 1e-5);
    for i in 0..3 {
        let diff = (est_analytical.vi[i] - est_fd.vi[i]).abs();
        assert!(
            diff < 1e-5,
            "ν_{i}: analytical {:.6}, central FD {:.6}, diff {diff:.2e}",
            est_analytical.vi[i],
            est_fd.vi[i]
        );
    }
}

// ── Artifact 4: convergence ─────────────────────────────────────────

#[test]
fn dgsm_ishigami_converges_with_n() {
    // ν_2 = 24.5 exactly. Realized errors:
    //   N=1024: 24.4987 (err 0.0013)
    //   N=4096: 24.5001 (err 0.0001)
    // Strict decay.
    let est_low = estimate_with_analytical_gradient(1024);
    let est_high = estimate_with_analytical_gradient(4096);
    let err_low = (est_low.vi[1] - 24.5).abs();
    let err_high = (est_high.vi[1] - 24.5).abs();
    assert!(
        err_high < err_low + 1e-12,
        "ν_2 should not regress: N=1024 → 4096 err {err_low:.4} → {err_high:.4}"
    );
    // Absolute bound at N=4096.
    assert!(
        err_high < 0.01,
        "ν_2 error at N=4096: {err_high:.4} should be < 0.01"
    );
}

// ── Bonus: factor ranking by ν_i ────────────────────────────────────

#[test]
fn dgsm_ishigami_ranks_factors_correctly() {
    // ν_2 ≈ 24.5 > ν_3 ≈ 10.99 > ν_1 ≈ 7.72.
    let est = estimate_with_analytical_gradient(4096);
    assert!(est.vi[1] > est.vi[2], "ν_2 should exceed ν_3");
    assert!(est.vi[2] > est.vi[0], "ν_3 should exceed ν_1");
}

// ── Forward FD also recovers, with looser tolerance ─────────────────

#[test]
fn dgsm_ishigami_forward_fd_recovers_within_o_eps() {
    // Forward FD has O(ε) truncation error. At ε=1e-6, individual
    // gradient elements have ~1e-6 error; squared gives ~1e-12;
    // averaged over N samples ν_i drift is tiny. Tolerance 0.01.
    let (x, _y, var_y) = lhs_ishigami(1024);
    let g = finite_difference_gradients(&x, 1e-6, FdKind::Forward, |xi: &[f64]| {
        ishigami::ishigami(&[xi[0], xi[1], xi[2]])
    });
    let cp = ishigami_poincare_constants();
    let est = estimate_dgsm(&g, &cp, var_y).expect("estimate");
    assert!((est.vi[1] - 24.5).abs() < 0.01);
}
