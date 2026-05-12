//! TCK harness for `estimate_fast` — Ishigami headline scenario.
//!
//! Wires `tck/salib/fast-estimator/features/fast_ishigami.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! ADR: `decisions/2026-04-29-saltelli-fast-estimator.md`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::needless_range_loop
)]

use std::f64::consts::PI;
use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use salib_core::RngState;
use salib_estimators::{estimate_fast, FastIndices};
use salib_samplers::{build_fast_design, FastDesign};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("fast-estimator")
        .join("features")
        .join("fast_ishigami.feature")
}

fn ishigami_on_unit_cube(u: &[f64]) -> f64 {
    let x: [f64; 3] = [
        -PI + 2.0 * PI * u[0],
        -PI + 2.0 * PI * u[1],
        -PI + 2.0 * PI * u[2],
    ];
    ishigami::ishigami(&x)
}

#[derive(Default)]
struct World {
    n: usize,
    m: u32,
    design: Option<FastDesign>,
    estimate: Option<FastIndices>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("n", &self.n)
            .field("m", &self.m)
            .field("has_design", &self.design.is_some())
            .field("has_estimate", &self.estimate.is_some())
            .finish_non_exhaustive()
    }
}

fn require_estimate(w: &World) -> Result<&FastIndices, StepError> {
    w.estimate
        .as_ref()
        .ok_or_else(|| StepError::new("no estimate; check When step"))
}

#[allow(clippy::too_many_lines)]
#[test]
fn fast_ishigami_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "fast_ishigami.feature").expect("parses cleanly");

    let runner = SyncRunner::new(World::default)
        .step("the Ishigami model on Uniform[-π, π]³", |_w, _| Ok(()))
        .step("a FAST design with N=1025 and harmonic M=4", |w, _| {
            w.n = 1025;
            w.m = 4;
            let mut rng = RngState::from_seed(FIXTURE_SEED);
            w.design = Some(
                build_fast_design(3, w.n, w.m, &mut rng)
                    .map_err(|e| StepError::new(format!("design: {e}")))?,
            );
            Ok(())
        })
        .step(
            "I estimate FAST first-order and total-order indices",
            |w, _| {
                let d = w
                    .design
                    .as_ref()
                    .ok_or_else(|| StepError::new("no design"))?;
                w.estimate = Some(
                    estimate_fast(d, ishigami_on_unit_cube)
                        .map_err(|e| StepError::new(format!("estimate: {e}")))?,
                );
                Ok(())
            },
        )
        .step("S approximates 0.314 0.442 0.000 within 0.05", |w, _| {
            let est = require_estimate(w)?;
            let want = [0.314_f64, 0.442, 0.000];
            for (i, &w_v) in want.iter().enumerate() {
                let err = (est.s[i] - w_v).abs();
                if err > 0.05 {
                    return Err(StepError::new(format!(
                        "S_{i}: got {:.4}, want {w_v}, err {err:.4}",
                        est.s[i]
                    )));
                }
            }
            Ok(())
        })
        .step("ST approximates 0.558 0.442 0.244 within 0.10", |w, _| {
            let est = require_estimate(w)?;
            let want = [0.558_f64, 0.442, 0.244];
            for (i, &w_v) in want.iter().enumerate() {
                let err = (est.st[i] - w_v).abs();
                if err > 0.10 {
                    return Err(StepError::new(format!(
                        "ST_{i}: got {:.4}, want {w_v}, err {err:.4}",
                        est.st[i]
                    )));
                }
            }
            Ok(())
        })
        .step("ST is at least S for every factor", |w, _| {
            let est = require_estimate(w)?;
            for i in 0..3 {
                if est.st[i] + 1e-9 < est.s[i] {
                    return Err(StepError::new(format!(
                        "factor {i}: ST = {} < S = {}",
                        est.st[i], est.s[i]
                    )));
                }
            }
            Ok(())
        })
        .step(
            "S is within 0.05 of SALib's frozen Ishigami reference",
            |w, _| {
                let est = require_estimate(w)?;
                let salib = [0.3120, 0.4441, 0.0198];
                for i in 0..3 {
                    let d = (est.s[i] - salib[i]).abs();
                    if d > 0.05 {
                        return Err(StepError::new(format!(
                            "S_{i}: ours {} SALib {} diff {d:.4}",
                            est.s[i], salib[i]
                        )));
                    }
                }
                Ok(())
            },
        )
        .step(
            "ST is within 0.05 of SALib's frozen Ishigami reference",
            |w, _| {
                let est = require_estimate(w)?;
                let salib = [0.5389, 0.4893, 0.2407];
                for i in 0..3 {
                    let d = (est.st[i] - salib[i]).abs();
                    if d > 0.05 {
                        return Err(StepError::new(format!(
                            "ST_{i}: ours {} SALib {} diff {d:.4}",
                            est.st[i], salib[i]
                        )));
                    }
                }
                Ok(())
            },
        );

    let report = runner.run(&feature);
    report.assert_all_passed();
}
