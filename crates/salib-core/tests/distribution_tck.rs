//! TCK harness for `Distribution` inverse-CDF properties.
//!
//! Wires `tck/salib/problem/features/inverse_cdf_round_trip.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::approx_constant
)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use salib_core::Distribution;

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("problem")
        .join("features")
        .join("inverse_cdf_round_trip.feature")
}

#[derive(Default)]
struct World {
    primary: Option<Distribution>,
    secondary: Option<Distribution>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("primary", &self.primary)
            .field("secondary", &self.secondary)
            .finish_non_exhaustive()
    }
}

fn check_quantile(d: &Distribution, u: f64, expected: f64) -> Result<(), StepError> {
    let got = d.quantile(u);
    if got == expected {
        Ok(())
    } else {
        Err(StepError::new(format!(
            "quantile({u}) on {d:?}: got {got}, expected {expected}"
        )))
    }
}

fn check_quantile_close(
    d: &Distribution,
    u: f64,
    expected: f64,
    tol: f64,
) -> Result<(), StepError> {
    let got = d.quantile(u);
    if (got - expected).abs() <= tol {
        Ok(())
    } else {
        Err(StepError::new(format!(
            "quantile({u}) on {d:?}: got {got}, expected ≈ {expected} (tol {tol})"
        )))
    }
}

fn primary(w: &World) -> Result<&Distribution, StepError> {
    w.primary
        .as_ref()
        .ok_or_else(|| StepError::new("no primary Distribution; check Given step"))
}

