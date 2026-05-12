//! Reviewer-affordance contract close for the ANOVA estimators.
//!
//! Contract artifacts in this diff:
//!
//! 1. Canonical analytic recovery on deterministic balanced factorial fixtures.
//! 2. Identity: component fractions sum to 1.
//! 3. Frozen deterministic reference fixture generated via a `SciPy` differential script.
//! 4. Stability/deformation checks on affine score transforms and factor relabelings.
//! 5. cargo-mutants deferred to nightly CI.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::fs;
use std::path::PathBuf;

use ndarray::{arr2, Array2, Array3};
use salib_core::RngState;
use salib_estimators::{
    estimate_anova_three_way, estimate_anova_three_way_with_bootstrap, estimate_anova_two_way,
    estimate_anova_two_way_with_bootstrap,
};

fn reference_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("salib-validation")
        .join("reference")
        .join("scipy_outputs")
}

fn two_way_grid() -> Array2<f64> {
    arr2(&[[9.0, 5.0], [7.0, 19.0]])
}

fn three_way_grid() -> Array3<f64> {
    let mut grid = Array3::<f64>::zeros((2, 2, 2));
    let levels = [-1.0_f64, 1.0_f64];
    for (i, &a) in levels.iter().enumerate() {
        for (j, &b) in levels.iter().enumerate() {
            for (k, &c) in levels.iter().enumerate() {
                grid[[i, j, k]] = 50.0
                    + 5.0 * a
                    + 3.0 * b
                    + 2.0 * c
                    + 4.0 * a * b
                    + 1.5 * a * c
                    + 1.0 * b * c
                    + 2.5 * a * b * c;
            }
        }
    }
    grid
}

fn affine_two_way_grid(scale: f64, shift: f64) -> Array2<f64> {
    two_way_grid().mapv(|value| (scale * value) + shift)
}

fn permuted_three_way_grid() -> Array3<f64> {
    let grid = three_way_grid();
    let mut permuted = Array3::<f64>::zeros((2, 2, 2));
    for i in 0..2 {
        for j in 0..2 {
            for k in 0..2 {
                permuted[[i, j, k]] = grid[[1 - i, j, 1 - k]];
            }
        }
    }
    permuted
}

fn parse_reference_csv(path: &str) -> std::collections::BTreeMap<String, f64> {
    let raw = fs::read_to_string(reference_dir().join(path)).expect("read reference csv");
    raw.lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut parts = line.split(',');
            let key = parts.next().expect("component").to_string();
            let value = parts.next().expect("value").parse::<f64>().expect("float");
            (key, value)
        })
        .collect()
}

#[test]
fn anova_two_way_recovers_analytic_component_fractions() {
    let est = estimate_anova_two_way(&two_way_grid()).expect("two-way estimate");
    let tol = 1.0e-12;
    assert!((est.v_row - 0.310_344_827_586_206_9).abs() < tol);
    assert!((est.v_column - 0.137_931_034_482_758_62).abs() < tol);
    assert!((est.v_interaction - 0.551_724_137_931_034_5).abs() < tol);
    assert!((est.v_residual - 0.0).abs() < tol);
}

#[test]
fn anova_three_way_recovers_analytic_component_fractions() {
    let est = estimate_anova_three_way(&three_way_grid()).expect("three-way estimate");
    let tol = 1.0e-12;
    assert!((est.v_data - 0.393_700_787_401_574_8).abs() < tol);
    assert!((est.v_brittleness - 0.141_732_283_464_566_93).abs() < tol);
    assert!((est.v_inference - 0.062_992_125_984_251_97).abs() < tol);
    assert!((est.v_data_brittleness - 0.251_968_503_937_007_87).abs() < tol);
    assert!((est.v_data_inference - 0.035_433_070_866_141_73).abs() < tol);
    assert!((est.v_brittleness_inference - 0.015_748_031_496_062_992).abs() < tol);
    assert!((est.v_data_brittleness_inference - 0.098_425_196_850_393_7).abs() < tol);
    assert!((est.v_residual - 0.0).abs() < tol);
}

