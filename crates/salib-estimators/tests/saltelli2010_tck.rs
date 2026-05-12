//! TCK harness for `estimate_saltelli2010` — the headline
//! reviewer-affordance contract close against Ishigami canonical.
//!
//! Wires `tck/salib/sobol-estimator/features/saltelli2010_ishigami.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! The other four reviewer-affordance contract artifacts (convergence
//! rate, `SALib` differential, identity tests) live in
//! `tests/ishigami_e2e.rs` because their shape (parameter sweeps,
//! file reads) doesn't fit Gherkin cleanly. The single Gherkin
//! scenario here is the headline behavior the reviewer reads first.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::approx_constant
)]

use std::f64::consts::PI;
use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use salib_core::RngState;
use salib_estimators::{estimate_saltelli2010, SobolIndices};
use salib_samplers::{build_saltelli_matrix, SobolSampler};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("sobol-estimator")
        .join("features")
        .join("saltelli2010_ishigami.feature")
}

#[derive(Default)]
struct World {
    a: f64,
    b: f64,
    sampler_dim: usize,
    skip_first: bool,
    indices: Option<SobolIndices>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("a", &self.a)
            .field("b", &self.b)
            .field("sampler_dim", &self.sampler_dim)
            .field("skip_first", &self.skip_first)
            .field("has_indices", &self.indices.is_some())
            .finish_non_exhaustive()
    }
}

fn require_indices(w: &World) -> Result<&SobolIndices, StepError> {
    w.indices
        .as_ref()
        .ok_or_else(|| StepError::new("no indices; check When step"))
}

fn assert_within(got: f64, want: f64, tol: f64, label: &str) -> Result<(), StepError> {
    if (got - want).abs() < tol {
        Ok(())
    } else {
        Err(StepError::new(format!(
            "{label}: got {got:.4}, want {want:.4} (tol {tol})"
        )))
    }
}

#[allow(clippy::too_many_lines)]
#[test]
fn saltelli2010_ishigami_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "saltelli2010_ishigami.feature").expect("parses cleanly");

    let runner = SyncRunner::new(World::default)
        .step("the Ishigami canonical model with a=7 and b=0.1", |w, _| {
            w.a = 7.0;
            w.b = 0.1;
            Ok(())
        })
        .step(
            "a Sobol base sampler at dim 6 with skip_first false",
            |w, _| {
                w.sampler_dim = 6;
                w.skip_first = false;
                Ok(())
            },
        )
        .step(
            "I build a Saltelli matrix at N=8192 and run Saltelli2010",
            |w, _| {
                let sampler = SobolSampler::standard(w.sampler_dim).with_skip_first(w.skip_first);
                let mut rng = RngState::from_seed(FIXTURE_SEED);
                let matrix = build_saltelli_matrix(&sampler, 8192, false, &mut rng)
                    .map_err(|e| StepError::new(format!("matrix build: {e}")))?;
                let a = w.a;
                let b = w.b;
                let model = move |x: &[f64]| -> f64 {
                    let mapped: [f64; 3] = [
                        -PI + x[0] * 2.0 * PI,
                        -PI + x[1] * 2.0 * PI,
                        -PI + x[2] * 2.0 * PI,
                    ];
                    ishigami::ishigami_with_params(&mapped, a, b)
                };
                w.indices = Some(estimate_saltelli2010(&matrix, model));
                Ok(())
            },
        )
        .step("S_1 is within 0.05 of analytic 0.3139", |w, _| {
            assert_within(require_indices(w)?.first_order[0], 0.3139, 0.05, "S_1")
        })
        .step("S_2 is within 0.05 of analytic 0.4424", |w, _| {
            assert_within(require_indices(w)?.first_order[1], 0.4424, 0.05, "S_2")
        })
        .step("S_3 is within 0.05 of analytic 0.0", |w, _| {
            assert_within(require_indices(w)?.first_order[2], 0.0, 0.05, "S_3")
        })
        .step("S_T1 is within 0.05 of analytic 0.5576", |w, _| {
            assert_within(require_indices(w)?.total_order[0], 0.5576, 0.05, "S_T1")
        })
        .step("S_T2 is within 0.05 of analytic 0.4424", |w, _| {
            assert_within(require_indices(w)?.total_order[1], 0.4424, 0.05, "S_T2")
        })
        .step("S_T3 is within 0.05 of analytic 0.2436", |w, _| {
            assert_within(require_indices(w)?.total_order[2], 0.2436, 0.05, "S_T3")
        })
        .step("S_3 is within 0.05 of zero", |w, _| {
            assert_within(require_indices(w)?.first_order[2], 0.0, 0.05, "S_3 canary")
        })
        .step("S_2 and S_T2 agree within 0.05", |w, _| {
            let i = require_indices(w)?;
            let s2 = i.first_order[1];
            let st2 = i.total_order[1];
            if (s2 - st2).abs() < 0.05 {
                Ok(())
            } else {
                Err(StepError::new(format!(
                    "S_2 = {s2:.4}, S_T2 = {st2:.4} differ > 0.05"
                )))
            }
        })
        .step("the sum of first-order indices is at most 1.05", |w, _| {
            let i = require_indices(w)?;
            let sum: f64 = i.first_order.iter().sum();
            if sum <= 1.05 {
                Ok(())
            } else {
                Err(StepError::new(format!("Σ S_i = {sum:.4} > 1.05")))
            }
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
