#![allow(clippy::unwrap_used, clippy::expect_used)]

use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_distr::{Bernoulli, Distribution};
use seq_anytime_valid::monitor::anytime::AnytimeMonitor;
use seq_anytime_valid::types::{DataFamily, MsprtConfig};

/// Gate 4: Monte Carlo coverage test for AnytimeMonitor production path.
///
/// For each p in {0.1, 0.3, 0.5, 0.7, 0.9}, generate 10,000 independent
/// Bernoulli(p) streams of length N=200. Feed each through AnytimeMonitor
/// configured with DataFamily::Bernoulli and alpha=0.05. Check that the
/// final confidence interval contains the true p in >=93% of replications.
/// (93% not 95% to account for finite-sample simulation noise.)
#[test]
fn anytime_monitor_bernoulli_coverage_gate4() {
    let alpha = 0.05;
    let n_reps = 10_000;
    let n_obs = 200;
    let test_ps = [0.1, 0.3, 0.5, 0.7, 0.9];

    for (p_idx, &true_p) in test_ps.iter().enumerate() {
        let mut covered = 0u32;
        let dist = Bernoulli::new(true_p).unwrap();

        for rep in 0..n_reps {
            let seed = (p_idx as u64) * 1_000_000 + rep as u64;
            let mut rng = StdRng::seed_from_u64(seed);

            let config = MsprtConfig {
                theta_0: 0.0,
                mixing_variance: 1.0,
                family: DataFamily::Bernoulli,
                max_samples: None,
            };
            let mut monitor = AnytimeMonitor::new(config, alpha).unwrap();

            let mut last_ci = None;
            for _ in 0..n_obs {
                let obs = if dist.sample(&mut rng) { 1.0 } else { 0.0 };
                let snap = monitor.update(obs).unwrap();
                last_ci = snap.confidence_interval;
            }

            if let Some((lo, hi)) = last_ci {
                if lo <= true_p && true_p <= hi {
                    covered += 1;
                }
            }
        }

        let coverage = covered as f64 / n_reps as f64;
        assert!(
            coverage >= 0.93,
            "Coverage at p={true_p}: {coverage:.4} < 0.93"
        );
    }
}

/// Gate 4: Type I error control test for AnytimeMonitor.
///
/// Under H0 (true_p == theta_0 == 0.5), the e-value should exceed
/// 1/alpha in at most alpha fraction of replications. This verifies
/// that the test does not falsely reject when the null is true.
#[test]
fn anytime_monitor_bernoulli_type1_error_gate4() {
    let alpha = 0.05;
    let n_reps = 10_000;
    let n_obs = 200;
    let true_p = 0.5;
    let theta_0 = 0.5;
    let threshold = 1.0 / alpha; // e-value threshold for rejection

    let mut false_rejections = 0u32;
    let dist = Bernoulli::new(true_p).unwrap();

    for rep in 0..n_reps {
        let seed = 7_000_000 + rep as u64;
        let mut rng = StdRng::seed_from_u64(seed);

        let config = MsprtConfig {
            theta_0,
            mixing_variance: 1.0,
            family: DataFamily::Bernoulli,
            max_samples: None,
        };
        let mut monitor = AnytimeMonitor::new(config, alpha).unwrap();

        let mut max_e_value = 0.0_f64;
        for _ in 0..n_obs {
            let obs = if dist.sample(&mut rng) { 1.0 } else { 0.0 };
            let snap = monitor.update(obs).unwrap();
            if let Some(e) = snap.e_value {
                max_e_value = max_e_value.max(e);
            }
        }

        if max_e_value >= threshold {
            false_rejections += 1;
        }
    }

    let false_rejection_rate = false_rejections as f64 / n_reps as f64;
    // By Ville's inequality, the false rejection rate should be <= alpha.
    // Allow a small margin for simulation noise.
    assert!(
        false_rejection_rate <= alpha + 0.02,
        "Type I error rate {false_rejection_rate:.4} exceeds alpha={alpha} + margin"
    );
}

/// Verify that AnytimeMonitor Bernoulli CI width shrinks with more data.
/// This catches degenerate intervals that never tighten.
#[test]
fn anytime_monitor_bernoulli_ci_shrinks() {
    let alpha = 0.05;
    let config = MsprtConfig {
        theta_0: 0.0,
        mixing_variance: 1.0,
        family: DataFamily::Bernoulli,
        max_samples: None,
    };
    let mut monitor = AnytimeMonitor::new(config, alpha).unwrap();

    let mut rng = StdRng::seed_from_u64(42);
    let dist = Bernoulli::new(0.5).unwrap();

    let mut width_at_50 = f64::INFINITY;
    let mut width_at_200 = f64::INFINITY;

    for i in 1..=200 {
        let obs = if dist.sample(&mut rng) { 1.0 } else { 0.0 };
        let snap = monitor.update(obs).unwrap();
        if let Some((lo, hi)) = snap.confidence_interval {
            if i == 50 {
                width_at_50 = hi - lo;
            }
            if i == 200 {
                width_at_200 = hi - lo;
            }
        }
    }

    assert!(
        width_at_200 < width_at_50,
        "CI should shrink: width@50={width_at_50:.4}, width@200={width_at_200:.4}"
    );
}
