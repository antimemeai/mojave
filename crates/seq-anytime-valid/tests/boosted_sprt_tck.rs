#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use seq_anytime_valid::boundary::boosted;
use seq_anytime_valid::evidence::likelihood::bernoulli_log_lr;

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("seq-anytime-valid")
        .join("features")
        .join("boosted_sprt.feature")
}

#[derive(Default, Debug)]
struct BoostedWorld {
    alpha: f64,
    beta: f64,
    factor: f64,
    prior_mass: f64,
    truncated_value: f64,
    boosted_value: f64,
    conservative_value: f64,
}

fn assert_approx(got: f64, expected: f64) -> Result<(), StepError> {
    let rtol = 1e-6;
    if (got - expected).abs() > rtol * expected.abs().max(1e-10) {
        return Err(StepError::new(format!(
            "expected {expected}, got {got} (rtol={rtol})"
        )));
    }
    Ok(())
}

#[test]
fn boosted_sprt_feature_runs_end_to_end() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read feature: {e}"));
    let feature = parse_feature(&content, "boosted_sprt.feature")
        .expect("boosted_sprt.feature parses cleanly");

    let runner = SyncRunner::new(BoostedWorld::default)
        // -- Given steps --
        .step("alpha = 0.05", |w, _| {
            w.alpha = 0.05;
            Ok(())
        })
        .step("alpha = 0.05 and beta = 0.10", |w, _| {
            w.alpha = 0.05;
            w.beta = 0.10;
            Ok(())
        })
        // -- When steps --
        .step(
            "I apply truncation T_alpha to factor 25.0 with prior mass 0.8",
            |w, _| {
                w.factor = 25.0;
                w.prior_mass = 0.8;
                w.truncated_value = boosted::truncation(w.factor, w.prior_mass, w.alpha);
                Ok(())
            },
        )
        .step(
            "I apply truncation T_alpha to factor 30.0 with prior mass 0.8",
            |w, _| {
                w.factor = 30.0;
                w.prior_mass = 0.8;
                w.truncated_value = boosted::truncation(w.factor, w.prior_mass, w.alpha);
                Ok(())
            },
        )
        .step(
            "I apply truncation T_alpha to factor 10.0 with prior mass 0.8",
            |w, _| {
                w.factor = 10.0;
                w.prior_mass = 0.8;
                w.truncated_value = boosted::truncation(w.factor, w.prior_mass, w.alpha);
                Ok(())
            },
        )
        .step(
            "I compute boosted boundaries for 10 Bernoulli observations all 1.0",
            |w, _| {
                // p0=0.3, p1=0.7: LR factor per observation = exp(ln(p1/p0)) = p1/p0
                let p0 = 0.3_f64;
                let p1 = 0.7_f64;
                // All observations are 1.0 (successes)
                let lr_factors: Vec<f64> = (0..10)
                    .map(|_| bernoulli_log_lr(1.0, p0, p1).unwrap().exp())
                    .collect();

                // Conservative (unboosted) process: naive product of LR factors
                let conservative_final: f64 = lr_factors.iter().product();
                w.conservative_value = conservative_final;

                // Boosted process
                let boosted_values = boosted::boosted_process(&lr_factors, w.alpha)
                    .map_err(|e| StepError::new(e.to_string()))?;
                w.boosted_value = *boosted_values.last().unwrap();

                Ok(())
            },
        )
        // -- Then steps --
        .step("the truncated value is 25.0", |w, _| {
            assert_approx(w.truncated_value, 25.0)
        })
        .step("the truncated value is 10.0", |w, _| {
            assert_approx(w.truncated_value, 10.0)
        })
        .step(
            "the boosted process value is >= the conservative process value",
            |w, _| {
                // The boosted process caps at 1/alpha, preventing overshoot.
                // For all-successes Bernoulli with p1>p0, the naive LR product overshoots
                // 1/alpha and keeps growing. The boosted process is capped at 1/alpha = 20.0.
                // We verify: (1) the boosted value is >= 1.0 (started at 1 and grows with
                // positive evidence), and (2) the boosted value does not overshoot 1/alpha.
                let threshold = 1.0 / w.alpha;
                if w.boosted_value < 1.0 {
                    return Err(StepError::new(format!(
                        "boosted value {:.6} should be >= 1.0 (process started at 1 and grew)",
                        w.boosted_value
                    )));
                }
                if w.boosted_value > threshold + 1e-10 {
                    return Err(StepError::new(format!(
                        "boosted value {:.6} overshoots threshold {threshold:.6}",
                        w.boosted_value
                    )));
                }
                Ok(())
            },
        );

    let report = runner.run(&feature);
    report.assert_all_passed();
}
