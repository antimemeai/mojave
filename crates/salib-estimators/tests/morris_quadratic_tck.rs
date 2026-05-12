//! TCK harness for `estimate_morris_effects` — Morris quadratic-
//! additive at d=8.
//!
//! Wires `tck/salib/morris-estimator/features/morris_quadratic_additive.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! ADR: `decisions/2026-04-29-saltelli-morris-quadratic-contract.md`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::cast_precision_loss
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
        .join("morris_quadratic_additive.feature")
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

fn build_trajectories(w: &mut World, r: usize) -> Result<(), StepError> {
    w.r = r;
    w.levels = 4;
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    w.trajectories = Some(
        build_morris_trajectories(w.d, w.r, w.levels, &mut rng)
            .map_err(|e| StepError::new(format!("trajectories: {e}")))?,
    );
    Ok(())
}

#[allow(clippy::too_many_lines)]
#[test]
fn morris_quadratic_additive_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature =
        parse_feature(&content, "morris_quadratic_additive.feature").expect("parses cleanly");

    let runner = SyncRunner::new(World::default)
        .step("the Morris quadratic-additive model with d=8", |w, _| {
            w.d = 8;
            Ok(())
        })
        .step("Morris trajectories with R=1000 and levels=4", |w, _| {
            build_trajectories(w, 1000)
        })
        .step("Morris trajectories with R=100 and levels=4", |w, _| {
            build_trajectories(w, 100)
        })
        .step("I estimate Morris elementary effects", |w, _| {
            let traj = w
                .trajectories
                .as_ref()
                .ok_or_else(|| StepError::new("no trajectories"))?;
            let d = w.d;
            let model =
                move |x: &[f64]| -> f64 { morris_test::morris_quadratic_additive_with_dim(x, d) };
            w.effects = Some(
                estimate_morris_effects(traj, model)
                    .map_err(|e| StepError::new(format!("estimate: {e}")))?,
            );
            Ok(())
        })
        .step("μ approximates 2 4 6 8 10 12 14 16 within 0.1", |w, _| {
            let e = require_effects(w)?;
            let want = [2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0];
            for (i, &w_v) in want.iter().enumerate() {
                let err = (e.mu[i] - w_v).abs();
                if err > 0.1 {
                    return Err(StepError::new(format!(
                        "μ_{i}: got {:.4}, want {w_v}, err {err:.4}",
                        e.mu[i]
                    )));
                }
            }
            Ok(())
        })
        .step(
            "σ approximates 0.333 0.667 1.0 1.333 1.667 2.0 2.333 2.667 within 0.15",
            |w, _| {
                let e = require_effects(w)?;
                let want = [
                    1.0_f64 / 3.0,
                    2.0 / 3.0,
                    1.0,
                    4.0 / 3.0,
                    5.0 / 3.0,
                    2.0,
                    7.0 / 3.0,
                    8.0 / 3.0,
                ];
                for (i, &w_v) in want.iter().enumerate() {
                    let err = (e.sigma[i] - w_v).abs();
                    if err > 0.15 {
                        return Err(StepError::new(format!(
                            "σ_{i}: got {:.4}, want {w_v:.4}, err {err:.4}",
                            e.sigma[i]
                        )));
                    }
                }
                Ok(())
            },
        )
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
        .step("μ error for factor 7 is below 0.15", |w, _| {
            let e = require_effects(w)?;
            let analytic = morris_test::analytic_quadratic_effects(e.d);
            let err = (e.mu[7] - analytic.mu[7]).abs();
            if err >= 0.15 {
                return Err(StepError::new(format!(
                    "μ_7 error: {err:.4} not < 0.15 (got {:.4}, want {:.4})",
                    e.mu[7], analytic.mu[7]
                )));
            }
            Ok(())
        })
        .step("σ error for factor 7 is below 0.15", |w, _| {
            let e = require_effects(w)?;
            let analytic = morris_test::analytic_quadratic_effects(e.d);
            let err = (e.sigma[7] - analytic.sigma[7]).abs();
            if err >= 0.15 {
                return Err(StepError::new(format!(
                    "σ_7 error: {err:.4} not < 0.15 (got {:.4}, want {:.4})",
                    e.sigma[7], analytic.sigma[7]
                )));
            }
            Ok(())
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
