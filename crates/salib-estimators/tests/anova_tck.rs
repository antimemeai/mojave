//! TCK harness for the ANOVA estimators.
//!
//! Wires `tck/salib/anova-estimator/features/anova_factorial.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines
)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use ndarray::{arr2, Array2, Array3};
use salib_core::RngState;
use salib_estimators::{
    estimate_anova_three_way, estimate_anova_three_way_with_bootstrap, estimate_anova_two_way,
};

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("anova-estimator")
        .join("features")
        .join("anova_factorial.feature")
}

fn two_way_grid() -> Array2<f64> {
    arr2(&[[9.0, 5.0], [7.0, 19.0]])
}

fn three_way_grid() -> Array3<f64> {
    let mut grid = Array3::<f64>::zeros((2, 2, 2));
    let levels = [-1.0_f64, 1.0_f64];
    for (i, &a) in levels.iter().enumerate() {
        for (j, &b) in levels.iter().enumerate() {
            for (k, &c) in levels.iter().enumerate() {
                grid[[i, j, k]] = 50.0
                    + 5.0 * a
                    + 3.0 * b
                    + 2.0 * c
                    + 4.0 * a * b
                    + 1.5 * a * c
                    + 1.0 * b * c
                    + 2.5 * a * b * c;
            }
        }
    }
    grid
}

#[derive(Default)]
struct World {
    two_way: Option<salib_estimators::AnovaTwoWayResult>,
    three_way: Option<salib_estimators::AnovaThreeWayResult>,
}

fn require_two_way(w: &World) -> Result<&salib_estimators::AnovaTwoWayResult, StepError> {
    w.two_way
        .as_ref()
        .ok_or_else(|| StepError::new("missing two-way estimate"))
}

fn require_three_way(w: &World) -> Result<&salib_estimators::AnovaThreeWayResult, StepError> {
    w.three_way
        .as_ref()
        .ok_or_else(|| StepError::new("missing three-way estimate"))
}

