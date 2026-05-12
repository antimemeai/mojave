//! TCK harness for `SobolSampler` — canonical values + structural +
//! determinism invariants.
//!
//! Wires
//! `tck/salib/sobol-sampler/features/{sobol_canonical_values,sobol_structural,sobol_determinism}.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use ndarray::Array2;
use salib_core::RngState;
use salib_samplers::{Sampler, SobolDimSet, SobolSampler};

fn feature_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("sobol-sampler")
        .join("features")
        .join(name)
}

const FIXTURE_SEED: [u8; 32] = [0; 32];

#[derive(Default)]
struct World {
    sampler: Option<SobolSampler>,
    matrix_a: Option<Array2<f64>>,
    matrix_b: Option<Array2<f64>>,
    rng_a: Option<RngState>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("has_sampler", &self.sampler.is_some())
            .field(
                "matrix_a_shape",
                &self.matrix_a.as_ref().map(|m| m.shape().to_vec()),
            )
            .finish_non_exhaustive()
    }
}

fn require_sampler(w: &World) -> Result<&SobolSampler, StepError> {
    w.sampler
        .as_ref()
        .ok_or_else(|| StepError::new("no sampler"))
}

fn require_a(w: &World) -> Result<&Array2<f64>, StepError> {
    w.matrix_a
        .as_ref()
        .ok_or_else(|| StepError::new("no matrix_a"))
}

fn set_sampler(w: &mut World, dim: usize, dim_set: SobolDimSet, skip_first: bool) {
    w.sampler = Some(SobolSampler::with(dim, dim_set, skip_first));
}

