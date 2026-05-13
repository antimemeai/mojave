#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use seq_anytime_valid::evidence::msprt;

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("seq-anytime-valid")
        .join("features")
        .join("msprt.feature")
}

#[derive(Default, Debug)]
struct MsprtWorld {
    theta_0: f64,
    mixing_variance: f64,
    observations: Vec<f64>,
    p_value: Option<f64>,
}

#[test]
fn msprt_feature_runs_end_to_end() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read feature: {e}"));
    let feature = parse_feature(&content, "msprt.feature").expect("msprt.feature parses cleanly");

    let runner = SyncRunner::new(MsprtWorld::default)
        // --- Given steps ---
        .step(
            "mSPRT config with theta_0 = 0.0 and mixing_variance = 1.0",
            |w, _| {
                w.theta_0 = 0.0;
                w.mixing_variance = 1.0;
                w.observations.clear();
                Ok(())
            },
        )
        // --- When steps ---
        .step("I have observed 0 data points", |w, _| {
            w.observations = vec![];
            w.p_value = Some(
                msprt::always_valid_p(&w.observations, w.theta_0, w.mixing_variance)
                    .map_err(|e| StepError::new(e.to_string()))?,
            );
            Ok(())
        })
        .step("I observe 10 values all equal to 2.0", |w, _| {
            w.observations = vec![2.0; 10];
            w.p_value = Some(
                msprt::always_valid_p(&w.observations, w.theta_0, w.mixing_variance)
                    .map_err(|e| StepError::new(e.to_string()))?,
            );
            Ok(())
        })
        .step("I observe 10 values all equal to 0.0", |w, _| {
            w.observations = vec![0.0; 10];
            w.p_value = Some(
                msprt::always_valid_p(&w.observations, w.theta_0, w.mixing_variance)
                    .map_err(|e| StepError::new(e.to_string()))?,
            );
            Ok(())
        })
        // --- Then steps ---
        .step("the always-valid p-value is 1.0", |w, _| {
            let p = w
                .p_value
                .ok_or_else(|| StepError::new("p-value not computed"))?;
            if (p - 1.0).abs() < 1e-10 {
                Ok(())
            } else {
                Err(StepError::new(format!("expected p=1.0, got {p}")))
            }
        })
        .step("the always-valid p-value is less than 0.05", |w, _| {
            let p = w
                .p_value
                .ok_or_else(|| StepError::new("p-value not computed"))?;
            if p < 0.05 {
                Ok(())
            } else {
                Err(StepError::new(format!("expected p < 0.05, got {p}")))
            }
        })
        .step("the always-valid p-value is greater than 0.5", |w, _| {
            let p = w
                .p_value
                .ok_or_else(|| StepError::new("p-value not computed"))?;
            if p > 0.5 {
                Ok(())
            } else {
                Err(StepError::new(format!("expected p > 0.5, got {p}")))
            }
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