#[test]
fn anova_component_fractions_sum_to_one() {
    let two = estimate_anova_two_way(&two_way_grid()).expect("two-way estimate");
    let three = estimate_anova_three_way(&three_way_grid()).expect("three-way estimate");

    let two_sum = two.v_row + two.v_column + two.v_interaction + two.v_residual;
    let three_sum = three.v_data
        + three.v_brittleness
        + three.v_inference
        + three.v_data_brittleness
        + three.v_data_inference
        + three.v_brittleness_inference
        + three.v_data_brittleness_inference
        + three.v_residual;

    assert!((two_sum - 1.0).abs() < 1.0e-12);
    assert!((three_sum - 1.0).abs() < 1.0e-12);
}

#[test]
fn anova_matches_frozen_python_reference_fixture() {
    let two = estimate_anova_two_way(&two_way_grid()).expect("two-way estimate");
    let three = estimate_anova_three_way(&three_way_grid()).expect("three-way estimate");
    let two_ref = parse_reference_csv("anova_two_way_reference.csv");
    let three_ref = parse_reference_csv("anova_three_way_reference.csv");
    let tol = 1.0e-12;

    assert!((two.v_row - two_ref["row"]).abs() < tol);
    assert!((two.v_column - two_ref["column"]).abs() < tol);
    assert!((two.v_interaction - two_ref["interaction"]).abs() < tol);

    assert!((three.v_data - three_ref["data"]).abs() < tol);
    assert!((three.v_brittleness - three_ref["brittleness"]).abs() < tol);
    assert!((three.v_inference - three_ref["inference"]).abs() < tol);
    assert!((three.v_data_brittleness - three_ref["data_brittleness"]).abs() < tol);
    assert!((three.v_data_inference - three_ref["data_inference"]).abs() < tol);
    assert!((three.v_brittleness_inference - three_ref["brittleness_inference"]).abs() < tol);
    assert!(
        (three.v_data_brittleness_inference - three_ref["data_brittleness_inference"]).abs() < tol
    );
}

#[test]
fn anova_two_way_variance_fractions_are_affine_invariant() {
    let baseline = estimate_anova_two_way(&two_way_grid()).expect("two-way estimate");
    let transformed =
        estimate_anova_two_way(&affine_two_way_grid(7.5, 11.0)).expect("affine two-way estimate");
    let tol = 1.0e-12;

    assert!((baseline.v_row - transformed.v_row).abs() < tol);
    assert!((baseline.v_column - transformed.v_column).abs() < tol);
    assert!((baseline.v_interaction - transformed.v_interaction).abs() < tol);
    assert!((baseline.v_residual - transformed.v_residual).abs() < tol);
}

#[test]
fn anova_three_way_variance_fractions_are_invariant_to_factor_level_relabeling() {
    let baseline = estimate_anova_three_way(&three_way_grid()).expect("three-way estimate");
    let permuted = estimate_anova_three_way(&permuted_three_way_grid()).expect("permuted estimate");
    let tol = 1.0e-12;

    assert!((baseline.v_data - permuted.v_data).abs() < tol);
    assert!((baseline.v_brittleness - permuted.v_brittleness).abs() < tol);
    assert!((baseline.v_inference - permuted.v_inference).abs() < tol);
    assert!((baseline.v_data_brittleness - permuted.v_data_brittleness).abs() < tol);
    assert!((baseline.v_data_inference - permuted.v_data_inference).abs() < tol);
    assert!((baseline.v_brittleness_inference - permuted.v_brittleness_inference).abs() < tol);
    assert!(
        (baseline.v_data_brittleness_inference - permuted.v_data_brittleness_inference).abs() < tol
    );
}

