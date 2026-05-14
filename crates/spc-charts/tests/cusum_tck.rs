#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use spc_charts::{ControlLimits, CusumChart, CusumConfig};

fn default_cusum() -> CusumChart {
    let limits = ControlLimits::new(0.0, 1.0).unwrap();
    CusumChart::new(CusumConfig::default_for(limits)).unwrap()
}

#[test]
fn in_control_stays_low() {
    let mut chart = default_cusum();
    for &x in &[0.1, -0.2, 0.3, -0.1, 0.0] {
        chart.observe(x);
    }
    assert!(chart.c_plus() < 1.0, "C⁺ = {}", chart.c_plus());
    assert!(chart.c_minus() < 1.0, "C⁻ = {}", chart.c_minus());
}

#[test]
fn sustained_shift_triggers() {
    let mut chart = default_cusum();
    let mut signaled = false;
    for t in 0..20 {
        if chart.observe(1.0).is_out_of_control() {
            signaled = true;
            assert!(t < 20, "should signal before t=20");
            break;
        }
    }
    assert!(signaled, "1σ shift should trigger CUSUM within 20 obs");
}

#[test]
fn c_plus_c_minus_always_non_negative() {
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;
    use rand_distr::{Distribution, StandardNormal};

    let mut rng = ChaCha20Rng::seed_from_u64(99);
    let mut chart = default_cusum();
    for _ in 0..1000 {
        let x: f64 = StandardNormal.sample(&mut rng);
        chart.observe(x);
        assert!(chart.c_plus() >= 0.0, "C⁺ went negative");
        assert!(chart.c_minus() >= 0.0, "C⁻ went negative");
    }
}

#[test]
fn reset_restores_initial() {
    let mut chart = default_cusum();
    for &x in &[1.0, 1.5, 2.0, 1.0, 1.5] {
        chart.observe(x);
    }
    chart.reset();
    assert_eq!(chart.c_plus(), 0.0);
    assert_eq!(chart.c_minus(), 0.0);
    assert_eq!(chart.n_observations(), 0);
}

#[test]
fn known_cusum_trace() {
    let mut chart = default_cusum();
    // z = x - 0 / 1 = x. k=0.5.
    // x=0.8: C+ = max(0, 0+0.8-0.5)=0.3, C- = max(0, 0-0.8-0.5)=0
    let s = chart.observe(0.8);
    assert!(s.is_in_control());
    assert!((chart.c_plus() - 0.3).abs() < 1e-10);
    assert_eq!(chart.c_minus(), 0.0);

    // x=0.6: C+ = max(0, 0.3+0.6-0.5)=0.4
    chart.observe(0.6);
    assert!((chart.c_plus() - 0.4).abs() < 1e-10);

    // x=-1.0: C+ = max(0, 0.4-1.0-0.5)=0, C- = max(0, 0+1.0-0.5)=0.5
    chart.observe(-1.0);
    assert_eq!(chart.c_plus(), 0.0);
    assert!((chart.c_minus() - 0.5).abs() < 1e-10);
}
