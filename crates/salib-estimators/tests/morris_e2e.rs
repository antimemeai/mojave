//! End-to-end reviewer-affordance contract close for the Morris
//! elementary-effects estimator against the additive-linear test
//! function `Y = Σ i · x_i` for `d = 8`.
//!
//! Per `decisions/2026-04-28-saltelli-tck-posture.md` — second PR
//! (after PR 7's Saltelli2010 close) exercising the contract pattern.
//!
//! Contract artifacts:
//!
//! 1. **Canonical analytic test function** — `salib_validation::morris_test`
//!    additive-linear with closed-form `μ_i = μ*_i = i`, `σ_i = 0`.
//! 2. **Model-free identity test** — `μ*_i ≥ |μ_i|` always (Campolongo
//!    2007).
//! 3. **Frozen `SALib` differential** — `SALib` Morris analyze on the
//!    same function at canonical `R=50, p=4` recovers the closed-form
//!    `μ` exactly (purely linear; σ = 0; `SALib` agrees bit-exactly
//!    with analytic). Hardcoded inline.
//! 4. **Convergence-rate test** — `|μ_i - μ_i_analytic|` at three R
//!    values. For purely linear, Morris recovers `μ` exactly at any
//!    R, so this is a degenerate convergence — the test asserts
//!    "error ≈ 0 at all R" rather than O(1/√R) decay. The R=1
//!    convergence-rate property exercises the framework even though
//!    linearity collapses the rate.
//! 5. **cargo-mutants kill rate** — deferred to nightly CI bead.
//!
//! For the *non-linear* convergence-rate test, see the inner unit
//! test `morris::tests::quadratic_factor_has_nonzero_sigma` plus
//! follow-on PRs that add quadratic / interaction test functions.
//!
//! # PR 8.6 appendix
//!
//! This module is extended below with a substantive contract close
//! over the **quadratic-additive** function `Y = Σ bᵢxᵢ + cᵢxᵢ²`
//! (`bᵢ = cᵢ = i+1`). That function has real MC noise on EE, so
//! artifacts 3 (`SALib` differential) and 4 (convergence-rate) are
//! non-degenerate. The linear contract above is preserved (it pins
//! a stronger bit-exact recovery claim in the linear regime).
//! Per `decisions/2026-04-29-saltelli-morris-quadratic-contract.md`.

#![allow(
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal,
    clippy::items_after_statements,
    clippy::expect_used,
    clippy::similar_names,
    clippy::cast_precision_loss
)]

use salib_core::RngState;
use salib_estimators::estimate_morris_effects;
use salib_samplers::build_morris_trajectories;
use salib_validation::{morris_test, MorrisEffectsAnalytic};

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn run_morris_at_r(d: usize, r: usize) -> salib_estimators::MorrisEffects {
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let trajectories = build_morris_trajectories(d, r, 4, &mut rng).expect("trajectories");
    let model = move |x: &[f64]| -> f64 { morris_test::morris_additive_linear_with_dim(x, d) };
    estimate_morris_effects(&trajectories, model).expect("estimate")
}

// ── Artifact 1: canonical analytic recovery ─────────────────────────

#[test]
fn morris_additive_linear_canonical_recovers_analytic_exactly() {
    // Linear function ⇒ μ_i = i exactly (no MC noise).
    let estimate = run_morris_at_r(8, 50);
    let analytic: MorrisEffectsAnalytic = morris_test::analytic_effects(8);
    for i in 0..8 {
        // Linear ⇒ μ_i = i exactly. Allow generous FP tolerance.
        assert!(
            (estimate.mu[i] - analytic.mu[i]).abs() < 1e-10,
            "μ_{i}: got {}, want {} (analytic exact)",
            estimate.mu[i],
            analytic.mu[i]
        );
        assert!(
            (estimate.mu_star[i] - analytic.mu_star[i]).abs() < 1e-10,
            "μ*_{i}: got {}, want {}",
            estimate.mu_star[i],
            analytic.mu_star[i]
        );
        // σ should be exactly 0 for purely linear.
        assert!(
            estimate.sigma[i].abs() < 1e-10,
            "σ_{i}: got {}, want 0 (purely linear)",
            estimate.sigma[i]
        );
    }
}

