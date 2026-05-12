//! TCK harness for `fit_sparse_pce` — Ishigami at canonical
//! `(a=7, b=0.1)` + a sparse-additive `d=10` recovery scenario.
//!
//! Wires `tck/salib/sparse-pce-estimator/features/sparse_pce.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! ADR: `decisions/2026-04-29-saltelli-sparse-pce.md`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::needless_range_loop,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]

use std::f64::consts::PI;
use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use ndarray::Array2;
use salib_core::RngState;
use salib_samplers::{LhsSampler, Sampler, SobolSampler};
use salib_surrogate::multi_index::total_degree_basis_size;
use salib_surrogate::{
    enumerate_hyperbolic, fit_sparse_pce, sobol_indices_from_pce, PolynomialChaos,
    PolynomialFamily, SobolFromPce, SparseFitDiagnostic, SparseSolver, TruncationScheme,
};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("sparse-pce-estimator")
        .join("features")
        .join("sparse_pce.feature")
}

fn ishigami_canonical_inputs(n: usize) -> (Array2<f64>, Vec<f64>) {
    let sampler = SobolSampler::standard(3).with_skip_first(false);
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let unit = sampler.unit_sample(n, &mut rng);
    let mut x = Array2::<f64>::zeros((n, 3));
    let mut y = Vec::with_capacity(n);
    for i in 0..n {
        for k in 0..3 {
            x[[i, k]] = 2.0 * unit[[i, k]] - 1.0;
        }
        let x_real = [PI * x[[i, 0]], PI * x[[i, 1]], PI * x[[i, 2]]];
        y.push(ishigami::ishigami(&x_real));
    }
    (x, y)
}

fn additive_d10_inputs(n: usize) -> (Array2<f64>, Vec<f64>) {
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let unit = LhsSampler::classic(10).unit_sample(n, &mut rng);
    let mut x = Array2::<f64>::zeros((n, 10));
    for i in 0..n {
        for j in 0..10 {
            x[[i, j]] = 2.0 * unit[[i, j]] - 1.0;
        }
    }
    let y: Vec<f64> = (0..n)
        .map(|i| x[[i, 0]] + 0.5 * x[[i, 2]] + 2.0 * x[[i, 4]])
        .collect();
    (x, y)
}

#[derive(Default)]
struct World {
    pce: Option<PolynomialChaos>,
    sobol: Option<SobolFromPce>,
    diag: Option<SparseFitDiagnostic>,
    pce_b: Option<PolynomialChaos>,
    using_additive_d10: bool,
    n: usize,
    hyperbolic_basis_size: Option<usize>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("has_pce", &self.pce.is_some())
            .field("has_sobol", &self.sobol.is_some())
            .field("has_diag", &self.diag.is_some())
            .finish_non_exhaustive()
    }
}

fn require_sobol(w: &World) -> Result<&SobolFromPce, StepError> {
    w.sobol
        .as_ref()
        .ok_or_else(|| StepError::new("no Sobol' indices computed"))
}

fn require_diag(w: &World) -> Result<&SparseFitDiagnostic, StepError> {
    w.diag
        .as_ref()
        .ok_or_else(|| StepError::new("no diagnostic"))
}

fn require_pce(w: &World) -> Result<&PolynomialChaos, StepError> {
    w.pce.as_ref().ok_or_else(|| StepError::new("no PCE fit"))
}

