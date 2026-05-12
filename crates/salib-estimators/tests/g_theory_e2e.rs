#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use ndarray::Array3;
use salib_core::RngState;
use salib_estimators::{
    estimate_g_theory_pir, estimate_g_theory_pir_with_bootstrap, GTheoryDesign,
};

fn grid() -> Array3<f64> {
    let mut grid = Array3::<f64>::zeros((2, 2, 2));
    let levels = [-1.0_f64, 1.0_f64];
    for (ip, &p) in levels.iter().enumerate() {
        for (ii, &i) in levels.iter().enumerate() {
            for (ir, &r) in levels.iter().enumerate() {
                grid[[ip, ii, ir]] = 50.0
                    + 6.0 * p
                    + 4.0 * i
                    + 2.0 * r
                    + 3.0 * p * i
                    + 1.5 * p * r
                    + 1.0 * i * r
                    + 0.5 * p * i * r;
            }
        }
    }
    grid
}

#[test]
fn g_theory_recovers_expected_components_and_coefficients() {
    let r = estimate_g_theory_pir(&grid(), GTheoryDesign::Crossed).unwrap();
    let tol = 1.0e-12;
    assert!((r.sigma_p - 50.0).abs() < tol);
    assert!((r.sigma_i - 12.5).abs() < tol);
    assert!((r.sigma_r - 2.0).abs() < tol);
    assert!((r.sigma_pi - 35.0).abs() < tol);
    assert!((r.sigma_pr - 8.0).abs() < tol);
    assert!((r.sigma_ir - 3.0).abs() < tol);
    assert!((r.sigma_pir - 2.0).abs() < tol);
    assert!((r.g_coefficient - 0.694_444_444_444_444_4).abs() < tol);
    assert!((r.phi_coefficient - 0.625).abs() < tol);
}

#[test]
fn g_theory_bootstrap_is_deterministic_and_contains_point_estimate() {
    let mut rng_a = RngState::from_seed([0x44; 32]);
    let mut rng_b = RngState::from_seed([0x44; 32]);
    let a = estimate_g_theory_pir_with_bootstrap(
        &grid(),
        GTheoryDesign::Crossed,
        128,
        0.05,
        &mut rng_a,
    )
    .expect("g-theory bootstrap estimate");
    let b = estimate_g_theory_pir_with_bootstrap(
        &grid(),
        GTheoryDesign::Crossed,
        128,
        0.05,
        &mut rng_b,
    )
    .expect("g-theory bootstrap estimate");
    assert_eq!(a.variance_component_ci_low, b.variance_component_ci_low);
    assert_eq!(a.variance_component_ci_high, b.variance_component_ci_high);
    assert_eq!(a.g_coefficient_ci_low, b.g_coefficient_ci_low);
    assert_eq!(a.g_coefficient_ci_high, b.g_coefficient_ci_high);
    assert_eq!(a.phi_coefficient_ci_low, b.phi_coefficient_ci_low);
    assert_eq!(a.phi_coefficient_ci_high, b.phi_coefficient_ci_high);
    let low = a.variance_component_ci_low.as_ref().unwrap();
    let high = a.variance_component_ci_high.as_ref().unwrap();
    assert!(low[0] <= a.sigma_p && a.sigma_p <= high[0]);
    assert!(low[1] <= a.sigma_i && a.sigma_i <= high[1]);
    assert!(
        a.g_coefficient_ci_low.unwrap() <= a.g_coefficient
            && a.g_coefficient <= a.g_coefficient_ci_high.unwrap()
    );
    assert!(
        a.phi_coefficient_ci_low.unwrap() <= a.phi_coefficient
            && a.phi_coefficient <= a.phi_coefficient_ci_high.unwrap()
    );
}
