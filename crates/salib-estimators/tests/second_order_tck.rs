#![allow(clippy::float_cmp, clippy::approx_constant, clippy::expect_used)]

use salib_core::RngState;
use salib_estimators::estimate_saltelli2010;
use salib_samplers::{build_saltelli_matrix, SobolSampler};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn run_ishigami_s2_at_n(n: usize) -> salib_estimators::SobolIndices {
    let sampler = SobolSampler::standard(6).with_skip_first(false);
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    // second_order = true
    let matrix = build_saltelli_matrix(&sampler, n, true, &mut rng).expect("matrix");

    use std::f64::consts::PI;
    let model = |x: &[f64]| -> f64 {
        let mapped: [f64; 3] = [
            -PI + x[0] * 2.0 * PI,
            -PI + x[1] * 2.0 * PI,
            -PI + x[2] * 2.0 * PI,
        ];
        ishigami::ishigami(&mapped)
    };
    estimate_saltelli2010(&matrix, model)
}

#[test]
fn s2_02_recovers_x1_x3_interaction() {
    let est = run_ishigami_s2_at_n(8192);
    let analytic = ishigami::analytic_indices(7.0, 0.1);
    let s2 = est.second_order.expect("second_order should be Some");
    let s2_analytic = analytic.second_order.expect("analytic S2");
    // S2_{0,2} = s2[0][1] (second element of first row)
    let got = s2[0][1];
    let want = s2_analytic[0][1];
    assert!(
        (got - want).abs() < 0.05,
        "S2_02: got {got:.4}, want {want:.4}"
    );
}

#[test]
fn s2_01_and_s2_12_near_zero() {
    let est = run_ishigami_s2_at_n(8192);
    let s2 = est.second_order.expect("second_order should be Some");
    // S2_{0,1} = s2[0][0]
    assert!(
        s2[0][0].abs() < 0.05,
        "S2_01 should be near zero, got {}",
        s2[0][0]
    );
    // S2_{1,2} = s2[1][0]
    assert!(
        s2[1][0].abs() < 0.05,
        "S2_12 should be near zero, got {}",
        s2[1][0]
    );
}

#[test]
fn sum_s1_plus_s2_at_most_one() {
    let est = run_ishigami_s2_at_n(8192);
    let s2 = est.second_order.expect("second_order should be Some");
    let s1_sum: f64 = est.first_order.iter().sum();
    let s2_sum: f64 = s2.iter().flat_map(|row| row.iter()).sum();
    assert!(
        s1_sum + s2_sum <= 1.05,
        "S1+S2 sum = {} (should be <= 1.05)",
        s1_sum + s2_sum
    );
}
