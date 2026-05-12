//! TCK harness for `LhsSampler` — structural + determinism invariants.
//!
//! Wires `tck/salib/lhs-sampler/features/{lhs_structural,lhs_determinism}.feature`
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
use salib_samplers::{LhsSampler, Sampler};

fn feature_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("lhs-sampler")
        .join("features")
        .join(name)
}

const FIXTURE_SEED: [u8; 32] = [0x42; 32];

#[derive(Default)]
struct World {
    sampler: Option<LhsSampler>,
    sampler_secondary: Option<LhsSampler>,
    matrix_a: Option<Array2<f64>>,
    matrix_b: Option<Array2<f64>>,
    rng_a: Option<RngState>,
    rng_b: Option<RngState>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("has_sampler", &self.sampler.is_some())
            .field(
                "matrix_a_shape",
                &self.matrix_a.as_ref().map(|m| m.shape().to_vec()),
            )
            .field(
                "matrix_b_shape",
                &self.matrix_b.as_ref().map(|m| m.shape().to_vec()),
            )
            .finish_non_exhaustive()
    }
}

fn require_a(w: &World) -> Result<&Array2<f64>, StepError> {
    w.matrix_a
        .as_ref()
        .ok_or_else(|| StepError::new("no matrix_a; check When step"))
}

fn require_sampler(w: &World) -> Result<&LhsSampler, StepError> {
    w.sampler
        .as_ref()
        .ok_or_else(|| StepError::new("no sampler; check Given step"))
}

fn draw_into_a(w: &mut World, n: usize, stream: u64) -> Result<(), StepError> {
    let s = require_sampler(w)?;
    let mut rng = RngState::from_parts(FIXTURE_SEED, stream, 0);
    let m = s.unit_sample(n, &mut rng);
    w.matrix_a = Some(m);
    w.rng_a = Some(rng);
    Ok(())
}

