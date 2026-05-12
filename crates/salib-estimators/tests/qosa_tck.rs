//! TCK harness for `estimate_qosa` — Ishigami + sanity + tail-vs-
//! median scenarios.
//!
//! Wires `tck/salib/qosa-estimator/features/qosa.feature` against
//! [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! ADR: `decisions/2026-04-29-saltelli-qosa.md`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::cast_precision_loss
)]

use std::f64::consts::PI;
use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use ndarray::Array2;
use salib_core::RngState;
use salib_estimators::{estimate_qosa, QosaIndices};
use salib_samplers::{LhsSampler, Sampler};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("qosa-estimator")
        .join("features")
        .join("qosa.feature")
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Model {
    Ishigami,
    YEqualsX0,
    GatedTail,
}

#[derive(Default)]
struct World {
    model: Option<Model>,
    n: usize,
    result: Option<QosaIndices>,
    result_b: Option<QosaIndices>,
    median_s: Option<Vec<f64>>,
    tail_s: Option<Vec<f64>>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("model", &self.model)
            .field("n", &self.n)
            .finish_non_exhaustive()
    }
}

fn build_inputs(model: Model, n: usize) -> (Array2<f64>, Vec<f64>) {
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let unit = LhsSampler::classic(3).unit_sample(n, &mut rng);
    match model {
        Model::Ishigami => {
            let mut x = Array2::<f64>::zeros((n, 3));
            for i in 0..n {
                for j in 0..3 {
                    x[[i, j]] = -PI + 2.0 * PI * unit[[i, j]];
                }
            }
            let y: Vec<f64> = (0..n)
                .map(|k| ishigami::ishigami(&[x[[k, 0]], x[[k, 1]], x[[k, 2]]]))
                .collect();
            (x, y)
        }
        Model::YEqualsX0 => {
            let y: Vec<f64> = (0..n).map(|k| unit[[k, 0]]).collect();
            (unit, y)
        }
        Model::GatedTail => {
            let y: Vec<f64> = (0..n)
                .map(|k| {
                    let base = unit[[k, 0]];
                    let tail = if unit[[k, 2]] > 0.95 {
                        8.0 * unit[[k, 1]]
                    } else {
                        0.0
                    };
                    base + tail
                })
                .collect();
            (unit, y)
        }
    }
}

fn require(w: &World) -> Result<&QosaIndices, StepError> {
    w.result.as_ref().ok_or_else(|| StepError::new("no result"))
}

