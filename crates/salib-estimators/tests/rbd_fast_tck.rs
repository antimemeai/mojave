//! TCK harness for `estimate_rbd_fast` — Ishigami headline scenarios.
//!
//! Wires `tck/salib/rbd-fast-estimator/features/rbd_fast_ishigami.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! ADR: `decisions/2026-04-29-saltelli-rbd-fast.md`.

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
use ndarray::Array2;
use salib_core::RngState;
use salib_estimators::{estimate_rbd_fast, RbdFastIndices};
use salib_samplers::{LhsSampler, Sampler};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("rbd-fast-estimator")
        .join("features")
        .join("rbd_fast_ishigami.feature")
}

fn build_inputs(n: usize) -> (Array2<f64>, Vec<f64>) {
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let sampler = LhsSampler::classic(3);
    let unit = sampler.unit_sample(n, &mut rng);
    let mut x = Array2::<f64>::zeros((n, 3));
    for i in 0..n {
        for j in 0..3 {
            x[[i, j]] = -PI + 2.0 * PI * unit[[i, j]];
        }
    }
    let y: Vec<f64> = (0..n)
        .map(|i| ishigami::ishigami(&[x[[i, 0]], x[[i, 1]], x[[i, 2]]]))
        .collect();
    (x, y)
}

#[derive(Default)]
struct World {
    n: usize,
    m: u32,
    estimate: Option<RbdFastIndices>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("n", &self.n)
            .field("m", &self.m)
            .field("has_estimate", &self.estimate.is_some())
            .finish_non_exhaustive()
    }
}

fn require_estimate(w: &World) -> Result<&RbdFastIndices, StepError> {
    w.estimate
        .as_ref()
        .ok_or_else(|| StepError::new("no estimate; check When step"))
}

#[allow(clippy::too_many_lines)]
#[test]
fn rbd_fast_ishigami_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "rbd_fast_ishigami.feature").expect("parses cleanly");

    let runner = SyncRunner::new(World::default)
        .step("the Ishigami model on Uniform[-π, π]³", |_w, _| Ok(()))
        .step("LHS samples at N=4096 with harmonic M=10", |w, _| {
            w.n = 4096;
            w.m = 10;
            Ok(())
        })
        .step("I estimate RBD-FAST first-order indices", |w, _| {
            let (x, y) = build_inputs(w.n);
            w.estimate = Some(
                estimate_rbd_fast(&x, &y, w.m)
                    .map_err(|e| StepError::new(format!("estimate: {e}")))?,
            );
            Ok(())
        })
        .step("S approximates 0.314 0.442 0.000 within 0.06", |w, _| {
            let est = require_estimate(w)?;
            let want = [0.314_f64, 0.442, 0.000];
            for (i, &w_v) in want.iter().enumerate() {
                let err = (est.s[i] - w_v).abs();
                if err > 0.06 {
                    return Err(StepError::new(format!(
                        "S_{i}: got {:.4}, want {w_v}, err {err:.4}",
                        est.s[i]
                    )));
                }
            }
            Ok(())
        })
        .step("every S is within negative 0.05 to 1.05", |w, _| {
            let est = require_estimate(w)?;
            for (i, &v) in est.s.iter().enumerate() {
                if !(-0.05..=1.05).contains(&v) {
                    return Err(StepError::new(format!("S_{i} = {v} out of [-0.05, 1.05]")));
                }
            }
            Ok(())
        })
        .step("S_2 strictly exceeds S_1", |w, _| {
            let est = require_estimate(w)?;
            if est.s[1] <= est.s[0] {
                return Err(StepError::new(format!(
                    "S_2 = {} should exceed S_1 = {}",
                    est.s[1], est.s[0]
                )));
            }
            Ok(())
        })
        .step("S_1 strictly exceeds S_3", |w, _| {
            let est = require_estimate(w)?;
            if est.s[0] <= est.s[2] {
                return Err(StepError::new(format!(
                    "S_1 = {} should exceed S_3 = {}",
                    est.s[0], est.s[2]
                )));
            }
            Ok(())
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
