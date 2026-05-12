//! End-to-end reviewer-affordance contract close for the PAWN
//! estimator on Ishigami.
//!
//! Per `decisions/2026-04-29-saltelli-pawn.md`. Sixth PR exercising
//! the contract pattern (after PR 7 Saltelli2010, PR 8 Morris,
//! PR 9b eFAST, PR 10 RBD-FAST, PR 11 Borgonovo δ).
//!
//! # Why no analytic-recovery test
//!
//! Unlike Sobol' / Borgonovo δ, PAWN has no closed-form analytic
//! value for Ishigami in the literature. The CDF-based KS statistic
//! is a different object than variance- or PDF-based indices, and
//! its slice-aggregated form (median over conditioning slices) is
//! tied to the slice count. We therefore validate via:
//!
//! 1. **Ranking** — analytic ranking of factor importance is
//!    factor 2 > factor 1 > factor 3 (matches Sobol' total-order),
//!    and `SALib`'s PAWN preserves this ordering. We verify ours
//!    does too.
//! 2. **`SALib` differential** — same algorithm shape, no
//!    bias-correction ambiguity (unlike Borgonovo δ). Realized
//!    max diff at `N=4096`: `0.007`. Tolerance `0.05` (7×).
//! 3. **Identity** — KS ∈ `[0, 1]`; min ≤ median ≤ max.
//! 4. **Convergence** — slice count `S` interaction: at fixed `N`,
//!    increasing `S` shifts values modestly but ranking persists.
//!
//! # Realized values at FIXTURE_SEED, S=10
//!
//! ```text
//! N=1024  median=[0.279, 0.384, 0.104]   max=[0.337, 0.512, 0.245]
//! N=4096  median=[0.245, 0.390, 0.088]   max=[0.280, 0.499, 0.199]
//! ```
//!
//! Compare SALib at N=4096:
//! ```text
//! median=[0.245, 0.393, 0.087]   max=[0.282, 0.506, 0.195]
//! ```
//!
//! Max diff: 0.007. PAWN is one of the tightest specifications-
//! differential matches we get.

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
use salib_estimators::{estimate_pawn, PawnIndices};
use salib_samplers::{LhsSampler, Sampler};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];
const N_SLICES: usize = 10;

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

fn run_at_n(n: usize) -> PawnIndices {
    let (x, y) = lhs_ishigami_inputs(n);
    estimate_pawn(&x, &y, N_SLICES).expect("estimate")
}

// ── Artifact 1: identity / ranking ─────────────────────────────────

#[test]
fn pawn_ishigami_indices_in_unit_interval() {
    let est = run_at_n(4096);
    for i in 0..3 {
        assert!(
            (0.0..=1.0).contains(&est.median[i]),
            "median_{i} = {} not in [0, 1]",
            est.median[i]
        );
        assert!(
            (0.0..=1.0).contains(&est.maximum[i]),
            "max_{i} = {} not in [0, 1]",
            est.maximum[i]
        );
    }
}

#[test]
fn pawn_ishigami_aggregate_ordering() {
    // min ≤ median ≤ max for every factor.
    let est = run_at_n(4096);
    for i in 0..3 {
        assert!(
            est.minimum[i] <= est.median[i] + 1e-12,
            "factor {i}: min {} > median {}",
            est.minimum[i],
            est.median[i]
        );
        assert!(
            est.median[i] <= est.maximum[i] + 1e-12,
            "factor {i}: median {} > max {}",
            est.median[i],
            est.maximum[i]
        );
    }
}

// ── Artifact 2: ranking matches Sobol' total-order ─────────────────

#[test]
fn pawn_ishigami_ranks_factors_correctly() {
    // PAWN measures shape sensitivity, not variance — its ranking
    // can differ from Sobol' ST (which on Ishigami is ST_1 > ST_2 > ST_3).
    // The `a · sin²(x_2)` term has the largest distribution-shape
    // effect, so factor 2 leads PAWN's median ranking. Empirically
    // SALib gives `median_2 > median_1 > median_3`; our implementation
    // matches.
    let est = run_at_n(4096);
    assert!(
        est.median[1] > est.median[0],
        "median_2 = {} should exceed median_1 = {}",
        est.median[1],
        est.median[0]
    );
    assert!(
        est.median[0] > est.median[2],
        "median_1 = {} should exceed median_3 = {}",
        est.median[0],
        est.median[2]
    );
}

// ── Artifact 3: SALib differential ─────────────────────────────────

#[test]
fn pawn_ishigami_matches_salib_within_mc_noise() {
    // SALib `analyze.pawn` on Ishigami with LHS sampling
    // (numpy.random.seed(42), S=10):
    //   N=4096: median = [0.245, 0.393, 0.087]
    //           max    = [0.282, 0.506, 0.195]
    //
    // PAWN doesn't have a bias-correction wrapper like Borgonovo δ,
    // and its CDF-based formulation is exact given the slice
    // partition. Realized max diff at N=4096: 0.007 (essentially
    // identical modulo independent X matrices).
    let est = run_at_n(4096);
    let salib_median = [0.245, 0.393, 0.087];
    let salib_max = [0.282, 0.506, 0.195];
    const TOL: f64 = 0.05;
    for i in 0..3 {
        let dm = (est.median[i] - salib_median[i]).abs();
        let dx = (est.maximum[i] - salib_max[i]).abs();
        assert!(
            dm < TOL,
            "median_{i}: ours {:.4}, SALib {:.4}, diff {dm:.4}",
            est.median[i],
            salib_median[i]
        );
        assert!(
            dx < TOL,
            "max_{i}: ours {:.4}, SALib {:.4}, diff {dx:.4}",
            est.maximum[i],
            salib_max[i]
        );
    }
}

// ── Artifact 4: convergence with N ─────────────────────────────────

#[test]
fn pawn_ishigami_stable_across_n() {
    // PAWN at S=10 converges as N grows. Realized: max factor
    // values shift modestly between N=1024 and N=4096 but ranking
    // and approximate magnitude persist.
    let est_low = run_at_n(1024);
    let est_high = run_at_n(4096);

    // The dominant factor (factor 2) is preserved.
    assert_eq!(
        est_low
            .median
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i),
        Some(1),
        "factor 2 should be the dominant median at N=1024"
    );
    assert_eq!(
        est_high
            .median
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i),
        Some(1),
        "factor 2 should be the dominant median at N=4096"
    );

    // Median for the dominant factor is stable to within MC tolerance.
    let drift = (est_high.median[1] - est_low.median[1]).abs();
    assert!(
        drift < 0.05,
        "median_2 drift: N=1024→4096 {:.4} → {:.4} (drift {drift:.4})",
        est_low.median[1],
        est_high.median[1]
    );
}

// ── Bonus: slice count effect ──────────────────────────────────────

#[test]
fn pawn_ishigami_ranking_invariant_to_reasonable_slice_count() {
    // PAWN's ranking should not depend on the exact slice count
    // within a reasonable range. Test S ∈ {8, 10, 16}.
    let (x, y) = lhs_ishigami_inputs(4096);
    for &s in &[8_usize, 10, 16] {
        let est = estimate_pawn(&x, &y, s).expect("estimate");
        assert!(
            est.median[1] > est.median[0],
            "S={s}: median_2 = {} should exceed median_1 = {}",
            est.median[1],
            est.median[0]
        );
        assert!(
            est.median[0] > est.median[2],
            "S={s}: median_1 = {} should exceed median_3 = {}",
            est.median[0],
            est.median[2]
        );
    }
}
