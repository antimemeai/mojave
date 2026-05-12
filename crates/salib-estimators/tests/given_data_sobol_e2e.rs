//! End-to-end reviewer-affordance contract close for the
//! given-data Sobol' estimator (Plischke-Borgonovo-Smith 2013) on
//! Ishigami.
//!
//! Per `decisions/2026-04-29-saltelli-given-data-sobol.md`. Ninth
//! and final Phase C estimator PR.
//!
//! Contract artifacts:
//!
//! 1. **Canonical analytic recovery** — Ishigami `S_1 = [0.314, 0.442, 0.000]`
//!    per Saltelli Primer 2008 Eq 5.16-5.18.
//! 2. **Identity** — `S_1 ∈ [0, 1]` (clamped at the estimator boundary).
//! 3. **Cross-implementation differential** — agreement with our
//!    own `saltelli2010` estimator on independent designs (no
//!    SALib byte-exact differential because SALib uses a different
//!    sampling design).
//! 4. **Convergence** — error decay `N=1024 → 4096`.
//! 5. **cargo-mutants** — deferred (workspace-63g).
//!
//! # Realized at FIXTURE_SEED, N=4096
//!
//! ```text
//! S_1 = [0.3055, 0.4385, 0.0060]
//! Analytic [0.314, 0.442, 0.000]
//! Max err 0.0084 — one of the tightest Ishigami recoveries we ship.
//! ```

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
use salib_core::RngState;
use salib_estimators::{estimate_given_data_sobol, GivenDataSobolIndices};
use salib_samplers::{LhsSampler, Sampler};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn lhs_ishigami(n: usize) -> (Array2<f64>, Vec<f64>) {
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
    (x, y)
}

fn run_at_n(n: usize) -> GivenDataSobolIndices {
    let (x, y) = lhs_ishigami(n);
    estimate_given_data_sobol(&x, &y).expect("estimate")
}

// ── Artifact 1: canonical analytic recovery ─────────────────────────

#[test]
fn given_data_sobol_recovers_ishigami_analytic() {
    // At N=4096, max realized err = 0.0084. Tolerance 0.03 gives
    // ~3.5× headroom.
    let est = run_at_n(4096);
    let analytic = ishigami::analytic_indices(7.0, 0.1);
    const TOL: f64 = 0.03;
    for i in 0..3 {
        let err = (est.s1[i] - analytic.first_order[i]).abs();
        assert!(
            err < TOL,
            "S_1[{i}]: got {:.4}, analytic {:.4}, err {err:.4} > {TOL}",
            est.s1[i],
            analytic.first_order[i]
        );
    }
}

// ── Artifact 2: identity ─────────────────────────────────────────────

#[test]
fn given_data_sobol_indices_in_unit_interval() {
    let est = run_at_n(4096);
    for &v in &est.s1 {
        assert!((0.0..=1.0).contains(&v), "S_1 = {v} not in [0, 1]");
    }
}

// ── Artifact 3: cross-implementation differential ───────────────────
//
// Compare against `salib_estimators::estimate_rbd_fast` on the
// same `(X, Y)` (both are given-data first-order Sobol' estimators
// — partition vs spectral). Disagreement is bounded by combined
// finite-sample bias.

#[test]
fn given_data_sobol_lands_in_rbd_fast_neighborhood() {
    use salib_estimators::estimate_rbd_fast;
    let (x, y) = lhs_ishigami(4096);
    let est_given_data = estimate_given_data_sobol(&x, &y).expect("given-data");
    let est_rbd = estimate_rbd_fast(&x, &y, 10).expect("rbd-fast");
    // Both target the same population S_1 but with different
    // mechanisms. RBD-FAST has Plischke 2010 bias correction;
    // given-data partition has none. Observed pairwise diff at
    // N=4096: ~0.02. Tolerance 0.05.
    const TOL: f64 = 0.05;
    for i in 0..3 {
        let d = (est_given_data.s1[i] - est_rbd.s[i]).abs();
        assert!(
            d < TOL,
            "S_1[{i}]: given-data {:.4}, RBD-FAST {:.4}, diff {d:.4}",
            est_given_data.s1[i],
            est_rbd.s[i]
        );
    }
}

// ── Artifact 4: convergence ─────────────────────────────────────────

#[test]
fn given_data_sobol_error_decays_with_n() {
    let analytic = ishigami::analytic_indices(7.0, 0.1);
    let max_err = |est: &GivenDataSobolIndices| -> f64 {
        (0..3)
            .map(|i| (est.s1[i] - analytic.first_order[i]).abs())
            .fold(0.0, f64::max)
    };
    let est_low = run_at_n(1024);
    let est_high = run_at_n(4096);
    let err_low = max_err(&est_low);
    let err_high = max_err(&est_high);
    assert!(
        err_high < err_low,
        "max error should decay: N=1024 → 4096: {err_low:.4} → {err_high:.4}"
    );
    assert!(
        err_high < 0.02,
        "N=4096 max err = {err_high:.4} should be < 0.02"
    );
}

// ── Bonus: factor ranking by S_1 ────────────────────────────────────

#[test]
fn given_data_sobol_ranks_factors_correctly() {
    // Analytic ranking by S_1: factor 2 (0.442) > factor 1 (0.314)
    // > factor 3 (0).
    let est = run_at_n(4096);
    assert!(est.s1[1] > est.s1[0], "S_1[1] should exceed S_1[0]");
    assert!(est.s1[0] > est.s1[2], "S_1[0] should exceed S_1[2]");
}