// ── Artifact 2: model-free identity test ────────────────────────────

#[test]
fn morris_mu_star_at_least_absolute_mu_for_canonical() {
    // Per Campolongo 2007: μ*_i ≥ |μ_i| always (mean of abs ≥ abs of mean).
    let estimate = run_morris_at_r(8, 50);
    for i in 0..8 {
        assert!(
            estimate.mu_star[i] >= estimate.mu[i].abs() - 1e-12,
            "μ*_{i} = {} should be ≥ |μ_{i}| = {}",
            estimate.mu_star[i],
            estimate.mu[i].abs()
        );
    }
}

// ── Artifact 3: SALib differential (degenerate for linear) ─────────
//
// **Honest disclosure.** For the additive-linear test function,
// SALib's Morris analyzer trivially recovers the exact analytic μ
// values — there is no MC noise on EE for purely linear functions.
// The "SALib reference" values below ARE the analytic values, by
// the math of the function. This artifact is therefore *not* a
// real differential test against SALib's implementation — it's a
// duplicate of Artifact 1's analytic check at a different name.
//
// A non-degenerate SALib differential lands when the Morris 1991
// §4 20-factor function (with non-linear terms producing real
// MC noise) is added in a follow-on PR. Bead-eligible.
//
// Documented in `decisions/2026-04-29-saltelli-morris-estimator.md`
// § "Reviewer-affordance contract — second close" item 3.

#[test]
fn morris_estimate_agrees_with_salib_for_linear_degenerate_case() {
    let estimate = run_morris_at_r(8, 50);
    // SALib values for additive-linear are the analytic coefficient
    // vector by definition (purely linear; SALib has nothing to
    // estimate beyond it). See `assert_salib_reference_is_analytic`
    // below for the explicit identity check.
    let salib_mu = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let salib_mu_star = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let salib_sigma = [0.0; 8];
    for i in 0..8 {
        assert!(
            (estimate.mu[i] - salib_mu[i]).abs() < 1e-10,
            "μ_{i}: ours {}, SALib {}",
            estimate.mu[i],
            salib_mu[i]
        );
        assert!(
            (estimate.mu_star[i] - salib_mu_star[i]).abs() < 1e-10,
            "μ*_{i}: ours {}, SALib {}",
            estimate.mu_star[i],
            salib_mu_star[i]
        );
        assert!(
            (estimate.sigma[i] - salib_sigma[i]).abs() < 1e-10,
            "σ_{i}: ours {}, SALib {}",
            estimate.sigma[i],
            salib_sigma[i]
        );
    }
}

#[test]
fn assert_salib_reference_is_analytic_for_linear_function() {
    // Self-disclosure of the degeneracy: the "SALib reference" used
    // above is the analytic coefficient vector for the additive-
    // linear function. This test fails if a future refactor
    // accidentally puts non-trivial values in the SALib reference
    // without also dropping the artifact-3 disclaimer above.
    let analytic = salib_validation::morris_test::analytic_effects(8);
    let salib_mu = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    for (i, (&salib, want)) in salib_mu.iter().zip(analytic.mu.iter()).enumerate() {
        assert_eq!(
            salib, *want,
            "factor {i}: the SALib reference values are degenerate copies of the analytic μ — \
             update the disclaimer in the e2e module's Artifact 3 comment if this changes"
        );
    }
}

// ── Artifact 4: convergence-rate (degenerate for purely linear) ────

#[test]
fn morris_converges_for_purely_linear_at_all_r() {
    // For a purely linear function, Morris elementary effects are
    // CONSTANT (independent of base point). So estimator error
    // is ≈0 at any R ≥ 1. The convergence-rate "test" here is
    // degenerate — just asserts the error stays at FP-zero across
    // R ∈ {1, 10, 50}. The framework runs; the rate is collapsed
    // by linearity.
    let analytic = morris_test::analytic_effects(8);
    let r_values = [1, 10, 50];
    for &r in &r_values {
        let estimate = run_morris_at_r(8, r);
        for i in 0..8 {
            assert!(
                (estimate.mu[i] - analytic.mu[i]).abs() < 1e-10,
                "R={r} μ_{i} error: got {}, want {} (linear convergence is exact)",
                estimate.mu[i],
                analytic.mu[i]
            );
        }
    }
}

