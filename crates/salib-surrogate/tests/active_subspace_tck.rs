//! TCK harness for `compute_active_subspace` — ridge function +
//! Ishigami spectrum scenarios.
//!
//! Wires `tck/salib/active-subspace/features/active_subspace.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! ADR: `decisions/2026-04-29-saltelli-active-subspace.md`.

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
use salib_estimators::{finite_difference_gradients, FdKind};
use salib_samplers::{LhsSampler, Sampler};
use salib_surrogate::{compute_active_subspace, ActiveSubspace};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("active-subspace")
        .join("features")
        .join("active_subspace.feature")
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Model {
    Ridge,
    Ishigami,
    Polynomial,
}

#[derive(Default)]
struct World {
    model: Option<Model>,
    n: usize,
    lo: f64,
    hi: f64,
    gradients: Option<Array2<f64>>,
    result: Option<ActiveSubspace>,
    result_b: Option<ActiveSubspace>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("model", &self.model)
            .field("n", &self.n)
            .finish_non_exhaustive()
    }
}

fn lhs_inputs(n: usize, d: usize, lo: f64, hi: f64) -> Array2<f64> {
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let unit = LhsSampler::classic(d).unit_sample(n, &mut rng);
    let mut x = Array2::<f64>::zeros((n, d));
    for i in 0..n {
        for j in 0..d {
            x[[i, j]] = lo + (hi - lo) * unit[[i, j]];
        }
    }
    x
}

fn require_result(w: &World) -> Result<&ActiveSubspace, StepError> {
    w.result.as_ref().ok_or_else(|| StepError::new("no result"))
}

