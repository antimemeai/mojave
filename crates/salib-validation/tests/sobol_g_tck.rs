//! TCK harness for Sobol' G's closed-form first-order Sobol' indices.
//!
//! Wires `tck/salib/validation/features/sobol_g_analytic.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::approx_constant
)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use salib_core::{Distribution, Problem};
use salib_validation::{sobol_g, SobolIndicesAnalytic};

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("validation")
        .join("features")
        .join("sobol_g_analytic.feature")
}

#[derive(Default)]
struct World {
    a_vector: Option<Vec<f64>>,
    indices: Option<SobolIndicesAnalytic>,
    distribution: Option<Problem>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("a_vector", &self.a_vector)
            .field("has_indices", &self.indices.is_some())
            .field("has_distribution", &self.distribution.is_some())
            .finish_non_exhaustive()
    }
}

fn require_indices(w: &World) -> Result<&SobolIndicesAnalytic, StepError> {
    w.indices
        .as_ref()
        .ok_or_else(|| StepError::new("no indices; check Given step"))
}

fn require_a(w: &World) -> Result<&Vec<f64>, StepError> {
    w.a_vector
        .as_ref()
        .ok_or_else(|| StepError::new("no a vector; check Given step"))
}

fn set_a(w: &mut World, a: Vec<f64>) {
    w.indices = Some(sobol_g::analytic_indices(&a));
    w.a_vector = Some(a);
}

