//! TCK harness for `estimate_pawn` — Ishigami headline scenarios.
//!
//! Wires `tck/salib/pawn-estimator/features/pawn_ishigami.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! ADR: `decisions/2026-04-29-saltelli-pawn.md`.

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
use salib_estimators::{estimate_pawn, PawnIndices};
use salib_samplers::{LhsSampler, Sampler};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("pawn-estimator")
        .join("features")
        .join("pawn_ishigami.feature")
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
    s: usize,
    estimate: Option<PawnIndices>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("n", &self.n)
            .field("s", &self.s)
            .field("has_estimate", &self.estimate.is_some())
            .finish_non_exhaustive()
    }
}

fn require(w: &World) -> Result<&PawnIndices, StepError> {
    w.estimate
        .as_ref()
        .ok_or_else(|| StepError::new("no estimate"))
}

#[allow(clippy::too_many_lines)]
#[test]
fn pawn_ishigami_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "pawn_ishigami.feature").expect("parses cleanly");

    let runner = SyncRunner::new(World::default)
        .step("the Ishigami model on Uniform[-π, π]³", |_w, _| Ok(()))
        .step("LHS samples at N=4096 with S=10 slices", |w, _| {
            w.n = 4096;
            w.s = 10;
            Ok(())
        })
        .step("I estimate PAWN", |w, _| {
            let (x, y) = build_inputs(w.n);
            w.estimate = Some(
                estimate_pawn(&x, &y, w.s).map_err(|e| StepError::new(format!("estimate: {e}")))?,
            );
            Ok(())
        })
        .step("every median is in 0 to 1", |w, _| {
            let est = require(w)?;
            for (i, &v) in est.median.iter().enumerate() {
                if !(0.0..=1.0).contains(&v) {
                    return Err(StepError::new(format!("median_{i} = {v} out of [0, 1]")));
                }
            }
            Ok(())
        })
        .step("every max is in 0 to 1", |w, _| {
            let est = require(w)?;
            for (i, &v) in est.maximum.iter().enumerate() {
                if !(0.0..=1.0).contains(&v) {
                    return Err(StepError::new(format!("max_{i} = {v} out of [0, 1]")));
                }
            }
            Ok(())
        })
        .step("for every factor min is at most median", |w, _| {
            let est = require(w)?;
            for i in 0..est.d() {
                if est.minimum[i] > est.median[i] + 1e-12 {
                    return Err(StepError::new(format!(
                        "factor {i}: min {} > median {}",
                        est.minimum[i], est.median[i]
                    )));
                }
            }
            Ok(())
        })
        .step("for every factor median is at most max", |w, _| {
            let est = require(w)?;
            for i in 0..est.d() {
                if est.median[i] > est.maximum[i] + 1e-12 {
                    return Err(StepError::new(format!(
                        "factor {i}: median {} > max {}",
                        est.median[i], est.maximum[i]
                    )));
                }
            }
            Ok(())
        })
        .step("median_2 strictly exceeds median_1", |w, _| {
            let est = require(w)?;
            if est.median[1] <= est.median[0] {
                return Err(StepError::new(format!(
                    "median_2 = {} should exceed median_1 = {}",
                    est.median[1], est.median[0]
                )));
            }
            Ok(())
        })
        .step("median_1 strictly exceeds median_3", |w, _| {
            let est = require(w)?;
            if est.median[0] <= est.median[2] {
                return Err(StepError::new(format!(
                    "median_1 = {} should exceed median_3 = {}",
                    est.median[0], est.median[2]
                )));
            }
            Ok(())
        })
        .step(
            "median is within 0.05 of SALib's frozen reference",
            |w, _| {
                let est = require(w)?;
                let salib = [0.245_f64, 0.393, 0.087];
                for i in 0..3 {
                    let d = (est.median[i] - salib[i]).abs();
                    if d > 0.05 {
                        return Err(StepError::new(format!(
                            "median_{i}: ours {} SALib {} diff {d:.4}",
                            est.median[i], salib[i]
                        )));
                    }
                }
                Ok(())
            },
        );

    let report = runner.run(&feature);
    report.assert_all_passed();
}
