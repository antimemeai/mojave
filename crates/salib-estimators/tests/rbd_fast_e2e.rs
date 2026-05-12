//! End-to-end reviewer-affordance contract close for the RBD-FAST
//! estimator (Plischke 2010 bias-corrected) on Ishigami.
//!
//! Per `decisions/2026-04-29-saltelli-rbd-fast.md`. Fourth PR
//! exercising the contract pattern (after PR 7's Saltelli2010,
//! PR 8's Morris, PR 9b's eFAST).
//!
//! Contract artifacts:
//!
//! 1. **Canonical analytic test function** — Ishigami at `(a=7, b=0.1)`,
//!    closed-form `S` per Saltelli Primer 2008.
//! 2. **Model-free identity test** — `0 ≤ Sᵢ ≤ 1` (modulo bias-
//!    corrected MC slack) for additive-pure models.
//! 3. **Frozen `SALib` differential** — agreement with `SALib`'s
//!    `analyze.rbd_fast` at `N=1024`, `M=10`. Both implementations
//!    sample independent `X` matrices (LHS-from-`RngState` vs
//!    LHS-from-numpy.seed(42)), so the comparison is "specification
//!    differential" not byte-exact: we assert both impls land in
//!    the same `~0.06` neighborhood around analytic.
//! 4. **Convergence-rate test** — bias-floor decay across
//!    `N ∈ {256, 1024, 4096}`.
//! 5. **cargo-mutants kill rate** — deferred.
//!
//! # RBD-FAST's bias floor on Ishigami
//!
//! With Plischke 2010 correction, RBD-FAST has a smaller bias
//! than uncorrected eFAST on Ishigami. Realized at FIXTURE_SEED:
//!
//! - `N=256`:  max err 0.041 (S_2)
//! - `N=1024`: max err 0.023 (S_1)
//! - `N=4096`: max err 0.019 (S_2)
//!
//! Both `S_3` (analytic 0) and `S_2` (analytic 0.442) approach
//! their analytic values with small residual bias.

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
use salib_estimators::{estimate_rbd_fast, RbdFastIndices};
use salib_samplers::{LhsSampler, Sampler};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];
const HARMONIC: u32 = 10;

fn lhs_ishigami_inputs(n: usize) -> (Array2<f64>, Vec<f64>) {
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let sampler = LhsSampler::classic(3);
    let unit = sampler.unit_sample(n, &mut rng);
    // Map [0, 1]^3 to Ishigami's [-π, π]^3.
    let mut x = Array2::<f64>::zeros((n, 3));
    for i in 0..n {
        for j in 0..3 {
            x[[i, j]] = -PI + 2.0 * PI * unit[[i, j]];
        }
    }
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let row = [x[[i, 0]], x[[i, 1]], x[[i, 2]]];
            ishigami::ishigami(&row)
        })
        .collect();
    (x, y)
}

fn run_rbd_fast(n: usize) -> RbdFastIndices {
    let (x, y) = lhs_ishigami_inputs(n);
    estimate_rbd_fast(&x, &y, HARMONIC).expect("estimate")
}

// ── Artifact 1: canonical analytic recovery ─────────────────────────

#[test]
fn rbd_fast_ishigami_recovers_analytic_within_bias_floor() {
    // Realized at FIXTURE_SEED, N=4096:
    //   max err = 0.0190 (S_2)
    // Tolerance 0.06 = ~3× realized headroom.
    let estimate = run_rbd_fast(4096);
    let analytic = ishigami::analytic_indices(7.0, 0.1);
    const TOL: f64 = 0.06;
    for i in 0..3 {
        let err = (estimate.s[i] - analytic.first_order[i]).abs();
        assert!(
            err < TOL,
            "S_{i}: got {:.4}, analytic {:.4}, err {err:.4} > {TOL}",
            estimate.s[i],
            analytic.first_order[i]
        );
    }
}

// ── Artifact 2: model-free identity test ────────────────────────────

#[test]
fn rbd_fast_ishigami_indices_in_unit_with_bias_slack() {
    // Plischke-corrected indices can be slightly negative due to
    // unbiased estimation around true zero. They should be bounded
    // in `[-0.05, 1.05]` for Ishigami at N=4096.
    let estimate = run_rbd_fast(4096);
    for i in 0..3 {
        assert!(
            (-0.05..=1.05).contains(&estimate.s[i]),
            "S_{i} = {} outside [-0.05, 1.05]",
            estimate.s[i]
        );
    }
}

// ── Artifact 3: SALib differential ──────────────────────────────────

#[test]
fn rbd_fast_ishigami_lands_in_salib_neighborhood() {
    // SALib `analyze.rbd_fast` on Ishigami with LHS sampling
    // (numpy.random.seed(42), M=10):
    //   N=1024: S = [0.3248, 0.4521, 0.0085]
    //   N=4096: S = [0.3263, 0.4670, -0.0018]
    //
    // Note: this is NOT a byte-exact differential — SALib and we
    // sample independent X matrices (different RNG sources). The
    // comparison validates that both implementations land in the
    // same neighborhood around analytic, modulo Plischke-corrected
    // MC noise. Realized max diff at N=4096: 0.018; tolerance 0.06.
    let estimate = run_rbd_fast(4096);
    let salib = [0.3263, 0.4670, -0.0018];
    const TOL: f64 = 0.06;
    for i in 0..3 {
        let d = (estimate.s[i] - salib[i]).abs();
        assert!(
            d < TOL,
            "S_{i}: ours {:.4}, SALib {:.4}, diff {d:.4} > {TOL}",
            estimate.s[i],
            salib[i]
        );
    }
}

// ── Artifact 4: convergence-rate ────────────────────────────────────

#[test]
fn rbd_fast_ishigami_error_decays_with_n() {
    // Realized at FIXTURE_SEED:
    //   N=256:  max err 0.0409 (factor 2)
    //   N=1024: max err 0.0226 (factor 1)
    //   N=4096: max err 0.0190 (factor 2)
    // We assert the maximum-of-errors at N=4096 is ≤ at N=256.
    let analytic = ishigami::analytic_indices(7.0, 0.1);

    let max_err = |est: &RbdFastIndices| -> f64 {
        (0..3)
            .map(|i| (est.s[i] - analytic.first_order[i]).abs())
            .fold(0.0, f64::max)
    };

    let est_low = run_rbd_fast(256);
    let est_high = run_rbd_fast(4096);

    let err_low = max_err(&est_low);
    let err_high = max_err(&est_high);

    assert!(
        err_high < err_low,
        "max error should decay: N=256 → 4096: {err_low:.4} → {err_high:.4}"
    );
    assert!(
        err_high < 0.05,
        "N=4096 max err = {err_high:.4} should be < 0.05"
    );
}

// ── Bonus: factor ranking by S is exactly correct ───────────────────

#[test]
fn rbd_fast_ishigami_ranks_factors_by_first_order_correctly() {
    // Analytic ranking by S: factor 2 (0.442) > factor 1 (0.314) > factor 3 (0).
    let estimate = run_rbd_fast(4096);
    assert!(
        estimate.s[1] > estimate.s[0],
        "S_2 = {} should exceed S_1 = {}",
        estimate.s[1],
        estimate.s[0]
    );
    assert!(
        estimate.s[0] > estimate.s[2],
        "S_1 = {} should exceed S_3 = {}",
        estimate.s[0],
        estimate.s[2]
    );
}
