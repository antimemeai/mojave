//! TCK harness for Ishigami's closed-form Sobol' indices.
//!
//! Wires `tck/salib/validation/features/ishigami_analytic.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::approx_constant
)]

use std::f64::consts::PI;
use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use salib_core::{Distribution, Problem};
use salib_validation::{ishigami, SobolIndicesAnalytic};

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("validation")
        .join("features")
        .join("ishigami_analytic.feature")
}

#[derive(Default)]
struct World {
    indices: Option<SobolIndicesAnalytic>,
    distribution: Option<Problem>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("has_indices", &self.indices.is_some())
            .field("has_distribution", &self.distribution.is_some())
            .finish_non_exhaustive()
    }
}

fn require_indices(w: &World) -> Result<&SobolIndicesAnalytic, StepError> {
    w.indices
        .as_ref()
        .ok_or_else(|| StepError::new("no indices; check Given step"))
}

fn assert_close(got: f64, want: f64, tol: f64, ctx: &str) -> Result<(), StepError> {
    if (got - want).abs() <= tol {
        Ok(())
    } else {
        Err(StepError::new(format!(
            "{ctx}: got {got}, want {want} (tol {tol})"
        )))
    }
}

#[allow(clippy::too_many_lines)]
#[test]
fn ishigami_analytic_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "ishigami_analytic.feature")
        .expect("ishigami_analytic.feature parses cleanly");

    let runner = SyncRunner::new(World::default)
        // ── Givens ─────────────────────────────────────────────────
        .step(
            "Ishigami analytic indices at canonical (a=7, b=0.1)",
            |w, _| {
                w.indices = Some(ishigami::analytic_indices(7.0, 0.1));
                Ok(())
            },
        )
        .step("Ishigami analytic indices at (a=0, b=0.1)", |w, _| {
            w.indices = Some(ishigami::analytic_indices(0.0, 0.1));
            Ok(())
        })
        .step("Ishigami analytic indices at (a=7, b=0)", |w, _| {
            w.indices = Some(ishigami::analytic_indices(7.0, 0.0));
            Ok(())
        })
        .step("Ishigami analytic indices at (a=0, b=0)", |w, _| {
            w.indices = Some(ishigami::analytic_indices(0.0, 0.0));
            Ok(())
        })
        .step("the Ishigami input distribution", |w, _| {
            w.distribution = Some(ishigami::input_distribution());
            Ok(())
        })
        // ── Thens ─────────────────────────────────────────────────
        .step("the X_3 first-order index is exactly 0", |w, _| {
            let s = require_indices(w)?;
            if s.first_order[2] == 0.0 {
                Ok(())
            } else {
                Err(StepError::new(format!(
                    "S_3 = {} (expected 0)",
                    s.first_order[2]
                )))
            }
        })
        .step(
            "the X_2 total-order index equals the X_2 first-order index",
            |w, _| {
                let s = require_indices(w)?;
                assert_close(s.total_order[1], s.first_order[1], 1e-12, "S_T2 vs S_2")
            },
        )
        .step(
            "for every factor the total-order index is at least the first-order index",
            |w, _| {
                let s = require_indices(w)?;
                for i in 0..s.dim() {
                    if s.total_order[i] < s.first_order[i] - 1e-12 {
                        return Err(StepError::new(format!(
                            "S_T_{i} = {} < S_{i} = {}",
                            s.total_order[i], s.first_order[i]
                        )));
                    }
                }
                Ok(())
            },
        )
        .step("every first-order index is non-negative", |w, _| {
            let s = require_indices(w)?;
            for v in &s.first_order {
                if *v < 0.0 {
                    return Err(StepError::new(format!("negative S_i = {v}")));
                }
            }
            Ok(())
        })
        .step("the sum of first-order indices is at most 1", |w, _| {
            let s = require_indices(w)?;
            let sum: f64 = s.first_order.iter().sum();
            if sum <= 1.0 + 1e-12 {
                Ok(())
            } else {
                Err(StepError::new(format!("Σ S_i = {sum} > 1")))
            }
        })
        .step("S_1 is approximately 0.3139 within 5e-4", |w, _| {
            assert_close(require_indices(w)?.first_order[0], 0.3139, 5e-4, "S_1")
        })
        .step("S_2 is approximately 0.4424 within 5e-4", |w, _| {
            assert_close(require_indices(w)?.first_order[1], 0.4424, 5e-4, "S_2")
        })
        .step("S_3 is exactly 0", |w, _| {
            let v = require_indices(w)?.first_order[2];
            if v == 0.0 {
                Ok(())
            } else {
                Err(StepError::new(format!("S_3 = {v} (expected 0)")))
            }
        })
        .step("S_T1 is approximately 0.5576 within 5e-4", |w, _| {
            assert_close(require_indices(w)?.total_order[0], 0.5576, 5e-4, "S_T1")
        })
        .step("S_T2 is approximately 0.4424 within 5e-4", |w, _| {
            assert_close(require_indices(w)?.total_order[1], 0.4424, 5e-4, "S_T2")
        })
        .step("S_T3 is approximately 0.2436 within 5e-4", |w, _| {
            assert_close(require_indices(w)?.total_order[2], 0.2436, 5e-4, "S_T3")
        })
        .step("the X_2 first-order index is exactly 0", |w, _| {
            let v = require_indices(w)?.first_order[1];
            if v == 0.0 {
                Ok(())
            } else {
                Err(StepError::new(format!("S_2 = {v}")))
            }
        })
        .step("the X_3 total-order index is exactly 0", |w, _| {
            let v = require_indices(w)?.total_order[2];
            if v == 0.0 {
                Ok(())
            } else {
                Err(StepError::new(format!("S_T3 = {v}")))
            }
        })
        .step("S_1 is approximately 1 within 1e-12", |w, _| {
            assert_close(require_indices(w)?.first_order[0], 1.0, 1e-12, "S_1")
        })
        .step("S_T1 is approximately 1 within 1e-12", |w, _| {
            assert_close(require_indices(w)?.total_order[0], 1.0, 1e-12, "S_T1")
        })
        .step("the total variance is positive", |w, _| {
            let v = require_indices(w)?.total_variance;
            if v > 0.0 {
                Ok(())
            } else {
                Err(StepError::new(format!("D = {v} not positive")))
            }
        })
        .step("it has three factors named x1 x2 x3", |w, _| {
            let p = w
                .distribution
                .as_ref()
                .ok_or_else(|| StepError::new("no distribution"))?;
            let names: Vec<&str> = p.factors().iter().map(|f| f.name.as_str()).collect();
            if names == vec!["x1", "x2", "x3"] {
                Ok(())
            } else {
                Err(StepError::new(format!("got names {names:?}")))
            }
        })
        .step("every factor is Uniform(-π, π)", |w, _| {
            let p = w
                .distribution
                .as_ref()
                .ok_or_else(|| StepError::new("no distribution"))?;
            for f in p.factors() {
                match &f.distribution {
                    Distribution::Uniform { lo, hi } => {
                        if (lo - (-PI)).abs() > 1e-12 || (hi - PI).abs() > 1e-12 {
                            return Err(StepError::new(format!(
                                "factor {} bounds {lo} {hi} not (-π, π)",
                                f.name
                            )));
                        }
                    }
                    other => return Err(StepError::new(format!("non-Uniform: {other:?}"))),
                }
            }
            Ok(())
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}

// Use the unused-import lint suppressor: ishigami's `analytic_indices`
// is invoked via the explicit module path above; lint false-positive
// dance happens when `pub use` is added later.
#[allow(dead_code)]
fn _force_link() -> f64 {
    ishigami::ishigami(&[0.0, 0.0, 0.0])
}