#[test]
fn sparse_pce_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "sparse_pce.feature").expect("parses");

    let runner = SyncRunner::new(World::default)
        .step(
            "the Ishigami model on Uniform[-π, π]³ mapped to Legendre canonical [-1, 1]³",
            |w, _| {
                w.using_additive_d10 = false;
                Ok(())
            },
        )
        .step("Sobol' QMC samples at N=4096", |w, _| {
            w.n = 4096;
            Ok(())
        })
        .step(
            "the additive model Y = ξ_0 + 0.5·ξ_2 + 2·ξ_4 on d=10",
            |w, _| {
                w.using_additive_d10 = true;
                Ok(())
            },
        )
        .step("LHS samples at N=512", |w, _| {
            w.n = 512;
            Ok(())
        })
        .step(
            "I fit a sparse Legendre PCE of total degree 10 with hyperbolic q=0.75",
            |w, _| {
                let (x, y) = if w.using_additive_d10 {
                    additive_d10_inputs(w.n)
                } else {
                    ishigami_canonical_inputs(w.n)
                };
                let d = x.ncols();
                let (pce, diag) = fit_sparse_pce(
                    &x,
                    &y,
                    &vec![PolynomialFamily::Legendre; d],
                    10,
                    TruncationScheme::Hyperbolic { q: 0.75 },
                    SparseSolver::Omp,
                    None,
                )
                .map_err(|e| StepError::new(format!("fit: {e}")))?;
                w.sobol = Some(
                    sobol_indices_from_pce(&pce)
                        .map_err(|e| StepError::new(format!("Sobol: {e}")))?,
                );
                w.pce = Some(pce);
                w.diag = Some(diag);
                Ok(())
            },
        )
        .step("I fit a sparse Legendre PCE of total degree 4", |w, _| {
            let (x, y) = if w.using_additive_d10 {
                additive_d10_inputs(w.n)
            } else {
                ishigami_canonical_inputs(w.n)
            };
            let d = x.ncols();
            let (pce, diag) = fit_sparse_pce(
                &x,
                &y,
                &vec![PolynomialFamily::Legendre; d],
                4,
                TruncationScheme::TotalDegree,
                SparseSolver::Omp,
                None,
            )
            .map_err(|e| StepError::new(format!("fit: {e}")))?;
            w.sobol = Some(
                sobol_indices_from_pce(&pce).map_err(|e| StepError::new(format!("Sobol: {e}")))?,
            );
            w.pce = Some(pce);
            w.diag = Some(diag);
            Ok(())
        })
        .step(
            "I compute Sobol' indices from the PCE coefficients",
            |w, _| {
                let pce = require_pce(w)?;
                w.sobol = Some(
                    sobol_indices_from_pce(pce)
                        .map_err(|e| StepError::new(format!("Sobol: {e}")))?,
                );
                Ok(())
            },
        )
        .step("S_1 approximates 0.3139 within 0.02", |w, _| {
            let s = require_sobol(w)?;
            let err = (s.first_order[0] - 0.3139).abs();
            if err >= 0.02 {
                return Err(StepError::new(format!("S_1 err = {err:.4}")));
            }
            Ok(())
        })
        .step("S_2 approximates 0.4424 within 0.02", |w, _| {
            let s = require_sobol(w)?;
            let err = (s.first_order[1] - 0.4424).abs();
            if err >= 0.02 {
                return Err(StepError::new(format!("S_2 err = {err:.4}")));
            }
            Ok(())
        })
        .step("S_3 approximates 0.0 within 0.02", |w, _| {
            let s = require_sobol(w)?;
            if s.first_order[2].abs() >= 0.02 {
                return Err(StepError::new(format!("S_3 = {:.4}", s.first_order[2])));
            }
            Ok(())
        })
        .step(
            "the sparse PCE keeps at most 80 non-zero coefficients out of 286 candidates",
            |w, _| {
                let d = require_diag(w)?;
                if d.num_active > 80 {
                    return Err(StepError::new(format!("active = {} > 80", d.num_active)));
                }
                Ok(())
            },
        )
        .step(
            "only factors 0, 2, 4 carry non-trivial first-order Sobol' indices",
            |w, _| {
                let s = require_sobol(w)?;
                for i in 0..10 {
                    let active_factor = matches!(i, 0 | 2 | 4);
                    if active_factor {
                        if s.first_order[i] < 0.01 {
                            return Err(StepError::new(format!(
                                "factor {i} should be active but S = {:.4}",
                                s.first_order[i]
                            )));
                        }
                    } else if s.first_order[i] > 0.01 {
                        return Err(StepError::new(format!(
                            "factor {i} should be inactive but S = {:.4}",
                            s.first_order[i]
                        )));
                    }
                }
                Ok(())
            },
        )
        .step("factor 4 dominates over factor 0 over factor 2", |w, _| {
            let s = require_sobol(w)?;
            if s.first_order[4] <= s.first_order[0] {
                return Err(StepError::new(format!(
                    "S_4 ({}) should exceed S_0 ({})",
                    s.first_order[4], s.first_order[0]
                )));
            }
            if s.first_order[0] <= s.first_order[2] {
                return Err(StepError::new(format!(
                    "S_0 ({}) should exceed S_2 ({})",
                    s.first_order[0], s.first_order[2]
                )));
            }
            Ok(())
        })
        .step(
            "I enumerate the hyperbolic-truncated basis at d=10, p=4, q=0.5",
            |w, _| {
                let basis = enumerate_hyperbolic(10, 4, 0.5)
                    .map_err(|e| StepError::new(format!("enum: {e}")))?;
                w.hyperbolic_basis_size = Some(basis.len());
                Ok(())
            },
        )
        .step("the basis size is at most 200", |w, _| {
            let size = w
                .hyperbolic_basis_size
                .ok_or_else(|| StepError::new("no hyperbolic size"))?;
            if size > 200 {
                return Err(StepError::new(format!("size = {size} > 200")));
            }
            Ok(())
        })
        .step(
            "the basis size for total-degree truncation at d=10, p=4 is 1001",
            |_w, _| {
                let s = total_degree_basis_size(10, 4);
                if s != 1001 {
                    return Err(StepError::new(format!("size = {s} ≠ 1001")));
                }
                Ok(())
            },
        )
        .step(
            "I fit a sparse Legendre PCE of total degree 10 with hyperbolic q=0.75 twice",
            |w, _| {
                let (x, y) = ishigami_canonical_inputs(w.n);
                let (pce_a, _) = fit_sparse_pce(
                    &x,
                    &y,
                    &[PolynomialFamily::Legendre; 3],
                    10,
                    TruncationScheme::Hyperbolic { q: 0.75 },
                    SparseSolver::Omp,
                    None,
                )
                .map_err(|e| StepError::new(format!("fit a: {e}")))?;
                let (pce_b, _) = fit_sparse_pce(
                    &x,
                    &y,
                    &[PolynomialFamily::Legendre; 3],
                    10,
                    TruncationScheme::Hyperbolic { q: 0.75 },
                    SparseSolver::Omp,
                    None,
                )
                .map_err(|e| StepError::new(format!("fit b: {e}")))?;
                w.pce = Some(pce_a);
                w.pce_b = Some(pce_b);
                Ok(())
            },
        )
        .step("the two PCEs have identical coefficients", |w, _| {
            let a = require_pce(w)?;
            let b = w
                .pce_b
                .as_ref()
                .ok_or_else(|| StepError::new("no second PCE"))?;
            if a.coefficients != b.coefficients {
                return Err(StepError::new("coefficients differ"));
            }
            Ok(())
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
