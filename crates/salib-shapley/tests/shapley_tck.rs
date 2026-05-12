//! TCK harness for `estimate_shapley` — Ishigami at canonical
//! `(a=7, b=0.1)`.
//!
//! Wires `tck/salib/shapley-estimator/features/shapley_ishigami.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! ADR: `decisions/2026-04-29-salib-shapley.md`.

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
use salib_core::{Distribution, RngState};
use salib_shapley::{estimate_shapley, ShapleyIndices};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("shapley-estimator")
        .join("features")
        .join("shapley_ishigami.feature")
}

fn ishigami_distributions() -> Vec<Distribution> {
    (0..3)
        .map(|_| Distribution::Uniform { lo: -PI, hi: PI })
        .collect()
}

fn fit(n_perm: usize, n_var: usize) -> ShapleyIndices {
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    estimate_shapley(
        &ishigami_distributions(),
        |x: &[f64]| ishigami::ishigami(x),
        n_perm,
        1,
        3,
        n_var,
        &mut rng,
    )
    .expect("shapley fit")
}

#[derive(Default)]
struct World {
    result: Option<ShapleyIndices>,
    result_b: Option<ShapleyIndices>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("has_result", &self.result.is_some())
            .finish_non_exhaustive()
    }
}

fn require(w: &World) -> Result<&ShapleyIndices, StepError> {
    w.result
        .as_ref()
        .ok_or_else(|| StepError::new("no shapley result"))
}

#[allow(clippy::too_many_lines)]
#[test]
fn shapley_ishigami_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "shapley_ishigami.feature").expect("parses");

    let runner = SyncRunner::new(World::default)
        .step("the Ishigami model on Uniform[-π, π]³", |_w, _| Ok(()))
        .step(
            "I estimate Shapley effects with m=2000, N_O=1, N_I=3, N_V=4000",
            |w, _| {
                w.result = Some(fit(2000, 4000));
                Ok(())
            },
        )
        .step(
            "I estimate Shapley effects with m=4000, N_O=1, N_I=3, N_V=8000",
            |w, _| {
                w.result = Some(fit(4000, 8000));
                Ok(())
            },
        )
        .step(
            "I estimate Shapley effects twice with m=64, N_O=1, N_I=3, N_V=256",
            |w, _| {
                w.result = Some(fit(64, 256));
                w.result_b = Some(fit(64, 256));
                Ok(())
            },
        )
        .step(
            "the sum of Shapley indices equals Var(Y) within 1e-9",
            |w, _| {
                let r = require(w)?;
                let sum: f64 = r.sh.iter().sum();
                if (sum - r.var_y).abs() >= 1e-9 {
                    return Err(StepError::new(format!(
                        "Σ Sh_i = {sum}, Var(Y) = {} (diff {})",
                        r.var_y,
                        sum - r.var_y
                    )));
                }
                Ok(())
            },
        )
        .step("Sh_1 approximates 6.0327 within 1.0", |w, _| {
            let r = require(w)?;
            if (r.sh[0] - 6.0327).abs() >= 1.0 {
                return Err(StepError::new(format!("Sh_1 = {}", r.sh[0])));
            }
            Ok(())
        })
        .step("Sh_2 approximates 6.1250 within 1.0", |w, _| {
            let r = require(w)?;
            if (r.sh[1] - 6.1250).abs() >= 1.0 {
                return Err(StepError::new(format!("Sh_2 = {}", r.sh[1])));
            }
            Ok(())
        })
        .step("Sh_3 approximates 1.6868 within 1.0", |w, _| {
            let r = require(w)?;
            if (r.sh[2] - 1.6868).abs() >= 1.0 {
                return Err(StepError::new(format!("Sh_3 = {}", r.sh[2])));
            }
            Ok(())
        })
        .step(
            "every Sh_i lies between V_i and V_T_i within MC slack 0.5",
            |w, _| {
                let r = require(w)?;
                let analytic = ishigami::analytic_indices(7.0, 0.1);
                let d = analytic.total_variance;
                for i in 0..3 {
                    let v_i = analytic.first_order[i] * d;
                    let v_t_i = analytic.total_order[i] * d;
                    if r.sh[i] < v_i - 0.5 {
                        return Err(StepError::new(format!(
                            "Sh_{i} = {} < V_{i} = {v_i} (lower bound)",
                            r.sh[i]
                        )));
                    }
                    if r.sh[i] > v_t_i + 0.5 {
                        return Err(StepError::new(format!(
                            "Sh_{i} = {} > V_T_{i} = {v_t_i} (upper bound)",
                            r.sh[i]
                        )));
                    }
                }
                Ok(())
            },
        )
        .step("the two index sets are bit-identical", |w, _| {
            let a = require(w)?;
            let b = w
                .result_b
                .as_ref()
                .ok_or_else(|| StepError::new("no second result"))?;
            if a.sh != b.sh {
                return Err(StepError::new("sh differs"));
            }
            if a.var_y != b.var_y {
                return Err(StepError::new("var_y differs"));
            }
            Ok(())
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
