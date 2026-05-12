//! TCK harness for Phase D PR 15: alternative first-order Sobol'
//! estimators (Janon, Jansen, Owen).
//!
//! Wires `tck/salib/phase-d-efficiency/features/efficiency_ishigami.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! ADR: `decisions/2026-04-29-saltelli-phase-d-pr15.md`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::needless_range_loop,
    clippy::cast_precision_loss
)]

use std::f64::consts::PI;
use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use salib_core::RngState;
use salib_estimators::{estimate_janon, estimate_jansen, estimate_owen, estimate_saltelli2010};
use salib_samplers::{
    build_owen_matrix, build_saltelli_matrix, LhsSampler, OwenMatrix, SaltelliMatrix,
};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("phase-d-efficiency")
        .join("features")
        .join("efficiency_ishigami.feature")
}

fn ishigami_model(x: &[f64]) -> f64 {
    let mapped = [
        -PI + x[0] * 2.0 * PI,
        -PI + x[1] * 2.0 * PI,
        -PI + x[2] * 2.0 * PI,
    ];
    ishigami::ishigami(&mapped)
}

#[derive(Default)]
struct World {
    n: usize,
    saltelli_matrix: Option<SaltelliMatrix>,
    owen_matrix: Option<OwenMatrix>,
    s_estimate: Option<Vec<f64>>,
    s_saltelli: Option<Vec<f64>>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("n", &self.n)
            .field("has_saltelli", &self.saltelli_matrix.is_some())
            .field("has_owen", &self.owen_matrix.is_some())
            .field("has_estimate", &self.s_estimate.is_some())
            .finish_non_exhaustive()
    }
}

fn require_estimate(w: &World) -> Result<&Vec<f64>, StepError> {
    w.s_estimate
        .as_ref()
        .ok_or_else(|| StepError::new("no estimate"))
}

#[allow(clippy::too_many_lines)]
#[test]
fn phase_d_pr15_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "efficiency_ishigami.feature").expect("parses cleanly");

    let runner = SyncRunner::new(World::default)
        .step("the Ishigami model on Uniform[-π, π]³", |_w, _| Ok(()))
        .step("a Saltelli matrix at N=4096", |w, _| {
            w.n = 4096;
            let s = LhsSampler::classic(6); // 2d
            let mut rng = RngState::from_seed(FIXTURE_SEED);
            w.saltelli_matrix = Some(
                build_saltelli_matrix(&s, w.n, false, &mut rng)
                    .map_err(|e| StepError::new(format!("saltelli matrix: {e}")))?,
            );
            Ok(())
        })
        .step("an Owen matrix at N=4096", |w, _| {
            w.n = 4096;
            let s = LhsSampler::classic(9); // 3d
            let mut rng = RngState::from_seed(FIXTURE_SEED);
            w.owen_matrix = Some(
                build_owen_matrix(&s, w.n, &mut rng)
                    .map_err(|e| StepError::new(format!("owen matrix: {e}")))?,
            );
            Ok(())
        })
        .step("I estimate Janon first-order", |w, _| {
            let m = w
                .saltelli_matrix
                .as_ref()
                .ok_or_else(|| StepError::new("no Saltelli matrix"))?;
            w.s_estimate = Some(estimate_janon(m, ishigami_model).first_order);
            Ok(())
        })
        .step("I estimate Jansen first-order", |w, _| {
            let m = w
                .saltelli_matrix
                .as_ref()
                .ok_or_else(|| StepError::new("no Saltelli matrix"))?;
            w.s_estimate = Some(estimate_jansen(m, ishigami_model).first_order);
            Ok(())
        })
        .step("I estimate Owen first-order", |w, _| {
            let m = w
                .owen_matrix
                .as_ref()
                .ok_or_else(|| StepError::new("no Owen matrix"))?;
            w.s_estimate = Some(estimate_owen(m, ishigami_model).first_order);
            Ok(())
        })
        .step("I estimate both Saltelli2010 and Janon", |w, _| {
            let m = w
                .saltelli_matrix
                .as_ref()
                .ok_or_else(|| StepError::new("no Saltelli matrix"))?;
            w.s_saltelli = Some(estimate_saltelli2010(m, ishigami_model).first_order);
            w.s_estimate = Some(estimate_janon(m, ishigami_model).first_order);
            Ok(())
        })
        .step("S approximates 0.314 0.442 0.000 within 0.05", |w, _| {
            let est = require_estimate(w)?;
            let want = [0.314, 0.442, 0.000];
            for (i, &v) in want.iter().enumerate() {
                let err = (est[i] - v).abs();
                if err > 0.05 {
                    return Err(StepError::new(format!(
                        "S_{i}: got {:.4}, want {v}, err {err:.4}",
                        est[i]
                    )));
                }
            }
            Ok(())
        })
        .step(
            "Janon max-error does not exceed Saltelli2010 max-error",
            |w, _| {
                let janon = require_estimate(w)?;
                let saltelli = w
                    .s_saltelli
                    .as_ref()
                    .ok_or_else(|| StepError::new("no saltelli"))?;
                let analytic = [0.314, 0.442, 0.000];
                let janon_err = (0..3)
                    .map(|i| (janon[i] - analytic[i]).abs())
                    .fold(0.0, f64::max);
                let saltelli_err = (0..3)
                    .map(|i| (saltelli[i] - analytic[i]).abs())
                    .fold(0.0, f64::max);
                if janon_err > saltelli_err + 1e-9 {
                    return Err(StepError::new(format!(
                        "Janon err {janon_err:.4} exceeds Saltelli err {saltelli_err:.4}"
                    )));
                }
                Ok(())
            },
        )
        .step("S for factor 2 has magnitude below 0.05", |w, _| {
            let est = require_estimate(w)?;
            if est[2].abs() >= 0.05 {
                return Err(StepError::new(format!(
                    "S_3 = {:.4} should be < 0.05 in magnitude",
                    est[2]
                )));
            }
            Ok(())
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