// ── Bonus: factor ranking is exact ──────────────────────────────────

#[test]
fn morris_factor_ranking_exactly_matches_coefficient_ordering() {
    // For the additive-linear function with b = [1, 2, …, 8],
    // factor ordering by μ* is exactly [factor 0, factor 1, …,
    // factor 7] from smallest to largest effect.
    let estimate = run_morris_at_r(8, 30);
    for i in 1..8 {
        assert!(
            estimate.mu_star[i] > estimate.mu_star[i - 1],
            "μ*_{i} = {} should exceed μ*_{} = {}",
            estimate.mu_star[i],
            i - 1,
            estimate.mu_star[i - 1]
        );
    }
}

// ════════════════════════════════════════════════════════════════════
// PR 8.6 — quadratic-additive contract close.
//
// The additive-linear case above is degenerate on artifacts 3 and 4
// (no MC noise on EE for purely linear). The quadratic-additive
// function `Y = Σ bᵢxᵢ + cᵢxᵢ²` with `bᵢ = cᵢ = i+1` produces real
// MC noise on σ (and on μ at finite R), making both artifacts
// substantive. Per
// `decisions/2026-04-29-saltelli-morris-quadratic-contract.md`.
//
// Closed-form (Morris p=4, base ∈ {0, 1/3}, Δ=2/3):
//   μᵢ = bᵢ + cᵢ = 2(i+1)            → [2, 4, 6, ..., 16]
//   σᵢ = |cᵢ| / 3 = (i+1) / 3          → [0.333, 0.667, ..., 2.667]
// ════════════════════════════════════════════════════════════════════

fn run_morris_quadratic_at_r(d: usize, r: usize) -> salib_estimators::MorrisEffects {
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let trajectories = build_morris_trajectories(d, r, 4, &mut rng).expect("trajectories");
    let model = move |x: &[f64]| -> f64 { morris_test::morris_quadratic_additive_with_dim(x, d) };
    estimate_morris_effects(&trajectories, model).expect("estimate")
}

// ── Artifact 1 (substantive): canonical analytic recovery ──────────

#[test]
fn morris_quadratic_canonical_recovers_analytic_within_mc_tolerance() {
    let estimate = run_morris_quadratic_at_r(8, 1000);
    let analytic = morris_test::analytic_quadratic_effects(8);
    // Realized errors at FIXTURE_SEED, R=1000:
    //   max |μ_est - μ_analytic| = 0.0513 (factor 6)
    //   max |σ_est - σ_analytic| = 0.0013 (factor 7)
    // Tolerances ≈ 3× realized for headroom against benign numerical
    // perturbation in the trajectory pipeline.
    const MU_TOL: f64 = 0.15;
    const SIGMA_TOL: f64 = 0.05;
    for i in 0..8 {
        assert!(
            (estimate.mu[i] - analytic.mu[i]).abs() < MU_TOL,
            "μ_{i}: got {:.4}, want {:.4}",
            estimate.mu[i],
            analytic.mu[i]
        );
        assert!(
            (estimate.sigma[i] - analytic.sigma[i]).abs() < SIGMA_TOL,
            "σ_{i}: got {:.4}, want {:.4}",
            estimate.sigma[i],
            analytic.sigma[i]
        );
    }
}

// ── Artifact 2 (substantive): identity test on quadratic ───────────

#[test]
fn morris_quadratic_mu_star_at_least_absolute_mu() {
    let estimate = run_morris_quadratic_at_r(8, 100);
    for i in 0..8 {
        assert!(
            estimate.mu_star[i] >= estimate.mu[i].abs() - 1e-12,
            "μ*_{i} = {} should be ≥ |μ_{i}| = {}",
            estimate.mu_star[i],
            estimate.mu[i].abs()
        );
    }
}

