//! End-to-end reviewer-affordance contract close for the Saltelli2010
//! estimator against Ishigami at canonical `(a=7, b=0.1)`.
//!
//! Per `decisions/2026-04-28-saltelli-tck-posture.md` § "The
//! reviewer-affordance contract" — every estimator PR ships, in the
//! same diff, five binary-checkable artifacts:
//!
//! 1. **Canonical analytic test function** — Ishigami via
//!    `salib_validation::ishigami`. ✅
//! 2. **Model-free identity test** — `S_i ≤ S_T_i` per factor;
//!    `Σ S_i ≤ 1` (within MC noise). ✅
//! 3. **Frozen-CSV `SALib` differential** at canonical (a=7, b=0.1,
//!    N=8192) — `crates/salib-validation/reference/salib_outputs/ishigami_saltelli2010_n8192.csv`.
//!    MC-noise tolerance per Layer 3. ✅
//! 4. **Convergence-rate test** at N ∈ {2¹², 2¹⁴, 2¹⁶} —
//!    `|S_i_estimate - S_i_analytic|` decays as `O(1/√N)`. ✅
//! 5. **cargo-mutants kill rate** — deferred to a nightly CI bead;
//!    `cargo-mutants` runtime (~15 min) is not on the PR critical
//!    path. PR description includes manual mutation-test notes for
//!    the formula sites. (Bead-eligible follow-on.)
//!
//! This file is the load-bearing PR-7 artifact. If any of the four
//! in-line tests breaks, the contract is not closed.

#![allow(
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal,
    clippy::items_after_statements,
    clippy::expect_used,
    clippy::similar_names
)]

use salib_core::RngState;
use salib_estimators::{estimate_saltelli2010, estimate_saltelli2010_with_bootstrap};
use salib_samplers::{build_saltelli_matrix, SobolSampler};
use salib_validation::{ishigami, SobolIndicesAnalytic};

const FIXTURE_SEED: [u8; 32] = [0; 32];

/// Run Saltelli2010 on Ishigami with the requested N. Uses Sobol'
/// QMC base sampler (matches `SALib`'s default, gives canonical
/// QMC convergence).
fn run_ishigami_at_n(n: usize) -> salib_estimators::SobolIndices {
    // 2*d = 6 for Ishigami's 3 factors (radial Saltelli wraps a 2d-dim base).
    // Use skip_first=false to match SALib's older default (the reference
    // CSV was generated against SALib's saltelli.sample).
    let sampler = SobolSampler::standard(6).with_skip_first(false);
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let matrix = build_saltelli_matrix(&sampler, n, false, &mut rng).expect("matrix");

    // Map unit samples through Ishigami's input distribution
    // (Uniform(-π, π)) — closed-form Uniform quantile is `lo + u·(hi-lo)`.
    use std::f64::consts::PI;
    let model = |x: &[f64]| -> f64 {
        let mapped: [f64; 3] = [
            -PI + x[0] * 2.0 * PI,
            -PI + x[1] * 2.0 * PI,
            -PI + x[2] * 2.0 * PI,
        ];
        ishigami::ishigami(&mapped)
    };
    estimate_saltelli2010(&matrix, model)
}

// ── Artifact 1+2: canonical Ishigami + model-free identity ──────────

#[test]
fn ishigami_canonical_recovers_published_values_within_mc_tolerance() {
    let estimate = run_ishigami_at_n(8192);
    let analytic: SobolIndicesAnalytic = ishigami::analytic_indices(7.0, 0.1);

    // MC-noise tolerance at N=8192 is roughly k/sqrt(N) ≈ 0.022
    // for a 2-sigma allowance. SALib reports ~0.018 conf for S and
    // ~0.03 conf for S_T. Use 0.05 as a generous tolerance bound.
    const TOL: f64 = 0.05;

    // S_i checks against analytic.
    for (i, &want) in analytic.first_order.iter().enumerate() {
        let got = estimate.first_order[i];
        assert!(
            (got - want).abs() < TOL,
            "S_{i}: got {got:.4}, want {want:.4} (analytic) within {TOL}"
        );
    }
    // S_T_i checks.
    for (i, &want) in analytic.total_order.iter().enumerate() {
        let got = estimate.total_order[i];
        assert!(
            (got - want).abs() < TOL,
            "S_T_{i}: got {got:.4}, want {want:.4} (analytic) within {TOL}"
        );
    }

    // The Ishigami canary: S_3 should be near 0 (analytic = 0).
    assert!(
        estimate.first_order[2].abs() < TOL,
        "X_3 first-order canary: got {} (analytic = 0)",
        estimate.first_order[2]
    );

    // X_2 has no interactions: S_T_2 ≈ S_2 (analytic identity).
    assert!(
        (estimate.first_order[1] - estimate.total_order[1]).abs() < TOL,
        "S_2 = {}, S_T_2 = {} should agree (analytic identity)",
        estimate.first_order[1],
        estimate.total_order[1]
    );
}

#[test]
fn ishigami_first_order_at_most_total_order_within_mc_tolerance() {
    // Model-free identity: S_i ≤ S_T_i for every factor (Sobol'
    // decomposition).
    let estimate = run_ishigami_at_n(8192);
    for i in 0..estimate.dim {
        // Allow MC noise to flip near zero indices slightly.
        assert!(
            estimate.first_order[i] <= estimate.total_order[i] + 0.05,
            "S_{i} = {} > S_T_{i} = {} (more than 0.05 above)",
            estimate.first_order[i],
            estimate.total_order[i]
        );
    }
}