fn assert_monotone(d: &Distribution) -> Result<(), StepError> {
    let us = [
        0.0, 0.001, 0.01, 0.05, 0.1, 0.25, 0.5, 0.75, 0.9, 0.95, 0.99, 0.999, 1.0,
    ];
    let mut prev = f64::NEG_INFINITY;
    for &u in &us {
        let q = d.quantile(u);
        if q < prev {
            return Err(StepError::new(format!(
                "monotonicity violated on {d:?}: q({u}) = {q} < prev {prev}"
            )));
        }
        prev = q;
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
#[test]
fn inverse_cdf_round_trip_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "inverse_cdf_round_trip.feature")
        .expect("inverse_cdf_round_trip.feature parses cleanly");

    let runner = SyncRunner::new(World::default)
        // ── Givens ─────────────────────────────────────────────────
        .step("a Uniform distribution on [10, 30]", |w, _| {
            w.primary = Some(Distribution::Uniform { lo: 10.0, hi: 30.0 });
            Ok(())
        })
        .step("a Triangular distribution on [-1, 0, 1]", |w, _| {
            w.primary = Some(Distribution::Triangular {
                lo: -1.0,
                mode: 0.0,
                hi: 1.0,
            });
            Ok(())
        })
        .step("a Beta(2, 5) distribution on [0.5, 1.5]", |w, _| {
            w.primary = Some(Distribution::Beta {
                alpha: 2.0,
                beta: 5.0,
                lo: 0.5,
                hi: 1.5,
            });
            Ok(())
        })
        .step("a Normal distribution with mu 7.0 and sigma 2.0", |w, _| {
            w.primary = Some(Distribution::Normal {
                mu: 7.0,
                sigma: 2.0,
            });
            Ok(())
        })
        .step("a Beta(3, 3) distribution on [0, 1]", |w, _| {
            w.primary = Some(Distribution::Beta {
                alpha: 3.0,
                beta: 3.0,
                lo: 0.0,
                hi: 1.0,
            });
            Ok(())
        })
        .step(
            "a LogNormal distribution with mu_log 1.0 and sigma_log 0.5",
            |w, _| {
                w.primary = Some(Distribution::LogNormal {
                    mu_log: 1.0,
                    sigma_log: 0.5,
                });
                Ok(())
            },
        )
        .step("a Uniform distribution on [0, 1]", |w, _| {
            w.primary = Some(Distribution::Uniform { lo: 0.0, hi: 1.0 });
            Ok(())
        })
        .step("a Normal distribution with mu 0.0 and sigma 1.0", |w, _| {
            w.primary = Some(Distribution::Normal {
                mu: 0.0,
                sigma: 1.0,
            });
            Ok(())
        })
        .step("a Beta(2, 5) distribution on [0, 1]", |w, _| {
            w.primary = Some(Distribution::Beta {
                alpha: 2.0,
                beta: 5.0,
                lo: 0.0,
                hi: 1.0,
            });
            Ok(())
        })
        .step("a Bernoulli distribution with p 0.4", |w, _| {
            w.primary = Some(Distribution::Bernoulli { p: 0.4 });
            Ok(())
        })
        .step(
            "a Beta(1, 1) distribution on [0, 1] and a Uniform on [0, 1]",
            |w, _| {
                w.primary = Some(Distribution::Beta {
                    alpha: 1.0,
                    beta: 1.0,
                    lo: 0.0,
                    hi: 1.0,
                });
                w.secondary = Some(Distribution::Uniform { lo: 0.0, hi: 1.0 });
                Ok(())
            },
        )
        .step(
            "a Weibull(1, 4) distribution and an Exponential(0.25)",
            |w, _| {
                w.primary = Some(Distribution::Weibull {
                    shape: 1.0,
                    scale: 4.0,
                });
                w.secondary = Some(Distribution::Exponential { lambda: 0.25 });
                Ok(())
            },
        )
        .step(
            "a Gamma(1, 2) distribution and an Exponential(0.5)",
            |w, _| {
                w.primary = Some(Distribution::Gamma {
                    shape: 1.0,
                    scale: 2.0,
                });
                w.secondary = Some(Distribution::Exponential { lambda: 0.5 });
                Ok(())
            },
        )
        // ── Thens — exact-equal boundaries ────────────────────────
        .step("quantile(0.0) is 10.0", |w, _| {
            check_quantile(primary(w)?, 0.0, 10.0)
        })
        .step("quantile(0.5) is 20.0", |w, _| {
            check_quantile(primary(w)?, 0.5, 20.0)
        })
        .step("quantile(1.0) is 30.0", |w, _| {
            check_quantile(primary(w)?, 1.0, 30.0)
        })
        .step("quantile(0.0) is -1.0", |w, _| {
            check_quantile(primary(w)?, 0.0, -1.0)
        })
        .step("quantile(1.0) is 1.0", |w, _| {
            check_quantile(primary(w)?, 1.0, 1.0)
        })
        .step("quantile(-0.5) is 10.0", |w, _| {
            check_quantile(primary(w)?, -0.5, 10.0)
        })
        .step("quantile(1.5) is 30.0", |w, _| {
            check_quantile(primary(w)?, 1.5, 30.0)
        })
        // ── Thens — close-with-tolerance ──────────────────────────
        .step("quantile(0.0) is approximately 0.5 within 1e-9", |w, _| {
            check_quantile_close(primary(w)?, 0.0, 0.5, 1e-9)
        })
        .step("quantile(1.0) is approximately 1.5 within 1e-9", |w, _| {
            check_quantile_close(primary(w)?, 1.0, 1.5, 1e-9)
        })
        .step("quantile(0.5) is approximately 7.0 within 1e-9", |w, _| {
            check_quantile_close(primary(w)?, 0.5, 7.0, 1e-9)
        })
        .step("quantile(0.5) is approximately 0.5 within 1e-9", |w, _| {
            check_quantile_close(primary(w)?, 0.5, 0.5, 1e-9)
        })
        .step(
            "quantile(0.5) is approximately 2.718281828 within 1e-6",
            |w, _| check_quantile_close(primary(w)?, 0.5, 2.718_281_828, 1e-6),
        )
        // ── Thens — monotonicity ──────────────────────────────────
        .step(
            "quantile is monotone non-decreasing across [0, 1]",
            |w, _| assert_monotone(primary(w)?),
        )
        // ── Thens — special-case identities ───────────────────────
        .step(
            "their quantiles agree at 5 sample points within 1e-9",
            |w, _| {
                let p = primary(w)?;
                let s = w
                    .secondary
                    .as_ref()
                    .ok_or_else(|| StepError::new("no secondary"))?;
                for u in [0.1_f64, 0.3, 0.5, 0.7, 0.9] {
                    let qp = p.quantile(u);
                    let qs = s.quantile(u);
                    if (qp - qs).abs() > 1e-9 {
                        return Err(StepError::new(format!(
                            "agreement failed at u={u}: {p:?} → {qp}, {s:?} → {qs}"
                        )));
                    }
                }
                Ok(())
            },
        )
        .step(
            "their quantiles agree at 5 sample points within 1e-12",
            |w, _| {
                let p = primary(w)?;
                let s = w
                    .secondary
                    .as_ref()
                    .ok_or_else(|| StepError::new("no secondary"))?;
                for u in [0.1_f64, 0.3, 0.5, 0.7, 0.9] {
                    let qp = p.quantile(u);
                    let qs = s.quantile(u);
                    if (qp - qs).abs() > 1e-12 {
                        return Err(StepError::new(format!(
                            "agreement failed at u={u}: {p:?} → {qp}, {s:?} → {qs}"
                        )));
                    }
                }
                Ok(())
            },
        )
        .step(
            "their quantiles agree at 5 sample points within 1e-7",
            |w, _| {
                let p = primary(w)?;
                let s = w
                    .secondary
                    .as_ref()
                    .ok_or_else(|| StepError::new("no secondary"))?;
                for u in [0.1_f64, 0.3, 0.5, 0.7, 0.9] {
                    let qp = p.quantile(u);
                    let qs = s.quantile(u);
                    if (qp - qs).abs() > 1e-7 {
                        return Err(StepError::new(format!(
                            "agreement failed at u={u}: {p:?} → {qp}, {s:?} → {qs}"
                        )));
                    }
                }
                Ok(())
            },
        );

    let report = runner.run(&feature);
    report.assert_all_passed();
}
