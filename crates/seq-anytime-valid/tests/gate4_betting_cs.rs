#![allow(clippy::unwrap_used, clippy::expect_used)]

use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_distr::{Bernoulli, Distribution};
use seq_anytime_valid::monitor::anytime::AnytimeMonitor;
use seq_anytime_valid::monitor::betting::BettingMonitor;
use seq_anytime_valid::types::{DataFamily, MsprtConfig};

// ---------------------------------------------------------------------------
// Gate 3: Property tests
// ---------------------------------------------------------------------------

/// Betting CS contains the true mean for Bernoulli(0.3) data.
#[test]
fn betting_cs_contains_true_mean() {
    let true_p = 0.3;
    let alpha = 0.05;
    let grid_size = 500;
    let n_obs = 200;

    let mut monitor = BettingMonitor::new(alpha, grid_size).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let dist = Bernoulli::new(true_p).unwrap();

    let mut last_ci = None;
    for _ in 0..n_obs {
        let obs = if dist.sample(&mut rng) { 1.0 } else { 0.0 };
        let snap = monitor.update(obs).unwrap();
        last_ci = snap.confidence_interval;
    }

    let (lo, hi) = last_ci.unwrap();
    assert!(
        lo <= true_p && true_p <= hi,
        "CI [{lo:.4}, {hi:.4}] does not contain true_p={true_p}"
    );
}

/// Betting CS narrows with more observations.
#[test]
fn betting_cs_narrows_with_data() {
    let alpha = 0.05;
    let grid_size = 500;

    let mut monitor = BettingMonitor::new(alpha, grid_size).unwrap();
    let mut rng = StdRng::seed_from_u64(123);
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
        "CI should narrow: width@50={width_at_50:.4}, width@200={width_at_200:.4}"
    );
}

/// Betting CS is tighter than the sigma=0.5 conservative AnytimeMonitor bound.
#[test]
fn betting_cs_tighter_than_conservative() {
    let alpha = 0.05;
    let n_obs = 200;

    let mut betting = BettingMonitor::new(alpha, 500).unwrap();

    let config = MsprtConfig {
        theta_0: 0.0,
        mixing_variance: 1.0,
        family: DataFamily::Bernoulli,
        max_samples: None,
    };
    let mut anytime = AnytimeMonitor::new(config, alpha).unwrap();

    let mut rng = StdRng::seed_from_u64(999);
    let dist = Bernoulli::new(0.5).unwrap();

    let mut betting_width = f64::INFINITY;
    let mut anytime_width = f64::INFINITY;

    for _ in 0..n_obs {
        let obs = if dist.sample(&mut rng) { 1.0 } else { 0.0 };
        let snap_b = betting.update(obs).unwrap();
        let snap_a = anytime.update(obs).unwrap();

        if let Some((lo, hi)) = snap_b.confidence_interval {
            betting_width = hi - lo;
        }
        if let Some((lo, hi)) = snap_a.confidence_interval {
            anytime_width = hi - lo;
        }
    }

    assert!(
        betting_width < anytime_width,
        "Betting CI width ({betting_width:.4}) should be less than AnytimeMonitor CI width ({anytime_width:.4})"
    );
}

// ---------------------------------------------------------------------------
// Gate 4: Monte Carlo calibration
// ---------------------------------------------------------------------------

/// Betting CS achieves >= 93% coverage at each p in {0.1, 0.3, 0.5, 0.7, 0.9}
/// across 10,000 replications of N=200 Bernoulli observations.
#[test]
fn betting_cs_coverage_gate4() {
    let alpha = 0.05;
    let n_reps = 10_000;
    let n_obs = 200;
    let grid_size = 500;
    let test_ps = [0.1, 0.3, 0.5, 0.7, 0.9];

    for (p_idx, &true_p) in test_ps.iter().enumerate() {
        let mut covered = 0u32;
        let dist = Bernoulli::new(true_p).unwrap();

        for rep in 0..n_reps {
            let seed = (p_idx as u64) * 1_000_000 + rep as u64;
            let mut rng = StdRng::seed_from_u64(seed);

            let mut monitor = BettingMonitor::new(alpha, grid_size).unwrap();

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
