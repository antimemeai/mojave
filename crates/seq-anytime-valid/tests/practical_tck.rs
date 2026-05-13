#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use seq_anytime_valid::practical;
use seq_anytime_valid::SeqError;

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("seq-anytime-valid")
        .join("features")
        .join("practical.feature")
}

#[derive(Default, Debug)]
struct PracticalWorld {
    delta: f64,
    mixing_variance: f64,
    observations: Vec<f64>,
    p_value: Option<f64>,
    last_error: Option<SeqError>,
}

#[test]
fn practical_feature_runs_end_to_end() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read feature: {e}"));
    let feature =
        parse_feature(&content, "practical.feature").expect("practical.feature parses cleanly");

    let runner = SyncRunner::new(PracticalWorld::default)
        // --- Given steps ---
        .step(
            "delta = 0.5 and mixing_variance = 1.0 and alpha = 0.05",
            |w, _| {
                w.delta = 0.5;
                w.mixing_variance = 1.0;
                w.p_value = None;
                w.last_error = None;
                Ok(())
            },
        )
        .step(
            "delta = 1.0 and mixing_variance = 1.0 and alpha = 0.05",
            |w, _| {
                w.delta = 1.0;
                w.mixing_variance = 1.0;
                w.p_value = None;
                w.last_error = None;
                Ok(())
            },
        )
        // --- When steps ---
        .step("I observe 20 values all equal to 2.0", |w, _| {
            w.observations = vec![2.0; 20];
            match practical::practical_significance_p(&w.observations, w.delta, w.mixing_variance) {
                Ok(p) => {
                    w.p_value = Some(p);
                }
                Err(e) => {
                    w.last_error = Some(e);
                }
            }
            Ok(())
        })
        .step("I observe 20 values all equal to 0.1", |w, _| {
            w.observations = vec![0.1; 20];
            match practical::practical_significance_p(&w.observations, w.delta, w.mixing_variance) {
                Ok(p) => {
                    w.p_value = Some(p);
                }
                Err(e) => {
                    w.last_error = Some(e);
                }
            }
            Ok(())
        })
        .step("I try to create with delta = -0.5", |w, _| {
            w.observations = vec![1.0];
            match practical::practical_significance_p(&w.observations, -0.5, 1.0) {
                Ok(p) => {
                    w.p_value = Some(p);
                }
                Err(e) => {
                    w.last_error = Some(e);
                }
            }
            Ok(())
        })
        // --- Then steps ---
        .step(
            "the practical significance p-value is less than 0.05",
            |w, _| {
                let p = w
                    .p_value
                    .ok_or_else(|| StepError::new("p-value not computed"))?;
                if p < 0.05 {
                    Ok(())
                } else {
                    Err(StepError::new(format!("expected p < 0.05, got {p}")))
                }
            },
        )
        .step(
            "the practical significance p-value is greater than 0.05",
            |w, _| {
                let p = w
                    .p_value
                    .ok_or_else(|| StepError::new("p-value not computed"))?;
                if p > 0.05 {
                    Ok(())
                } else {
                    Err(StepError::new(format!("expected p > 0.05, got {p}")))
                }
            },
        )
        .step("I get an InvalidPracticalDelta error", |w, _| {
            match &w.last_error {
                Some(SeqError::InvalidPracticalDelta(_)) => Ok(()),
                Some(e) => Err(StepError::new(format!(
                    "expected InvalidPracticalDelta, got {e}"
                ))),
                None => Err(StepError::new(
                    "expected an error but none was recorded".to_string(),
                )),
            }
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
