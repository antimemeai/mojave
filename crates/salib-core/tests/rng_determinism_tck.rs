//! TCK harness for `RngState` — multi-stream `ChaCha20` with
//! deterministic salt-derived forking.
//!
//! Wires `tck/salib/rng-determinism/features/multi_stream_chacha.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! Per `decisions/2026-04-28-saltelli-tck-posture.md` Layer 1 (outer
//! Gherkin TCK) and `decisions/2026-04-28-saltelli-rng-determinism.md`
//! § "What this gates — Mechanized."

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use rand_chacha::rand_core::RngCore;
use rand_chacha::ChaCha20Rng;
use salib_core::{rng::RngAlgorithm, RngState};

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("rng-determinism")
        .join("features")
        .join("multi_stream_chacha.feature")
}

const SEED_BYTES: [u8; 32] = [0x42; 32];

#[derive(Default)]
struct World {
    parent: Option<RngState>,
    fresh_state: Option<RngState>,
    bytes_a: Option<Vec<u8>>,
    bytes_b: Option<Vec<u8>>,
    fork_a: Option<RngState>,
    fork_b: Option<RngState>,
    snapshot: Option<RngState>,
    original_continued_bytes: Option<Vec<u8>>,
    resumed_bytes: Option<Vec<u8>>,
    rederived_fork: Option<RngState>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("has_parent", &self.parent.is_some())
            .field("has_fork_a", &self.fork_a.is_some())
            .field("has_fork_b", &self.fork_b.is_some())
            .finish_non_exhaustive()
    }
}

fn draw_n(rng: &mut ChaCha20Rng, n: usize) -> Vec<u8> {
    let mut buf = vec![0u8; n];
    rng.fill_bytes(&mut buf);
    buf
}

