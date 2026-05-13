#![allow(clippy::unwrap_used, clippy::expect_used)]

use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_distr::{Bernoulli, Distribution, StandardNormal};
use seq_anytime_valid::monitor::sprt::SprtMonitor;
use seq_anytime_valid::types::*;

const N_REPS: usize = 100_000;
const MAX_OBS: usize = 10_000;

#[test]
fn sprt_type_i_under_h0() {
    let alpha = 0.05;
    let config = SprtConfig {
        theta_0: 0.3,
        theta_1: 0.6,
        alpha,
        beta: 0.10,
        variant: SprtVariant::Approximate,
        family: DataFamily::Bernoulli,
    };

    let mut rng = StdRng::seed_from_u64(12345);
    let dist = Bernoulli::new(0.3).unwrap();
    let mut rejections = 0usize;

    for _ in 0..N_REPS {
        let mut monitor = SprtMonitor::new(config.clone()).unwrap();
        for _ in 0..MAX_OBS {
            let obs = if dist.sample(&mut rng) { 1.0 } else { 0.0 };
            match monitor.update(obs).unwrap() {
                Decision::Reject => {
                    rejections += 1;
                    break;
                }
                Decision::Accept => break,
                Decision::Continue => {}
            }
        }
    }

    let rejection_rate = rejections as f64 / N_REPS as f64;
    let mc_error = (alpha * (1.0 - alpha) / N_REPS as f64).sqrt();
    assert!(
        rejection_rate <= alpha + 3.0 * mc_error,
        "Type-I rate {rejection_rate:.4} exceeds alpha={alpha} + 3*MC_err={:.4}",
        3.0 * mc_error
    );
}

#[test]
fn always_valid_p_type_i_under_h0() {
    let alpha = 0.05;
    let mut rng = StdRng::seed_from_u64(54321);
    let normal = StandardNormal;
    let mut false_rejections = 0usize;
    let n_max = 200;

    for _ in 0..N_REPS {
        let mut obs = Vec::with_capacity(n_max);
        let mut rejected = false;
        for _ in 0..n_max {
            let x: f64 = normal.sample(&mut rng);
            obs.push(x);
            let p = seq_anytime_valid::evidence::msprt::always_valid_p(&obs, 0.0, 1.0).unwrap();
            if p <= alpha {
                rejected = true;
                break;
            }
        }
        if rejected {
            false_rejections += 1;
        }
    }

    let false_rejection_rate = false_rejections as f64 / N_REPS as f64;
    let mc_error = (alpha * (1.0 - alpha) / N_REPS as f64).sqrt();
    assert!(
        false_rejection_rate <= alpha + 3.0 * mc_error,
        "always-valid p Type-I rate {false_rejection_rate:.4} exceeds tolerance"
    );
}

#[test]
fn cs_coverage_under_true_mean() {
    let n_reps = 10_000;
    let n_max = 200;
    let alpha = 0.05;
    let true_mean = 0.0;
    let mut rng = StdRng::seed_from_u64(99999);
    let normal = StandardNormal;
    let mut covered_count = 0usize;

    for _ in 0..n_reps {
        let mut obs = Vec::with_capacity(n_max);
        let mut all_covered = true;
        for _ in 0..n_max {
            let x: f64 = normal.sample(&mut rng);
            obs.push(x);
            if obs.len() >= 2 {
                // Use the known-sigma variant (sigma=1.0 for N(0,1)).
                // The estimated-sigma variant does not guarantee anytime-valid coverage.
                let (lo, hi) = seq_anytime_valid::evidence::confseq::normal_mixture_cs_known_sigma(
                    &obs, 1.0, alpha,
                )
                .unwrap();
                if true_mean < lo || true_mean > hi {
                    all_covered = false;
                    break;
                }
            }
        }
        if all_covered {
            covered_count += 1;
        }
    }

    let coverage = covered_count as f64 / n_reps as f64;
    // Howard et al. CS is conservative (super-martingale): empirical coverage can
    // exceed 1 − alpha.  Accept anything in [0.93, 0.99]: the lower bound guards
    // against anti-conservative behaviour; the upper bound catches degenerate
    // intervals that always contain the mean.
    assert!(
        (0.93..=0.99).contains(&coverage),
        "CS coverage {coverage:.3} not in [0.93, 0.99]"
    );
}
