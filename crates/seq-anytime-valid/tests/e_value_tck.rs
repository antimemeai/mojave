#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use seq_anytime_valid::evidence::e_value;
use seq_anytime_valid::Decision;

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("seq-anytime-valid")
        .join("features")
        .join("e_value.feature")
}

#[derive(Default, Debug)]
struct EValueWorld {
    e_values: Vec<f64>,
    e_val: f64,
    alpha: f64,
    product: Option<f64>,
    p_value: Option<f64>,
    decision: Option<Decision>,
}

#[test]
fn e_value_feature_runs_end_to_end() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read feature: {e}"));
    let feature =
        parse_feature(&content, "e_value.feature").expect("e_value.feature parses cleanly");

    let runner = SyncRunner::new(EValueWorld::default)
        // --- Given steps ---
        .step("two independent e-values 3.0 and 4.0", |w, _| {
            w.e_values = vec![3.0, 4.0];
            Ok(())
        })
        .step("an e-value of 25.0", |w, _| {
            w.e_val = 25.0;
            Ok(())
        })
        .step("an e-value of 25.0 and alpha = 0.05", |w, _| {
            w.e_val = 25.0;
            w.alpha = 0.05;
            Ok(())
        })
        .step("an e-value of 15.0 and alpha = 0.05", |w, _| {
            w.e_val = 15.0;
            w.alpha = 0.05;
            Ok(())
        })
        // --- When steps ---
        .step("I compute the product e-value", |w, _| {
            w.product = Some(
                e_value::product_e_value(&w.e_values).map_err(|e| StepError::new(e.to_string()))?,
            );
            Ok(())
        })
        .step("I convert to a conservative p-value", |w, _| {
            w.p_value = Some(e_value::e_to_p(w.e_val));
            Ok(())
        })
        .step("I check the threshold 1/alpha = 20.0", |w, _| {
            w.decision = Some(e_value::threshold_decision(w.e_val, w.alpha));
            Ok(())
        })
        // --- Then steps ---
        .step("the result is 12.0", |w, _| {
            let product = w
                .product
                .ok_or_else(|| StepError::new("product not computed"))?;
            if (product - 12.0).abs() < 1e-10 {
                Ok(())
            } else {
                Err(StepError::new(format!("expected 12.0, got {product}")))
            }
        })
        .step("the p-value is 0.04", |w, _| {
            let p = w
                .p_value
                .ok_or_else(|| StepError::new("p-value not computed"))?;
            if (p - 0.04).abs() < 1e-10 {
                Ok(())
            } else {
                Err(StepError::new(format!("expected 0.04, got {p}")))
            }
        })
        .step("the decision is Reject", |w, _| {
            if w.decision == Some(Decision::Reject) {
                Ok(())
            } else {
                Err(StepError::new(format!(
                    "expected Reject, got {:?}",
                    w.decision
                )))
            }
        })
        .step("the decision is Continue", |w, _| {
            if w.decision == Some(Decision::Continue) {
                Ok(())
            } else {
                Err(StepError::new(format!(
                    "expected Continue, got {:?}",
                    w.decision
                )))
            }
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