#[allow(clippy::too_many_lines)]
fn build_runner() -> SyncRunner<World> {
    SyncRunner::new(World::default)
        // ── Givens — explicit-tuple sampler constructors ──────────
        .step(
            "a Sobol sampler with dim 1 dim_set Standard skip_first false",
            |w, _| {
                set_sampler(w, 1, SobolDimSet::Standard, false);
                Ok(())
            },
        )
        .step(
            "a Sobol sampler with dim 2 dim_set Standard skip_first false",
            |w, _| {
                set_sampler(w, 2, SobolDimSet::Standard, false);
                Ok(())
            },
        )
        .step(
            "a Sobol sampler with dim 3 dim_set Standard skip_first false",
            |w, _| {
                set_sampler(w, 3, SobolDimSet::Standard, false);
                Ok(())
            },
        )
        .step(
            "a Sobol sampler with dim 5 dim_set Standard skip_first false",
            |w, _| {
                set_sampler(w, 5, SobolDimSet::Standard, false);
                Ok(())
            },
        )
        .step(
            "a Sobol sampler with dim 1 dim_set Standard skip_first true",
            |w, _| {
                set_sampler(w, 1, SobolDimSet::Standard, true);
                Ok(())
            },
        )
        .step(
            "a Sobol sampler with dim 3 dim_set Standard skip_first true",
            |w, _| {
                set_sampler(w, 3, SobolDimSet::Standard, true);
                Ok(())
            },
        )
        .step(
            "a Sobol sampler with dim 4 dim_set Standard skip_first true",
            |w, _| {
                set_sampler(w, 4, SobolDimSet::Standard, true);
                Ok(())
            },
        )
        .step(
            "a Sobol sampler with dim 5 dim_set Standard skip_first true",
            |w, _| {
                set_sampler(w, 5, SobolDimSet::Standard, true);
                Ok(())
            },
        )
        .step(
            "a Sobol sampler with dim 100 dim_set Minimal skip_first true",
            |w, _| {
                set_sampler(w, 100, SobolDimSet::Minimal, true);
                Ok(())
            },
        )
        .step(
            "a Sobol sampler with dim 500 dim_set Standard skip_first true",
            |w, _| {
                set_sampler(w, 500, SobolDimSet::Standard, true);
                Ok(())
            },
        )
        // ── Whens — draws ────────────────────────────────────────
        .step("I draw a unit sample of size 8", |w, _| {
            let s = require_sampler(w)?;
            let mut rng = RngState::from_seed(FIXTURE_SEED);
            w.matrix_a = Some(s.unit_sample(8, &mut rng));
            w.rng_a = Some(rng);
            Ok(())
        })
        .step("I draw a unit sample of size 7", |w, _| {
            let s = require_sampler(w)?;
            let mut rng = RngState::from_seed(FIXTURE_SEED);
            w.matrix_a = Some(s.unit_sample(7, &mut rng));
            w.rng_a = Some(rng);
            Ok(())
        })
        .step("I draw a unit sample of size 4", |w, _| {
            let s = require_sampler(w)?;
            let mut rng = RngState::from_seed(FIXTURE_SEED);
            w.matrix_a = Some(s.unit_sample(4, &mut rng));
            w.rng_a = Some(rng);
            Ok(())
        })
        .step("I draw a unit sample of size 64", |w, _| {
            let s = require_sampler(w)?;
            let mut rng = RngState::from_seed(FIXTURE_SEED);
            w.matrix_a = Some(s.unit_sample(64, &mut rng));
            w.rng_a = Some(rng);
            Ok(())
        })
        .step("I draw a unit sample of size 0", |w, _| {
            let s = require_sampler(w)?;
            let mut rng = RngState::from_seed(FIXTURE_SEED);
            w.matrix_a = Some(s.unit_sample(0, &mut rng));
            w.rng_a = Some(rng);
            Ok(())
        })
        .step("I draw a unit sample of size 256", |w, _| {
            let s = require_sampler(w)?;
            let mut rng = RngState::from_seed(FIXTURE_SEED);
            w.matrix_a = Some(s.unit_sample(256, &mut rng));
            w.rng_a = Some(rng);
            Ok(())
        })
        .step("I draw a unit sample of size 16", |w, _| {
            let s = require_sampler(w)?;
            let mut rng = RngState::from_seed(FIXTURE_SEED);
            w.matrix_a = Some(s.unit_sample(16, &mut rng));
            w.rng_a = Some(rng);
            Ok(())
        })
        .step("I draw a unit sample of size 1024", |w, _| {
            let s = require_sampler(w)?;
            let mut rng = RngState::from_seed(FIXTURE_SEED);
            w.matrix_a = Some(s.unit_sample(1024, &mut rng));
            w.rng_a = Some(rng);
            Ok(())
        })
        // ── Whens — determinism (with stream control) ───────────
        .step("I draw a unit sample of size 128 with stream 0", |w, _| {
            let s = require_sampler(w)?;
            let mut rng = RngState::from_parts(FIXTURE_SEED, 0, 0);
            w.matrix_a = Some(s.unit_sample(128, &mut rng));
            w.rng_a = Some(rng);
            Ok(())
        })
        .step(
            "I draw a second unit sample of size 128 with stream 0",
            |w, _| {
                let s = require_sampler(w)?;
                let mut rng = RngState::from_parts(FIXTURE_SEED, 0, 0);
                w.matrix_b = Some(s.unit_sample(128, &mut rng));
                Ok(())
            },
        )
        .step("I draw a unit sample of size 64 with stream 1", |w, _| {
            let s = require_sampler(w)?;
            let mut rng = RngState::from_parts(FIXTURE_SEED, 1, 0);
            w.matrix_a = Some(s.unit_sample(64, &mut rng));
            Ok(())
        })
        .step(
            "I draw a second unit sample of size 64 with stream 999",
            |w, _| {
                let s = require_sampler(w)?;
                let mut rng = RngState::from_parts(FIXTURE_SEED, 999, 0);
                w.matrix_b = Some(s.unit_sample(64, &mut rng));
                Ok(())
            },
        )
        .step("I draw a unit sample of size 64 with stream 0", |w, _| {
            let s = require_sampler(w)?;
            let mut rng = RngState::from_parts(FIXTURE_SEED, 0, 0);
            w.matrix_a = Some(s.unit_sample(64, &mut rng));
            w.rng_a = Some(rng);
            Ok(())
        })
        // ── Thens — canonical values ─────────────────────────────
        .step(
            "the dim-1 column equals 0.0 0.5 0.75 0.25 0.375 0.875 0.625 0.125 in order",
            |w, _| {
                let m = require_a(w)?;
                let want = [0.0, 0.5, 0.75, 0.25, 0.375, 0.875, 0.625, 0.125];
                for (i, &w_v) in want.iter().enumerate() {
                    if m[[i, 0]] != w_v {
                        return Err(StepError::new(format!(
                            "row {i} dim 0: got {}, want {w_v}",
                            m[[i, 0]]
                        )));
                    }
                }
                Ok(())
            },
        )
        .step(
            "the dim-2 column equals 0.0 0.5 0.25 0.75 0.375 0.875 0.125 0.625 in order",
            |w, _| {
                let m = require_a(w)?;
                let want = [0.0, 0.5, 0.25, 0.75, 0.375, 0.875, 0.125, 0.625];
                for (i, &w_v) in want.iter().enumerate() {
                    if m[[i, 1]] != w_v {
                        return Err(StepError::new(format!(
                            "row {i} dim 1: got {}, want {w_v}",
                            m[[i, 1]]
                        )));
                    }
                }
                Ok(())
            },
        )
        .step(
            "the dim-1 column equals 0.5 0.75 0.25 0.375 0.875 0.625 0.125 in order",
            |w, _| {
                let m = require_a(w)?;
                let want = [0.5, 0.75, 0.25, 0.375, 0.875, 0.625, 0.125];
                for (i, &w_v) in want.iter().enumerate() {
                    if m[[i, 0]] != w_v {
                        return Err(StepError::new(format!(
                            "row {i} dim 0: got {}, want {w_v}",
                            m[[i, 0]]
                        )));
                    }
                }
                Ok(())
            },
        )
        .step("row 0 is the all-zeros origin", |w, _| {
            let m = require_a(w)?;
            let dim = m.shape()[1];
            for j in 0..dim {
                if m[[0, j]] != 0.0 {
                    return Err(StepError::new(format!(
                        "row 0 dim {j}: got {}, want 0.0",
                        m[[0, j]]
                    )));
                }
            }
            Ok(())
        })
        // ── Thens — structural ───────────────────────────────────
        .step("the matrix shape is 64 by 4", |w, _| {
            let m = require_a(w)?;
            if m.shape() == [64, 4] {
                Ok(())
            } else {
                Err(StepError::new(format!("shape {:?}", m.shape())))
            }
        })
        .step("the matrix shape is 0 by 3", |w, _| {
            let m = require_a(w)?;
            if m.shape() == [0, 3] {
                Ok(())
            } else {
                Err(StepError::new(format!("shape {:?}", m.shape())))
            }
        })
        .step("the matrix shape is 16 by 100", |w, _| {
            let m = require_a(w)?;
            if m.shape() == [16, 100] {
                Ok(())
            } else {
                Err(StepError::new(format!("shape {:?}", m.shape())))
            }
        })
        .step("the matrix shape is 8 by 500", |w, _| {
            let m = require_a(w)?;
            if m.shape() == [8, 500] {
                Ok(())
            } else {
                Err(StepError::new(format!("shape {:?}", m.shape())))
            }
        })
        .step("every value is in [0, 1)", |w, _| {
            let m = require_a(w)?;
            for &v in m {
                if !(0.0..1.0).contains(&v) {
                    return Err(StepError::new(format!("out-of-range {v}")));
                }
            }
            Ok(())
        })
        .step(
            "for every column the floor of value times n is a permutation of 0 through n-1",
            |w, _| {
                let m = require_a(w)?;
                let n = m.shape()[0];
                let dim = m.shape()[1];
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let cell_index = |x: f64, n_f: f64| (x * n_f).floor() as usize;
                for j in 0..dim {
                    let mut bins: Vec<usize> =
                        (0..n).map(|i| cell_index(m[[i, j]], n as f64)).collect();
                    bins.sort_unstable();
                    let expected: Vec<usize> = (0..n).collect();
                    if bins != expected {
                        return Err(StepError::new(format!(
                            "dim {j} stratification: got {bins:?}"
                        )));
                    }
                }
                Ok(())
            },
        )
        // ── Thens — determinism ──────────────────────────────────
        .step("both matrices are bit-identical", |w, _| {
            let a = w
                .matrix_a
                .as_ref()
                .ok_or_else(|| StepError::new("no matrix_a"))?;
            let b = w
                .matrix_b
                .as_ref()
                .ok_or_else(|| StepError::new("no matrix_b"))?;
            if a == b {
                Ok(())
            } else {
                Err(StepError::new("matrices differ"))
            }
        })
        .step("the post-draw RngState word_pos is 0", |w, _| {
            let r = w.rng_a.as_ref().ok_or_else(|| StepError::new("no rng_a"))?;
            if r.word_pos == 0 {
                Ok(())
            } else {
                Err(StepError::new(format!("word_pos {}", r.word_pos)))
            }
        })
}

#[test]
fn sobol_canonical_values_feature_runs() {
    let path = feature_path("sobol_canonical_values.feature");
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature =
        parse_feature(&content, "sobol_canonical_values.feature").expect("parses cleanly");
    let report = build_runner().run(&feature);
    report.assert_all_passed();
}

#[test]
fn sobol_structural_feature_runs() {
    let path = feature_path("sobol_structural.feature");
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "sobol_structural.feature").expect("parses cleanly");
    let report = build_runner().run(&feature);
    report.assert_all_passed();
}

#[test]
fn sobol_determinism_feature_runs() {
    let path = feature_path("sobol_determinism.feature");
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "sobol_determinism.feature").expect("parses cleanly");
    let report = build_runner().run(&feature);
    report.assert_all_passed();
}
