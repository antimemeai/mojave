#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use ndarray::Array3;
use salib_estimators::{
    estimate_g_theory_pir, project_g_theory_d_study, DStudyPoint, GTheoryDesign,
};

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("g-theory-estimator")
        .join("features")
        .join("g_theory_d_study.feature")
}

fn grid() -> Array3<f64> {
    let mut grid = Array3::<f64>::zeros((2, 2, 2));
    let levels = [-1.0_f64, 1.0_f64];
    for (ip, &p) in levels.iter().enumerate() {
        for (ii, &i) in levels.iter().enumerate() {
            for (ir, &r) in levels.iter().enumerate() {
                grid[[ip, ii, ir]] = 50.0
                    + 6.0 * p
                    + 4.0 * i
                    + 2.0 * r
                    + 3.0 * p * i
                    + 1.5 * p * r
                    + 1.0 * i * r
                    + 0.5 * p * i * r;
            }
        }
    }
    grid
}

#[derive(Default)]
struct World {
    estimate: Option<salib_estimators::GTheoryResult>,
    current: Option<DStudyPoint>,
    double_items: Option<DStudyPoint>,
    double_raters: Option<DStudyPoint>,
    expanded: Option<DStudyPoint>,
}

#[test]
fn g_theory_d_study_feature_runs() {
    let content = std::fs::read_to_string(feature_path()).unwrap();
    let feature = parse_feature(&content, "g_theory_d_study.feature").unwrap();
    let runner = SyncRunner::new(World::default)
        .step("a crossed p x i x r G-theory estimate", |w, _| {
            w.estimate = Some(
                estimate_g_theory_pir(&grid(), GTheoryDesign::Crossed)
                    .map_err(|e| StepError::new(format!("estimate: {e}")))?,
            );
            Ok(())
        })
        .step("I project a D-study at 2 items and 2 raters", |w, _| {
            let estimate = w
                .estimate
                .as_ref()
                .ok_or_else(|| StepError::new("missing estimate"))?;
            w.current = Some(
                project_g_theory_d_study(estimate, 2, 2)
                    .map_err(|e| StepError::new(format!("current projection: {e}")))?,
            );
            Ok(())
        })
        .step("I project a D-study at 4 items and 2 raters", |w, _| {
            let estimate = w
                .estimate
                .as_ref()
                .ok_or_else(|| StepError::new("missing estimate"))?;
            w.double_items = Some(
                project_g_theory_d_study(estimate, 4, 2)
                    .map_err(|e| StepError::new(format!("double-items projection: {e}")))?,
            );
            Ok(())
        })
        .step("I project a D-study at 2 items and 4 raters", |w, _| {
            let estimate = w
                .estimate
                .as_ref()
                .ok_or_else(|| StepError::new("missing estimate"))?;
            w.double_raters = Some(
                project_g_theory_d_study(estimate, 2, 4)
                    .map_err(|e| StepError::new(format!("double-raters projection: {e}")))?,
            );
            Ok(())
        })
        .step("I project a D-study at 4 items and 4 raters", |w, _| {
            let estimate = w
                .estimate
                .as_ref()
                .ok_or_else(|| StepError::new("missing estimate"))?;
            w.expanded = Some(
                project_g_theory_d_study(estimate, 4, 4)
                    .map_err(|e| StepError::new(format!("expanded projection: {e}")))?,
            );
            Ok(())
        })
        .step("projected G increases", |w, _| {
            if w.expanded.as_ref().unwrap().g_coefficient
                <= w.current.as_ref().unwrap().g_coefficient
            {
                return Err(StepError::new("projected G did not increase"));
            }
            Ok(())
        })
        .step("projected Phi increases", |w, _| {
            if w.expanded.as_ref().unwrap().phi_coefficient
                <= w.current.as_ref().unwrap().phi_coefficient
            {
                return Err(StepError::new("projected Phi did not increase"));
            }
            Ok(())
        })
        .step(
            "projected G exceeds projected Phi at both points",
            |w, _| {
                let current = w.current.as_ref().unwrap();
                let expanded = w.expanded.as_ref().unwrap();
                if current.g_coefficient <= current.phi_coefficient
                    || expanded.g_coefficient <= expanded.phi_coefficient
                {
                    return Err(StepError::new("G did not exceed Phi"));
                }
                Ok(())
            },
        )
        .step(
            "the V1 D-study surface includes exactly four projected points",
            |w, _| {
                let points = [
                    w.current.as_ref(),
                    w.double_items.as_ref(),
                    w.double_raters.as_ref(),
                    w.expanded.as_ref(),
                ];
                if points.iter().filter(|point| point.is_some()).count() != 4 {
                    return Err(StepError::new("expected exactly four projected points"));
                }
                if w.double_items.as_ref().map(|p| (p.n_items, p.n_raters)) != Some((4, 2)) {
                    return Err(StepError::new("double-items point was not (4,2)"));
                }
                if w.double_raters.as_ref().map(|p| (p.n_items, p.n_raters)) != Some((2, 4)) {
                    return Err(StepError::new("double-raters point was not (2,4)"));
                }
                Ok(())
            },
        );
    let report = runner.run(&feature);
    report.assert_all_passed();
}
