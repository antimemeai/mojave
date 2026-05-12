//! TCK harness for `estimate_given_data_sobol` — Ishigami headline
//! scenarios.
//!
//! Wires `tck/salib/given-data-sobol-estimator/features/given_data_sobol_ishigami.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! ADR: `decisions/2026-04-29-saltelli-given-data-sobol.md`.

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
use salib_estimators::{estimate_given_data_sobol, GivenDataSobolIndices};
use salib_samplers::{LhsSampler, Sampler};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("given-data-sobol-estimator")
        .join("features")
        .join("given_data_sobol_ishigami.feature")
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
    estimate: Option<GivenDataSobolIndices>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("n", &self.n)
            .field("has_estimate", &self.estimate.is_some())
            .finish_non_exhaustive()
    }
}

fn require(w: &World) -> Result<&GivenDataSobolIndices, StepError> {
    w.estimate
        .as_ref()
        .ok_or_else(|| StepError::new("no estimate"))
}

#[allow(clippy::too_many_lines)]
#[test]
fn given_data_sobol_ishigami_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature =
        parse_feature(&content, "given_data_sobol_ishigami.feature").expect("parses cleanly");

    let runner = SyncRunner::new(World::default)
        .step("the Ishigami model on Uniform[-π, π]³", |_w, _| Ok(()))
        .step("LHS samples at N=4096", |w, _| {
            w.n = 4096;
            Ok(())
        })
        .step("I estimate given-data Sobol indices", |w, _| {
            let (x, y) = build_inputs(w.n);
            w.estimate = Some(
                estimate_given_data_sobol(&x, &y)
                    .map_err(|e| StepError::new(format!("estimate: {e}")))?,
            );
            Ok(())
        })
        .step("S_1 approximates 0.314 0.442 0.000 within 0.03", |w, _| {
            let est = require(w)?;
            let want = [0.314_f64, 0.442, 0.000];
            for (i, &v) in want.iter().enumerate() {
                let err = (est.s1[i] - v).abs();
                if err > 0.03 {
                    return Err(StepError::new(format!(
                        "S_1[{i}]: got {:.4}, want {v}, err {err:.4}",
                        est.s1[i]
                    )));
                }
            }
            Ok(())
        })
        .step("every S_1 is in 0 to 1", |w, _| {
            let est = require(w)?;
            for (i, &v) in est.s1.iter().enumerate() {
                if !(0.0..=1.0).contains(&v) {
                    return Err(StepError::new(format!("S_1[{i}] = {v} out of [0, 1]")));
                }
            }
            Ok(())
        })
        .step(
            "S_1 for factor 1 strictly exceeds S_1 for factor 0",
            |w, _| {
                let est = require(w)?;
                if est.s1[1] <= est.s1[0] {
                    return Err(StepError::new(format!(
                        "S_1[1] = {} should exceed S_1[0] = {}",
                        est.s1[1], est.s1[0]
                    )));
                }
                Ok(())
            },
        )
        .step(
            "S_1 for factor 0 strictly exceeds S_1 for factor 2",
            |w, _| {
                let est = require(w)?;
                if est.s1[0] <= est.s1[2] {
                    return Err(StepError::new(format!(
                        "S_1[0] = {} should exceed S_1[2] = {}",
                        est.s1[0], est.s1[2]
                    )));
                }
                Ok(())
            },
        );

    let report = runner.run(&feature);
    report.assert_all_passed();
}