#[allow(clippy::too_many_lines)]
#[test]
fn sobol_g_analytic_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "sobol_g_analytic.feature")
        .expect("sobol_g_analytic.feature parses cleanly");

    let runner = SyncRunner::new(World::default)
        // ── Givens — set a vector ──────────────────────────────────
        .step(
            "Sobol' G analytic indices with a vector [0, 1, 9]",
            |w, _| {
                set_a(w, vec![0.0, 1.0, 9.0]);
                Ok(())
            },
        )
        .step(
            "Sobol' G analytic indices with a vector [0, 1, 9, 99]",
            |w, _| {
                set_a(w, vec![0.0, 1.0, 9.0, 99.0]);
                Ok(())
            },
        )
        .step(
            "Sobol' G analytic indices with a vector [0, 1, 4.5, 9, 99]",
            |w, _| {
                set_a(w, vec![0.0, 1.0, 4.5, 9.0, 99.0]);
                Ok(())
            },
        )
        .step(
            "Sobol' G analytic indices with a vector [0, 99]",
            |w, _| {
                set_a(w, vec![0.0, 99.0]);
                Ok(())
            },
        )
        .step("Sobol' G analytic indices with a vector [1, 2]", |w, _| {
            set_a(w, vec![1.0, 2.0]);
            Ok(())
        })
        .step(
            "Sobol' G analytic indices with a vector [0, 1, 4.5, 9, 99, 99, 99, 99]",
            |w, _| {
                set_a(w, vec![0.0, 1.0, 4.5, 9.0, 99.0, 99.0, 99.0, 99.0]);
                Ok(())
            },
        )
        .step("the Sobol' G input distribution at dim 5", |w, _| {
            w.distribution = Some(sobol_g::input_distribution(5));
            Ok(())
        })
        // ── Thens ──────────────────────────────────────────────────
        .step(
            "for every factor i, V_i (recovered from S_i and D) is approximately (1/3) / (1 + a_i)² within 1e-9",
            |w, _| {
                let s = require_indices(w)?;
                let a = require_a(w)?;
                for (i, ai) in a.iter().enumerate() {
                    let v_i = s.first_order[i] * s.total_variance;
                    let want = (1.0 / 3.0) / (1.0 + ai).powi(2);
                    if (v_i - want).abs() > 1e-9 {
                        return Err(StepError::new(format!(
                            "V_{i}: got {v_i}, want {want}"
                        )));
                    }
                }
                Ok(())
            },
        )
        .step(
            "D equals (1 + 1/3)(1 + 1/12)(1 + 1/300) - 1 within 1e-12",
            |w, _| {
                let s = require_indices(w)?;
                let want = (1.0 + 1.0 / 3.0) * (1.0 + 1.0 / 12.0) * (1.0 + 1.0 / 300.0) - 1.0;
                if (s.total_variance - want).abs() <= 1e-12 {
                    Ok(())
                } else {
                    Err(StepError::new(format!(
                        "D = {}, want {want}",
                        s.total_variance
                    )))
                }
            },
        )
        .step("the factors are ranked S_1 > S_2 > S_3 > S_4", |w, _| {
            let s = require_indices(w)?;
            for i in 0..3 {
                if s.first_order[i] <= s.first_order[i + 1] {
                    return Err(StepError::new(format!(
                        "ranking violated: S_{i} = {} not > S_{} = {}",
                        s.first_order[i],
                        i + 1,
                        s.first_order[i + 1]
                    )));
                }
            }
            Ok(())
        })
        .step("every first-order index is positive", |w, _| {
            let s = require_indices(w)?;
            for v in &s.first_order {
                if *v <= 0.0 {
                    return Err(StepError::new(format!("non-positive S_i = {v}")));
                }
            }
            Ok(())
        })
        .step("the sum of first-order indices is at most 1", |w, _| {
            let s = require_indices(w)?;
            let sum: f64 = s.first_order.iter().sum();
            if sum <= 1.0 + 1e-12 {
                Ok(())
            } else {
                Err(StepError::new(format!("Σ S_i = {sum} > 1")))
            }
        })
        .step("S_2 is below 0.01", |w, _| {
            let s = require_indices(w)?;
            if s.first_order[1] < 0.01 {
                Ok(())
            } else {
                Err(StepError::new(format!("S_2 = {}", s.first_order[1])))
            }
        })
        .step("every total-order index is NaN", |w, _| {
            let s = require_indices(w)?;
            for v in &s.total_order {
                if !v.is_nan() {
                    return Err(StepError::new(format!("non-NaN total-order: {v}")));
                }
            }
            Ok(())
        })
        .step(
            "the first four factors strictly dominate the last four",
            |w, _| {
                let s = require_indices(w)?;
                let min_first_four = s.first_order[..4]
                    .iter()
                    .copied()
                    .fold(f64::INFINITY, f64::min);
                let max_last_four = s.first_order[4..]
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max);
                if min_first_four > max_last_four {
                    Ok(())
                } else {
                    Err(StepError::new(format!(
                        "min first-four = {min_first_four}, max last-four = {max_last_four}"
                    )))
                }
            },
        )
        .step(
            "every last-four factor first-order index is below 1e-3",
            |w, _| {
                let s = require_indices(w)?;
                for (i, v) in s.first_order[4..].iter().enumerate() {
                    if *v >= 1e-3 {
                        return Err(StepError::new(format!(
                            "S_{} = {v} not below 1e-3",
                            i + 4
                        )));
                    }
                }
                Ok(())
            },
        )
        .step("it has 5 factors named x1 through x5", |w, _| {
            let p = w
                .distribution
                .as_ref()
                .ok_or_else(|| StepError::new("no distribution"))?;
            let names: Vec<&str> = p.factors().iter().map(|f| f.name.as_str()).collect();
            if names == vec!["x1", "x2", "x3", "x4", "x5"] {
                Ok(())
            } else {
                Err(StepError::new(format!("got names {names:?}")))
            }
        })
        .step("every factor is Uniform(0, 1)", |w, _| {
            let p = w
                .distribution
                .as_ref()
                .ok_or_else(|| StepError::new("no distribution"))?;
            for f in p.factors() {
                match &f.distribution {
                    Distribution::Uniform { lo, hi } => {
                        if *lo != 0.0 || *hi != 1.0 {
                            return Err(StepError::new(format!(
                                "factor {} bounds {lo} {hi} not (0, 1)",
                                f.name
                            )));
                        }
                    }
                    other => return Err(StepError::new(format!("non-Uniform: {other:?}"))),
                }
            }
            Ok(())
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}

#[allow(dead_code)]
fn _force_link() -> f64 {
    sobol_g::sobol_g(&[0.5, 0.5], &[1.0, 1.0])
}
