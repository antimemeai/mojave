#![allow(clippy::unwrap_used, clippy::expect_used)]

use proptest::prelude::*;
use seq_anytime_valid::boundary::{obf, spending};
use seq_anytime_valid::evidence::e_value;
use seq_anytime_valid::types::Decision;

/// Inline A&S 26.2.23 normal quantile (mirrors `obf::normal_quantile`
/// which is `pub(crate)` and therefore inaccessible from integration tests).
fn normal_quantile(p: f64) -> f64 {
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    if (p - 0.5).abs() < f64::EPSILON {
        return 0.0;
    }
    let (sign, pp) = if p < 0.5 { (-1.0, p) } else { (1.0, 1.0 - p) };
    let t = (-2.0 * pp.ln()).sqrt();
    let c0 = 2.515_517;
    let c1 = 0.802_853;
    let c2 = 0.010_328;
    let d1 = 1.432_788;
    let d2 = 0.189_269;
    let d3 = 0.001_308;
    sign * (t - (c0 + c1 * t + c2 * t * t) / (1.0 + d1 * t + d2 * t * t + d3 * t * t * t))
}

proptest! {
    // K=1 group-sequential = fixed-sample z_{alpha/2}
    #[test]
    fn obf_k1_equals_z_alpha_half(alpha in 0.001..0.2_f64) {
        let b_obf = obf::boundary(1, 1, alpha).unwrap();
        let z = normal_quantile(1.0 - alpha / 2.0);
        prop_assert!((b_obf - z).abs() < 0.01,
            "OBF K=1 should equal z_alpha/2: {b_obf} vs {z}");
    }

    // OBF boundaries are non-increasing in look index
    #[test]
    fn obf_boundaries_monotone_decreasing(k in 2..10_usize) {
        let bs = obf::boundaries(k, 0.05).unwrap();
        for i in 1..bs.len() {
            prop_assert!(bs[i] <= bs[i - 1] + 1e-10,
                "OBF boundary at look {} ({}) > look {} ({})", i+1, bs[i], i, bs[i-1]);
        }
    }

    // Alpha-spending cumulative at t=1.0 = nominal alpha
    #[test]
    fn spending_exhaustion(alpha in 0.001..0.2_f64) {
        let s = spending::pocock_spending(1.0, alpha);
        prop_assert!((s - alpha).abs() < 1e-10,
            "Pocock spending at t=1 should equal alpha: {s} vs {alpha}");
    }

    // Spending function is non-decreasing
    #[test]
    fn spending_monotone(alpha in 0.001..0.2_f64) {
        let mut prev = 0.0_f64;
        for i in 0..=100 {
            let t = i as f64 / 100.0;
            let s = spending::pocock_spending(t, alpha);
            prop_assert!(s >= prev - 1e-10,
                "spending not monotone at t={t}: {s} < {prev}");
            prev = s;
        }
    }

    // E-value threshold: E >= 1/alpha => Reject
    #[test]
    fn e_value_threshold_consistency(alpha in 0.001..0.5_f64, e in 0.1..100.0_f64) {
        let d = e_value::threshold_decision(e, alpha);
        if e >= 1.0 / alpha {
            prop_assert_eq!(d, Decision::Reject);
        } else {
            prop_assert_eq!(d, Decision::Continue);
        }
    }

    // SPRT with degenerate H0=H1 always errors
    #[test]
    fn sprt_degenerate_errors(theta in -10.0..10.0_f64) {
        let p = theta.abs().clamp(0.001, 0.999);
        let result = seq_anytime_valid::evidence::likelihood::bernoulli_cumulative_log_lr(
            &[1.0], p, p
        );
        prop_assert!(matches!(result, Err(seq_anytime_valid::SeqError::DegenerateHypotheses)));
    }

    // Observation reorder invariance for cumulative LR
    #[test]
    fn observation_reorder_invariance(
        seed in 0..1000_u64,
    ) {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        use rand::seq::SliceRandom;

        let mut rng = StdRng::seed_from_u64(seed);
        let obs: Vec<f64> = vec![1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0];
        let lr1 = seq_anytime_valid::evidence::likelihood::bernoulli_cumulative_log_lr(
            &obs, 0.3, 0.7
        ).unwrap();
        let mut shuffled = obs.clone();
        shuffled.shuffle(&mut rng);
        let lr2 = seq_anytime_valid::evidence::likelihood::bernoulli_cumulative_log_lr(
            &shuffled, 0.3, 0.7
        ).unwrap();
        prop_assert!((lr1 - lr2).abs() < 1e-10,
            "cumulative LR should be permutation-invariant: {lr1} vs {lr2}");
    }
}
