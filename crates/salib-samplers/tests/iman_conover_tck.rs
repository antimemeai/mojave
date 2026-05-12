//! TCK harness for `iman_conover_transform` — marginal preservation,
//! correlation recovery, dependent-input Sobol' pipeline, and
//! determinism.
//!
//! Wires `tck/salib/iman-conover/features/iman_conover.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! ADR: `decisions/2026-04-29-saltelli-iman-conover.md`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::cast_precision_loss,
    clippy::similar_names
)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use ndarray::{array, Array2};
use rand::RngCore;
use salib_core::{Distribution, RngState};
use salib_estimators::estimate_given_data_sobol;
use salib_samplers::iman_conover_transform;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("iman-conover")
        .join("features")
        .join("iman_conover.feature")
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum InputKind {
    StandardNormal,
    Uniform01,
}

#[derive(Default)]
struct World {
    n: usize,
    input_kind: Option<InputKind>,
    input: Option<Array2<f64>>,
    target: Option<Array2<f64>>,
    output: Option<Array2<f64>>,
    output_b: Option<Array2<f64>>,
    sobol: Option<Vec<f64>>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("n", &self.n)
            .field("input_kind", &self.input_kind)
            .finish_non_exhaustive()
    }
}

fn standard_normal_samples(n: usize, d: usize, rng: &mut RngState) -> Array2<f64> {
    let mut chacha = rng.clone().into_chacha();
    let normal = Distribution::Normal {
        mu: 0.0,
        sigma: 1.0,
    };
    let u32_norm = 1.0_f64 / (f64::from(u32::MAX) + 1.0);
    let mut x = Array2::<f64>::zeros((n, d));
    for i in 0..n {
        for j in 0..d {
            let u = f64::from(chacha.next_u32()) * u32_norm;
            x[[i, j]] = normal.quantile(u);
        }
    }
    *rng = RngState::snapshot(&chacha, rng);
    x
}

fn uniform01_samples(n: usize, d: usize, rng: &mut RngState) -> Array2<f64> {
    let mut chacha = rng.clone().into_chacha();
    let u32_norm = 1.0_f64 / (f64::from(u32::MAX) + 1.0);
    let mut x = Array2::<f64>::zeros((n, d));
    for i in 0..n {
        for j in 0..d {
            x[[i, j]] = f64::from(chacha.next_u32()) * u32_norm;
        }
    }
    *rng = RngState::snapshot(&chacha, rng);
    x
}

