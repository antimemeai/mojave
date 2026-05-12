//! End-to-end reviewer-affordance contract close for the
//! regression-based estimators (SRC/SRRC/PCC/PRCC + R²).
//!
//! Per `decisions/2026-04-29-saltelli-regression.md`. Eighth PR
//! exercising the contract pattern.
//!
//! # Why this contract validates "diagnostic correctness," not
//! "estimator accuracy"
//!
//! Regression-based indices are valid only under linearity (SRC)
//! or monotonicity (SRRC/PRCC). On Ishigami — non-linear,
//! non-monotonic in `x_2` (`sin²` oscillates) — they **should
//! not recover** Sobol' indices. The point of shipping them is
//! the `R²` diagnostic: a low `R²_linear` correctly flags SRC as
//! untrustworthy.
//!
//! Contract artifacts:
//!
//! 1. **Linear-fixture recovery** — for a known linear model
//!    `Y = 2·X[:, 0] + X[:, 1]`, SRC ratios match coefficient
//!    ratios within MC tolerance.
//! 2. **R² diagnostic on Ishigami** — `R²_linear << 0.7` correctly
//!    flags Ishigami as untrustworthy for SRC/PCC.
//! 3. **Monotonicity diagnostic on Ishigami's x_2** — SRRC[1] ≈ 0
//!    correctly identifies non-monotonic behavior (sin² over
//!    [-π, π] is symmetric and non-monotonic).
//! 4. **Identity** — `|SRC|, |PCC|, |PRCC| ≤ 1` (correlation bounds).
//! 5. **Convergence** — N=1024 → 4096 stability.
//! 6. **cargo-mutants kill rate** — deferred (workspace-63g).
//!
//! # Realized at FIXTURE_SEED, N=4096
//!
//! ```text
//! SRC  = [0.4364, 0.0071, 0.0013]   R²_linear = 0.1906
//! SRRC = [0.4381, 0.0093, 0.0122]   R²_rank   = 0.1923
//! ```
//!
//! Both R² are well below 0.7 — diagnostic is doing its job.

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
use salib_estimators::{estimate_regression_indices, RegressionIndices};
use salib_samplers::{LhsSampler, Sampler};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn lhs_x(n: usize, d: usize) -> Array2<f64> {
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    LhsSampler::classic(d).unit_sample(n, &mut rng)
}

