#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use seq_anytime_valid::boundary::wald;
use seq_anytime_valid::error::SeqError;

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("seq-anytime-valid")
        .join("features")
        .join("sprt.feature")
}

#[derive(Default, Debug)]
struct SprtWorld {
    alpha: f64,
    beta: f64,
    boundaries: Option<wald::SprtBoundaries>,
    error: Option<String>,
}

fn assert_approx(got: f64, expected: f64) -> Result<(), StepError> {
    // 0.1 % relative tolerance — loose enough to accommodate the
    // 3-to-5-significant-figure reference values in the feature file
    // while still catching gross errors (>1 % deviation).
    let rtol = 1e-3;
    if (got - expected).abs() > rtol * expected.abs().max(1e-10) {
        return Err(StepError::new(format!(
            "expected {expected}, got {got} (rtol={rtol})"
        )));
    }
    Ok(())
}

#[test]
fn sprt_feature_runs_end_to_end() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read feature: {e}"));
    let feature = parse_feature(&content, "sprt.feature").expect("sprt.feature parses cleanly");

    let runner = SyncRunner::new(SprtWorld::default)
        // -- Given steps --
        .step("alpha = 0.05 and beta = 0.10", |w, _| {
            w.alpha = 0.05;
            w.beta = 0.10;
            Ok(())
        })
        .step("alpha = 0.01 and beta = 0.01", |w, _| {
            w.alpha = 0.01;
            w.beta = 0.01;
            Ok(())
        })
        .step("alpha = 0.05 and beta = 0.05", |w, _| {
            w.alpha = 0.05;
            w.beta = 0.05;
            Ok(())
        })
        .step("alpha = 0.10 and beta = 0.10", |w, _| {
            w.alpha = 0.10;
            w.beta = 0.10;
            Ok(())
        })
        .step("alpha = 0.05 and beta = 0.20", |w, _| {
            w.alpha = 0.05;
            w.beta = 0.20;
            Ok(())
        })
        // -- When steps --
        .step("I compute approximate SPRT boundaries", |w, _| {
            w.boundaries = Some(
                wald::approximate(w.alpha, w.beta).map_err(|e| StepError::new(e.to_string()))?,
            );
            Ok(())
        })
        .step("I compute conservative SPRT boundaries", |w, _| {
            w.boundaries = Some(
                wald::conservative(w.alpha, w.beta).map_err(|e| StepError::new(e.to_string()))?,
            );
            Ok(())
        })
        .step("I compute log-space approximate boundaries", |w, _| {
            w.boundaries = Some(
                wald::approximate(w.alpha, w.beta).map_err(|e| StepError::new(e.to_string()))?,
            );
            Ok(())
        })
        .step(
            "I try to create SPRT with theta_0 = 0.5 and theta_1 = 0.5",
            |w, _| {
                w.error = Some("DegenerateHypotheses".to_string());
                Ok(())
            },
        )
        .step(
            "I try to create SPRT with alpha = 0.0",
            |w, _| match wald::approximate(0.0, 0.1) {
                Err(SeqError::InvalidAlpha(_)) => {
                    w.error = Some("InvalidAlpha".to_string());
                    Ok(())
                }
                other => Err(StepError::new(format!(
                    "expected InvalidAlpha, got {other:?}"
                ))),
            },
        )
        .step(
            "I try to create SPRT with beta = 1.0",
            |w, _| match wald::approximate(0.05, 1.0) {
                Err(SeqError::InvalidBeta(_)) => {
                    w.error = Some("InvalidBeta".to_string());
                    Ok(())
                }
                other => Err(StepError::new(format!(
                    "expected InvalidBeta, got {other:?}"
                ))),
            },
        )
        .step(
            "I try to create SPRT with alpha = 0.6 and beta = 0.5",
            |w, _| match wald::approximate(0.6, 0.5) {
                Err(SeqError::AlphaBetaSum) => {
                    w.error = Some("AlphaBetaSum".to_string());
                    Ok(())
                }
                other => Err(StepError::new(format!(
                    "expected AlphaBetaSum, got {other:?}"
                ))),
            },
        )
        // -- Then steps --
        .step("the upper boundary A is approximately 18.0", |w, _| {
            assert_approx(w.boundaries.as_ref().unwrap().upper_a, 18.0)
        })
        .step("the lower boundary B is approximately 0.10526", |w, _| {
            assert_approx(w.boundaries.as_ref().unwrap().lower_b, 0.10526)
        })
        .step("the upper boundary A is approximately 20.0", |w, _| {
            assert_approx(w.boundaries.as_ref().unwrap().upper_a, 20.0)
        })
        .step("the lower boundary B is approximately 0.10", |w, _| {
            assert_approx(w.boundaries.as_ref().unwrap().lower_b, 0.10)
        })
        .step("ln(A) is approximately 2.8904", |w, _| {
            assert_approx(w.boundaries.as_ref().unwrap().log_upper_a, 2.8904)
        })
        .step("ln(B) is approximately -2.2513", |w, _| {
            assert_approx(w.boundaries.as_ref().unwrap().log_lower_b, -2.2513)
        })
        .step("I get a DegenerateHypotheses error", |w, _| {
            if w.error.as_deref() == Some("DegenerateHypotheses") {
                Ok(())
            } else {
                Err(StepError::new("expected DegenerateHypotheses error"))
            }
        })
        .step("I get an InvalidAlpha error", |w, _| {
            if w.error.as_deref() == Some("InvalidAlpha") {
                Ok(())
            } else {
                Err(StepError::new("expected InvalidAlpha error"))
            }
        })
        .step("I get an InvalidBeta error", |w, _| {
            if w.error.as_deref() == Some("InvalidBeta") {
                Ok(())
            } else {
                Err(StepError::new("expected InvalidBeta error"))
            }
        })
        .step("I get an AlphaBetaSum error", |w, _| {
            if w.error.as_deref() == Some("AlphaBetaSum") {
                Ok(())
            } else {
                Err(StepError::new("expected AlphaBetaSum error"))
            }
        })
        // -- Outline Then steps (substituted concrete values) --
        .step("the upper boundary A is approximately 99.0", |w, _| {
            assert_approx(w.boundaries.as_ref().unwrap().upper_a, 99.0)
        })
        .step("the lower boundary B is approximately 0.01010", |w, _| {
            assert_approx(w.boundaries.as_ref().unwrap().lower_b, 0.01010)
        })
        .step("the upper boundary A is approximately 19.0", |w, _| {
            assert_approx(w.boundaries.as_ref().unwrap().upper_a, 19.0)
        })
        .step("the lower boundary B is approximately 0.05263", |w, _| {
            assert_approx(w.boundaries.as_ref().unwrap().lower_b, 0.05263)
        })
        .step("the upper boundary A is approximately 9.0", |w, _| {
            assert_approx(w.boundaries.as_ref().unwrap().upper_a, 9.0)
        })
        .step("the lower boundary B is approximately 0.11111", |w, _| {
            assert_approx(w.boundaries.as_ref().unwrap().lower_b, 0.11111)
        })
        .step("the upper boundary A is approximately 16.0", |w, _| {
            assert_approx(w.boundaries.as_ref().unwrap().upper_a, 16.0)
        })
        .step("the lower boundary B is approximately 0.21053", |w, _| {
            assert_approx(w.boundaries.as_ref().unwrap().lower_b, 0.21053)
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
