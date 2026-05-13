#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use seq_anytime_valid::bias;

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("seq-anytime-valid")
        .join("features")
        .join("bias.feature")
}

#[derive(Default, Debug)]
struct BiasWorld {
    n: usize,
    mle: f64,
    mu0: f64,
    mu1: f64,
    corrected_estimate: Option<f64>,
    median_estimate: Option<f64>,
}

#[test]
fn bias_feature_runs_end_to_end() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read feature: {e}"));
    let feature = parse_feature(&content, "bias.feature").expect("bias.feature parses cleanly");

    let runner = SyncRunner::new(BiasWorld::default)
        // --- Given steps ---
        .step("a normal-mean SPRT that stopped at n = 20", |w, _| {
            w.n = 20;
            Ok(())
        })
        .step("the MLE at stopping is 0.8", |w, _| {
            w.mle = 0.8;
            Ok(())
        })
        .step(
            "the SPRT config is mu0 = 0.0, mu1 = 0.5, alpha = 0.05",
            |w, _| {
                w.mu0 = 0.0;
                w.mu1 = 0.5;
                Ok(())
            },
        )
        // --- When steps ---
        .step("I compute the bias-corrected estimate", |w, _| {
            let corrected = bias::bias_corrected_mle(w.mle, w.n, w.mu0, w.mu1)
                .map_err(|e| StepError::new(e.to_string()))?;
            w.corrected_estimate = Some(corrected);
            Ok(())
        })
        .step("I compute the median-unbiased estimate", |w, _| {
            let estimate = bias::median_unbiased_estimate(w.mle, w.n, w.mu0)
                .map_err(|e| StepError::new(e.to_string()))?;
            w.median_estimate = Some(estimate);
            Ok(())
        })
        // --- Then steps ---
        .step(
            "the corrected estimate is less than 0.8 in absolute value",
            |w, _| {
                let corrected = w
                    .corrected_estimate
                    .ok_or_else(|| StepError::new("corrected estimate not computed"))?;
                if corrected.abs() < 0.8 {
                    Ok(())
                } else {
                    Err(StepError::new(format!(
                        "expected |corrected| < 0.8, got {corrected}"
                    )))
                }
            },
        )
        .step("the estimate is between 0.0 and 0.8", |w, _| {
            let estimate = w
                .median_estimate
                .ok_or_else(|| StepError::new("median estimate not computed"))?;
            if estimate > 0.0 && estimate < 0.8 {
                Ok(())
            } else {
                Err(StepError::new(format!(
                    "expected estimate in (0.0, 0.8), got {estimate}"
                )))
            }
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
