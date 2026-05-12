//! TCK harness for `fit_full_pce` + `sobol_indices_from_pce` —
//! Ishigami at canonical `(a=7, b=0.1)`.
//!
//! Wires `tck/salib/pce-estimator/features/pce_ishigami.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].
//!
//! ADR: `decisions/2026-04-29-saltelli-pce-fit.md`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::needless_range_loop,
    clippy::cast_precision_loss
)]

use std::f64::consts::PI;
use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use ndarray::Array2;
use salib_core::RngState;
use salib_samplers::{Sampler, SobolSampler};
use salib_surrogate::{
    fit_full_pce, sobol_indices_from_pce, PolynomialChaos, PolynomialFamily, SobolFromPce,
};
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("pce-estimator")
        .join("features")
        .join("pce_ishigami.feature")
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

#[derive(Default)]
struct World {
    n: usize,
    pce: Option<PolynomialChaos>,
    sobol: Option<SobolFromPce>,
    pce_b: Option<PolynomialChaos>,
    sobol_b: Option<SobolFromPce>,
    low_p: Option<f64>,
    high_p: Option<f64>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("n", &self.n)
            .field("has_pce", &self.pce.is_some())
            .field("has_sobol", &self.sobol.is_some())
            .finish_non_exhaustive()
    }
}

fn require_pce(w: &World) -> Result<&PolynomialChaos, StepError> {
    w.pce.as_ref().ok_or_else(|| StepError::new("no PCE fit"))
}

fn require_sobol(w: &World) -> Result<&SobolFromPce, StepError> {
    w.sobol
        .as_ref()
        .ok_or_else(|| StepError::new("no Sobol' indices computed"))
}

fn fit_pce(n: usize, p: usize) -> PolynomialChaos {
    let (x, y) = ishigami_canonical_inputs(n);
    fit_full_pce(&x, &y, &[PolynomialFamily::Legendre; 3], p).expect("PCE fit")
}

