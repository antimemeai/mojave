//! TCK harness for `tree_sum` / `par_tree_sum` / `tree_dot` /
//! `par_tree_dot` / `tree_var` / `par_tree_var` — bit-identical
//! reductions invariant under rayon thread count.
//!
//! Wires `tck/salib/rng-determinism/features/tree_fold_invariance.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! Per `decisions/2026-04-28-saltelli-tck-posture.md` Layer 1 (outer
//! Gherkin TCK) and `decisions/2026-04-28-saltelli-rng-determinism.md`
//! § "Tree-fold reductions and the float-associativity defense."

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use salib_core::{par_tree_dot, par_tree_sum, par_tree_var, tree_dot, tree_sum, tree_var};

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("rng-determinism")
        .join("features")
        .join("tree_fold_invariance.feature")
}

const FIXTURE_SEED: [u8; 32] = [0x42; 32];

fn fixture_vec(stream: u64, n: usize) -> Vec<f64> {
    let mut rng = ChaCha20Rng::from_seed(FIXTURE_SEED);
    rng.set_stream(stream);
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        // Sample u64, normalize to [-1, 1] via two-step f64 widening.
        // Deterministic across platforms because the integer-to-f64
        // conversion is exact for `u64::MAX`-bounded values.
        let x = rng.next_u64();
        #[allow(clippy::cast_precision_loss)]
        let f = (x as f64) / (u64::MAX as f64);
        out.push(f.mul_add(2.0, -1.0));
    }
    out
}

#[derive(Default)]
struct World {
    xs: Option<Vec<f64>>,
    a: Option<Vec<f64>>,
    b: Option<Vec<f64>>,
    par_result: Option<f64>,
    par_result_2: Option<f64>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("xs_len", &self.xs.as_ref().map(Vec::len))
            .field("a_len", &self.a.as_ref().map(Vec::len))
            .field("b_len", &self.b.as_ref().map(Vec::len))
            .field("par_result", &self.par_result)
            .finish_non_exhaustive()
    }
}

fn install_pool<R, F>(threads: usize, f: F) -> R
where
    F: FnOnce() -> R + Send,
    R: Send,
{
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("build rayon pool");
    pool.install(f)
}

fn build_xs(w: &mut World, n: usize, stream: u64) {
    w.xs = Some(fixture_vec(stream, n));
}

fn assert_bits_eq(got: f64, want: f64, label: &str) -> Result<(), StepError> {
    if got.to_bits() == want.to_bits() {
        Ok(())
    } else {
        Err(StepError::new(format!(
            "{label}: got {got:?} ({:#x}); want {want:?} ({:#x})",
            got.to_bits(),
            want.to_bits()
        )))
    }
}

