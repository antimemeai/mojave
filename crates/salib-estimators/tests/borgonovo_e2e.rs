//! End-to-end reviewer-affordance contract close for the Borgonovo δ
//! estimator on Ishigami.
//!
//! Per `decisions/2026-04-29-saltelli-borgonovo-delta.md`. Fifth PR
//! exercising the contract pattern (after PR 7 Saltelli2010, PR 8
//! Morris, PR 9b eFAST, PR 10 RBD-FAST).
//!
//! Contract artifacts:
//!
//! 1. **Canonical analytic test function** — Ishigami at `(a=7, b=0.1)`.
//!    Literature analytic `δ ≈ [0.214, 0.371, 0.157]`
//!    (Plischke-Borgonovo-Smith 2013).
//! 2. **Model-free identity test** — `δᵢ ∈ [0, 1]` (with KDE-
//!    integration ε slack).
//! 3. **Frozen `SALib` differential** — `SALib`'s `analyze.delta`
//!    (which applies Plischke 2013 Eq 30 bias reduction).
//!    Specification differential: independent X matrices, both
//!    converge to analytic.
//! 4. **Convergence-rate test** — max-of-errors strictly decays
//!    `N ∈ {256, 4096}`.
//! 5. **cargo-mutants kill rate** — deferred.
//!
//! # Realized errors at FIXTURE_SEED
//!
//! Raw `calc_delta` (Plischke 2013 Eq 26) on Ishigami:
//!
//! - `N=256`: max err 0.159 (factor 2)
//! - `N=1024`: max err 0.113 (factor 2)
//! - `N=4096`: max err 0.028 (factor 2)
//!
//! KDE-based density estimation needs many samples for tight
//! convergence on Ishigami's bimodal output. `N=4096` is the
//! tolerance regime; below that, errors are large.

#![allow(
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::similar_names,
    clippy::items_after_statements,
    clippy::needless_range_loop,
    clippy::doc_markdown
)]

use std::f64::consts::PI;

use ndarray::Array2;
use salib_core::RngState;
use salib_estimators::{estimate_borgonovo_delta, BorgonovoIndices};
use salib_samplers::{LhsSampler, Sampler};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

/// Approximate analytic δ for Ishigami at `(a=7, b=0.1)` from
/// Plischke-Borgonovo-Smith 2013.
const ISHIGAMI_DELTA_ANALYTIC: [f64; 3] = [0.214, 0.371, 0.157];

fn lhs_ishigami_inputs(n: usize) -> (Array2<f64>, Vec<f64>) {
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

fn run_at_n(n: usize) -> BorgonovoIndices {
    let (x, y) = lhs_ishigami_inputs(n);
    estimate_borgonovo_delta(&x, &y).expect("estimate")
}

// ── Artifact 1: canonical analytic recovery ─────────────────────────

#[test]
fn borgonovo_ishigami_recovers_analytic_within_kde_bias() {
    // At N=4096, max realized err = 0.028. Tolerance 0.06 ≈ 2×.
    let estimate = run_at_n(4096);
    const TOL: f64 = 0.06;
    for i in 0..3 {
        let err = (estimate.delta[i] - ISHIGAMI_DELTA_ANALYTIC[i]).abs();
        assert!(
            err < TOL,
            "δ_{i}: got {:.4}, analytic {:.4}, err {err:.4} > {TOL}",
            estimate.delta[i],
            ISHIGAMI_DELTA_ANALYTIC[i]
        );
    }
}

// ── Artifact 2: model-free identity test ────────────────────────────

#[test]
fn borgonovo_ishigami_indices_in_unit_interval() {
    // δᵢ ∈ [0, 1] by definition. KDE-based numerical integration
    // can produce ε slack; allow [-0.05, 1.05].
    let estimate = run_at_n(4096);
    for i in 0..3 {
        assert!(
            (-0.05..=1.05).contains(&estimate.delta[i]),
            "δ_{i} = {} outside [-0.05, 1.05]",
            estimate.delta[i]
        );
    }
}

// ── Artifact 3: SALib differential ──────────────────────────────────

#[test]
fn borgonovo_ishigami_lands_in_salib_neighborhood() {
    // SALib `analyze.delta` on Ishigami with LHS sampling
    // (numpy.random.seed(42), with Plischke 2013 Eq 30 bias
    // reduction):
    //   N=4096: δ = [0.2276, 0.3745, 0.1653]
    //
    // We ship raw calc_delta (Eq 26) without the bias-reduction
    // wrapper. SALib's bias-reduced values differ from raw by
    // ~0.02–0.04 per factor (Plischke 2013 § 5.2). On top of that,
    // independent X matrices add MC noise. Tolerance 0.10 allows
    // both gaps; specification differential, not byte-exact.
    let estimate = run_at_n(4096);
    let salib = [0.2276, 0.3745, 0.1653];
    const TOL: f64 = 0.10;
    for i in 0..3 {
        let d = (estimate.delta[i] - salib[i]).abs();
        assert!(
            d < TOL,
            "δ_{i}: ours {:.4}, SALib {:.4}, diff {d:.4} > {TOL}",
            estimate.delta[i],
            salib[i]
        );
    }
}

// ── Artifact 4: convergence-rate ────────────────────────────────────

#[test]
fn borgonovo_ishigami_error_decays_with_n() {
    let max_err = |est: &BorgonovoIndices| -> f64 {
        (0..3)
            .map(|i| (est.delta[i] - ISHIGAMI_DELTA_ANALYTIC[i]).abs())
            .fold(0.0, f64::max)
    };

    let est_low = run_at_n(256);
    let est_high = run_at_n(4096);

    let err_low = max_err(&est_low);
    let err_high = max_err(&est_high);

    assert!(
        err_high < err_low,
        "max error should decay: N=256 → 4096: {err_low:.4} → {err_high:.4}"
    );
    // At N=4096 the bias floor is ~0.03; require strictly < 0.06.
    assert!(
        err_high < 0.06,
        "N=4096 max err = {err_high:.4} should be < 0.06"
    );
}

// ── Bonus: factor ranking by δ ──────────────────────────────────────

#[test]
fn borgonovo_ishigami_ranks_factors_by_delta_correctly() {
    // Analytic ranking: δ_2 (0.371) > δ_1 (0.214) > δ_3 (0.157).
    let estimate = run_at_n(4096);
    assert!(
        estimate.delta[1] > estimate.delta[0],
        "δ_2 = {} should exceed δ_1 = {}",
        estimate.delta[1],
        estimate.delta[0]
    );
    assert!(
        estimate.delta[0] > estimate.delta[2],
        "δ_1 = {} should exceed δ_3 = {}",
        estimate.delta[0],
        estimate.delta[2]
    );
}
