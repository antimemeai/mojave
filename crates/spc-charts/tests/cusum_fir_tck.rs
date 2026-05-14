#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use spc_charts::{ControlLimits, CusumChart, CusumConfig, FirCusumChart, FirCusumConfig};

fn default_fir() -> FirCusumChart {
    let limits = ControlLimits::new(0.0, 1.0).unwrap();
    FirCusumChart::new(FirCusumConfig::default_for(limits)).unwrap()
}

fn default_cusum() -> CusumChart {
    let limits = ControlLimits::new(0.0, 1.0).unwrap();
    CusumChart::new(CusumConfig::default_for(limits)).unwrap()
}

#[test]
fn fir_starts_at_head_start() {
    let chart = default_fir();
    assert!(
        (chart.c_plus() - 2.5).abs() < 1e-10,
        "C⁺ should start at h/2=2.5"
    );
    assert!((chart.c_minus() - 2.5).abs() < 1e-10);
}

#[test]
fn fir_detects_initial_shift_faster_than_standard() {
    let mut fir = default_fir();
    let mut std = default_cusum();

    let shift_values: Vec<f64> = (0..50).map(|_| 1.0).collect();

    let mut fir_rl = 50;
    let mut std_rl = 50;
    for (t, &x) in shift_values.iter().enumerate() {
        if fir_rl == 50 && fir.observe(x).is_out_of_control() {
            fir_rl = t + 1;
        }
        if std_rl == 50 && std.observe(x).is_out_of_control() {
            std_rl = t + 1;
        }
    }
    assert!(
        fir_rl < std_rl,
        "FIR (rl={fir_rl}) should detect faster than standard (rl={std_rl})"
    );
}

#[test]
fn fir_reset_restores_head_start() {
    let mut chart = default_fir();
    for _ in 0..10 {
        chart.observe(0.0);
    }
    chart.reset();
    assert!((chart.c_plus() - 2.5).abs() < 1e-10);
    assert!((chart.c_minus() - 2.5).abs() < 1e-10);
    assert_eq!(chart.n_observations(), 0);
}