#[test]
fn anova_two_way_bootstrap_is_deterministic_and_contains_point_estimate() {
    let mut rng_a = RngState::from_seed([0x11; 32]);
    let mut rng_b = RngState::from_seed([0x11; 32]);
    let a = estimate_anova_two_way_with_bootstrap(&two_way_grid(), 128, 0.05, &mut rng_a)
        .expect("two-way bootstrap estimate");
    let b = estimate_anova_two_way_with_bootstrap(&two_way_grid(), 128, 0.05, &mut rng_b)
        .expect("two-way bootstrap estimate");

    assert_eq!(a.variance_fraction_ci_low, b.variance_fraction_ci_low);
    assert_eq!(a.variance_fraction_ci_high, b.variance_fraction_ci_high);
    assert_eq!(a.bootstrap_iterations, Some(128));
    assert_eq!(a.bootstrap_alpha, Some(0.05));
    let lows = a.variance_fraction_ci_low.as_ref().unwrap();
    let highs = a.variance_fraction_ci_high.as_ref().unwrap();
    assert!(lows[0] <= a.v_row && a.v_row <= highs[0]);
    assert!(lows[1] <= a.v_column && a.v_column <= highs[1]);
    assert!(lows[2] <= a.v_interaction && a.v_interaction <= highs[2]);
}

#[test]
fn anova_three_way_bootstrap_is_deterministic_and_contains_point_estimate() {
    let mut rng_a = RngState::from_seed([0x22; 32]);
    let mut rng_b = RngState::from_seed([0x22; 32]);
    let a = estimate_anova_three_way_with_bootstrap(&three_way_grid(), 128, 0.05, &mut rng_a)
        .expect("three-way bootstrap estimate");
    let b = estimate_anova_three_way_with_bootstrap(&three_way_grid(), 128, 0.05, &mut rng_b)
        .expect("three-way bootstrap estimate");

    assert_eq!(a.variance_fraction_ci_low, b.variance_fraction_ci_low);
    assert_eq!(a.variance_fraction_ci_high, b.variance_fraction_ci_high);
    assert_eq!(a.bootstrap_iterations, Some(128));
    assert_eq!(a.bootstrap_alpha, Some(0.05));
    let lows = a.variance_fraction_ci_low.as_ref().unwrap();
    let highs = a.variance_fraction_ci_high.as_ref().unwrap();
    assert!(lows[0] <= a.v_data && a.v_data <= highs[0]);
    assert!(lows[1] <= a.v_brittleness && a.v_brittleness <= highs[1]);
    assert!(lows[2] <= a.v_inference && a.v_inference <= highs[2]);
}

#[test]
fn anova_inferential_statistics_match_ratified_denominators() {
    let two = estimate_anova_two_way(&two_way_grid()).expect("two-way estimate");
    let three = estimate_anova_three_way(&three_way_grid()).expect("three-way estimate");
    let tol = 1.0e-12;

    assert!((two.f_row.unwrap() - 0.5625).abs() < tol);
    assert!((two.f_column.unwrap() - 0.25).abs() < tol);
    assert!((two.p_row.unwrap() - 0.590_334_470_601_733).abs() < tol);
    assert!((two.p_column.unwrap() - 0.704_832_764_699_133_5).abs() < tol);
    assert!(two.f_interaction.is_none());
    assert!(two.p_interaction.is_none());

    assert!((three.f_data.unwrap() - 4.0).abs() < tol);
    assert!((three.f_brittleness.unwrap() - 1.44).abs() < tol);
    assert!((three.f_inference.unwrap() - 0.64).abs() < tol);
    assert!((three.f_data_brittleness.unwrap() - 2.56).abs() < tol);
    assert!((three.f_data_inference.unwrap() - 0.36).abs() < tol);
    assert!((three.f_brittleness_inference.unwrap() - 0.16).abs() < tol);
    assert!((three.p_data.unwrap() - 0.295_167_235_300_866_5).abs() < tol);
    assert!((three.p_brittleness.unwrap() - 0.442_284_123_247_391_1).abs() < tol);
    assert!((three.p_inference.unwrap() - 0.570_446_574_954_554_6).abs() < tol);
    assert!((three.p_data_brittleness.unwrap() - 0.355_615_368_978_705_4).abs() < tol);
    assert!((three.p_data_inference.unwrap() - 0.655_958_260_754_738_5).abs() < tol);
    assert!((three.p_brittleness_inference.unwrap() - 0.757_762_116_818_313_2).abs() < tol);
    assert!(three.f_data_brittleness_inference.is_none());
    assert!(three.p_data_brittleness_inference.is_none());
}
