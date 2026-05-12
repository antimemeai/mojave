#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines
)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use ndarray::Array3;
use salib_core::RngState;
use salib_estimators::{
    estimate_g_theory_pir, estimate_g_theory_pir_with_bootstrap, GTheoryDesign, GTheoryResult,
};

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("g-theory-estimator")
        .join("features")
        .join("g_theory_pir.feature")
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
    result: Option<GTheoryResult>,
}

fn result(w: &World) -> Result<&GTheoryResult, StepError> {
    w.result
        .as_ref()
        .ok_or_else(|| StepError::new("missing g-theory estimate"))
}

#[test]
fn g_theory_feature_runs() {
    let content = std::fs::read_to_string(feature_path()).unwrap();
    let feature = parse_feature(&content, "g_theory_pir.feature").unwrap();
    let runner = SyncRunner::new(World::default)
        .step(
            "a balanced 2 x 2 x 2 p x i x r grid with crossed random effects",
            |_w, _| Ok(()),
        )
        .step("I estimate G-theory p x i x r components", |w, _| {
            w.result = Some(
                estimate_g_theory_pir(&grid(), GTheoryDesign::Crossed)
                    .map_err(|e| StepError::new(format!("estimate: {e}")))?,
            );
            Ok(())
        })
        .step(
            "I estimate G-theory p x i x r components with bootstrap confidence intervals",
            |w, _| {
                let mut rng = RngState::from_seed([0x44; 32]);
                w.result = Some(
                    estimate_g_theory_pir_with_bootstrap(
                        &grid(),
                        GTheoryDesign::Crossed,
                        128,
                        0.05,
                        &mut rng,
                    )
                    .map_err(|e| StepError::new(format!("bootstrap estimate: {e}")))?,
                );
                Ok(())
            },
        )
        .step("sigma_p exceeds sigma_i", |w, _| {
            let r = result(w)?;
            if r.sigma_p <= r.sigma_i {
                return Err(StepError::new("sigma_p <= sigma_i"));
            }
            Ok(())
        })
        .step("sigma_i exceeds sigma_r", |w, _| {
            let r = result(w)?;
            if r.sigma_i <= r.sigma_r {
                return Err(StepError::new("sigma_i <= sigma_r"));
            }
            Ok(())
        })
        .step("G exceeds Phi", |w, _| {
            let r = result(w)?;
            if r.g_coefficient <= r.phi_coefficient {
                return Err(StepError::new("G <= Phi"));
            }
            Ok(())
        })
        .step("G and Phi lie strictly between 0 and 1", |w, _| {
            let r = result(w)?;
            if !(r.g_coefficient > 0.0
                && r.g_coefficient < 1.0
                && r.phi_coefficient > 0.0
                && r.phi_coefficient < 1.0)
            {
                return Err(StepError::new("G/Phi out of bounds"));
            }
            Ok(())
        })
        .step(
            "bootstrap confidence intervals exist for every variance component",
            |w, _| {
                let r = result(w)?;
                let lows_ok = r
                    .variance_component_ci_low
                    .as_ref()
                    .is_some_and(|ci| ci.len() == 7);
                let highs_ok = r
                    .variance_component_ci_high
                    .as_ref()
                    .is_some_and(|ci| ci.len() == 7);
                if !lows_ok {
                    return Err(StepError::new("missing variance_component_ci_low"));
                }
                if !highs_ok {
                    return Err(StepError::new("missing variance_component_ci_high"));
                }
                Ok(())
            },
        )
        .step(
            "bootstrap confidence intervals exist for G and Phi",
            |w, _| {
                let r = result(w)?;
                if r.g_coefficient_ci_low.is_none()
                    || r.g_coefficient_ci_high.is_none()
                    || r.phi_coefficient_ci_low.is_none()
                    || r.phi_coefficient_ci_high.is_none()
                {
                    return Err(StepError::new("missing scalar CI"));
                }
                Ok(())
            },
        )
        .step(
            "the bootstrap metadata records 128 resamples at alpha 0.05",
            |w, _| {
                let r = result(w)?;
                if r.bootstrap_iterations != Some(128) || r.bootstrap_alpha != Some(0.05) {
                    return Err(StepError::new("wrong bootstrap metadata"));
                }
                Ok(())
            },
        )
        .step(
            "the bootstrap metadata records skipped resamples",
            |w, _| {
                let r = result(w)?;
                if r.bootstrap_skipped.is_none() {
                    return Err(StepError::new("missing bootstrap_skipped"));
                }
                Ok(())
            },
        );
    let report = runner.run(&feature);
    report.assert_all_passed();
}
