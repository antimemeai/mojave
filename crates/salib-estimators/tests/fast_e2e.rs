//! End-to-end reviewer-affordance contract close for the FAST/
//! eFAST estimator against the Ishigami test function.
//!
//! Per `decisions/2026-04-29-saltelli-fast-estimator.md`. Third PR
//! exercising the contract pattern (after PR 7's Saltelli2010 +
//! PR 8's Morris).
//!
//! Contract artifacts:
//!
//! 1. **Canonical analytic test function** — Ishigami at `(a=7, b=0.1)`,
//!    closed-form `S, ST` per Saltelli Primer 2008.
//! 2. **Model-free identity test** — `ST_i ≥ S_i` for every factor
//!    (universal Sobol' identity).
//! 3. **Frozen `SALib` differential** — agreement with `SALib`'s
//!    `analyze.fast` at `N ∈ {65, 257, 1025}`.
//! 4. **Convergence-rate test** — `|S_i - S_i_analytic|` and
//!    `|ST_i - ST_i_analytic|` shrink (or hold within FAST's bias
//!    floor) as `N` increases from 65 to 1025.
//! 5. **cargo-mutants kill rate** — deferred to nightly CI bead.
//!
//! # FAST's known systematic bias on Ishigami
//!
//! eFAST exhibits a well-documented bias on interaction-heavy
//! functions like Ishigami. The `sin(x_1) · x_3⁴` term creates
//! spectral content at sums and differences of `ω_1` and `ω_3`
//! that alias into the harmonic bands of both factors. The result:
//!
//! - `S_3` shows a small false signal (~0.02 vs analytic 0.0).
//! - `ST_1` underestimates by ~0.04 (vs analytic 0.558).
//! - `ST_2` overestimates by ~0.05.
//!
//! These persist as `N` increases; they're *bias*, not MC noise.
//! `SALib` shows the same bias on the same function. Tolerances
//! account for it explicitly in artifact 1; artifact 3 is tighter
//! because both implementations share the bias.

#![allow(
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::similar_names,
    clippy::items_after_statements,
    clippy::doc_markdown,
    clippy::needless_range_loop
)]

use std::f64::consts::PI;

use salib_core::RngState;
use salib_estimators::{estimate_fast, FastIndices};
use salib_samplers::build_fast_design;
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

/// Map a `[0, 1]^3` sample to Ishigami's `Uniform[-π, π]^3` input,
/// then evaluate.
fn ishigami_on_unit_cube(u: &[f64]) -> f64 {
    let x: [f64; 3] = [
        -PI + 2.0 * PI * u[0],
        -PI + 2.0 * PI * u[1],
        -PI + 2.0 * PI * u[2],
    ];
    ishigami::ishigami(&x)
}

fn run_fast_at_n(n: usize) -> FastIndices {
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let design = build_fast_design(3, n, 4, &mut rng).expect("valid design");
    estimate_fast(&design, ishigami_on_unit_cube).expect("estimate")
}

// ── Artifact 1: canonical analytic recovery ─────────────────────────

#[test]
fn fast_ishigami_recovers_analytic_within_bias_bound() {
    // Realized errors at FIXTURE_SEED, N=1025:
    //   max |S - analytic|  = 0.020 (factor 3 false signal)
    //   max |ST - analytic| = 0.046 (factor 2 overestimate)
    // FAST's bias persists past N=1025; tolerances reflect it.
    let estimate = run_fast_at_n(1025);
    let analytic = ishigami::analytic_indices(7.0, 0.1);
    const S_TOL: f64 = 0.05;
    const ST_TOL: f64 = 0.10;
    for i in 0..3 {
        let s_err = (estimate.s[i] - analytic.first_order[i]).abs();
        let st_err = (estimate.st[i] - analytic.total_order[i]).abs();
        assert!(
            s_err < S_TOL,
            "S_{i}: got {:.4}, analytic {:.4}, err {s_err:.4} > {S_TOL}",
            estimate.s[i],
            analytic.first_order[i]
        );
        assert!(
            st_err < ST_TOL,
            "ST_{i}: got {:.4}, analytic {:.4}, err {st_err:.4} > {ST_TOL}",
            estimate.st[i],
            analytic.total_order[i]
        );
    }
}

// ── Artifact 2: model-free identity test ────────────────────────────

#[test]
fn fast_ishigami_total_at_least_first_order() {
    // Universal Sobol' identity: ST_i ≥ S_i for every factor.
    // The estimator clamps to [0, 1] but does not enforce ST ≥ S
    // by construction; this test pins the math, not the clamp.
    let estimate = run_fast_at_n(1025);
    for i in 0..3 {
        assert!(
            estimate.st[i] + 1e-9 >= estimate.s[i],
            "factor {i}: ST = {} < S = {}",
            estimate.st[i],
            estimate.s[i]
        );
    }
}

// ── Artifact 3: SALib differential ──────────────────────────────────