fn pearson(x: &Array2<f64>, i: usize, j: usize) -> f64 {
    let n = x.nrows() as f64;
    let mean_i: f64 = (0..x.nrows()).map(|k| x[[k, i]]).sum::<f64>() / n;
    let mean_j: f64 = (0..x.nrows()).map(|k| x[[k, j]]).sum::<f64>() / n;
    let mut num = 0.0;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    for k in 0..x.nrows() {
        let dx = x[[k, i]] - mean_i;
        let dy = x[[k, j]] - mean_j;
        num += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    num / (sxx * syy).sqrt()
}

#[allow(clippy::too_many_lines)]
#[test]
fn iman_conover_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "iman_conover.feature").expect("parses");

    let runner = SyncRunner::new(World::default)
        .step(
            "N=1024 independent standard-normal samples on d=3",
            |w, _| {
                w.n = 1024;
                w.input_kind = Some(InputKind::StandardNormal);
                let mut rng = RngState::from_seed(FIXTURE_SEED);
                w.input = Some(standard_normal_samples(w.n, 3, &mut rng));
                Ok(())
            },
        )
        .step(
            "N=4096 independent standard-normal samples on d=3",
            |w, _| {
                w.n = 4096;
                w.input_kind = Some(InputKind::StandardNormal);
                let mut rng = RngState::from_seed(FIXTURE_SEED);
                w.input = Some(standard_normal_samples(w.n, 3, &mut rng));
                Ok(())
            },
        )
        .step(
            "N=8192 independent standard-normal samples on d=3",
            |w, _| {
                w.n = 8192;
                w.input_kind = Some(InputKind::StandardNormal);
                let mut rng = RngState::from_seed(FIXTURE_SEED);
                w.input = Some(standard_normal_samples(w.n, 3, &mut rng));
                Ok(())
            },
        )
        .step("N=2000 independent uniform samples on d=3", |w, _| {
            w.n = 2000;
            w.input_kind = Some(InputKind::Uniform01);
            let mut rng = RngState::from_seed(FIXTURE_SEED);
            w.input = Some(uniform01_samples(w.n, 3, &mut rng));
            Ok(())
        })
        .step("N=512 independent uniform samples on d=3", |w, _| {
            w.n = 512;
            w.input_kind = Some(InputKind::Uniform01);
            let mut rng = RngState::from_seed(FIXTURE_SEED);
            w.input = Some(uniform01_samples(w.n, 3, &mut rng));
            Ok(())
        })
        .step(
            "I apply Iman-Conover with target ρ_01=0.5, ρ_02=0.3, ρ_12=0.2",
            |w, _| {
                let x = w.input.as_ref().ok_or_else(|| StepError::new("no input"))?;
                let r = array![[1.0, 0.5, 0.3], [0.5, 1.0, 0.2], [0.3, 0.2, 1.0]];
                let mut rng = RngState::from_seed([7; 32]);
                w.output = Some(
                    iman_conover_transform(x, &r, &mut rng)
                        .map_err(|e| StepError::new(format!("IC: {e}")))?,
                );
                w.target = Some(r);
                Ok(())
            },
        )
        .step("I apply Iman-Conover with target ρ_01=0.6", |w, _| {
            let x = w.input.as_ref().ok_or_else(|| StepError::new("no input"))?;
            let r = array![[1.0, 0.6, 0.0], [0.6, 1.0, 0.0], [0.0, 0.0, 1.0]];
            let mut rng = RngState::from_seed([8; 32]);
            w.output = Some(
                iman_conover_transform(x, &r, &mut rng)
                    .map_err(|e| StepError::new(format!("IC: {e}")))?,
            );
            w.target = Some(r);
            Ok(())
        })
        .step(
            "I apply Iman-Conover with the identity correlation matrix",
            |w, _| {
                let x = w.input.as_ref().ok_or_else(|| StepError::new("no input"))?;
                let mut r = Array2::<f64>::zeros((3, 3));
                for i in 0..3 {
                    r[[i, i]] = 1.0;
                }
                let mut rng = RngState::from_seed([9; 32]);
                w.output = Some(
                    iman_conover_transform(x, &r, &mut rng)
                        .map_err(|e| StepError::new(format!("IC: {e}")))?,
                );
                Ok(())
            },
        )
        .step(
            "I apply Iman-Conover twice with the same RngState",
            |w, _| {
                let x = w.input.as_ref().ok_or_else(|| StepError::new("no input"))?;
                let r = array![[1.0, 0.4, 0.2], [0.4, 1.0, 0.1], [0.2, 0.1, 1.0]];
                let mut rng_a = RngState::from_seed([10; 32]);
                let mut rng_b = RngState::from_seed([10; 32]);
                w.output = Some(iman_conover_transform(x, &r, &mut rng_a).expect("a"));
                w.output_b = Some(iman_conover_transform(x, &r, &mut rng_b).expect("b"));
                Ok(())
            },
        )
        .step(
            "I evaluate Y = X_0 + X_1 + X_2 on the transformed samples",
            |w, _| {
                let out = w
                    .output
                    .as_ref()
                    .ok_or_else(|| StepError::new("no output"))?;
                let n = out.nrows();
                let y: Vec<f64> = (0..n)
                    .map(|k| out[[k, 0]] + out[[k, 1]] + out[[k, 2]])
                    .collect();
                let result = estimate_given_data_sobol(out, &y)
                    .map_err(|e| StepError::new(format!("Sobol: {e}")))?;
                w.sobol = Some(result.s1);
                Ok(())
            },
        )
        .step(
            "I estimate first-order Sobol' indices on the (X, Y) data",
            |_w, _| {
                // Already computed in the prior step — consolidated for
                // Gherkin readability.
                Ok(())
            },
        )
        .step(
            "each output column is a permutation of the corresponding input column",
            |w, _| {
                let x = w.input.as_ref().ok_or_else(|| StepError::new("no input"))?;
                let out = w
                    .output
                    .as_ref()
                    .ok_or_else(|| StepError::new("no output"))?;
                let n = x.nrows();
                let d = x.ncols();
                for j in 0..d {
                    let mut input_col: Vec<f64> = (0..n).map(|i| x[[i, j]]).collect();
                    let mut output_col: Vec<f64> = (0..n).map(|i| out[[i, j]]).collect();
                    input_col.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    output_col.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    for k in 0..n {
                        if (input_col[k] - output_col[k]).abs() > 1e-12 {
                            return Err(StepError::new(format!(
                                "column {j}: position {k} differs ({} vs {})",
                                input_col[k], output_col[k]
                            )));
                        }
                    }
                }
                Ok(())
            },
        )
        .step(
            "the output Pearson correlation between factors 0 and 1 is within 0.05 of 0.6",
            |w, _| {
                let out = w
                    .output
                    .as_ref()
                    .ok_or_else(|| StepError::new("no output"))?;
                let realized = pearson(out, 0, 1);
                if (realized - 0.6).abs() >= 0.05 {
                    return Err(StepError::new(format!("ρ = {realized:.3}")));
                }
                Ok(())
            },
        )
        .step(
            "every pairwise output Pearson correlation is below 0.1 in magnitude",
            |w, _| {
                let out = w
                    .output
                    .as_ref()
                    .ok_or_else(|| StepError::new("no output"))?;
                for i in 0..3 {
                    for j in (i + 1)..3 {
                        let rho = pearson(out, i, j);
                        if rho.abs() >= 0.1 {
                            return Err(StepError::new(format!("ρ({i},{j}) = {rho:.3}")));
                        }
                    }
                }
                Ok(())
            },
        )
        .step("S_0 approximates 0.610 within 0.10", |w, _| {
            let s = w.sobol.as_ref().ok_or_else(|| StepError::new("no Sobol"))?;
            if (s[0] - 0.610).abs() >= 0.10 {
                return Err(StepError::new(format!("S_0 = {:.3}", s[0])));
            }
            Ok(())
        })
        .step("S_1 approximates 0.610 within 0.10", |w, _| {
            let s = w.sobol.as_ref().ok_or_else(|| StepError::new("no Sobol"))?;
            if (s[1] - 0.610).abs() >= 0.10 {
                return Err(StepError::new(format!("S_1 = {:.3}", s[1])));
            }
            Ok(())
        })
        .step("S_2 approximates 0.238 within 0.10", |w, _| {
            let s = w.sobol.as_ref().ok_or_else(|| StepError::new("no Sobol"))?;
            if (s[2] - 0.238).abs() >= 0.10 {
                return Err(StepError::new(format!("S_2 = {:.3}", s[2])));
            }
            Ok(())
        })
        .step(
            "the sum of first-order Sobol' indices exceeds 1.0",
            |w, _| {
                let s = w.sobol.as_ref().ok_or_else(|| StepError::new("no Sobol"))?;
                let sum: f64 = s.iter().sum();
                if sum <= 1.0 {
                    return Err(StepError::new(format!("Σ S_i = {sum:.3}")));
                }
                Ok(())
            },
        )
        .step("the two output matrices are bit-identical", |w, _| {
            let a = w.output.as_ref().ok_or_else(|| StepError::new("no a"))?;
            let b = w.output_b.as_ref().ok_or_else(|| StepError::new("no b"))?;
            if a.shape() != b.shape() {
                return Err(StepError::new("shape differs"));
            }
            for i in 0..a.nrows() {
                for j in 0..a.ncols() {
                    if a[[i, j]] != b[[i, j]] {
                        return Err(StepError::new(format!("[{i},{j}] differs")));
                    }
                }
            }
            Ok(())
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