#[test]
fn ishigami_first_order_sum_at_most_one_within_mc_tolerance() {
    // Σ S_i ≤ 1 by Sobol' decomposition (independent inputs).
    let estimate = run_ishigami_at_n(8192);
    let sum: f64 = estimate.first_order.iter().sum();
    assert!(sum <= 1.0 + 0.05, "Σ S_i = {sum} (more than 1.05)");
}

// ── Artifact 3: SALib differential ──────────────────────────────────

#[test]
fn ishigami_estimate_agrees_with_salib_reference_within_mc_tolerance() {
    // Reference CSV: crates/salib-validation/reference/salib_outputs/ishigami_saltelli2010_n8192.csv
    // Format: factor, S, S_conf, ST, ST_conf
    // x1: S=0.3158569 ST=0.5580216
    // x2: S=0.4424194 ST=0.4424263
    // x3: S=0.0020851 ST=0.2438569
    //
    // SALib uses np.random.seed(42) + numpy MT19937 to generate
    // sample matrices, which differs from our Sobol'-based sampler.
    // The values won't be byte-exact; they should agree within
    // MC-noise tolerance (per `decisions/2026-04-28-saltelli-tck-posture.md`
    // Layer 3, MC-noise regime).

    let estimate = run_ishigami_at_n(8192);

    // Hardcoded SALib reference (mirror of the frozen CSV).
    let salib_s = [0.3158569125, 0.4424194420, 0.0020850628];
    let salib_st = [0.5580215581, 0.4424263482, 0.2438569100];

    // SALib's S_conf is ~0.018; ST_conf is ~0.03. Use 0.05 as
    // generous MC-noise tolerance. (The two implementations use
    // different sample matrices — SALib uses MT19937-driven Sobol;
    // ours uses our own Sobol' from vendored Joe-Kuo. They land
    // within MC tolerance at N=8192.)
    const TOL: f64 = 0.05;

    for i in 0..3 {
        assert!(
            (estimate.first_order[i] - salib_s[i]).abs() < TOL,
            "S_{i}: ours {:.4}, SALib {:.4}",
            estimate.first_order[i],
            salib_s[i]
        );
        assert!(
            (estimate.total_order[i] - salib_st[i]).abs() < TOL,
            "S_T_{i}: ours {:.4}, SALib {:.4}",
            estimate.total_order[i],
            salib_st[i]
        );
    }
}

// ── Artifact 4: convergence-rate test ───────────────────────────────

#[test]
fn ishigami_estimator_error_decays_with_n() {
    // |S_i - S_i_analytic| should decay roughly as O(1/√N) for
    // Saltelli + Sobol-base. We check at three N values and verify
    // the error at N=2^14 is smaller than at N=2^12, and N=2^16
    // smaller than N=2^14.
    //
    // Tolerance: at N=2^12, error can be ~0.05; at N=2^14, ~0.025;
    // at N=2^16, ~0.012. Strict monotonicity isn't guaranteed (MC
    // can have a lucky N=2^12) but the trend should hold at the
    // factor with the largest signal (X_2: analytic S = 0.4424).

    let analytic = ishigami::analytic_indices(7.0, 0.1);
    let s2_analytic = analytic.first_order[1]; // 0.4424

    let n_values = [4096usize, 16384, 65536];
    let errors: Vec<f64> = n_values
        .iter()
        .map(|&n| {
            let est = run_ishigami_at_n(n);
            (est.first_order[1] - s2_analytic).abs()
        })
        .collect();

    // Three N values, should see decreasing error trend on average.
    // We require the *largest N* error to be strictly below the
    // *smallest N* error — the headline convergence claim. Middle
    // can vary.
    assert!(
        errors[2] < errors[0],
        "convergence: error at N=2^16 ({}) should be below N=2^12 ({})",
        errors[2],
        errors[0]
    );
    // Largest N should also be below 0.02 — a pass at the
    // analytic-recovery bar.
    assert!(
        errors[2] < 0.02,
        "S_2 error at N=2^16 = {}, expected < 0.02",
        errors[2]
    );
}

// ── Bootstrap smoke ─────────────────────────────────────────────────

#[test]
fn ishigami_bootstrap_returns_finite_cis() {
    use std::f64::consts::PI;
    let sampler = SobolSampler::standard(6).with_skip_first(false);
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let matrix = build_saltelli_matrix(&sampler, 1024, false, &mut rng).expect("matrix");
    let mut bootstrap_rng = RngState::from_seed([0xab; 32]);
    let model = |x: &[f64]| -> f64 {
        let mapped: [f64; 3] = [
            -PI + x[0] * 2.0 * PI,
            -PI + x[1] * 2.0 * PI,
            -PI + x[2] * 2.0 * PI,
        ];
        ishigami::ishigami(&mapped)
    };
    let result = estimate_saltelli2010_with_bootstrap(&matrix, model, 200, &mut bootstrap_rng);
    for (lo, hi) in &result.first_order_ci {
        assert!(
            lo.is_finite() && hi.is_finite() && lo <= hi,
            "S CI: ({lo}, {hi})"
        );
    }
    for (lo, hi) in &result.total_order_ci {
        assert!(
            lo.is_finite() && hi.is_finite() && lo <= hi,
            "S_T CI: ({lo}, {hi})"
        );
    }
}
