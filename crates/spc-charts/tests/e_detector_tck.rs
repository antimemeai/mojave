#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use spc_charts::{EDetector, EDetectorConfig, EDetectorWindow, GaussianEValue};

fn gaussian_detector(alpha: f64) -> EDetector<GaussianEValue> {
    // mixing_variance = 0.1 keeps e-values close to 1 under H_0, ensuring
    // the max(1, M*e) e-detector false-alarm rate stays near Ville's bound.
    let source = GaussianEValue::new(0.0, 1.0, 0.1).unwrap();
    let config = EDetectorConfig {
        alpha,
        window: EDetectorWindow::Growing,
    };
    EDetector::new(config, source).unwrap()
}

#[test]
fn initial_e_process_is_one() {
    let det = gaussian_detector(0.05);
    assert_eq!(det.e_process(), 1.0);
}

#[test]
fn detects_shift() {
    let mut det = gaussian_detector(0.05);
    let mut detected = false;
    for _ in 0..200 {
        if det.observe(1.5).unwrap().is_out_of_control() {
            detected = true;
            break;
        }
    }
    assert!(detected, "should detect 1.5σ shift");
}

#[test]
fn e_process_floor_is_one() {
    let mut det = gaussian_detector(0.05);
    // Feed observations that produce e-values < 1 (near mu_0).
    for _ in 0..100 {
        det.observe(0.0).unwrap();
        assert!(
            det.e_process() >= 1.0 - 1e-10,
            "M_t = {} < 1",
            det.e_process()
        );
    }
}

#[test]
fn mc_false_alarm_rate() {
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;
    use rand_distr::{Distribution, StandardNormal};

    let alpha = 0.05;
    let n_sims = 10_000;
    let seq_len = 500;
    let mut rng = ChaCha20Rng::seed_from_u64(123);
    let mut false_alarms = 0;

    for _ in 0..n_sims {
        let mut det = gaussian_detector(alpha);
        for _ in 0..seq_len {
            let x: f64 = StandardNormal.sample(&mut rng);
            if det.observe(x).unwrap().is_out_of_control() {
                false_alarms += 1;
                break;
            }
        }
    }

    let empirical_rate = false_alarms as f64 / n_sims as f64;
    assert!(
        empirical_rate <= alpha + 0.01,
        "false alarm rate = {empirical_rate}, should be ≤ {alpha}+margin"
    );
}

#[test]
fn reset_clears_state() {
    let mut det = gaussian_detector(0.05);
    for _ in 0..10 {
        det.observe(2.0).unwrap();
    }
    det.reset();
    assert_eq!(det.e_process(), 1.0);
    assert_eq!(det.n_observations(), 0);
}

#[test]
fn fixed_window_mode() {
    let source = GaussianEValue::new(0.0, 1.0, 1.0).unwrap();
    let config = EDetectorConfig {
        alpha: 0.05,
        window: EDetectorWindow::Fixed { width: 5 },
    };
    let mut det = EDetector::new(config, source).unwrap();

    // Feed 10 observations. After 5, the window should be sliding.
    for _ in 0..10 {
        det.observe(0.0).unwrap();
    }
    assert_eq!(det.n_observations(), 10);
}

#[test]
fn e_detector_rejects_nan() {
    let mut det = gaussian_detector(0.05);
    assert!(det.observe(f64::NAN).is_err());
}