fn draw_into_b(w: &mut World, n: usize, stream: u64) -> Result<(), StepError> {
    let s = require_sampler(w)?;
    let mut rng = RngState::from_parts(FIXTURE_SEED, stream, 0);
    let m = s.unit_sample(n, &mut rng);
    w.matrix_b = Some(m);
    w.rng_b = Some(rng);
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn build_runner() -> SyncRunner<World> {
    SyncRunner::new(World::default)
        // ── Givens ─────────────────────────────────────────────────
        .step("a classic LHS sampler with dim 4", |w, _| {
            w.sampler = Some(LhsSampler::classic(4));
            Ok(())
        })
        .step("a classic LHS sampler with dim 3", |w, _| {
            w.sampler = Some(LhsSampler::classic(3));
            Ok(())
        })
        .step("a classic LHS sampler with dim 5", |w, _| {
            w.sampler = Some(LhsSampler::classic(5));
            Ok(())
        })
        .step("a classic LHS sampler with dim 2", |w, _| {
            w.sampler = Some(LhsSampler::classic(2));
            Ok(())
        })
        .step("a classic LHS sampler with dim 0", |w, _| {
            w.sampler = Some(LhsSampler::classic(0));
            Ok(())
        })
        .step("a centered LHS sampler with dim 3", |w, _| {
            w.sampler = Some(LhsSampler::centered(3));
            Ok(())
        })
        .step("a centered LHS sampler with dim 2", |w, _| {
            w.sampler = Some(LhsSampler::centered(2));
            Ok(())
        })
        .step(
            "a classic LHS sampler with dim 2 and a centered LHS sampler with dim 2",
            |w, _| {
                w.sampler = Some(LhsSampler::classic(2));
                w.sampler_secondary = Some(LhsSampler::centered(2));
                Ok(())
            },
        )
        // ── Whens — structural draws ───────────────────────────────
        .step(
            "I draw a unit sample of size 64 with seed [0x42; 32] and stream 0",
            |w, _| draw_into_a(w, 64, 0),
        )
        .step(
            "I draw a unit sample of size 128 with seed [0x42; 32] and stream 0",
            |w, _| draw_into_a(w, 128, 0),
        )
        .step(
            "I draw a unit sample of size 32 with seed [0x42; 32] and stream 0",
            |w, _| draw_into_a(w, 32, 0),
        )
        .step(
            "I draw a unit sample of size 16 with seed [0x42; 32] and stream 0",
            |w, _| draw_into_a(w, 16, 0),
        )
        .step(
            "I draw a unit sample of size 0 with seed [0x42; 32] and stream 0",
            |w, _| draw_into_a(w, 0, 0),
        )
        .step(
            "I draw a unit sample of size 8 with seed [0x42; 32] and stream 0",
            |w, _| draw_into_a(w, 8, 0),
        )
        .step(
            "I draw a unit sample of size 1 with seed [0x42; 32] and stream 0",
            |w, _| draw_into_a(w, 1, 0),
        )
        // ── Whens — determinism ────────────────────────────────────
        .step(
            "I draw a unit sample of size 64 with seed [0x42; 32] and stream 7",
            |w, _| draw_into_a(w, 64, 7),
        )
        .step(
            "I draw another unit sample of size 64 with seed [0x42; 32] and stream 7",
            |w, _| draw_into_b(w, 64, 7),
        )
        .step(
            "I draw a unit sample of size 32 with seed [0x42; 32] and stream 0 and capture the post-draw RngState",
            |w, _| draw_into_a(w, 32, 0),
        )
        .step(
            "I draw a second unit sample of size 32 with seed [0x42; 32] and stream 0 and capture the post-draw RngState",
            |w, _| draw_into_b(w, 32, 0),
        )
        .step(
            "I draw a unit sample of size 32 with seed [0x42; 32] and stream 1",
            |w, _| draw_into_a(w, 32, 1),
        )
        .step(
            "I draw a unit sample of size 32 with seed [0x42; 32] and stream 2",
            |w, _| draw_into_b(w, 32, 2),
        )
        .step(
            "I draw size-32 samples from each with seed [0x42; 32] and stream 0",
            |w, _| {
                let s_classic = *w
                    .sampler
                    .as_ref()
                    .ok_or_else(|| StepError::new("no classic"))?;
                let s_centered = *w
                    .sampler_secondary
                    .as_ref()
                    .ok_or_else(|| StepError::new("no centered"))?;
                let mut r1 = RngState::from_parts(FIXTURE_SEED, 0, 0);
                let mut r2 = RngState::from_parts(FIXTURE_SEED, 0, 0);
                w.matrix_a = Some(s_classic.unit_sample(32, &mut r1));
                w.matrix_b = Some(s_centered.unit_sample(32, &mut r2));
                w.rng_a = Some(r1);
                w.rng_b = Some(r2);
                Ok(())
            },
        )
        // ── Thens ──────────────────────────────────────────────────
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
        .step("the matrix shape is 8 by 0", |w, _| {
            let m = require_a(w)?;
            if m.shape() == [8, 0] {
                Ok(())
            } else {
                Err(StepError::new(format!("shape {:?}", m.shape())))
            }
        })
        .step("the matrix shape is 1 by 2", |w, _| {
            let m = require_a(w)?;
            if m.shape() == [1, 2] {
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
                    let mut cells: Vec<usize> =
                        (0..n).map(|i| cell_index(m[[i, j]], n as f64)).collect();
                    cells.sort_unstable();
                    let expected: Vec<usize> = (0..n).collect();
                    if cells != expected {
                        return Err(StepError::new(format!(
                            "column {j} stratification: got {cells:?}"
                        )));
                    }
                }
                Ok(())
            },
        )
        .step(
            "for every column the sorted values equal the cell-center sequence",
            |w, _| {
                let m = require_a(w)?;
                let n = m.shape()[0];
                let dim = m.shape()[1];
                for j in 0..dim {
                    let mut col: Vec<f64> = (0..n).map(|i| m[[i, j]]).collect();
                    col.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    for (k, got) in col.iter().enumerate() {
                        let want = (k as f64 + 0.5) / n as f64;
                        if (got - want).abs() > 1e-12 {
                            return Err(StepError::new(format!(
                                "col {j} pos {k}: got {got}, want {want}"
                            )));
                        }
                    }
                }
                Ok(())
            },
        )
        .step("every value equals 0.5", |w, _| {
            let m = require_a(w)?;
            for &v in m {
                if v != 0.5 {
                    return Err(StepError::new(format!("got {v}")));
                }
            }
            Ok(())
        })
        // ── Determinism Thens ──────────────────────────────────────
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
        .step("both post-draw RngStates are equal", |w, _| {
            let a = w
                .rng_a
                .as_ref()
                .ok_or_else(|| StepError::new("no rng_a"))?;
            let b = w
                .rng_b
                .as_ref()
                .ok_or_else(|| StepError::new("no rng_b"))?;
            if a == b {
                Ok(())
            } else {
                Err(StepError::new(format!(
                    "post-draw rng differ: {a:?} vs {b:?}"
                )))
            }
        })
        .step("the two matrices differ", |w, _| {
            let a = w
                .matrix_a
                .as_ref()
                .ok_or_else(|| StepError::new("no matrix_a"))?;
            let b = w
                .matrix_b
                .as_ref()
                .ok_or_else(|| StepError::new("no matrix_b"))?;
            if a == b {
                Err(StepError::new("matrices unexpectedly equal"))
            } else {
                Ok(())
            }
        })
        .step("the post-draw RngState word_pos is greater than 0", |w, _| {
            let r = w
                .rng_a
                .as_ref()
                .ok_or_else(|| StepError::new("no rng_a"))?;
            if r.word_pos > 0 {
                Ok(())
            } else {
                Err(StepError::new(format!("word_pos {}", r.word_pos)))
            }
        })
        .step("the post-draw RngState word_pos is 0", |w, _| {
            let r = w
                .rng_a
                .as_ref()
                .ok_or_else(|| StepError::new("no rng_a"))?;
            if r.word_pos == 0 {
                Ok(())
            } else {
                Err(StepError::new(format!("word_pos {}", r.word_pos)))
            }
        })
        .step(
            "the classic sampler's post-draw word_pos exceeds the centered sampler's",
            |w, _| {
                let a = w
                    .rng_a
                    .as_ref()
                    .ok_or_else(|| StepError::new("no rng_a"))?;
                let b = w
                    .rng_b
                    .as_ref()
                    .ok_or_else(|| StepError::new("no rng_b"))?;
                if a.word_pos > b.word_pos {
                    Ok(())
                } else {
                    Err(StepError::new(format!(
                        "classic {} not > centered {}",
                        a.word_pos, b.word_pos
                    )))
                }
            },
        )
}

#[test]
fn lhs_structural_feature_runs() {
    let path = feature_path("lhs_structural.feature");
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "lhs_structural.feature").expect("parses cleanly");
    let report = build_runner().run(&feature);
    report.assert_all_passed();
}

#[test]
fn lhs_determinism_feature_runs() {
    let path = feature_path("lhs_determinism.feature");
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "lhs_determinism.feature").expect("parses cleanly");
    let report = build_runner().run(&feature);
    report.assert_all_passed();
}