// ── Artifact 3 (substantive): SALib differential ────────────────────

#[test]
fn morris_quadratic_estimate_agrees_with_salib_within_mc_tolerance() {
    // Frozen reference from SALib morris.analyze on the quadratic-
    // additive function at R=1000, p=4, numpy seed=42:
    //   μ = [2.000, 3.993, 5.988, 8.080, 9.967, 12.012, 13.995, 15.941]
    //   σ = [0.333, 0.667, 1.000, 1.332, 1.667, 2.001, 2.335, 2.667]
    //
    // Both implementations use different trajectory samplers (SALib's
    // numpy-seeded vs ours `RngState::from_seed([0; 32])`), so the
    // values differ by MC noise — but both converge to the analytic
    // (μ = [2, 4, ..., 16], σ = [0.333, ..., 2.667]) as R → ∞.
    //
    // Tolerance: each implementation has ~0.1 MC noise at R=1000;
    // their *difference* is bounded by ~0.2 (variance adds).
    let estimate = run_morris_quadratic_at_r(8, 1000);
    let salib_mu = [2.000, 3.993, 5.988, 8.080, 9.967, 12.012, 13.995, 15.941];
    let salib_sigma = [0.333, 0.667, 1.000, 1.332, 1.667, 2.001, 2.335, 2.667];
    const MU_TOL: f64 = 0.2;
    const SIGMA_TOL: f64 = 0.2;
    for i in 0..8 {
        let our_mu = estimate.mu[i];
        let our_sigma = estimate.sigma[i];
        let s_mu = salib_mu[i];
        let s_sigma = salib_sigma[i];
        assert!(
            (our_mu - s_mu).abs() < MU_TOL,
            "μ_{i}: ours {our_mu:.4}, SALib {s_mu:.4}"
        );
        assert!(
            (our_sigma - s_sigma).abs() < SIGMA_TOL,
            "σ_{i}: ours {our_sigma:.4}, SALib {s_sigma:.4}"
        );
    }
}

// ── Artifact 4 (substantive): convergence-rate ──────────────────────

#[test]
fn morris_quadratic_error_decays_with_r() {
    // For non-linear EE distributions, |μ_estimate - μ_analytic|
    // decays as O(1/√R). At FIXTURE_SEED with the largest-σ factor
    // (i = 7, σ_pop = 8/3), realized errors are:
    //   R=50:    μ_err = 0.5333, σ_err = 0.0274
    //   R=1000:  μ_err = 0.0107, σ_err = 0.0013
    // Both monotonically decay — assert both, deterministically.
    let analytic = morris_test::analytic_quadratic_effects(8);
    let i = 7;

    let est_low = run_morris_quadratic_at_r(8, 50);
    let err_low_mu = (est_low.mu[i] - analytic.mu[i]).abs();
    let err_low_sigma = (est_low.sigma[i] - analytic.sigma[i]).abs();

    let est_high = run_morris_quadratic_at_r(8, 1000);
    let err_high_mu = (est_high.mu[i] - analytic.mu[i]).abs();
    let err_high_sigma = (est_high.sigma[i] - analytic.sigma[i]).abs();

    assert!(
        err_high_mu < err_low_mu,
        "μ did not decay for factor {i}: R=50 → R=1000: \
         {err_low_mu:.4} → {err_high_mu:.4}"
    );
    assert!(
        err_high_sigma < err_low_sigma,
        "σ did not decay for factor {i}: R=50 → R=1000: \
         {err_low_sigma:.4} → {err_high_sigma:.4}"
    );
    // Absolute bound at R=1000 (3× realized headroom).
    assert!(
        err_high_mu < 0.05,
        "μ_{i} error at R=1000: {err_high_mu:.4} not < 0.05"
    );
    assert!(
        err_high_sigma < 0.01,
        "σ_{i} error at R=1000: {err_high_sigma:.4} not < 0.01"
    );
}
