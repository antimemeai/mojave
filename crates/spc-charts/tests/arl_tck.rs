#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use spc_charts::{cusum_arl, ewma_arl};

#[test]
fn cusum_arl0_montgomery_table() {
    // Montgomery Table 9.3: k=0.5, h=5, shift=0 → ARL₀ ≈ 465
    let arl = cusum_arl(0.5, 5.0, 0.0, 200).unwrap();
    assert!(
        (arl - 465.0).abs() / 465.0 < 0.05,
        "CUSUM ARL₀ = {arl}, expected ~465"
    );
}

#[test]
fn cusum_arl1_one_sigma_shift() {
    // Montgomery Table 9.3: k=0.5, h=5, shift=1.0 → ARL₁ ≈ 10.4
    let arl = cusum_arl(0.5, 5.0, 1.0, 200).unwrap();
    assert!(
        (arl - 10.4).abs() / 10.4 < 0.10,
        "CUSUM ARL₁(δ=1) = {arl}, expected ~10.4"
    );
}

#[test]
fn cusum_arl1_half_sigma_shift() {
    // Montgomery Table 9.3: k=0.5, h=5, shift=0.5 → ARL₁ ≈ 38
    let arl = cusum_arl(0.5, 5.0, 0.5, 200).unwrap();
    assert!(
        (arl - 38.0).abs() / 38.0 < 0.10,
        "CUSUM ARL₁(δ=0.5) = {arl}, expected ~38"
    );
}

#[test]
fn cusum_arl_decreases_with_shift() {
    let arl_0 = cusum_arl(0.5, 5.0, 0.0, 200).unwrap();
    let arl_05 = cusum_arl(0.5, 5.0, 0.5, 200).unwrap();
    let arl_10 = cusum_arl(0.5, 5.0, 1.0, 200).unwrap();
    let arl_20 = cusum_arl(0.5, 5.0, 2.0, 200).unwrap();
    assert!(arl_0 > arl_05, "ARL(0)={arl_0} > ARL(0.5)={arl_05}");
    assert!(arl_05 > arl_10, "ARL(0.5)={arl_05} > ARL(1.0)={arl_10}");
    assert!(arl_10 > arl_20, "ARL(1.0)={arl_10} > ARL(2.0)={arl_20}");
}

#[test]
fn ewma_arl0_montgomery_table() {
    // Montgomery Table 9.9: λ=0.2, L=2.962 → ARL₀ ≈ 500
    let arl = ewma_arl(0.2, 2.962, 0.0, 200).unwrap();
    assert!(
        (arl - 500.0).abs() / 500.0 < 0.10,
        "EWMA ARL₀ = {arl}, expected ~500"
    );
}

#[test]
fn ewma_arl_decreases_with_shift() {
    let arl_0 = ewma_arl(0.2, 2.962, 0.0, 200).unwrap();
    let arl_05 = ewma_arl(0.2, 2.962, 0.5, 200).unwrap();
    let arl_10 = ewma_arl(0.2, 2.962, 1.0, 200).unwrap();
    assert!(arl_0 > arl_05, "ARL(0)={arl_0} > ARL(0.5)={arl_05}");
    assert!(arl_05 > arl_10, "ARL(0.5)={arl_05} > ARL(1.0)={arl_10}");
}

#[test]
fn shewhart_arl0_analytic() {
    // ARL₀ = 1 / (2 * Φ(-3)) ≈ 370.4 for k=3.
    let p_tail = 2.0 * (1.0 - normal_cdf(3.0));
    let arl = 1.0 / p_tail;
    assert!(
        (arl - 370.4).abs() / 370.4 < 0.01,
        "Shewhart ARL₀ = {arl}, expected ~370.4"
    );
}

fn normal_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf_approx(x / std::f64::consts::SQRT_2))
}

fn erf_approx(x: f64) -> f64 {
    let a1 = 0.254_829_592;
    let a2 = -0.284_496_736;
    let a3 = 1.421_413_741;
    let a4 = -1.453_152_027;
    let a5 = 1.061_405_429;
    let p = 0.327_591_1;
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
    sign * y
}