#[allow(clippy::too_many_lines)]
#[test]
fn active_subspace_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "active_subspace.feature").expect("parses");

    let runner = SyncRunner::new(World::default)
        .step(
            "the ridge model f(x) = 3·x_0 + 4·x_2 on Uniform[-1, 1]³",
            |w, _| {
                w.model = Some(Model::Ridge);
                w.lo = -1.0;
                w.hi = 1.0;
                Ok(())
            },
        )
        .step("the Ishigami model on Uniform[-π, π]³", |w, _| {
            w.model = Some(Model::Ishigami);
            w.lo = -PI;
            w.hi = PI;
            Ok(())
        })
        .step(
            "the model f(x) = x_0² + 2·x_1 + sin(x_2) on Uniform[-1, 1]³",
            |w, _| {
                w.model = Some(Model::Polynomial);
                w.lo = -1.0;
                w.hi = 1.0;
                w.n = 64;
                let x = lhs_inputs(w.n, 3, w.lo, w.hi);
                let g = finite_difference_gradients(&x, 1e-6, FdKind::Central, |xs: &[f64]| {
                    xs[0] * xs[0] + 2.0 * xs[1] + xs[2].sin()
                });
                w.gradients = Some(g);
                Ok(())
            },
        )
        .step(
            "I compute finite-difference gradients at N=32 LHS samples",
            |w, _| {
                w.n = 32;
                let x = lhs_inputs(w.n, 3, w.lo, w.hi);
                let model = w.model.ok_or_else(|| StepError::new("no model"))?;
                let g = match model {
                    Model::Ridge => {
                        finite_difference_gradients(&x, 1e-6, FdKind::Central, |xs: &[f64]| {
                            3.0 * xs[0] + 4.0 * xs[2]
                        })
                    }
                    Model::Ishigami => {
                        finite_difference_gradients(&x, 1e-5, FdKind::Central, |xs: &[f64]| {
                            ishigami::ishigami(xs)
                        })
                    }
                    Model::Polynomial => {
                        return Err(StepError::new("polynomial uses different fixture"));
                    }
                };
                w.gradients = Some(g);
                Ok(())
            },
        )
        .step(
            "I compute finite-difference gradients at N=256 LHS samples",
            |w, _| {
                w.n = 256;
                let x = lhs_inputs(w.n, 3, w.lo, w.hi);
                let model = w.model.ok_or_else(|| StepError::new("no model"))?;
                let g = match model {
                    Model::Ridge => {
                        finite_difference_gradients(&x, 1e-6, FdKind::Central, |xs: &[f64]| {
                            3.0 * xs[0] + 4.0 * xs[2]
                        })
                    }
                    Model::Ishigami => {
                        finite_difference_gradients(&x, 1e-5, FdKind::Central, |xs: &[f64]| {
                            ishigami::ishigami(xs)
                        })
                    }
                    Model::Polynomial => {
                        return Err(StepError::new("polynomial uses different fixture"));
                    }
                };
                w.gradients = Some(g);
                Ok(())
            },
        )
        .step("I compute the active subspace", |w, _| {
            let g = w
                .gradients
                .as_ref()
                .ok_or_else(|| StepError::new("no gradients"))?;
            w.result = Some(
                compute_active_subspace(g, None)
                    .map_err(|e| StepError::new(format!("active-subspace: {e}")))?,
            );
            Ok(())
        })
        .step(
            "I compute the active subspace twice on the same gradient samples",
            |w, _| {
                let g = w
                    .gradients
                    .as_ref()
                    .ok_or_else(|| StepError::new("no gradients"))?;
                w.result = Some(compute_active_subspace(g, None).expect("a"));
                w.result_b = Some(compute_active_subspace(g, None).expect("b"));
                Ok(())
            },
        )
        .step(
            "the leading eigenvalue approximates 25 within 1e-6",
            |w, _| {
                let r = require_result(w)?;
                if (r.eigenvalues[0] - 25.0).abs() >= 1e-6 {
                    return Err(StepError::new(format!("λ_1 = {}", r.eigenvalues[0])));
                }
                Ok(())
            },
        )
        .step(
            "the second and third eigenvalues are at most 1e-6",
            |w, _| {
                let r = require_result(w)?;
                if r.eigenvalues[1].abs() > 1e-6 || r.eigenvalues[2].abs() > 1e-6 {
                    return Err(StepError::new(format!(
                        "λ_2 = {}, λ_3 = {}",
                        r.eigenvalues[1], r.eigenvalues[2]
                    )));
                }
                Ok(())
            },
        )
        .step(
            "the leading eigenvector is aligned with a/||a|| up to sign",
            |w, _| {
                let r = require_result(w)?;
                let v: Vec<f64> = (0..3).map(|i| r.eigenvectors[[i, 0]]).collect();
                let dot = 3.0 * v[0] + 4.0 * v[2];
                let alignment = dot.abs() / 5.0;
                if (alignment - 1.0).abs() >= 1e-6 {
                    return Err(StepError::new(format!("alignment = {alignment}")));
                }
                Ok(())
            },
        )
        .step("k_active equals 1", |w, _| {
            let r = require_result(w)?;
            if r.k_active != 1 {
                return Err(StepError::new(format!("k_active = {}", r.k_active)));
            }
            Ok(())
        })
        .step("all three eigenvalues are strictly positive", |w, _| {
            let r = require_result(w)?;
            for (i, &lambda) in r.eigenvalues.iter().enumerate() {
                if lambda <= 1e-3 {
                    return Err(StepError::new(format!("λ_{i} = {lambda}")));
                }
            }
            Ok(())
        })
        .step(
            "the leading eigenvalue approximates 24.5 within 3.0",
            |w, _| {
                let r = require_result(w)?;
                if (r.eigenvalues[0] - 24.5).abs() >= 3.0 {
                    return Err(StepError::new(format!("λ_1 = {}", r.eigenvalues[0])));
                }
                Ok(())
            },
        )
        .step(
            "the leading eigenvector is X_2-aligned with magnitude at least 0.95",
            |w, _| {
                let r = require_result(w)?;
                if r.eigenvectors[[1, 0]].abs() < 0.95 {
                    return Err(StepError::new(format!(
                        "v_1[1] = {}",
                        r.eigenvectors[[1, 0]].abs()
                    )));
                }
                Ok(())
            },
        )
        .step("the two eigendecompositions are bit-identical", |w, _| {
            let a = require_result(w)?;
            let b = w
                .result_b
                .as_ref()
                .ok_or_else(|| StepError::new("no second result"))?;
            if a.eigenvalues != b.eigenvalues {
                return Err(StepError::new("eigenvalues differ"));
            }
            for col in 0..a.eigenvalues.len() {
                for row in 0..a.eigenvalues.len() {
                    if a.eigenvectors[[row, col]] != b.eigenvectors[[row, col]] {
                        return Err(StepError::new("eigenvectors differ"));
                    }
                }
            }
            Ok(())
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