#[allow(clippy::too_many_lines)]
#[test]
fn multi_stream_chacha_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "multi_stream_chacha.feature")
        .expect("multi_stream_chacha.feature parses cleanly");

    let runner = SyncRunner::new(World::default)
        // Given variants — each scenario sets up a parent RngState.
        .step(
            "an RngState seeded with 0x42 repeated and stream 7",
            |w, _| {
                w.parent = Some(RngState::from_parts(SEED_BYTES, 7, 0));
                Ok(())
            },
        )
        .step(
            "an RngState seeded with 0x42 repeated and stream 0",
            |w, _| {
                w.parent = Some(RngState::from_parts(SEED_BYTES, 0, 0));
                Ok(())
            },
        )
        .step(
            "an RngState seeded with 0x42 repeated and stream 12345",
            |w, _| {
                w.parent = Some(RngState::from_parts(SEED_BYTES, 12345, 0));
                Ok(())
            },
        )
        // ── Scenario: same seed + stream + word_pos produces identical bytes
        .step("I draw 1024 bytes from the underlying ChaCha20", |w, _| {
            let parent = w
                .parent
                .clone()
                .ok_or_else(|| StepError::new("no parent; check Given"))?;
            let mut rng = parent.into_chacha();
            w.bytes_a = Some(draw_n(&mut rng, 1024));
            Ok(())
        })
        .step(
            "I draw 1024 bytes from a fresh RngState with the same seed and stream",
            |w, _| {
                let parent = w
                    .parent
                    .as_ref()
                    .ok_or_else(|| StepError::new("no parent; check Given"))?;
                let mut rng = RngState::from_parts(parent.seed, parent.stream, 0).into_chacha();
                w.bytes_b = Some(draw_n(&mut rng, 1024));
                Ok(())
            },
        )
        .step("both draws are bit-identical", |w, _| {
            let a = w
                .bytes_a
                .as_ref()
                .ok_or_else(|| StepError::new("missing bytes_a"))?;
            let b = w
                .bytes_b
                .as_ref()
                .ok_or_else(|| StepError::new("missing bytes_b"))?;
            if a == b {
                Ok(())
            } else {
                Err(StepError::new("byte streams differ"))
            }
        })
        // ── Scenario: forking with the same salt is deterministic
        .step(r#"I fork it with salt "saltelli-block-0""#, |w, _| {
            let parent = w
                .parent
                .as_ref()
                .ok_or_else(|| StepError::new("no parent"))?;
            w.fork_a = Some(parent.fork(b"saltelli-block-0"));
            Ok(())
        })
        .step(
            r#"I fork the same parent again with salt "saltelli-block-0""#,
            |w, _| {
                let parent = w
                    .parent
                    .as_ref()
                    .ok_or_else(|| StepError::new("no parent"))?;
                w.fork_b = Some(parent.fork(b"saltelli-block-0"));
                Ok(())
            },
        )
        .step("both forked RngStates are equal field-for-field", |w, _| {
            let a = w
                .fork_a
                .as_ref()
                .ok_or_else(|| StepError::new("missing fork_a"))?;
            let b = w
                .fork_b
                .as_ref()
                .ok_or_else(|| StepError::new("missing fork_b"))?;
            if a == b {
                Ok(())
            } else {
                Err(StepError::new(format!("forks differ: {a:?} vs {b:?}")))
            }
        })
        // ── Scenario: distinct salts produce distinct streams
        .step(
            r#"I fork the same parent with salt "saltelli-block-1""#,
            |w, _| {
                let parent = w
                    .parent
                    .as_ref()
                    .ok_or_else(|| StepError::new("no parent"))?;
                w.fork_b = Some(parent.fork(b"saltelli-block-1"));
                Ok(())
            },
        )
        .step(
            "the two forked RngStates differ in their stream value",
            |w, _| {
                let a = w
                    .fork_a
                    .as_ref()
                    .ok_or_else(|| StepError::new("missing fork_a"))?;
                let b = w
                    .fork_b
                    .as_ref()
                    .ok_or_else(|| StepError::new("missing fork_b"))?;
                if a.stream == b.stream {
                    Err(StepError::new(format!(
                        "forks share stream value {}",
                        a.stream
                    )))
                } else {
                    Ok(())
                }
            },
        )
        .step(
            "drawing 1024 bytes from each fork produces different bytes",
            |w, _| {
                let a = w
                    .fork_a
                    .clone()
                    .ok_or_else(|| StepError::new("missing fork_a"))?;
                let b = w
                    .fork_b
                    .clone()
                    .ok_or_else(|| StepError::new("missing fork_b"))?;
                let mut ra = a.into_chacha();
                let mut rb = b.into_chacha();
                if draw_n(&mut ra, 1024) == draw_n(&mut rb, 1024) {
                    Err(StepError::new("forks produced identical byte streams"))
                } else {
                    Ok(())
                }
            },
        )
        // ── Scenario: forking is pure under the parent's stream
        .step(
            "the fork is a pure function of (parent.stream, salt) and the parent's seed",
            // Computational claim asserted alongside the next step;
            // by itself this is a no-op (the property is operationalized
            // in the re-derivation step).
            |_w, _| Ok(()),
        )
        .step(
            "re-deriving with the same parent.stream and salt produces an equal fork",
            |w, _| {
                let parent = w
                    .parent
                    .as_ref()
                    .ok_or_else(|| StepError::new("no parent"))?;
                let fresh_parent =
                    RngState::from_parts(parent.seed, parent.stream, parent.word_pos);
                let rederived = fresh_parent.fork(b"saltelli-block-0");
                w.rederived_fork = Some(rederived.clone());
                let original = w
                    .fork_a
                    .as_ref()
                    .ok_or_else(|| StepError::new("missing fork_a"))?;
                if &rederived == original {
                    Ok(())
                } else {
                    Err(StepError::new(format!(
                        "rederived fork differs: {rederived:?} vs original {original:?}"
                    )))
                }
            },
        )
        // ── Scenario: word_pos snapshot enables byte-for-byte resumption
        .step(
            "I draw 8192 bytes from the underlying ChaCha20",
            |w, _| {
                let parent = w
                    .parent
                    .clone()
                    .ok_or_else(|| StepError::new("no parent"))?;
                w.fresh_state = Some(parent.clone());
                let mut rng = parent.into_chacha();
                let _ = draw_n(&mut rng, 8192);
                // Continue draws from this point — the "unbroken" baseline.
                let baseline = draw_n(&mut rng, 1024);
                w.original_continued_bytes = Some(baseline);
                // Snapshot at the post-(8192+1024)-byte word_pos? No — the
                // scenario snapshots after the 8192-byte draw. Re-create
                // a fresh chacha and re-draw 8192 bytes to get the
                // snapshot at the right offset.
                let mut rng2 = w
                    .fresh_state
                    .clone()
                    .ok_or_else(|| StepError::new("missing fresh_state"))?
                    .into_chacha();
                let _ = draw_n(&mut rng2, 8192);
                w.snapshot = Some(RngState::snapshot(
                    &rng2,
                    w.fresh_state
                        .as_ref()
                        .ok_or_else(|| StepError::new("missing fresh_state"))?,
                ));
                Ok(())
            },
        )
        .step("I snapshot the RngState at the post-draw word_pos", |w, _| {
            // Already snapshotted above; this step is the narrative
            // marker. Verify the snapshot exists.
            if w.snapshot.is_some() {
                Ok(())
            } else {
                Err(StepError::new("no snapshot taken"))
            }
        })
        .step("I create a fresh ChaCha20 from the snapshot", |w, _| {
            let snap = w
                .snapshot
                .clone()
                .ok_or_else(|| StepError::new("missing snapshot"))?;
            let mut rng = snap.into_chacha();
            w.resumed_bytes = Some(draw_n(&mut rng, 1024));
            Ok(())
        })
        .step(
            "drawing 1024 more bytes from the resumed ChaCha20 matches the next 1024 bytes of an unbroken draw",
            |w, _| {
                let resumed = w
                    .resumed_bytes
                    .as_ref()
                    .ok_or_else(|| StepError::new("missing resumed bytes"))?;
                let baseline = w
                    .original_continued_bytes
                    .as_ref()
                    .ok_or_else(|| StepError::new("missing baseline bytes"))?;
                if resumed == baseline {
                    Ok(())
                } else {
                    Err(StepError::new(format!(
                        "resumed first 16 bytes {:?}; baseline {:?}",
                        &resumed[..16],
                        &baseline[..16]
                    )))
                }
            },
        );

    // Sanity check: the World pre-fills `algorithm` correctly when
    // `RngState` is parsed back from disk in some future replay path.
    // Touched here just to silence the dead-code lint on
    // `RngAlgorithm` re-export — the enum is part of the public surface.
    let _ = RngAlgorithm::ChaCha20;

    let report = runner.run(&feature);
    report.assert_all_passed();
}
