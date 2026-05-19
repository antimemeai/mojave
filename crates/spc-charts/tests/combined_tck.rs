#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use spc_charts::{CombinedChart, CombinedConfig, ControlLimits, CusumConfig};

fn default_combined() -> CombinedChart {
    let limits = ControlLimits::new(0.0, 1.0).unwrap();
    CombinedChart::new(CombinedConfig::default_for(limits)).unwrap()
}

#[test]
fn large_spike_triggers_shewhart_arm() {
    let mut chart = default_combined();
    // 4.0 > 3.5σ Shewhart limit.
    let signal = chart.observe(4.0).unwrap();
    assert!(signal.is_out_of_control());
}

#[test]
fn sustained_shift_triggers_cusum_arm() {
    let mut chart = default_combined();
    // 1.0σ shift is below 3.5σ Shewhart, but CUSUM accumulates.
    let mut detected = false;
    for _ in 0..30 {
        if chart.observe(1.0).unwrap().is_out_of_control() {
            detected = true;
            break;
        }
    }
    assert!(detected, "CUSUM arm should detect 1σ sustained shift");
}

#[test]
fn in_control_no_signal() {
    let mut chart = default_combined();
    for &x in &[0.1, -0.2, 0.3, -0.1, 0.0, 0.5, -0.5] {
        assert!(chart.observe(x).unwrap().is_in_control());
    }
}

#[test]
fn combined_detects_faster_than_either_alone() {
    use spc_charts::{CusumChart, ShewhartChart, ShewhartConfig};

    let limits = ControlLimits::new(0.0, 1.0).unwrap();

    // Sequence: small shift for a while, then a big spike.
    let sequence: Vec<f64> = (0..8).map(|_| 0.6).chain(std::iter::once(4.0)).collect();

    // Shewhart alone (k=3.5) won't catch the 0.6s.
    let mut shew = ShewhartChart::new(ShewhartConfig {
        limits: limits.clone(),
        k_sigma: 3.5,
        rules: vec![spc_charts::ShewhartRule::WE1],
    })
    .unwrap();
    let mut shew_rl = sequence.len();
    for (t, &x) in sequence.iter().enumerate() {
        if shew.observe(x).unwrap().is_out_of_control() {
            shew_rl = t + 1;
            break;
        }
    }

    // CUSUM alone (k=0.5, h=5) might not catch the spike as fast.
    let mut cusum = CusumChart::new(CusumConfig::default_for(limits.clone())).unwrap();
    let mut cusum_rl = sequence.len();
    for (t, &x) in sequence.iter().enumerate() {
        if cusum.observe(x).unwrap().is_out_of_control() {
            cusum_rl = t + 1;
            break;
        }
    }

    // Combined should detect no later than the minimum of the two.
    let mut comb = CombinedChart::new(CombinedConfig {
        cusum: CusumConfig::default_for(limits),
        shewhart_k: 3.5,
    })
    .unwrap();
    let mut comb_rl = sequence.len();
    for (t, &x) in sequence.iter().enumerate() {
        if comb.observe(x).unwrap().is_out_of_control() {
            comb_rl = t + 1;
            break;
        }
    }

    assert!(
        comb_rl <= shew_rl.min(cusum_rl),
        "combined (rl={comb_rl}) should detect ≤ min(shewhart={shew_rl}, cusum={cusum_rl})"
    );
}

#[test]
fn combined_rejects_nan() {
    let mut chart = default_combined();
    assert!(chart.observe(f64::NAN).is_err());
}
