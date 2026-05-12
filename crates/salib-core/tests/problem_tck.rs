//! TCK harness for `Problem` content-addressing.
//!
//! Wires `tck/salib/problem/features/content_addressing.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use salib_core::{Distribution, FactorKind, Problem, ProblemBuilder};

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("problem")
        .join("features")
        .join("content_addressing.feature")
}

#[derive(Default)]
struct World {
    primary: Option<Problem>,
    secondary: Option<Problem>,
    hash_a: Option<[u8; 32]>,
    hash_b: Option<[u8; 32]>,
    deserialized: Option<Problem>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("has_primary", &self.primary.is_some())
            .field("has_secondary", &self.secondary.is_some())
            .field("hash_a_set", &self.hash_a.is_some())
            .field("hash_b_set", &self.hash_b.is_some())
            .finish_non_exhaustive()
    }
}

#[allow(clippy::too_many_lines)]
#[test]
fn content_addressing_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "content_addressing.feature")
        .expect("content_addressing.feature parses cleanly");

    let runner = SyncRunner::new(World::default)
        // ── Givens ─────────────────────────────────────────────────
        .step(
            r#"a Problem with one Uniform factor "x" on [0, 1]"#,
            |w, _| {
                w.primary = Some(
                    ProblemBuilder::new()
                        .factor("x", Distribution::Uniform { lo: 0.0, hi: 1.0 })
                        .build()
                        .expect("builds"),
                );
                Ok(())
            },
        )
        .step(
            "two Problems independently built with the same factor specs",
            |w, _| {
                let make = || {
                    ProblemBuilder::new()
                        .factor("x", Distribution::Uniform { lo: 0.0, hi: 1.0 })
                        .factor(
                            "y",
                            Distribution::Normal {
                                mu: 0.0,
                                sigma: 2.0,
                            },
                        )
                        .build()
                        .expect("builds")
                };
                w.primary = Some(make());
                w.secondary = Some(make());
                Ok(())
            },
        )
        .step(
            r#"a Problem with one Uniform factor "x" on [0, 2]"#,
            |w, _| {
                w.secondary = Some(
                    ProblemBuilder::new()
                        .factor("x", Distribution::Uniform { lo: 0.0, hi: 2.0 })
                        .build()
                        .expect("builds"),
                );
                Ok(())
            },
        )
        .step(
            r#"a Problem with one Uniform factor "y" on [0, 1]"#,
            |w, _| {
                w.secondary = Some(
                    ProblemBuilder::new()
                        .factor("y", Distribution::Uniform { lo: 0.0, hi: 1.0 })
                        .build()
                        .expect("builds"),
                );
                Ok(())
            },
        )
        .step(
            r#"a Problem with two factors named "a" then "b""#,
            |w, _| {
                w.primary = Some(
                    ProblemBuilder::new()
                        .factor("a", Distribution::Uniform { lo: 0.0, hi: 1.0 })
                        .factor("b", Distribution::Uniform { lo: 2.0, hi: 3.0 })
                        .build()
                        .expect("builds"),
                );
                Ok(())
            },
        )
        .step(
            r#"a Problem with the same two factors in order "b" then "a""#,
            |w, _| {
                w.secondary = Some(
                    ProblemBuilder::new()
                        .factor("b", Distribution::Uniform { lo: 2.0, hi: 3.0 })
                        .factor("a", Distribution::Uniform { lo: 0.0, hi: 1.0 })
                        .build()
                        .expect("builds"),
                );
                Ok(())
            },
        )
        .step(
            r#"a Problem with one Continuous Uniform factor "x" on [0, 1]"#,
            |w, _| {
                w.primary = Some(
                    ProblemBuilder::new()
                        .factor_with_kind(
                            "x",
                            Distribution::Uniform { lo: 0.0, hi: 1.0 },
                            FactorKind::Continuous,
                        )
                        .build()
                        .expect("builds"),
                );
                Ok(())
            },
        )
        .step(
            r#"a Problem with one Discrete Uniform factor "x" on [0, 1]"#,
            |w, _| {
                w.secondary = Some(
                    ProblemBuilder::new()
                        .factor_with_kind(
                            "x",
                            Distribution::Uniform { lo: 0.0, hi: 1.0 },
                            FactorKind::Discrete,
                        )
                        .build()
                        .expect("builds"),
                );
                Ok(())
            },
        )
        // ── Whens ───────────────────────────────────────────────────
        .step("I compute its content_hash twice", |w, _| {
            let p = w
                .primary
                .as_ref()
                .ok_or_else(|| StepError::new("no primary Problem"))?;
            w.hash_a = Some(p.content_hash());
            w.hash_b = Some(p.content_hash());
            Ok(())
        })
        .step("I compute each Problem's content_hash", |w, _| {
            let p1 = w
                .primary
                .as_ref()
                .ok_or_else(|| StepError::new("no primary"))?;
            let p2 = w
                .secondary
                .as_ref()
                .ok_or_else(|| StepError::new("no secondary"))?;
            w.hash_a = Some(p1.content_hash());
            w.hash_b = Some(p2.content_hash());
            Ok(())
        })
        .step("I compute its content_hash", |w, _| {
            let p = w
                .primary
                .as_ref()
                .ok_or_else(|| StepError::new("no primary"))?;
            w.hash_a = Some(p.content_hash());
            Ok(())
        })
        .step("I serialize it to JSON and deserialize back", |w, _| {
            let p = w
                .primary
                .as_ref()
                .ok_or_else(|| StepError::new("no primary"))?;
            let json = serde_json::to_string(p).map_err(|e| StepError::new(format!("ser: {e}")))?;
            let back: Problem =
                serde_json::from_str(&json).map_err(|e| StepError::new(format!("de: {e}")))?;
            w.deserialized = Some(back);
            Ok(())
        })
        // ── Thens ───────────────────────────────────────────────────
        .step("both hashes are bit-identical", |w, _| {
            let a = w.hash_a.ok_or_else(|| StepError::new("no hash_a"))?;
            let b = w.hash_b.ok_or_else(|| StepError::new("no hash_b"))?;
            if a == b {
                Ok(())
            } else {
                Err(StepError::new(format!("hashes differ: {a:?} vs {b:?}")))
            }
        })
        .step("the two hashes are bit-identical", |w, _| {
            let a = w.hash_a.ok_or_else(|| StepError::new("no hash_a"))?;
            let b = w.hash_b.ok_or_else(|| StepError::new("no hash_b"))?;
            if a == b {
                Ok(())
            } else {
                Err(StepError::new(format!("hashes differ: {a:?} vs {b:?}")))
            }
        })
        .step("the two hashes differ", |w, _| {
            let a = w.hash_a.ok_or_else(|| StepError::new("no hash_a"))?;
            let b = w.hash_b.ok_or_else(|| StepError::new("no hash_b"))?;
            if a == b {
                Err(StepError::new(format!("hashes unexpectedly equal: {a:?}")))
            } else {
                Ok(())
            }
        })
        .step("the result is exactly 32 bytes", |w, _| {
            let h = w.hash_a.ok_or_else(|| StepError::new("no hash_a"))?;
            // [u8; 32] always has length 32, but make the assertion
            // explicit so the scenario reads cleanly.
            if h.len() == 32 {
                Ok(())
            } else {
                Err(StepError::new(format!("hash length {} != 32", h.len())))
            }
        })
        .step(
            "the deserialized Problem's content_hash equals the original",
            |w, _| {
                let original = w
                    .primary
                    .as_ref()
                    .ok_or_else(|| StepError::new("no primary"))?;
                let back = w
                    .deserialized
                    .as_ref()
                    .ok_or_else(|| StepError::new("no deserialized"))?;
                if original.content_hash() == back.content_hash() {
                    Ok(())
                } else {
                    Err(StepError::new("hash differs after serde round-trip"))
                }
            },
        );

    let report = runner.run(&feature);
    report.assert_all_passed();
}