#[allow(clippy::too_many_lines)]
#[test]
fn qosa_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "qosa.feature").expect("parses");

    let runner = SyncRunner::new(World::default)
        .step("the Ishigami model on Uniform[-π, π]³", |w, _| {
            w.model = Some(Model::Ishigami);
            Ok(())
        })
        .step("the Y = X_0 model on Uniform[0, 1]³", |w, _| {
            w.model = Some(Model::YEqualsX0);
            Ok(())
        })
        .step(
            "the gated tail model Y = X_0 + 8·X_1·1{X_2 > 0.95}",
            |w, _| {
                w.model = Some(Model::GatedTail);
                Ok(())
            },
        )
        .step("LHS samples at N=4096", |w, _| {
            w.n = 4096;
            Ok(())
        })
        .step("LHS samples at N=1024", |w, _| {
            w.n = 1024;
            Ok(())
        })
        .step("I estimate QOSA at α=0.5", |w, _| {
            let model = w.model.ok_or_else(|| StepError::new("no model"))?;
            let (x, y) = build_inputs(model, w.n);
            w.result =
                Some(estimate_qosa(&x, &y, 0.5).map_err(|e| StepError::new(format!("QOSA: {e}")))?);
            Ok(())
        })
        .step("I record S as median_S", |w, _| {
            w.median_s = Some(require(w)?.s.clone());
            Ok(())
        })
        .step("I estimate QOSA at α=0.95", |w, _| {
            let model = w.model.ok_or_else(|| StepError::new("no model"))?;
            let (x, y) = build_inputs(model, w.n);
            w.result = Some(
                estimate_qosa(&x, &y, 0.95).map_err(|e| StepError::new(format!("QOSA: {e}")))?,
            );
            Ok(())
        })
        .step("I record S as tail_S", |w, _| {
            w.tail_s = Some(require(w)?.s.clone());
            Ok(())
        })
        .step("I estimate QOSA at α=0.75 twice", |w, _| {
            let model = w.model.ok_or_else(|| StepError::new("no model"))?;
            let (x, y) = build_inputs(model, w.n);
            w.result = Some(estimate_qosa(&x, &y, 0.75).expect("a"));
            w.result_b = Some(estimate_qosa(&x, &y, 0.75).expect("b"));
            Ok(())
        })
        .step("S^α_2 exceeds S^α_1", |w, _| {
            let r = require(w)?;
            if r.s[1] <= r.s[0] {
                return Err(StepError::new(format!(
                    "S^α_2 ({:.3}) ≤ S^α_1 ({:.3})",
                    r.s[1], r.s[0]
                )));
            }
            Ok(())
        })
        .step("S^α_1 exceeds S^α_3", |w, _| {
            let r = require(w)?;
            if r.s[0] <= r.s[2] {
                return Err(StepError::new(format!(
                    "S^α_1 ({:.3}) ≤ S^α_3 ({:.3})",
                    r.s[0], r.s[2]
                )));
            }
            Ok(())
        })
        .step("S^α_3 is below 0.2", |w, _| {
            let r = require(w)?;
            if r.s[2] >= 0.2 {
                return Err(StepError::new(format!("S^α_3 = {:.3}", r.s[2])));
            }
            Ok(())
        })
        .step("S^α_1 is below 0.1", |w, _| {
            let r = require(w)?;
            if r.s[1] >= 0.1 {
                return Err(StepError::new(format!("S^α_1 = {:.3}", r.s[1])));
            }
            Ok(())
        })
        .step("S^α_2 is below 0.1", |w, _| {
            let r = require(w)?;
            if r.s[2] >= 0.1 {
                return Err(StepError::new(format!("S^α_2 = {:.3}", r.s[2])));
            }
            Ok(())
        })
        .step("S^α_0 exceeds 0.3", |w, _| {
            let r = require(w)?;
            if r.s[0] <= 0.3 {
                return Err(StepError::new(format!("S^α_0 = {:.3}", r.s[0])));
            }
            Ok(())
        })
        .step(
            "median_S[0] dominates median_S[1] and median_S[2]",
            |w, _| {
                let m = w
                    .median_s
                    .as_ref()
                    .ok_or_else(|| StepError::new("no median_S"))?;
                if !(m[0] > m[1] && m[0] > m[2]) {
                    return Err(StepError::new(format!("median ordering wrong: S = {m:?}")));
                }
                Ok(())
            },
        )
        .step("tail_S[2] exceeds tail_S[0]", |w, _| {
            let t = w
                .tail_s
                .as_ref()
                .ok_or_else(|| StepError::new("no tail_S"))?;
            if t[2] <= t[0] {
                return Err(StepError::new(format!(
                    "tail S_2 ({:.3}) ≤ S_0 ({:.3})",
                    t[2], t[0]
                )));
            }
            Ok(())
        })
        .step("tail_S[2] exceeds median_S[2]", |w, _| {
            let t = w
                .tail_s
                .as_ref()
                .ok_or_else(|| StepError::new("no tail_S"))?;
            let m = w
                .median_s
                .as_ref()
                .ok_or_else(|| StepError::new("no median_S"))?;
            if t[2] <= m[2] {
                return Err(StepError::new(format!(
                    "tail S_2 ({:.3}) should exceed median S_2 ({:.3})",
                    t[2], m[2]
                )));
            }
            Ok(())
        })
        .step("the two index sets are bit-identical", |w, _| {
            let a = require(w)?;
            let b = w
                .result_b
                .as_ref()
                .ok_or_else(|| StepError::new("no second result"))?;
            if a.s != b.s {
                return Err(StepError::new("s differs"));
            }
            Ok(())
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