#[allow(clippy::too_many_lines)]
#[test]
fn pce_ishigami_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "pce_ishigami.feature").expect("parses");

    let runner = SyncRunner::new(World::default)
        .step(
            "the Ishigami model on Uniform[-π, π]³ mapped to Legendre canonical [-1, 1]³",
            |_w, _| Ok(()),
        )
        .step("Sobol' QMC samples at N=4096", |w, _| {
            w.n = 4096;
            Ok(())
        })
        .step("I fit a Legendre PCE of total degree 10", |w, _| {
            w.pce = Some(fit_pce(w.n, 10));
            Ok(())
        })
        .step(
            "I compute Sobol' indices from the PCE coefficients",
            |w, _| {
                let pce = require_pce(w)?;
                w.sobol = Some(
                    sobol_indices_from_pce(pce)
                        .map_err(|e| StepError::new(format!("Sobol from PCE: {e}")))?,
                );
                Ok(())
            },
        )
        .step("S_1 approximates 0.3139 within 0.01", |w, _| {
            let s = require_sobol(w)?;
            let err = (s.first_order[0] - 0.3139).abs();
            if err >= 0.01 {
                return Err(StepError::new(format!(
                    "S_1 = {:.4}, want 0.3139 within 0.01 (err = {:.4})",
                    s.first_order[0], err
                )));
            }
            Ok(())
        })
        .step("S_2 approximates 0.4424 within 0.01", |w, _| {
            let s = require_sobol(w)?;
            let err = (s.first_order[1] - 0.4424).abs();
            if err >= 0.01 {
                return Err(StepError::new(format!(
                    "S_2 = {:.4}, want 0.4424 within 0.01 (err = {:.4})",
                    s.first_order[1], err
                )));
            }
            Ok(())
        })
        .step("S_3 approximates 0.0 within 0.01", |w, _| {
            let s = require_sobol(w)?;
            if s.first_order[2].abs() >= 0.01 {
                return Err(StepError::new(format!(
                    "S_3 = {:.4}, want 0 within 0.01",
                    s.first_order[2]
                )));
            }
            Ok(())
        })
        .step("S_T_1 approximates 0.5576 within 0.01", |w, _| {
            let s = require_sobol(w)?;
            let err = (s.total_order[0] - 0.5576).abs();
            if err >= 0.01 {
                return Err(StepError::new(format!(
                    "S_T_1 = {:.4}, want 0.5576 within 0.01 (err = {:.4})",
                    s.total_order[0], err
                )));
            }
            Ok(())
        })
        .step("S_T_2 approximates 0.4424 within 0.01", |w, _| {
            let s = require_sobol(w)?;
            let err = (s.total_order[1] - 0.4424).abs();
            if err >= 0.01 {
                return Err(StepError::new(format!(
                    "S_T_2 = {:.4}, want 0.4424 within 0.01 (err = {:.4})",
                    s.total_order[1], err
                )));
            }
            Ok(())
        })
        .step("S_T_3 approximates 0.2436 within 0.01", |w, _| {
            let s = require_sobol(w)?;
            let err = (s.total_order[2] - 0.2436).abs();
            if err >= 0.01 {
                return Err(StepError::new(format!(
                    "S_T_3 = {:.4}, want 0.2436 within 0.01 (err = {:.4})",
                    s.total_order[2], err
                )));
            }
            Ok(())
        })
        .step("every first-order is at most its total-order", |w, _| {
            let s = require_sobol(w)?;
            for i in 0..3 {
                if s.first_order[i] > s.total_order[i] + 1e-9 {
                    return Err(StepError::new(format!(
                        "S_{i} = {} > S_T_{i} = {}",
                        s.first_order[i], s.total_order[i]
                    )));
                }
            }
            Ok(())
        })
        .step("the sum of first-order indices is at most 1", |w, _| {
            let s = require_sobol(w)?;
            let sum: f64 = s.first_order.iter().sum();
            if sum > 1.0 + 1e-9 {
                return Err(StepError::new(format!("Σ S_i = {sum} > 1")));
            }
            Ok(())
        })
        .step(
            "I fit a Legendre PCE of total degree 4 with N=256",
            |w, _| {
                let pce = fit_pce(256, 4);
                w.sobol = Some(sobol_indices_from_pce(&pce).expect("Sobol"));
                Ok(())
            },
        )
        .step("I record S_1 as low_p", |w, _| {
            w.low_p = Some(require_sobol(w)?.first_order[0]);
            Ok(())
        })
        .step(
            "I fit a Legendre PCE of total degree 10 with N=4096",
            |w, _| {
                let pce = fit_pce(4096, 10);
                w.sobol = Some(sobol_indices_from_pce(&pce).expect("Sobol"));
                Ok(())
            },
        )
        .step("I record S_1 as high_p", |w, _| {
            w.high_p = Some(require_sobol(w)?.first_order[0]);
            Ok(())
        })
        .step(
            "high_p is closer than low_p to the analytic value 0.3139",
            |w, _| {
                let low = w.low_p.ok_or_else(|| StepError::new("low_p not set"))?;
                let high = w.high_p.ok_or_else(|| StepError::new("high_p not set"))?;
                let err_low = (low - 0.3139).abs();
                let err_high = (high - 0.3139).abs();
                if err_high >= err_low {
                    return Err(StepError::new(format!(
                        "convergence: err(p=10) = {err_high:.4} should be < err(p=4) = {err_low:.4}"
                    )));
                }
                Ok(())
            },
        )
        .step("I fit a Legendre PCE of total degree 10 twice", |w, _| {
            let pce_a = fit_pce(w.n, 10);
            let pce_b = fit_pce(w.n, 10);
            w.sobol = Some(sobol_indices_from_pce(&pce_a).expect("Sobol a"));
            w.sobol_b = Some(sobol_indices_from_pce(&pce_b).expect("Sobol b"));
            w.pce = Some(pce_a);
            w.pce_b = Some(pce_b);
            Ok(())
        })
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
        })
        .step("the two Sobol' index sets are identical", |w, _| {
            let a = require_sobol(w)?;
            let b = w
                .sobol_b
                .as_ref()
                .ok_or_else(|| StepError::new("no second Sobol"))?;
            if a.first_order != b.first_order || a.total_order != b.total_order {
                return Err(StepError::new("Sobol' indices differ"));
            }
            Ok(())
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
