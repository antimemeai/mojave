//! TCK harness for `estimate_borgonovo_delta` — Ishigami headline
//! scenarios.
//!
//! Wires `tck/salib/borgonovo-estimator/features/borgonovo_ishigami.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! ADR: `decisions/2026-04-29-saltelli-borgonovo-delta.md`.

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
use salib_estimators::{estimate_borgonovo_delta, BorgonovoIndices};
use salib_samplers::{LhsSampler, Sampler};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("borgonovo-estimator")
        .join("features")
        .join("borgonovo_ishigami.feature")
}

fn build_inputs(n: usize) -> (Array2<f64>, Vec<f64>) {
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let unit = LhsSampler::classic(3).unit_sample(n, &mut rng);
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
    estimate: Option<BorgonovoIndices>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("n", &self.n)
            .field("has_estimate", &self.estimate.is_some())
            .finish_non_exhaustive()
    }
}

fn require(w: &World) -> Result<&BorgonovoIndices, StepError> {
    w.estimate
        .as_ref()
        .ok_or_else(|| StepError::new("no estimate"))
}

#[allow(clippy::too_many_lines)]
#[test]
fn borgonovo_ishigami_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "borgonovo_ishigami.feature").expect("parses cleanly");

    let runner = SyncRunner::new(World::default)
        .step("the Ishigami model on Uniform[-π, π]³", |_w, _| Ok(()))
        .step("LHS samples at N=4096", |w, _| {
            w.n = 4096;
            Ok(())
        })
        .step("I estimate Borgonovo delta", |w, _| {
            let (x, y) = build_inputs(w.n);
            w.estimate = Some(
                estimate_borgonovo_delta(&x, &y)
                    .map_err(|e| StepError::new(format!("estimate: {e}")))?,
            );
            Ok(())
        })
        .step("δ approximates 0.214 0.371 0.157 within 0.06", |w, _| {
            let est = require(w)?;
            let want = [0.214_f64, 0.371, 0.157];
            for (i, &v) in want.iter().enumerate() {
                let err = (est.delta[i] - v).abs();
                if err > 0.06 {
                    return Err(StepError::new(format!(
                        "δ_{i}: got {:.4}, want {v}, err {err:.4}",
                        est.delta[i]
                    )));
                }
            }
            Ok(())
        })
        .step("every δ is within negative 0.05 to 1.05", |w, _| {
            let est = require(w)?;
            for (i, &v) in est.delta.iter().enumerate() {
                if !(-0.05..=1.05).contains(&v) {
                    return Err(StepError::new(format!("δ_{i} = {v} out of [-0.05, 1.05]")));
                }
            }
            Ok(())
        })
        .step("δ_2 strictly exceeds δ_1", |w, _| {
            let est = require(w)?;
            if est.delta[1] <= est.delta[0] {
                return Err(StepError::new(format!(
                    "δ_2 = {} should exceed δ_1 = {}",
                    est.delta[1], est.delta[0]
                )));
            }
            Ok(())
        })
        .step("δ_1 strictly exceeds δ_3", |w, _| {
            let est = require(w)?;
            if est.delta[0] <= est.delta[2] {
                return Err(StepError::new(format!(
                    "δ_1 = {} should exceed δ_3 = {}",
                    est.delta[0], est.delta[2]
                )));
            }
            Ok(())
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
