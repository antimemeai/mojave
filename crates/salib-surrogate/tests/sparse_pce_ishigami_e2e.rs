//! End-to-end reviewer-affordance contract close for `fit_sparse_pce`
//! on Ishigami canonical `(a=7, b=0.1)`. Mirrors `pce_ishigami_e2e`
//! (PR 16b's full-OLS contract) but exercises both `SparseSolver::Omp`
//! and `SparseSolver::Lars` in parallel — Patrick's "workspace our
//! own metrics" framing: ship both first-class and let the reviewer
//! see how they compare.
//!
//! At Ishigami `(a=7, b=0.1)`, sparse PCE should:
//! - Recover all six analytic Sobol' indices to ≤ 0.02 absolute.
//! - Use far fewer non-zero coefficients than the full-OLS PCE
//!   (PR 16b uses all 286; sparse should keep ≤ 80).
//! - Produce the same Sobol' decomposition identities exactly.

#![allow(
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::expect_used,
    clippy::similar_names,
    clippy::items_after_statements
)]

use std::f64::consts::PI;

use ndarray::Array2;
use salib_core::RngState;
use salib_samplers::{Sampler, SobolSampler};
use salib_surrogate::{
    fit_sparse_pce, sobol_indices_from_pce, PolynomialFamily, SparseSolver, TruncationScheme,
};
use salib_validation::{ishigami, SobolIndicesAnalytic};

const FIXTURE_SEED: [u8; 32] = [0; 32];
const N: usize = 4096;
const MAX_DEGREE: usize = 10;

fn build_inputs() -> (Array2<f64>, Vec<f64>) {
    let sampler = SobolSampler::standard(3).with_skip_first(false);
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let unit = sampler.unit_sample(N, &mut rng);
    let mut x = Array2::<f64>::zeros((N, 3));
    let mut y = Vec::with_capacity(N);
    for i in 0..N {
        for k in 0..3 {
            x[[i, k]] = 2.0 * unit[[i, k]] - 1.0;
        }
        let x_real = [PI * x[[i, 0]], PI * x[[i, 1]], PI * x[[i, 2]]];
        y.push(ishigami::ishigami(&x_real));
    }
    (x, y)
}

fn run_solver(solver: SparseSolver) -> (salib_surrogate::SobolFromPce, usize, usize) {
    let (x, y) = build_inputs();
    let (pce, diag) = fit_sparse_pce(
        &x,
        &y,
        &[PolynomialFamily::Legendre; 3],
        MAX_DEGREE,
        TruncationScheme::Hyperbolic { q: 0.75 },
        solver,
        None,
    )
    .expect("sparse PCE fit");
    let sobol = sobol_indices_from_pce(&pce).expect("Sobol from PCE");
    (sobol, diag.num_active, diag.candidate_basis_size)
}

// ── Recovery: both solvers ──────────────────────────────────────────

#[test]
fn omp_recovers_ishigami_first_order_within_tolerance() {
    let (sobol, _, _) = run_solver(SparseSolver::Omp);
    let analytic: SobolIndicesAnalytic = ishigami::analytic_indices(7.0, 0.1);
    const TOL: f64 = 0.02;
    for (i, &want) in analytic.first_order.iter().enumerate() {
        let got = sobol.first_order[i];
        assert!(
            (got - want).abs() < TOL,
            "OMP S_{i}: got {got:.4}, want {want:.4} within {TOL}"
        );
    }
}

#[test]
fn omp_recovers_ishigami_total_order_within_tolerance() {
    let (sobol, _, _) = run_solver(SparseSolver::Omp);
    let analytic = ishigami::analytic_indices(7.0, 0.1);
    const TOL: f64 = 0.02;
    for (i, &want) in analytic.total_order.iter().enumerate() {
        let got = sobol.total_order[i];
        assert!(
            (got - want).abs() < TOL,
            "OMP S_T_{i}: got {got:.4}, want {want:.4} within {TOL}"
        );
    }
}

#[test]
fn lars_recovers_ishigami_first_order_within_tolerance() {
    let (sobol, _, _) = run_solver(SparseSolver::Lars);
    let analytic = ishigami::analytic_indices(7.0, 0.1);
    const TOL: f64 = 0.02;
    for (i, &want) in analytic.first_order.iter().enumerate() {
        let got = sobol.first_order[i];
        assert!(
            (got - want).abs() < TOL,
            "LARS S_{i}: got {got:.4}, want {want:.4} within {TOL}"
        );
    }
}

#[test]
fn lars_recovers_ishigami_total_order_within_tolerance() {
    let (sobol, _, _) = run_solver(SparseSolver::Lars);
    let analytic = ishigami::analytic_indices(7.0, 0.1);
    const TOL: f64 = 0.02;
    for (i, &want) in analytic.total_order.iter().enumerate() {
        let got = sobol.total_order[i];
        assert!(
            (got - want).abs() < TOL,
            "LARS S_T_{i}: got {got:.4}, want {want:.4} within {TOL}"
        );
    }
}

// ── Engineering pay-off: sparse < full ──────────────────────────────

#[test]
fn omp_dramatically_fewer_active_terms_than_full_basis() {
    let (_, num_active, candidate) = run_solver(SparseSolver::Omp);
    // Full OLS at p=10 (total-degree) uses 286; hyperbolic at q=0.75
    // is the candidate pool here (smaller); active should be a
    // small subset of *that*.
    assert!(
        num_active <= 80,
        "OMP active = {num_active}, expected ≤ 80 (candidate basis = {candidate})"
    );
}

#[test]
fn lars_dramatically_fewer_active_terms_than_full_basis() {
    let (_, num_active, candidate) = run_solver(SparseSolver::Lars);
    assert!(
        num_active <= 80,
        "LARS active = {num_active}, expected ≤ 80 (candidate basis = {candidate})"
    );
}

// ── Decomposition identities (exact for PCE) ────────────────────────

#[test]
fn sparse_pce_preserves_sobol_identities() {
    for solver in [SparseSolver::Omp, SparseSolver::Lars] {
        let (sobol, _, _) = run_solver(solver);
        for i in 0..3 {
            assert!(
                sobol.first_order[i] <= sobol.total_order[i] + 1e-9,
                "{solver:?}: S_{i} = {} > S_T_{i} = {}",
                sobol.first_order[i],
                sobol.total_order[i]
            );
        }
        let sum: f64 = sobol.first_order.iter().sum();
        assert!(sum <= 1.0 + 1e-9, "{solver:?}: Σ S_i = {sum}");
    }
}

// ── Determinism ─────────────────────────────────────────────────────

#[test]
fn sparse_pce_is_deterministic_across_runs() {
    for solver in [SparseSolver::Omp, SparseSolver::Lars] {
        let (a, na, _) = run_solver(solver);
        let (b, nb, _) = run_solver(solver);
        assert_eq!(na, nb, "{solver:?}: active count differs across runs");
        assert_eq!(a.first_order, b.first_order, "{solver:?}: S differs");
        assert_eq!(a.total_order, b.total_order, "{solver:?}: S_T differs");
    }
}
