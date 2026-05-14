//! Tests for the g-theory feature gate. Only compiled with
//! `cargo test -p spc-charts --features g-theory`.

#![cfg(feature = "g-theory")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use spc_charts::control_limits_from_g_theory;

fn mock_g_theory_result() -> salib_estimators::GTheoryResult {
    salib_estimators::GTheoryResult::from_components(
        10.0, // sigma_p
        2.0,  // sigma_i
        1.5,  // sigma_r
        3.0,  // sigma_pi
        2.0,  // sigma_pr
        0.5,  // sigma_ir
        1.0,  // sigma_pir
        0.85, // g_coefficient
        0.80, // phi_coefficient
    )
}

#[test]
fn control_limits_from_g_theory_computes_sigma() {
    let result = mock_g_theory_result();
    let limits = control_limits_from_g_theory(&result, 50.0, 5, 3).unwrap();

    // σ² = σ²_pi/n_i + σ²_pr/n_r + σ²_pir/(n_i·n_r)
    //    = 3.0/5 + 2.0/3 + 1.0/15
    //    = 0.6 + 0.6667 + 0.0667
    //    = 1.3333
    // σ = √1.3333 ≈ 1.1547
    let expected_sigma = (3.0 / 5.0 + 2.0 / 3.0 + 1.0 / 15.0_f64).sqrt();
    assert!(
        (limits.sigma - expected_sigma).abs() < 1e-10,
        "sigma = {}, expected {expected_sigma}",
        limits.sigma
    );
    assert_eq!(limits.mu_0, 50.0);
}

#[test]
fn control_limits_feeds_into_chart() {
    let result = mock_g_theory_result();
    let limits = control_limits_from_g_theory(&result, 50.0, 5, 3).unwrap();

    // Should be usable with any chart.
    let mut chart =
        spc_charts::ShewhartChart::new(spc_charts::ShewhartConfig::default_for(limits)).unwrap();
    assert!(chart.observe(50.0).is_in_control());
}
