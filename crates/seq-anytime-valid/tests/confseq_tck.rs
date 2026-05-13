#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use seq_anytime_valid::evidence::confseq;

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("seq-anytime-valid")
        .join("features")
        .join("confseq.feature")
}

/// Deterministic 50-element dataset centered near 0 for the "CS contains true mean" scenario.
/// Cycles through a fixed pattern so the test is reproducible and doesn't rely on a PRNG.
fn centered_50_obs() -> Vec<f64> {
    let pattern = [0.1_f64, -0.2, 0.3, -0.1, 0.05, 0.15, -0.05, 0.2, -0.3, 0.1];
    pattern.iter().cycle().take(50).copied().collect()
}

#[derive(Default, Debug)]
struct ConfSeqWorld {
    alpha: f64,
    width_n100: Option<f64>,
    width_n1000: Option<f64>,
    interval: Option<(f64, f64)>,
}

#[test]
fn confseq_feature_runs_end_to_end() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read feature: {e}"));
    let feature =
        parse_feature(&content, "confseq.feature").expect("confseq.feature parses cleanly");

    let runner = SyncRunner::new(ConfSeqWorld::default)
        // --- Given steps ---
        .step("a normal-mixture CS at alpha = 0.05", |w, _| {
            w.alpha = 0.05;
            w.width_n100 = None;
            w.width_n1000 = None;
            w.interval = None;
            Ok(())
        })
        // --- When steps ---
        .step(
            "I compute CS at n = 100 with mean 0.0 and variance 1.0",
            |w, _| {
                let sigma = 1.0_f64;
                w.width_n100 = Some(
                    confseq::cs_width(100, sigma, w.alpha)
                        .map_err(|e| StepError::new(e.to_string()))?,
                );
                Ok(())
            },
        )
        .step(
            "I compute CS at n = 1000 with mean 0.0 and variance 1.0",
            |w, _| {
                let sigma = 1.0_f64;
                w.width_n1000 = Some(
                    confseq::cs_width(1000, sigma, w.alpha)
                        .map_err(|e| StepError::new(e.to_string()))?,
                );
                Ok(())
            },
        )
        .step("I compute CS for 50 observations from N(0, 1)", |w, _| {
            let obs = centered_50_obs();
            w.interval = Some(
                confseq::normal_mixture_cs(&obs, w.alpha)
                    .map_err(|e| StepError::new(e.to_string()))?,
            );
            Ok(())
        })
        // --- Then steps ---
        .step(
            "the width at n = 1000 is less than the width at n = 100",
            |w, _| {
                let w100 = w
                    .width_n100
                    .ok_or_else(|| StepError::new("width at n=100 not computed"))?;
                let w1000 = w
                    .width_n1000
                    .ok_or_else(|| StepError::new("width at n=1000 not computed"))?;
                if w1000 < w100 {
                    Ok(())
                } else {
                    Err(StepError::new(format!(
                        "expected width_1000={w1000} < width_100={w100}"
                    )))
                }
            },
        )
        .step("the interval contains 0.0", |w, _| {
            let (lo, hi) = w
                .interval
                .ok_or_else(|| StepError::new("interval not computed"))?;
            if lo <= 0.0 && hi >= 0.0 {
                Ok(())
            } else {
                Err(StepError::new(format!(
                    "interval [{lo}, {hi}] does not contain 0.0"
                )))
            }
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
