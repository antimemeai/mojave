#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use spc_charts::{ControlLimits, ShewhartChart, ShewhartConfig, ShewhartRule};

fn chart_3sigma(mu_0: f64, sigma: f64) -> ShewhartChart {
    let limits = ControlLimits::new(mu_0, sigma).unwrap();
    ShewhartChart::new(ShewhartConfig::default_for(limits)).unwrap()
}

fn chart_with_rules(mu_0: f64, sigma: f64, rules: Vec<ShewhartRule>) -> ShewhartChart {
    let limits = ControlLimits::new(mu_0, sigma).unwrap();
    ShewhartChart::new(ShewhartConfig {
        limits,
        k_sigma: 3.0,
        rules,
    })
    .unwrap()
}

#[test]
fn in_control_no_signal() {
    let mut chart = chart_3sigma(50.0, 2.0);
    for &x in &[49.0, 50.0, 51.0, 48.0, 52.0, 50.0] {
        assert!(
            !chart.observe(x).is_out_of_control(),
            "x={x} should be in control"
        );
    }
}

#[test]
fn three_sigma_violation() {
    let mut chart = chart_3sigma(50.0, 2.0);
    let signal = chart.observe(57.0);
    assert!(signal.is_out_of_control(), "57 is >3sigma above 50+-6");
}

#[test]
fn we2_two_of_three_beyond_2sigma() {
    let mut chart = chart_with_rules(50.0, 2.0, vec![ShewhartRule::WE1, ShewhartRule::WE2]);
    assert!(!chart.observe(55.0).is_out_of_control()); // z=2.5, >2sigma but only 1 of 1
    assert!(!chart.observe(49.0).is_out_of_control()); // z=-0.5, in zone C
    let signal = chart.observe(55.0); // z=2.5, now 2 of 3 >2sigma same side
    assert!(signal.is_out_of_control(), "WE-2: 2 of 3 beyond 2sigma");
}

#[test]
fn we4_eight_consecutive_one_side() {
    let mut chart = chart_with_rules(50.0, 2.0, vec![ShewhartRule::WE1, ShewhartRule::WE4]);
    for i in 0..7 {
        let signal = chart.observe(51.0); // z=0.5, above center
        assert!(
            !signal.is_out_of_control(),
            "observation {i} should not signal"
        );
    }
    let signal = chart.observe(51.0); // 8th consecutive
    assert!(signal.is_out_of_control(), "WE-4: 8 consecutive same side");
}

#[test]
fn reset_clears_state() {
    let mut chart = chart_3sigma(50.0, 2.0);
    chart.observe(57.0);
    chart.reset();
    assert_eq!(chart.n_observations(), 0);
    assert!(chart.observe(50.0).is_in_control());
}

#[test]
fn mc_shewhart_arl0() {
    use rand::distr::Distribution;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;
    use rand_distr::StandardNormal;

    let mut rng = ChaCha20Rng::seed_from_u64(42);
    let n_sims = 10_000;
    let max_len = 5_000;
    let mut total_rl: u64 = 0;

    for _ in 0..n_sims {
        let mut chart = chart_3sigma(0.0, 1.0);
        let mut rl = max_len;
        for t in 0..max_len {
            let x: f64 = StandardNormal.sample(&mut rng);
            if chart.observe(x).is_out_of_control() {
                rl = t + 1;
                break;
            }
        }
        total_rl += rl as u64;
    }

    let empirical_arl = total_rl as f64 / n_sims as f64;
    let expected = 370.4;
    let tolerance = 0.10;
    assert!(
        (empirical_arl - expected).abs() / expected < tolerance,
        "ARL0 = {empirical_arl}, expected ~{expected} +- 10%"
    );
}
