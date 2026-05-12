//! TCK harness for `build_saltelli_matrix` — radial-design
//! `(A, B, A_Bⁱ)` matrix construction over any `Sampler`.
//!
//! Wires `tck/salib/saltelli-matrix/features/{structure,determinism,validation}.feature`
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
use salib_samplers::{
    build_saltelli_matrix, LhsSampler, SaltelliError, SaltelliMatrix, Sampler, SobolDimSet,
    SobolSampler,
};

const FIXTURE_SEED: [u8; 32] = [0x42; 32];

fn feature_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("saltelli-matrix")
        .join("features")
        .join(name)
}

/// Sampler-as-trait-object so we can stash either LHS or Sobol' in
/// the same World field without generics.
type DynSampler = Box<dyn Sampler>;

#[derive(Default)]
struct World {
    sampler: Option<DynSampler>,
    matrix_a: Option<SaltelliMatrix>,
    matrix_b: Option<SaltelliMatrix>,
    error: Option<SaltelliError>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("has_sampler", &self.sampler.is_some())
            .field("has_matrix_a", &self.matrix_a.is_some())
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

fn require_sampler(w: &World) -> Result<&dyn Sampler, StepError> {
    w.sampler
        .as_deref()
        .ok_or_else(|| StepError::new("no sampler"))
}

fn require_matrix_a(w: &World) -> Result<&SaltelliMatrix, StepError> {
    w.matrix_a
        .as_ref()
        .ok_or_else(|| StepError::new("no matrix_a"))
}

fn build_into(w: &mut World, n: usize, second_order: bool, stream: u64, into_b: bool) {
    let mut rng = RngState::from_parts(FIXTURE_SEED, stream, 0);
    let s = w.sampler.as_ref().expect("no sampler").as_ref();
    let result = build_saltelli_matrix(s, n, second_order, &mut rng);
    match result {
        Ok(m) => {
            if into_b {
                w.matrix_b = Some(m);
            } else {
                w.matrix_a = Some(m);
            }
        }
        Err(e) => {
            w.error = Some(e);
        }
    }
}

#[allow(clippy::too_many_lines)]
fn build_runner() -> SyncRunner<World> {
    SyncRunner::new(World::default)
        // ── Givens — samplers ──────────────────────────────────────
        .step("a classic LHS sampler with dim 6", |w, _| {
            w.sampler = Some(Box::new(LhsSampler::classic(6)));
            Ok(())
        })
        .step("a classic LHS sampler with dim 5", |w, _| {
            w.sampler = Some(Box::new(LhsSampler::classic(5)));
            Ok(())
        })
        .step("a classic LHS sampler with dim 4", |w, _| {
            w.sampler = Some(Box::new(LhsSampler::classic(4)));
            Ok(())
        })
        .step(
            "a Sobol sampler with dim 8 dim_set Standard skip_first true",
            |w, _| {
                w.sampler = Some(Box::new(SobolSampler::with(
                    8,
                    SobolDimSet::Standard,
                    true,
                )));
                Ok(())
            },
        )
        .step(
            "a Sobol sampler with dim 6 dim_set Standard skip_first false",
            |w, _| {
                w.sampler = Some(Box::new(SobolSampler::with(
                    6,
                    SobolDimSet::Standard,
                    false,
                )));
                Ok(())
            },
        )
        // ── Whens — build (success path) ──────────────────────────
        .step(
            "I build a Saltelli matrix with n 64 second_order false",
            |w, _| {
                build_into(w, 64, false, 0, false);
                Ok(())
            },
        )
        .step(
            "I build a Saltelli matrix with n 32 second_order false",
            |w, _| {
                build_into(w, 32, false, 0, false);
                Ok(())
            },
        )
        .step(
            "I build a Saltelli matrix with n 16 second_order false",
            |w, _| {
                build_into(w, 16, false, 0, false);
                Ok(())
            },
        )
        .step(
            "I build a Saltelli matrix with n 16 second_order true",
            |w, _| {
                build_into(w, 16, true, 0, false);
                Ok(())
            },
        )
        .step(
            "I build a Saltelli matrix with n 32 second_order true",
            |w, _| {
                build_into(w, 32, true, 0, false);
                Ok(())
            },
        )
        // ── Whens — build (with stream control for determinism) ──
        .step(
            "I build a Saltelli matrix with n 64 second_order false using stream 0",
            |w, _| {
                build_into(w, 64, false, 0, false);
                Ok(())
            },
        )
        .step(
            "I build a second Saltelli matrix with n 64 second_order false using stream 0",
            |w, _| {
                build_into(w, 64, false, 0, true);
                Ok(())
            },
        )
        .step(
            "I build a Saltelli matrix with n 32 second_order false using stream 0",
            |w, _| {
                build_into(w, 32, false, 0, false);
                Ok(())
            },
        )
        .step(
            "I build a second Saltelli matrix with n 32 second_order false using stream 0",
            |w, _| {
                build_into(w, 32, false, 0, true);
                Ok(())
            },
        )
        .step(
            "I build a Saltelli matrix with n 32 second_order false using stream 1",
            |w, _| {
                build_into(w, 32, false, 1, false);
                Ok(())
            },
        )
        .step(
            "I build a second Saltelli matrix with n 32 second_order false using stream 2",
            |w, _| {
                build_into(w, 32, false, 2, true);
                Ok(())
            },
        )
        // ── Whens — build (error path) ─────────────────────────────
        .step(
            "I attempt to build a Saltelli matrix with n 0 second_order false",
            |w, _| {
                build_into(w, 0, false, 0, false);
                Ok(())
            },
        )
        .step(
            "I attempt to build a Saltelli matrix with n 32 second_order false",
            |w, _| {
                build_into(w, 32, false, 0, false);
                Ok(())
            },
        )
        // ── Thens — shape ──────────────────────────────────────────
        .step("the result has n 64 dim 3", |w, _| {
            let m = require_matrix_a(w)?;
            if m.n == 64 && m.dim == 3 {
                Ok(())
            } else {
                Err(StepError::new(format!("got n={} dim={}", m.n, m.dim)))
            }
        })
        .step("the result has n 32 dim 4", |w, _| {
            let m = require_matrix_a(w)?;
            if m.n == 32 && m.dim == 4 {
                Ok(())
            } else {
                Err(StepError::new(format!("got n={} dim={}", m.n, m.dim)))
            }
        })
        .step("A and B both have shape 64 by 3", |w, _| {
            let m = require_matrix_a(w)?;
            if m.a.shape() == [64, 3] && m.b.shape() == [64, 3] {
                Ok(())
            } else {
                Err(StepError::new(format!(
                    "A {:?} B {:?}",
                    m.a.shape(),
                    m.b.shape()
                )))
            }
        })
        .step("A and B both have shape 32 by 4", |w, _| {
            let m = require_matrix_a(w)?;
            if m.a.shape() == [32, 4] && m.b.shape() == [32, 4] {
                Ok(())
            } else {
                Err(StepError::new(format!(
                    "A {:?} B {:?}",
                    m.a.shape(),
                    m.b.shape()
                )))
            }
        })
        .step("there are 3 A_Bⁱ matrices each of shape 64 by 3", |w, _| {
            let m = require_matrix_a(w)?;
            if m.a_b.len() != 3 {
                return Err(StepError::new(format!("a_b len {}", m.a_b.len())));
            }
            for ab in &m.a_b {
                if ab.shape() != [64, 3] {
                    return Err(StepError::new(format!("a_b shape {:?}", ab.shape())));
                }
            }
            Ok(())
        })
        .step("there are 3 B_Aⁱ matrices", |w, _| {
            let m = require_matrix_a(w)?;
            let b_a = m
                .b_a
                .as_ref()
                .ok_or_else(|| StepError::new("b_a is None"))?;
            if b_a.len() == 3 {
                Ok(())
            } else {
                Err(StepError::new(format!("b_a len {}", b_a.len())))
            }
        })
        // ── Thens — column-replacement structure ──────────────────
        .step(
            "for every i in 0 to dim minus 1 the i-th A_Bⁱ has column i equal to B's column i",
            |w, _| {
                let m = require_matrix_a(w)?;
                for (i, ab_i) in m.a_b.iter().enumerate() {
                    for row in 0..m.n {
                        if ab_i[[row, i]] != m.b[[row, i]] {
                            return Err(StepError::new(format!(
                                "a_b[{i}] row {row} col {i} = {}, want {}",
                                ab_i[[row, i]],
                                m.b[[row, i]]
                            )));
                        }
                    }
                }
                Ok(())
            },
        )
        .step(
            "for every i and every j not equal to i the i-th A_Bⁱ has column j equal to A's column j",
            |w, _| {
                let m = require_matrix_a(w)?;
                for (i, ab_i) in m.a_b.iter().enumerate() {
                    for j in 0..m.dim {
                        if j == i {
                            continue;
                        }
                        for row in 0..m.n {
                            if ab_i[[row, j]] != m.a[[row, j]] {
                                return Err(StepError::new(format!(
                                    "a_b[{i}] row {row} col {j} should equal a"
                                )));
                            }
                        }
                    }
                }
                Ok(())
            },
        )
        .step(
            "for every i in 0 to dim minus 1 the i-th B_Aⁱ has column i equal to A's column i",
            |w, _| {
                let m = require_matrix_a(w)?;
                let b_a = m
                    .b_a
                    .as_ref()
                    .ok_or_else(|| StepError::new("b_a is None"))?;
                for (i, ba_i) in b_a.iter().enumerate() {
                    for row in 0..m.n {
                        if ba_i[[row, i]] != m.a[[row, i]] {
                            return Err(StepError::new(format!(
                                "b_a[{i}] row {row} col {i} != a"
                            )));
                        }
                    }
                }
                Ok(())
            },
        )
        .step(
            "for every i and every j not equal to i the i-th B_Aⁱ has column j equal to B's column j",
            |w, _| {
                let m = require_matrix_a(w)?;
                let b_a = m
                    .b_a
                    .as_ref()
                    .ok_or_else(|| StepError::new("b_a is None"))?;
                for (i, ba_i) in b_a.iter().enumerate() {
                    for j in 0..m.dim {
                        if j == i {
                            continue;
                        }
                        for row in 0..m.n {
                            if ba_i[[row, j]] != m.b[[row, j]] {
                                return Err(StepError::new(format!(
                                    "b_a[{i}] row {row} col {j} should equal b"
                                )));
                            }
                        }
                    }
                }
                Ok(())
            },
        )
        // ── Thens — total_evaluations ──────────────────────────────
        .step("total evaluations is 320", |w, _| {
            let m = require_matrix_a(w)?;
            if m.total_evaluations() == 320 {
                Ok(())
            } else {
                Err(StepError::new(format!("got {}", m.total_evaluations())))
            }
        })
        .step("total evaluations is 256", |w, _| {
            let m = require_matrix_a(w)?;
            if m.total_evaluations() == 256 {
                Ok(())
            } else {
                Err(StepError::new(format!("got {}", m.total_evaluations())))
            }
        })
        // ── Thens — A/B disjoint halves of base sample ────────────
        .step(
            "A's columns are the first 3 columns of the base 2d-dim sample",
            |w, _| {
                let m = require_matrix_a(w)?;
                let s = require_sampler(w)?;
                let mut rng = RngState::from_parts(FIXTURE_SEED, 0, 0);
                let base: Array2<f64> = s.unit_sample(m.n, &mut rng);
                for row in 0..m.n {
                    for col in 0..m.dim {
                        if m.a[[row, col]] != base[[row, col]] {
                            return Err(StepError::new(format!(
                                "A[{row},{col}] != base[{row},{col}]"
                            )));
                        }
                    }
                }
                Ok(())
            },
        )
        .step(
            "B's columns are the last 3 columns of the base 2d-dim sample",
            |w, _| {
                let m = require_matrix_a(w)?;
                let s = require_sampler(w)?;
                let mut rng = RngState::from_parts(FIXTURE_SEED, 0, 0);
                let base: Array2<f64> = s.unit_sample(m.n, &mut rng);
                for row in 0..m.n {
                    for col in 0..m.dim {
                        if m.b[[row, col]] != base[[row, col + m.dim]] {
                            return Err(StepError::new(format!(
                                "B[{row},{col}] != base[{row},{}]",
                                col + m.dim
                            )));
                        }
                    }
                }
                Ok(())
            },
        )
        // ── Thens — determinism ────────────────────────────────────
        .step("both Saltelli matrices are bit-identical", |w, _| {
            let a = w
                .matrix_a
                .as_ref()
                .ok_or_else(|| StepError::new("no matrix_a"))?;
            let b = w
                .matrix_b
                .as_ref()
                .ok_or_else(|| StepError::new("no matrix_b"))?;
            if a.a == b.a && a.b == b.b && a.a_b.len() == b.a_b.len() {
                for (x, y) in a.a_b.iter().zip(b.a_b.iter()) {
                    if x != y {
                        return Err(StepError::new("a_b differs"));
                    }
                }
                Ok(())
            } else {
                Err(StepError::new("matrices differ"))
            }
        })
        .step("the two Saltelli matrices differ", |w, _| {
            let a = w
                .matrix_a
                .as_ref()
                .ok_or_else(|| StepError::new("no matrix_a"))?;
            let b = w
                .matrix_b
                .as_ref()
                .ok_or_else(|| StepError::new("no matrix_b"))?;
            if a.a == b.a {
                Err(StepError::new("matrices unexpectedly equal"))
            } else {
                Ok(())
            }
        })
        // ── Thens — validation errors ──────────────────────────────
        .step("the result is a ZeroN error", |w, _| {
            let e = w
                .error
                .as_ref()
                .ok_or_else(|| StepError::new("no error"))?;
            if matches!(e, SaltelliError::ZeroN) {
                Ok(())
            } else {
                Err(StepError::new(format!("got {e:?}")))
            }
        })
        .step("the result is an OddBaseDim error with dim 5", |w, _| {
            let e = w
                .error
                .as_ref()
                .ok_or_else(|| StepError::new("no error"))?;
            if let SaltelliError::OddBaseDim { dim: 5 } = e {
                Ok(())
            } else {
                Err(StepError::new(format!("got {e:?}")))
            }
        })
}

#[test]
fn saltelli_matrix_structure_feature_runs() {
    let path = feature_path("saltelli_matrix_structure.feature");
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature =
        parse_feature(&content, "saltelli_matrix_structure.feature").expect("parses cleanly");
    let report = build_runner().run(&feature);
    report.assert_all_passed();
}

#[test]
fn saltelli_matrix_determinism_feature_runs() {
    let path = feature_path("saltelli_matrix_determinism.feature");
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature =
        parse_feature(&content, "saltelli_matrix_determinism.feature").expect("parses cleanly");
    let report = build_runner().run(&feature);
    report.assert_all_passed();
}

#[test]
fn saltelli_matrix_validation_feature_runs() {
    let path = feature_path("saltelli_matrix_validation.feature");
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature =
        parse_feature(&content, "saltelli_matrix_validation.feature").expect("parses cleanly");
    let report = build_runner().run(&feature);
    report.assert_all_passed();
}