#[test]
fn anova_factorial_feature_runs() {
    let path = feature_path();
    let content = std::fs::read_to_string(&path).unwrap();
    let feature = parse_feature(&content, "anova_factorial.feature").expect("feature parses");

    let runner = SyncRunner::new(World::default)
        .step(
            "a balanced 2 x 2 factorial grid with crossed interaction structure",
            |_w, _| Ok(()),
        )
        .step("I estimate two-way ANOVA components", |w, _| {
            w.two_way = Some(
                estimate_anova_two_way(&two_way_grid())
                    .map_err(|e| StepError::new(format!("two-way estimate: {e}")))?,
            );
            Ok(())
        })
        .step(
            "the two-way variance fractions sum to 1 within 1e-9",
            |w, _| {
                let est = require_two_way(w)?;
                let sum = est.v_row + est.v_column + est.v_interaction + est.v_residual;
                if (sum - 1.0).abs() > 1.0e-9 {
                    return Err(StepError::new(format!("two-way sum = {sum}")));
                }
                Ok(())
            },
        )
        .step(
            "the interaction component exceeds the row component",
            |w, _| {
                let est = require_two_way(w)?;
                if est.v_interaction <= est.v_row {
                    return Err(StepError::new(format!(
                        "interaction {} <= row {}",
                        est.v_interaction, est.v_row
                    )));
                }
                Ok(())
            },
        )
        .step("the row component exceeds the column component", |w, _| {
            let est = require_two_way(w)?;
            if est.v_row <= est.v_column {
                return Err(StepError::new(format!(
                    "row {} <= column {}",
                    est.v_row, est.v_column
                )));
            }
            Ok(())
        })
        .step(
            "inferential statistics exist for the two-way main effects",
            |w, _| {
                let est = require_two_way(w)?;
                let stats = [est.f_row, est.f_column];
                let pvals = [est.p_row, est.p_column];
                if stats.iter().any(Option::is_none) || pvals.iter().any(Option::is_none) {
                    return Err(StepError::new("missing two-way inferential statistic"));
                }
                Ok(())
            },
        )
        .step(
            "no inferential statistic is emitted for the two-way interaction term",
            |w, _| {
                let est = require_two_way(w)?;
                if est.f_interaction.is_some() || est.p_interaction.is_some() {
                    return Err(StepError::new(
                        "unexpected two-way interaction inferential statistic",
                    ));
                }
                Ok(())
            },
        )
        .step(
            "a balanced 2 x 2 x 2 factorial grid with named crossed effects",
            |_w, _| Ok(()),
        )
        .step("I estimate three-way ANOVA components", |w, _| {
            w.three_way = Some(
                estimate_anova_three_way(&three_way_grid())
                    .map_err(|e| StepError::new(format!("three-way estimate: {e}")))?,
            );
            Ok(())
        })
        .step(
            "I estimate three-way ANOVA components with bootstrap confidence intervals",
            |w, _| {
                let mut rng = RngState::from_seed([0x22; 32]);
                w.three_way = Some(
                    estimate_anova_three_way_with_bootstrap(&three_way_grid(), 128, 0.05, &mut rng)
                        .map_err(|e| {
                            StepError::new(format!("three-way bootstrap estimate: {e}"))
                        })?,
                );
                Ok(())
            },
        )
        .step(
            "the three-way variance fractions sum to 1 within 1e-9",
            |w, _| {
                let est = require_three_way(w)?;
                let sum = est.v_data
                    + est.v_brittleness
                    + est.v_inference
                    + est.v_data_brittleness
                    + est.v_data_inference
                    + est.v_brittleness_inference
                    + est.v_data_brittleness_inference
                    + est.v_residual;
                if (sum - 1.0).abs() > 1.0e-9 {
                    return Err(StepError::new(format!("three-way sum = {sum}")));
                }
                Ok(())
            },
        )
        .step(
            "the data component exceeds the brittleness component",
            |w, _| {
                let est = require_three_way(w)?;
                if est.v_data <= est.v_brittleness {
                    return Err(StepError::new(format!(
                        "data {} <= brittleness {}",
                        est.v_data, est.v_brittleness
                    )));
                }
                Ok(())
            },
        )
        .step(
            "the brittleness component exceeds the inference component",
            |w, _| {
                let est = require_three_way(w)?;
                if est.v_brittleness <= est.v_inference {
                    return Err(StepError::new(format!(
                        "brittleness {} <= inference {}",
                        est.v_brittleness, est.v_inference
                    )));
                }
                Ok(())
            },
        )
        .step(
            "the data-brittleness interaction exceeds the data-inference interaction",
            |w, _| {
                let est = require_three_way(w)?;
                if est.v_data_brittleness <= est.v_data_inference {
                    return Err(StepError::new(format!(
                        "data*brittleness {} <= data*inference {}",
                        est.v_data_brittleness, est.v_data_inference
                    )));
                }
                Ok(())
            },
        )
        .step(
            "inferential statistics exist for the main effects and two-way interactions",
            |w, _| {
                let est = require_three_way(w)?;
                let stats = [
                    est.f_data,
                    est.f_brittleness,
                    est.f_inference,
                    est.f_data_brittleness,
                    est.f_data_inference,
                    est.f_brittleness_inference,
                ];
                let pvals = [
                    est.p_data,
                    est.p_brittleness,
                    est.p_inference,
                    est.p_data_brittleness,
                    est.p_data_inference,
                    est.p_brittleness_inference,
                ];
                if stats.iter().any(Option::is_none) || pvals.iter().any(Option::is_none) {
                    return Err(StepError::new("missing inferential statistic"));
                }
                Ok(())
            },
        )
        .step(
            "no inferential statistic is emitted for the three-way interaction term",
            |w, _| {
                let est = require_three_way(w)?;
                if est.f_data_brittleness_inference.is_some()
                    || est.p_data_brittleness_inference.is_some()
                {
                    return Err(StepError::new("unexpected three-way inferential statistic"));
                }
                Ok(())
            },
        )
        .step(
            "bootstrap confidence intervals exist for every three-way variance fraction",
            |w, _| {
                let est = require_three_way(w)?;
                let lows = est
                    .variance_fraction_ci_low
                    .as_ref()
                    .ok_or_else(|| StepError::new("missing ci_low"))?;
                let highs = est
                    .variance_fraction_ci_high
                    .as_ref()
                    .ok_or_else(|| StepError::new("missing ci_high"))?;
                if lows.len() != 8 || highs.len() != 8 {
                    return Err(StepError::new("unexpected CI vector length"));
                }
                Ok(())
            },
        )
        .step(
            "the bootstrap metadata records 128 resamples at alpha 0.05",
            |w, _| {
                let est = require_three_way(w)?;
                if est.bootstrap_iterations != Some(128) {
                    return Err(StepError::new("wrong bootstrap_iterations"));
                }
                if est.bootstrap_alpha != Some(0.05) {
                    return Err(StepError::new("wrong bootstrap_alpha"));
                }
                Ok(())
            },
        );

    let report = runner.run(&feature);
    report.assert_all_passed();
}