fn ishigami_setup(n: usize) -> (Array2<f64>, Vec<f64>) {
    let unit = lhs_x(n, 3);
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

fn run_ishigami_at_n(n: usize) -> RegressionIndices {
    let (x, y) = ishigami_setup(n);
    estimate_regression_indices(&x, &y).expect("estimate")
}

// ── Artifact 1: linear-fixture recovery ─────────────────────────────

#[test]
fn regression_recovers_linear_coefficient_ratio() {
    // Y = 2·X[:, 0] + X[:, 1] over independent uniform X.
    // SRC_0 / SRC_1 ≈ 2 (coefficient ratio, equal stds).
    // R²_linear ≈ 1.0.
    let n = 1024;
    let x = lhs_x(n, 3);
    let y: Vec<f64> = (0..n).map(|k| 2.0 * x[[k, 0]] + x[[k, 1]]).collect();
    let est = estimate_regression_indices(&x, &y).expect("estimate");
    assert!(
        est.r2_linear > 0.99,
        "R²_linear = {:.4}, expected near 1",
        est.r2_linear
    );
    let ratio = est.src[0].abs() / est.src[1].abs();
    assert!(
        (ratio - 2.0).abs() < 0.2,
        "SRC ratio = {ratio:.4}, expected ≈ 2.0"
    );
    // Factor 2 (absent from model) should have SRC ≈ 0.
    assert!(
        est.src[2].abs() < 0.1,
        "SRC_2 = {:.4} should be near 0 (factor not in model)",
        est.src[2]
    );
}

// ── Artifact 2: R² diagnostic correctly flags Ishigami ──────────────

#[test]
fn r2_diagnostic_flags_ishigami_as_untrustworthy() {
    // Ishigami is non-linear and non-monotonic in x_2 → both R²
    // values should be well below the 0.7 trustworthiness threshold.
    let est = run_ishigami_at_n(4096);
    assert!(
        est.r2_linear < 0.5,
        "R²_linear = {:.4} on Ishigami — should flag as untrustworthy",
        est.r2_linear
    );
    assert!(
        est.r2_rank < 0.5,
        "R²_rank = {:.4} on Ishigami — should flag as untrustworthy",
        est.r2_rank
    );
}

// ── Artifact 3: SRRC ≈ 0 for non-monotonic factor 1 ─────────────────

#[test]
fn srrc_near_zero_for_non_monotonic_x2() {
    // x_2 enters via sin²(x_2), which is symmetric about 0 over
    // [-π, π] — non-monotonic. SRRC[1] should be near zero
    // because rank correlation can't see the U-shape.
    let est = run_ishigami_at_n(4096);
    assert!(
        est.srrc[1].abs() < 0.1,
        "|SRRC_1| = {:.4} should be near 0 (sin² non-monotonic)",
        est.srrc[1].abs()
    );
    assert!(
        est.prcc[1].abs() < 0.1,
        "|PRCC_1| = {:.4} should be near 0",
        est.prcc[1].abs()
    );
}

// ── Artifact 4: identity bounds ─────────────────────────────────────

#[test]
fn regression_indices_within_correlation_bounds() {
    // |SRC|, |PCC|, |PRCC| ≤ 1 by definition (they're
    // correlations or correlation-like quantities). Allow ε
    // overshoot for FP rounding.
    let est = run_ishigami_at_n(4096);
    for i in 0..3 {
        assert!(
            est.src[i].abs() <= 1.0 + 1e-9,
            "|SRC_{i}| = {} exceeds 1",
            est.src[i].abs()
        );
        assert!(
            est.srrc[i].abs() <= 1.0 + 1e-9,
            "|SRRC_{i}| = {} exceeds 1",
            est.srrc[i].abs()
        );
        assert!(
            est.pcc[i].abs() <= 1.0 + 1e-9,
            "|PCC_{i}| = {} exceeds 1",
            est.pcc[i].abs()
        );
        assert!(
            est.prcc[i].abs() <= 1.0 + 1e-9,
            "|PRCC_{i}| = {} exceeds 1",
            est.prcc[i].abs()
        );
    }
    // R² is in [0, 1].
    assert!((0.0..=1.0).contains(&est.r2_linear));
    assert!((0.0..=1.0).contains(&est.r2_rank));
}

// ── Artifact 5: convergence ─────────────────────────────────────────

#[test]
fn regression_indices_stable_across_n_on_ishigami() {
    // SRC[0] picks up factor 0's linear trend; should be stable
    // between N=1024 and N=4096 modulo MC noise.
    let est_low = run_ishigami_at_n(1024);
    let est_high = run_ishigami_at_n(4096);
    let drift = (est_high.src[0] - est_low.src[0]).abs();
    assert!(
        drift < 0.05,
        "SRC_0 drift across N: {:.4} → {:.4} (drift {drift:.4})",
        est_low.src[0],
        est_high.src[0]
    );
    // R² should also stabilize.
    let r2_drift = (est_high.r2_linear - est_low.r2_linear).abs();
    assert!(
        r2_drift < 0.05,
        "R²_linear drift across N: {:.4} → {:.4} (drift {r2_drift:.4})",
        est_low.r2_linear,
        est_high.r2_linear
    );
}

// ── Bonus: PCC and SRC agree in sign under independent X ──────────
//
// Note on the math: PCC and SRC are *not* numerically equivalent
// even under independent factors — they share a numerator
// (covariance of X_i with Y) but differ in the denominator
// (PCC normalizes by residual-Y variance after partialing out
// other factors; SRC normalizes by total Y variance). They agree
// in *sign* and rank-ordering, which is what matters for
// screening. A naive equivalence test would be a false claim.

#[test]
fn pcc_and_src_agree_in_sign_on_ishigami() {
    let est = run_ishigami_at_n(4096);
    for i in 0..3 {
        // Either both near zero, or same sign.
        let s = est.src[i];
        let p = est.pcc[i];
        if s.abs() > 0.01 && p.abs() > 0.01 {
            assert!(
                s.signum() == p.signum(),
                "factor {i}: SRC = {s:.4} sign-disagrees with PCC = {p:.4}"
            );
        }
    }
}
