//! TCK harness for `estimate_morris_effects` — Morris additive-linear
//! at d=8.
//!
//! Wires `tck/salib/morris-estimator/features/morris_additive_linear.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::approx_constant
)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use salib_core::RngState;
use salib_estimators::{estimate_morris_effects, MorrisEffects};
use salib_samplers::{build_morris_trajectories, MorrisTrajectories};
use salib_validation::morris_test;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("morris-estimator")
        .join("features")
        .join("morris_additive_linear.feature")
}

#[derive(Default)]
struct World {
    d: usize,
    r: usize,
    levels: u32,
    trajectories: Option<MorrisTrajectories>,
    effects: Option<MorrisEffects>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("d", &self.d)
            .field("r", &self.r)
            .field("levels", &self.levels)
            .field("has_traj", &self.trajectories.is_some())
            .field("has_effects", &self.effects.is_some())
            .finish_non_exhaustive()
    }
}

fn require_effects(w: &World) -> Result<&MorrisEffects, StepError> {
    w.effects
        .as_ref()
        .ok_or_else(|| StepError::new("no effects; check When step"))
}

#[allow(clippy::too_many_lines)]
#[test]
fn morris_additive_linear_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature =
        parse_feature(&content, "morris_additive_linear.feature").expect("parses cleanly");

    let runner = SyncRunner::new(World::default)
        .step("the Morris additive-linear model with d=8", |w, _| {
            w.d = 8;
            Ok(())
        })
        .step("Morris trajectories with R=50 and levels=4", |w, _| {
            w.r = 50;
            w.levels = 4;
            let mut rng = RngState::from_seed(FIXTURE_SEED);
            w.trajectories = Some(
                build_morris_trajectories(w.d, w.r, w.levels, &mut rng)
                    .map_err(|e| StepError::new(format!("trajectories: {e}")))?,
            );
            Ok(())
        })
        .step("Morris trajectories with R=30 and levels=4", |w, _| {
            w.r = 30;
            w.levels = 4;
            let mut rng = RngState::from_seed(FIXTURE_SEED);
            w.trajectories = Some(
                build_morris_trajectories(w.d, w.r, w.levels, &mut rng)
                    .map_err(|e| StepError::new(format!("trajectories: {e}")))?,
            );
            Ok(())
        })
        .step("I estimate Morris elementary effects", |w, _| {
            let traj = w
                .trajectories
                .as_ref()
                .ok_or_else(|| StepError::new("no trajectories"))?;
            let d = w.d;
            let model =
                move |x: &[f64]| -> f64 { morris_test::morris_additive_linear_with_dim(x, d) };
            w.effects = Some(
                estimate_morris_effects(traj, model)
                    .map_err(|e| StepError::new(format!("estimate: {e}")))?,
            );
            Ok(())
        })
        .step("μ equals 1 2 3 4 5 6 7 8 within 1e-10", |w, _| {
            let e = require_effects(w)?;
            let want = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
            for (i, &w_v) in want.iter().enumerate() {
                if (e.mu[i] - w_v).abs() > 1e-10 {
                    return Err(StepError::new(format!(
                        "μ_{i}: got {}, want {w_v}",
                        e.mu[i]
                    )));
                }
            }
            Ok(())
        })
        .step("μ* equals 1 2 3 4 5 6 7 8 within 1e-10", |w, _| {
            let e = require_effects(w)?;
            let want = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
            for (i, &w_v) in want.iter().enumerate() {
                if (e.mu_star[i] - w_v).abs() > 1e-10 {
                    return Err(StepError::new(format!(
                        "μ*_{i}: got {}, want {w_v}",
                        e.mu_star[i]
                    )));
                }
            }
            Ok(())
        })
        .step("σ equals 0 0 0 0 0 0 0 0 within 1e-10", |w, _| {
            let e = require_effects(w)?;
            for (i, &s) in e.sigma.iter().enumerate() {
                if s.abs() > 1e-10 {
                    return Err(StepError::new(format!("σ_{i}: got {s}")));
                }
            }
            Ok(())
        })
        .step("μ* is at least absolute μ for every factor", |w, _| {
            let e = require_effects(w)?;
            for i in 0..e.d {
                if e.mu_star[i] < e.mu[i].abs() - 1e-12 {
                    return Err(StepError::new(format!(
                        "μ*_{i} = {} < |μ_{i}| = {}",
                        e.mu_star[i],
                        e.mu[i].abs()
                    )));
                }
            }
            Ok(())
        })
        .step(
            "μ* is strictly increasing across factors 0 through 7",
            |w, _| {
                let e = require_effects(w)?;
                for i in 1..8 {
                    if e.mu_star[i] <= e.mu_star[i - 1] {
                        return Err(StepError::new(format!(
                            "ranking violated: μ*_{i} = {} not > μ*_{} = {}",
                            e.mu_star[i],
                            i - 1,
                            e.mu_star[i - 1]
                        )));
                    }
                }
                Ok(())
            },
        );

    let report = runner.run(&feature);
    report.assert_all_passed();
}
