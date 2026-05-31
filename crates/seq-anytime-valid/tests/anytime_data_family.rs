#![allow(clippy::unwrap_used, clippy::expect_used)]

//! TCK tests: AnytimeMonitor must dispatch on DataFamily for sigma selection.
//!
//! - Bernoulli: sigma = 0.5 (conservative upper bound, anytime-valid)
//! - Normal(known_variance=Some(v)): sigma = v.sqrt()
//! - Normal(known_variance=None): Welford online estimate (existing behavior)

use seq_anytime_valid::monitor::anytime::AnytimeMonitor;
use seq_anytime_valid::types::{DataFamily, MsprtConfig};

/// Helper: run monitor on data and return (ci_lo, ci_hi) at the final observation.
fn run_monitor(family: DataFamily, observations: &[f64], alpha: f64) -> (f64, f64) {
    let config = MsprtConfig {
        theta_0: 0.0,
        mixing_variance: 1.0,
        family,
        max_samples: None,
    };
    let mut monitor = AnytimeMonitor::new(config, alpha).unwrap();
    let mut last_ci = (0.0, 0.0);
    for &obs in observations {
        let snap = monitor.update(obs).unwrap();
        if let Some(ci) = snap.confidence_interval {
            last_ci = ci;
        }
    }
    last_ci
}

#[test]
fn bernoulli_uses_fixed_sigma() {
    // Feed identical Bernoulli(0.9) data to a Bernoulli monitor and a Normal(unknown) monitor.
    // The Bernoulli monitor should use sigma=0.5, producing wider intervals than
    // a Normal monitor that estimates sigma from the low-variance 0/1 data.
    let alpha = 0.05;
    let data: Vec<f64> = std::iter::repeat(1.0)
        .take(90)
        .chain(std::iter::repeat(0.0).take(10))
        .collect();

    let ci_bernoulli = run_monitor(DataFamily::Bernoulli, &data, alpha);
    let ci_normal_est = run_monitor(
        DataFamily::Normal {
            known_variance: None,
        },
        &data,
        alpha,
    );

    let width_bernoulli = ci_bernoulli.1 - ci_bernoulli.0;
    let width_normal = ci_normal_est.1 - ci_normal_est.0;

    // Bernoulli sigma=0.5 is always >= the true Bernoulli std dev (which is sqrt(p(1-p)) <= 0.5).
    // For p=0.9, true sigma = 0.3, so Bernoulli monitor with sigma=0.5 should give wider CI
    // than a Normal monitor that estimates sigma ~ 0.3.
    assert!(
        width_bernoulli > width_normal,
        "Bernoulli CI width ({width_bernoulli:.4}) should exceed estimated-sigma CI width ({width_normal:.4})"
    );
}

#[test]
fn bernoulli_ci_width_is_deterministic() {
    // Two runs with different Bernoulli data at same p should produce the same CI width,
    // because sigma is fixed at 0.5 regardless of the actual sample variance.
    let alpha = 0.05;

    // Dataset 1: p ~ 0.5 (high variance)
    let data1: Vec<f64> = (0..100).map(|i| if i % 2 == 0 { 1.0 } else { 0.0 }).collect();

    // Dataset 2: p ~ 0.9 (low variance)
    let data2: Vec<f64> = std::iter::repeat(1.0)
        .take(90)
        .chain(std::iter::repeat(0.0).take(10))
        .collect();

    let ci1 = run_monitor(DataFamily::Bernoulli, &data1, alpha);
    let ci2 = run_monitor(DataFamily::Bernoulli, &data2, alpha);

    let width1 = ci1.1 - ci1.0;
    let width2 = ci2.1 - ci2.0;

    // Both should use sigma=0.5, so widths must be identical
    assert!(
        (width1 - width2).abs() < 1e-10,
        "Bernoulli CI widths should be identical: {width1:.6} vs {width2:.6}"
    );
}

#[test]
fn normal_known_variance_uses_specified_sigma() {
    let alpha = 0.05;
    let data: Vec<f64> = vec![0.0; 100]; // constant data — Welford would estimate sigma~0

    let ci_known = run_monitor(
        DataFamily::Normal {
            known_variance: Some(4.0),
        },
        &data,
        alpha,
    );
    let width_known = ci_known.1 - ci_known.0;

    // With sigma=2.0, the CI should have substantial width even on constant data
    assert!(
        width_known > 0.1,
        "Known-variance CI should have nonzero width even on constant data, got {width_known:.6}"
    );
}

#[test]
fn normal_unknown_variance_uses_welford() {
    let alpha = 0.05;

    // For high-variance data, estimated sigma should be close to the true sigma
    let data: Vec<f64> = (0..200).map(|i| if i % 2 == 0 { 5.0 } else { -5.0 }).collect();

    let ci_est = run_monitor(
        DataFamily::Normal {
            known_variance: None,
        },
        &data,
        alpha,
    );
    let ci_known = run_monitor(
        DataFamily::Normal {
            known_variance: Some(25.0),
        },
        &data,
        alpha,
    );

    let width_est = ci_est.1 - ci_est.0;
    let width_known = ci_known.1 - ci_known.0;

    // Widths should be close (both use sigma~5.0)
    let ratio = width_est / width_known;
    assert!(
        (0.9..=1.1).contains(&ratio),
        "Welford-estimated width should be close to known-sigma width: ratio={ratio:.4}"
    );
}
