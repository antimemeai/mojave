//! TCK harness for `build_fast_design` — Saltelli-Tarantola-Chan
//! 1999 search-curve sampler structural + determinism invariants.
//!
//! Wires `tck/salib/fast-sampler/features/fast_design.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! ADR: `decisions/2026-04-29-saltelli-fast-sampler.md`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use salib_core::RngState;
use salib_samplers::{build_fast_design, FastDesign};

const FIXTURE_SEED: [u8; 32] = [0x42; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("fast-sampler")
        .join("features")
        .join("fast_design.feature")
}

#[derive(Default)]
struct World {
    d: usize,
    n: usize,
    m: u32,
    design: Option<FastDesign>,
    design_b: Option<FastDesign>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("d", &self.d)
            .field("n", &self.n)
            .field("m", &self.m)
            .field("has_design", &self.design.is_some())
            .field("has_design_b", &self.design_b.is_some())
            .finish_non_exhaustive()
    }
}

fn require_design(w: &World) -> Result<&FastDesign, StepError> {
    w.design
        .as_ref()
        .ok_or_else(|| StepError::new("no design; check When step"))
}

#[allow(clippy::too_many_lines)]
#[test]
fn fast_design_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "fast_design.feature").expect("parses cleanly");

    let runner = SyncRunner::new(World::default)
        .step(
            "d=6 factors and n_per_factor=129 with harmonic M=4",
            |w, _| {
                w.d = 6;
                w.n = 129;
                w.m = 4;
                Ok(())
            },
        )
        .step(
            "d=3 factors and n_per_factor=129 with harmonic M=4",
            |w, _| {
                w.d = 3;
                w.n = 129;
                w.m = 4;
                Ok(())
            },
        )
        .step("I build the FAST design", |w, _| {
            let mut rng = RngState::from_seed(FIXTURE_SEED);
            w.design = Some(
                build_fast_design(w.d, w.n, w.m, &mut rng)
                    .map_err(|e| StepError::new(format!("build_fast_design: {e}")))?,
            );
            Ok(())
        })
        .step(
            "I build the FAST design twice from the same seed",
            |w, _| {
                let mut rng_a = RngState::from_seed(FIXTURE_SEED);
                let mut rng_b = RngState::from_seed(FIXTURE_SEED);
                w.design = Some(
                    build_fast_design(w.d, w.n, w.m, &mut rng_a)
                        .map_err(|e| StepError::new(format!("build a: {e}")))?,
                );
                w.design_b = Some(
                    build_fast_design(w.d, w.n, w.m, &mut rng_b)
                        .map_err(|e| StepError::new(format!("build b: {e}")))?,
                );
                Ok(())
            },
        )
        .step(
            "every sample value is in the closed interval 0 to 1",
            |w, _| {
                let d = require_design(w)?;
                for &v in &d.samples {
                    if !(0.0..=1.0).contains(&v) {
                        return Err(StepError::new(format!("sample {v} not in [0, 1]")));
                    }
                }
                Ok(())
            },
        )
        .step(
            "for each block i, ω_i is the maximum frequency in row i",
            |w, _| {
                let d = require_design(w)?;
                for i in 0..d.d {
                    let omega_i = d.omegas[[i, i]];
                    for j in 0..d.d {
                        if j != i && d.omegas[[i, j]] >= omega_i {
                            return Err(StepError::new(format!(
                                "row {i}: ω[{i},{j}] = {} not strictly < ω_max = {omega_i}",
                                d.omegas[[i, j]]
                            )));
                        }
                    }
                }
                Ok(())
            },
        )
        .step(
            "complementary frequencies stay below the harmonic-bandwidth bound",
            |w, _| {
                let d = require_design(w)?;
                let omega_max = d.omegas[[0, 0]];
                let bound = omega_max / (2 * d.harmonic);
                for i in 0..d.d {
                    for j in 0..d.d {
                        if j != i && d.omegas[[i, j]] > bound {
                            return Err(StepError::new(format!(
                                "ω[{i},{j}] = {} exceeds bound {bound}",
                                d.omegas[[i, j]]
                            )));
                        }
                    }
                }
                Ok(())
            },
        )
        .step(
            "complementary frequencies within each block are pairwise distinct",
            |w, _| {
                // d=3 / n=129 / M=4 → ω_max=16, m=2, m≥d−1=2 ⇒
                // linspace regime. Strict pairwise distinctness.
                let d = require_design(w)?;
                for i in 0..d.d {
                    let row: Vec<u32> = (0..d.d)
                        .filter(|&j| j != i)
                        .map(|j| d.omegas[[i, j]])
                        .collect();
                    let mut sorted = row.clone();
                    sorted.sort_unstable();
                    sorted.dedup();
                    if sorted.len() != row.len() {
                        return Err(StepError::new(format!(
                            "row {i}: complementary not pairwise distinct: {row:?}"
                        )));
                    }
                }
                Ok(())
            },
        )
        .step("the two designs are bit-identical", |w, _| {
            let a = require_design(w)?;
            let b = w
                .design_b
                .as_ref()
                .ok_or_else(|| StepError::new("no design_b; check When step"))?;
            if a.samples != b.samples {
                return Err(StepError::new("samples differ"));
            }
            if a.omegas != b.omegas {
                return Err(StepError::new("omegas differ"));
            }
            if a.phases != b.phases {
                return Err(StepError::new("phases differ"));
            }
            Ok(())
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