#[allow(clippy::too_many_lines)]
#[test]
fn tree_fold_invariance_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "tree_fold_invariance.feature")
        .expect("tree_fold_invariance.feature parses cleanly");

    let runner = SyncRunner::new(World::default)
        // ── Given variants — vectors of various lengths and seedings.
        .step(
            "a length-4096 f64 vector seeded by ChaCha20 stream 0",
            |w, _| {
                build_xs(w, 4096, 0);
                Ok(())
            },
        )
        .step(
            "a length-65536 f64 vector seeded by ChaCha20 stream 0",
            |w, _| {
                build_xs(w, 65536, 0);
                Ok(())
            },
        )
        .step(
            "length-4096 f64 vectors a seeded by ChaCha20 stream 1 and b seeded by stream 2",
            |w, _| {
                w.a = Some(fixture_vec(1, 4096));
                w.b = Some(fixture_vec(2, 4096));
                Ok(())
            },
        )
        .step(
            "length-65536 f64 vectors a seeded by ChaCha20 stream 1 and b seeded by stream 2",
            |w, _| {
                w.a = Some(fixture_vec(1, 65536));
                w.b = Some(fixture_vec(2, 65536));
                Ok(())
            },
        )
        .step("an empty f64 vector", |w, _| {
            w.xs = Some(Vec::new());
            Ok(())
        })
        .step("a length-1 f64 vector with the value 1.5", |w, _| {
            w.xs = Some(vec![1.5]);
            Ok(())
        })
        // ── When variants — par_tree_sum at various thread counts.
        .step(
            "I compute par_tree_sum with 1 rayon worker threads",
            |w, _| {
                let xs = w.xs.clone().ok_or_else(|| StepError::new("no xs"))?;
                w.par_result = Some(install_pool(1, || par_tree_sum(&xs)));
                Ok(())
            },
        )
        .step(
            "I compute par_tree_sum with 2 rayon worker threads",
            |w, _| {
                let xs = w.xs.clone().ok_or_else(|| StepError::new("no xs"))?;
                w.par_result = Some(install_pool(2, || par_tree_sum(&xs)));
                Ok(())
            },
        )
        .step(
            "I compute par_tree_sum with 8 rayon worker threads",
            |w, _| {
                let xs = w.xs.clone().ok_or_else(|| StepError::new("no xs"))?;
                w.par_result = Some(install_pool(8, || par_tree_sum(&xs)));
                Ok(())
            },
        )
        .step(
            "I compute par_tree_sum with 32 rayon worker threads",
            |w, _| {
                let xs = w.xs.clone().ok_or_else(|| StepError::new("no xs"))?;
                w.par_result = Some(install_pool(32, || par_tree_sum(&xs)));
                Ok(())
            },
        )
        .step(
            "I compute par_tree_sum again with 8 rayon worker threads",
            |w, _| {
                let xs = w.xs.clone().ok_or_else(|| StepError::new("no xs"))?;
                w.par_result_2 = Some(install_pool(8, || par_tree_sum(&xs)));
                Ok(())
            },
        )
        // ── par_tree_dot at various thread counts.
        .step(
            "I compute par_tree_dot with 8 rayon worker threads",
            |w, _| {
                let a = w.a.clone().ok_or_else(|| StepError::new("no a"))?;
                let b = w.b.clone().ok_or_else(|| StepError::new("no b"))?;
                w.par_result = Some(install_pool(8, || par_tree_dot(&a, &b)));
                Ok(())
            },
        )
        .step(
            "I compute par_tree_dot with 32 rayon worker threads",
            |w, _| {
                let a = w.a.clone().ok_or_else(|| StepError::new("no a"))?;
                let b = w.b.clone().ok_or_else(|| StepError::new("no b"))?;
                w.par_result = Some(install_pool(32, || par_tree_dot(&a, &b)));
                Ok(())
            },
        )
        // ── par_tree_var.
        .step(
            "I compute par_tree_var with 32 rayon worker threads",
            |w, _| {
                let xs = w.xs.clone().ok_or_else(|| StepError::new("no xs"))?;
                w.par_result = Some(install_pool(32, || par_tree_var(&xs)));
                Ok(())
            },
        )
        // ── Then variants — bit-identity assertions.
        .step(
            "the result is bit-identical to tree_sum on the same vector",
            |w, _| {
                let xs = w.xs.as_ref().ok_or_else(|| StepError::new("no xs"))?;
                let par = w
                    .par_result
                    .ok_or_else(|| StepError::new("no par_result"))?;
                let seq = tree_sum(xs);
                assert_bits_eq(par, seq, "par_tree_sum vs tree_sum")
            },
        )
        .step(
            "the result is bit-identical to tree_dot on the same vectors",
            |w, _| {
                let a = w.a.as_ref().ok_or_else(|| StepError::new("no a"))?;
                let b = w.b.as_ref().ok_or_else(|| StepError::new("no b"))?;
                let par = w
                    .par_result
                    .ok_or_else(|| StepError::new("no par_result"))?;
                let seq = tree_dot(a, b);
                assert_bits_eq(par, seq, "par_tree_dot vs tree_dot")
            },
        )
        .step(
            "the result is bit-identical to tree_var on the same vector",
            |w, _| {
                let xs = w.xs.as_ref().ok_or_else(|| StepError::new("no xs"))?;
                let par = w
                    .par_result
                    .ok_or_else(|| StepError::new("no par_result"))?;
                let seq = tree_var(xs);
                assert_bits_eq(par, seq, "par_tree_var vs tree_var")
            },
        )
        .step("both results are bit-identical", |w, _| {
            let p1 = w
                .par_result
                .ok_or_else(|| StepError::new("no par_result"))?;
            let p2 = w
                .par_result_2
                .ok_or_else(|| StepError::new("no par_result_2"))?;
            assert_bits_eq(p1, p2, "rerun")
        })
        .step("the result is bit-identical to 0.0", |w, _| {
            let par = w
                .par_result
                .ok_or_else(|| StepError::new("no par_result"))?;
            assert_bits_eq(par, 0.0, "empty input")
        })
        .step("the result is bit-identical to 1.5", |w, _| {
            let par = w
                .par_result
                .ok_or_else(|| StepError::new("no par_result"))?;
            assert_bits_eq(par, 1.5, "single-element input")
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