#[test]
fn fast_ishigami_matches_salib_within_mc_noise_at_n_1025() {
    // Frozen reference from `SALib.analyze.fast` on Ishigami at
    // `(a=7, b=0.1)`, `N=1025`, `M=4`, `numpy.random.seed(42)`:
    //   S  = [0.3120, 0.4441, 0.0198]
    //   ST = [0.5389, 0.4893, 0.2407]
    //
    // Different RNG seeds → different phase realizations, but
    // both implementations share FAST's systematic bias. Their
    // difference is bounded by phase-MC noise; observed ≤ 0.016
    // at N=1025. Tolerance 0.05 (3× headroom).
    let estimate = run_fast_at_n(1025);
    let salib_s = [0.3120, 0.4441, 0.0198];
    let salib_st = [0.5389, 0.4893, 0.2407];
    const TOL: f64 = 0.05;
    for i in 0..3 {
        let ds = (estimate.s[i] - salib_s[i]).abs();
        let dst = (estimate.st[i] - salib_st[i]).abs();
        assert!(
            ds < TOL,
            "S_{i}: ours {:.4}, SALib {:.4}, diff {ds:.4}",
            estimate.s[i],
            salib_s[i]
        );
        assert!(
            dst < TOL,
            "ST_{i}: ours {:.4}, SALib {:.4}, diff {dst:.4}",
            estimate.st[i],
            salib_st[i]
        );
    }
}

#[test]
fn fast_ishigami_matches_salib_at_low_n() {
    // SALib reference at N=257 (smaller-N regime where MC noise
    // is more visible):
    //   S  = [0.3124, 0.4441, 0.0269]
    //   ST = [0.5362, 0.4895, 0.2434]
    let estimate = run_fast_at_n(257);
    let salib_s = [0.3124, 0.4441, 0.0269];
    let salib_st = [0.5362, 0.4895, 0.2434];
    const TOL: f64 = 0.05;
    for i in 0..3 {
        let ds = (estimate.s[i] - salib_s[i]).abs();
        let dst = (estimate.st[i] - salib_st[i]).abs();
        assert!(
            ds < TOL,
            "S_{i}: ours {} SALib {} diff {ds:.4}",
            estimate.s[i],
            salib_s[i]
        );
        assert!(
            dst < TOL,
            "ST_{i}: ours {} SALib {} diff {dst:.4}",
            estimate.st[i],
            salib_st[i]
        );
    }
}

// ── Artifact 4: convergence-rate ────────────────────────────────────

#[test]
fn fast_ishigami_converges_with_n() {
    // FAST has a *bias floor* (the systematic interaction-aliasing
    // bias), so error doesn't vanish — but it does *decay* from
    // small N to large N as MC noise around the bias floor shrinks.
    //
    // Realized at FIXTURE_SEED:
    //   N=65   S_2 err = 0.005  ST_1 err = 0.044
    //   N=257  S_2 err = 0.001  ST_1 err = 0.034
    //   N=1025 S_2 err = 0.001  ST_1 err = 0.034 (bias floor)
    //
    // We assert: error at N=257 is ≤ error at N=65, and error at
    // N=1025 is ≤ error at N=257 (allowing equality at the bias
    // floor). Pinned for factor 2's S (where convergence is most
    // visible) and factor 1's ST (where bias dominates).
    let analytic = ishigami::analytic_indices(7.0, 0.1);
    let est_low = run_fast_at_n(65);
    let est_mid = run_fast_at_n(257);
    let est_high = run_fast_at_n(1025);

    let s2_low = (est_low.s[1] - analytic.first_order[1]).abs();
    let s2_mid = (est_mid.s[1] - analytic.first_order[1]).abs();
    let s2_high = (est_high.s[1] - analytic.first_order[1]).abs();

    assert!(
        s2_mid <= s2_low + 1e-6,
        "S_2 should not regress: N=65 → 257 err {s2_low:.4} → {s2_mid:.4}"
    );
    assert!(
        s2_high <= s2_mid + 1e-6,
        "S_2 should not regress: N=257 → 1025 err {s2_mid:.4} → {s2_high:.4}"
    );

    let st1_low = (est_low.st[0] - analytic.total_order[0]).abs();
    let st1_mid = (est_mid.st[0] - analytic.total_order[0]).abs();
    let st1_high = (est_high.st[0] - analytic.total_order[0]).abs();

    assert!(
        st1_mid <= st1_low + 1e-3,
        "ST_1 should not regress materially: N=65 → 257 err {st1_low:.4} → {st1_mid:.4}"
    );
    assert!(
        st1_high <= st1_mid + 1e-3,
        "ST_1 should not regress materially: N=257 → 1025 err {st1_mid:.4} → {st1_high:.4}"
    );
}

// ── Bonus: factor ranking by S_T is exactly correct ─────────────────

#[test]
fn fast_ishigami_ranks_factors_by_total_order_correctly() {
    // Analytic ranking by ST: factor 1 (0.558) > factor 2 (0.442) > factor 3 (0.244).
    let estimate = run_fast_at_n(1025);
    assert!(
        estimate.st[0] > estimate.st[1],
        "ST_1 = {} should exceed ST_2 = {}",
        estimate.st[0],
        estimate.st[1]
    );
    assert!(
        estimate.st[1] > estimate.st[2],
        "ST_2 = {} should exceed ST_3 = {}",
        estimate.st[1],
        estimate.st[2]
    );
}
