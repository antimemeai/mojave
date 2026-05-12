//! TCK harness for `estimate_dgsm` — Ishigami headline scenarios.
//!
//! Wires `tck/salib/dgsm-estimator/features/dgsm_ishigami.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! ADR: `decisions/2026-04-29-saltelli-dgsm.md`.

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
use salib_core::{tree_var, Distribution, RngState};
use salib_estimators::{
    estimate_dgsm, finite_difference_gradients, poincare_constant, DgsmIndices, FdKind,
};
use salib_samplers::{LhsSampler, Sampler};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("dgsm-estimator")
        .join("features")
        .join("dgsm_ishigami.feature")
}

fn build_inputs(n: usize) -> (Array2<f64>, Vec<f64>, f64) {
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
    let var_y = tree_var(&y);
    (x, y, var_y)
}

fn cp_uniform_neg_pi_pi() -> [f64; 3] {
    let cp = poincare_constant(&Distribution::Uniform { lo: -PI, hi: PI }).unwrap();
    [cp, cp, cp]
}

#[derive(Default)]
struct World {
    n: usize,
    estimate: Option<DgsmIndices>,
    estimate_fd: Option<DgsmIndices>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("n", &self.n)
            .field("has_estimate", &self.estimate.is_some())
            .field("has_estimate_fd", &self.estimate_fd.is_some())
            .finish_non_exhaustive()
    }
}

fn require(w: &World) -> Result<&DgsmIndices, StepError> {
    w.estimate
        .as_ref()
        .ok_or_else(|| StepError::new("no estimate"))
}

#[allow(clippy::too_many_lines)]
#[test]
fn dgsm_ishigami_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "dgsm_ishigami.feature").expect("parses cleanly");

    let runner = SyncRunner::new(World::default)
        .step("the Ishigami model on Uniform[-π, π]³", |_w, _| Ok(()))
        .step("LHS samples at N=4096 with analytical gradients", |w, _| {
            w.n = 4096;
            let (x, _y, var_y) = build_inputs(w.n);
            let mut g = Array2::<f64>::zeros((w.n, 3));
            for k in 0..w.n {
                let grad = ishigami::ishigami_gradient(&[x[[k, 0]], x[[k, 1]], x[[k, 2]]]);
                for j in 0..3 {
                    g[[k, j]] = grad[j];
                }
            }
            let cp = cp_uniform_neg_pi_pi();
            w.estimate = Some(
                estimate_dgsm(&g, &cp, var_y)
                    .map_err(|e| StepError::new(format!("estimate: {e}")))?,
            );
            Ok(())
        })
        .step("LHS samples at N=4096", |w, _| {
            w.n = 4096;
            Ok(())
        })
        .step("I estimate DGSM", |_w, _| Ok(()))
        .step("I estimate DGSM with analytical gradients", |w, _| {
            let (x, _y, var_y) = build_inputs(w.n);
            let mut g = Array2::<f64>::zeros((w.n, 3));
            for k in 0..w.n {
                let grad = ishigami::ishigami_gradient(&[x[[k, 0]], x[[k, 1]], x[[k, 2]]]);
                for j in 0..3 {
                    g[[k, j]] = grad[j];
                }
            }
            let cp = cp_uniform_neg_pi_pi();
            w.estimate = Some(
                estimate_dgsm(&g, &cp, var_y)
                    .map_err(|e| StepError::new(format!("estimate: {e}")))?,
            );
            Ok(())
        })
        .step(
            "I estimate DGSM with central finite-difference at eps 1e-5",
            |w, _| {
                let (x, _y, var_y) = build_inputs(w.n);
                let g = finite_difference_gradients(&x, 1e-5, FdKind::Central, |xi: &[f64]| {
                    ishigami::ishigami(&[xi[0], xi[1], xi[2]])
                });
                let cp = cp_uniform_neg_pi_pi();
                w.estimate_fd = Some(
                    estimate_dgsm(&g, &cp, var_y)
                        .map_err(|e| StepError::new(format!("estimate fd: {e}")))?,
                );
                Ok(())
            },
        )
        .step("ν approximates 7.72 24.5 10.99 within 0.2", |w, _| {
            let est = require(w)?;
            let want = [7.72_f64, 24.5, 10.99];
            for (i, &v) in want.iter().enumerate() {
                let err = (est.vi[i] - v).abs();
                if err > 0.2 {
                    return Err(StepError::new(format!(
                        "ν_{i}: got {:.4}, want {v}, err {err:.4}",
                        est.vi[i]
                    )));
                }
            }
            Ok(())
        })
        .step(
            "ST analytic is at most ST upper for every factor",
            |w, _| {
                let est = require(w)?;
                let analytic = ishigami::analytic_indices(7.0, 0.1);
                for i in 0..3 {
                    if analytic.total_order[i] > est.st_upper[i] + 1e-9 {
                        return Err(StepError::new(format!(
                            "factor {i}: ST_analytic {:.4} > ST_upper {:.4}",
                            analytic.total_order[i], est.st_upper[i]
                        )));
                    }
                }
                Ok(())
            },
        )
        .step("the two ν vectors agree within 1e-5", |w, _| {
            let est = require(w)?;
            let est_fd = w
                .estimate_fd
                .as_ref()
                .ok_or_else(|| StepError::new("no estimate_fd"))?;
            for i in 0..3 {
                let d = (est.vi[i] - est_fd.vi[i]).abs();
                if d > 1e-5 {
                    return Err(StepError::new(format!(
                        "ν_{i}: analytical {} fd {} diff {d:.2e}",
                        est.vi[i], est_fd.vi[i]
                    )));
                }
            }
            Ok(())
        })
        .step("ν_2 strictly exceeds ν_3", |w, _| {
            let est = require(w)?;
            if est.vi[1] <= est.vi[2] {
                return Err(StepError::new(format!(
                    "ν_2 {} should exceed ν_3 {}",
                    est.vi[1], est.vi[2]
                )));
            }
            Ok(())
        })
        .step("ν_3 strictly exceeds ν_1", |w, _| {
            let est = require(w)?;
            if est.vi[2] <= est.vi[0] {
                return Err(StepError::new(format!(
                    "ν_3 {} should exceed ν_1 {}",
                    est.vi[2], est.vi[0]
                )));
            }
            Ok(())
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
