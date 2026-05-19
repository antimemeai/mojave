#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use spc_charts::{ControlLimits, EwmaChart, EwmaConfig};

fn default_ewma() -> EwmaChart {
    let limits = ControlLimits::new(0.0, 1.0).unwrap();
    EwmaChart::new(EwmaConfig::default_for(limits)).unwrap()
}

#[test]
fn ewma_first_observation() {
    let mut chart = default_ewma();
    chart.observe(1.0).unwrap();
    let expected = 0.2 * 1.0 + 0.8 * 0.0;
    assert!(
        (chart.z() - expected).abs() < 1e-10,
        "Z = {}, expected {expected}",
        chart.z()
    );
}

#[test]
fn ewma_detects_sustained_shift() {
    let mut chart = default_ewma();
    let mut detected = false;
    for _ in 0..100 {
        if chart.observe(1.5).unwrap().is_out_of_control() {
            detected = true;
            break;
        }
    }
    assert!(detected, "EWMA should detect 1.5σ shift within 100 obs");
}

#[test]
fn ewma_z_bounded_by_observations() {
    let mut chart = default_ewma();
    let values = [1.0, 2.0, 3.0, -1.0, 0.5, 2.5];
    let mut min_obs = f64::INFINITY;
    let mut max_obs = f64::NEG_INFINITY;
    for &x in &values {
        min_obs = min_obs.min(x);
        max_obs = max_obs.max(x);
        chart.observe(x).unwrap();
        assert!(
            chart.z() >= chart.z().min(0.0).min(min_obs) - 1e-10,
            "Z went below observation range"
        );
    }
    assert!(
        chart.z() <= max_obs + 1e-10,
        "Z={} > max_obs={max_obs}",
        chart.z()
    );
}

#[test]
fn ewma_asymptotic_limit_convergence() {
    let limits = ControlLimits::new(0.0, 1.0).unwrap();
    let config = EwmaConfig {
        limits,
        lambda: 0.2,
        l_sigma: 3.0,
    };
    let mut chart = EwmaChart::new(config).unwrap();

    for _ in 0..1000 {
        chart.observe(0.0).unwrap();
    }

    let asymptotic_ucl = 3.0 * 1.0 * (0.2 / 1.8_f64).sqrt();
    let time_factor = 1.0 - 0.8_f64.powi(2000);
    let ucl_1000 = 3.0 * 1.0 * (0.2 / 1.8 * time_factor).sqrt();
    let rel_diff = (ucl_1000 - asymptotic_ucl).abs() / asymptotic_ucl;
    assert!(
        rel_diff < 0.001,
        "UCL at i=1000 ({ucl_1000}) should be within 0.1% of asymptotic ({asymptotic_ucl})"
    );
}

#[test]
fn ewma_reset() {
    let mut chart = default_ewma();
    for _ in 0..10 {
        chart.observe(1.0).unwrap();
    }
    chart.reset();
    assert_eq!(chart.z(), 0.0);
    assert_eq!(chart.n_observations(), 0);
}

#[test]
fn ewma_invalid_lambda() {
    let limits = ControlLimits::new(0.0, 1.0).unwrap();
    assert!(EwmaChart::new(EwmaConfig {
        limits: limits.clone(),
        lambda: 0.0,
        l_sigma: 3.0,
    })
    .is_err());
    assert!(EwmaChart::new(EwmaConfig {
        limits,
        lambda: 1.5,
        l_sigma: 3.0,
    })
    .is_err());
}

#[test]
fn ewma_rejects_nan() {
    let mut chart = default_ewma();
    assert!(chart.observe(f64::NAN).is_err());
}

#[test]
fn ewma_rejects_infinity() {
    let mut chart = default_ewma();
    assert!(chart.observe(f64::INFINITY).is_err());
    assert!(chart.observe(f64::NEG_INFINITY).is_err());
}
