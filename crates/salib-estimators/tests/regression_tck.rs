//! TCK harness for `estimate_regression_indices` — Ishigami +
//! linear-fixture scenarios.
//!
//! Wires `tck/salib/regression-estimator/features/regression_ishigami.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! ADR: `decisions/2026-04-29-saltelli-regression.md`.

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
use salib_estimators::{estimate_regression_indices, RegressionIndices};
use salib_samplers::{LhsSampler, Sampler};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("regression-estimator")
        .join("features")
        .join("regression_ishigami.feature")
}

fn ishigami_inputs(n: usize) -> (Array2<f64>, Vec<f64>) {
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

fn linear_inputs(n: usize) -> (Array2<f64>, Vec<f64>) {
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let unit = LhsSampler::classic(3).unit_sample(n, &mut rng);
    let y: Vec<f64> = (0..n).map(|k| 2.0 * unit[[k, 0]] + unit[[k, 1]]).collect();
    (unit, y)
}

#[derive(Default)]
struct World {
    n: usize,
    using_linear: bool,
    estimate: Option<RegressionIndices>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("n", &self.n)
            .field("linear", &self.using_linear)
            .field("has_estimate", &self.estimate.is_some())
            .finish_non_exhaustive()
    }
}

fn require(w: &World) -> Result<&RegressionIndices, StepError> {
    w.estimate
        .as_ref()
        .ok_or_else(|| StepError::new("no estimate"))
}

#[allow(clippy::too_many_lines)]
#[test]
fn regression_ishigami_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "regression_ishigami.feature").expect("parses cleanly");

    let runner = SyncRunner::new(World::default)
        .step("the Ishigami model on Uniform[-π, π]³", |w, _| {
            w.using_linear = false;
            Ok(())
        })
        .step("the linear model Y = 2 X_0 + X_1", |w, _| {
            w.using_linear = true;
            Ok(())
        })
        .step("LHS samples at N=4096", |w, _| {
            w.n = 4096;
            Ok(())
        })
        .step("LHS samples at N=1024", |w, _| {
            w.n = 1024;
            Ok(())
        })
        .step("I estimate regression indices", |w, _| {
            let (x, y) = if w.using_linear {
                linear_inputs(w.n)
            } else {
                ishigami_inputs(w.n)
            };
            w.estimate = Some(
                estimate_regression_indices(&x, &y)
                    .map_err(|e| StepError::new(format!("estimate: {e}")))?,
            );
            Ok(())
        })
        .step("every SRC has magnitude at most 1", |w, _| {
            let est = require(w)?;
            for (i, &v) in est.src.iter().enumerate() {
                if v.abs() > 1.0 + 1e-9 {
                    return Err(StepError::new(format!("|SRC_{i}| = {} exceeds 1", v.abs())));
                }
            }
            Ok(())
        })
        .step("every PRCC has magnitude at most 1", |w, _| {
            let est = require(w)?;
            for (i, &v) in est.prcc.iter().enumerate() {
                if v.abs() > 1.0 + 1e-9 {
                    return Err(StepError::new(format!(
                        "|PRCC_{i}| = {} exceeds 1",
                        v.abs()
                    )));
                }
            }
            Ok(())
        })
        .step("R² linear is in 0 to 1", |w, _| {
            let est = require(w)?;
            if !(0.0..=1.0).contains(&est.r2_linear) {
                return Err(StepError::new(format!(
                    "R²_linear = {} out of [0, 1]",
                    est.r2_linear
                )));
            }
            Ok(())
        })
        .step("R² rank is in 0 to 1", |w, _| {
            let est = require(w)?;
            if !(0.0..=1.0).contains(&est.r2_rank) {
                return Err(StepError::new(format!(
                    "R²_rank = {} out of [0, 1]",
                    est.r2_rank
                )));
            }
            Ok(())
        })
        .step("R² linear is below 0.5", |w, _| {
            let est = require(w)?;
            if est.r2_linear >= 0.5 {
                return Err(StepError::new(format!(
                    "R²_linear = {:.4} should be < 0.5 (Ishigami non-linear)",
                    est.r2_linear
                )));
            }
            Ok(())
        })
        .step("SRRC for factor 1 has magnitude below 0.1", |w, _| {
            let est = require(w)?;
            if est.srrc[1].abs() >= 0.1 {
                return Err(StepError::new(format!(
                    "|SRRC_1| = {:.4} should be < 0.1 (sin² non-monotonic)",
                    est.srrc[1].abs()
                )));
            }
            Ok(())
        })
        .step("R² linear exceeds 0.99", |w, _| {
            let est = require(w)?;
            if est.r2_linear <= 0.99 {
                return Err(StepError::new(format!(
                    "R²_linear = {:.4} should be > 0.99",
                    est.r2_linear
                )));
            }
            Ok(())
        })
        .step(
            "the SRC ratio of factor 0 to factor 1 approximates 2 within 0.2",
            |w, _| {
                let est = require(w)?;
                let ratio = est.src[0].abs() / est.src[1].abs();
                if (ratio - 2.0).abs() > 0.2 {
                    return Err(StepError::new(format!(
                        "SRC ratio = {ratio:.4}, expected ≈ 2"
                    )));
                }
                Ok(())
            },
        );

    let report = runner.run(&feature);
    report.assert_all_passed();
}
