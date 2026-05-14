//! TCK harness for discrepancy indices — space-filling quality
//! metrics (CD, WD, MD, L2*).
//!
//! Wires `tck/salib/discrepancy/features/discrepancy.feature`
//! against [`metric_tck_harness::gherkin::SyncRunner`].

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use ndarray::{array, Array2};
use salib_core::RngState;
use salib_estimators::{compute_discrepancy, DiscrepancyError, DiscrepancyResult};
use salib_samplers::{LhsSampler, Sampler, SobolSampler};

const FIXTURE_SEED: [u8; 32] = [0; 32];

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("salib")
        .join("discrepancy")
        .join("features")
        .join("discrepancy.feature")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sobol_sample(n: usize, d: usize) -> Array2<f64> {
    let sampler = SobolSampler::standard(d);
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    sampler.unit_sample(n, &mut rng)
}

fn lhs_sample(n: usize, d: usize) -> Array2<f64> {
    let sampler = LhsSampler::classic(d);
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    sampler.unit_sample(n, &mut rng)
}

/// 2x2 regular grid in [0,1]^2: points at (0.25,0.25), (0.25,0.75),
/// (0.75,0.25), (0.75,0.75).
fn regular_grid_2d() -> Array2<f64> {
    array![[0.25, 0.25], [0.25, 0.75], [0.75, 0.25], [0.75, 0.75]]
}

// ---------------------------------------------------------------------------
// Scenario 1: Regular grid has known centered discrepancy
// ---------------------------------------------------------------------------

#[test]
fn regular_grid_centered_discrepancy() {
    let sample = regular_grid_2d();
    let result = compute_discrepancy(&sample).unwrap();

    // All four metrics should be positive for a 4-point grid.
    assert!(
        result.centered > 0.0,
        "CD should be positive, got {}",
        result.centered
    );
    assert!(
        result.wrap_around > 0.0,
        "WD should be positive, got {}",
        result.wrap_around
    );
    assert!(
        result.modified > 0.0,
        "MD should be positive, got {}",
        result.modified
    );
    assert!(
        result.l2_star > 0.0,
        "L2* should be positive, got {}",
        result.l2_star
    );

    // Analytic CD for the 2x2 centered grid.
    //
    // Each point has |x_k - 0.5| = 0.25 in both dims.
    // Per-dim single kernel: 1 + 0.125 - 0.03125 = 1.09375
    // Single sum product: 1.09375^2 = 1.19629..., times 4 = 4.78516...
    //
    // Double sum: 4 same-point (diff=0 both): (1.25)^2 = 1.5625, total 6.25
    //   8 pairs diff in 1 dim (0.5): 1.0 * 1.25 = 1.25, total 10.0
    //   4 pairs diff in 2 dims (0.5): 1.0 * 1.0 = 1.0, total 4.0
    //   Total = 20.25
    //
    // CD^2 = (13/12)^2 - (2/4)*4.78516 + (1/16)*20.25
    //      = 1.17361 - 2.39258 + 1.26563
    //      = 0.04666...
    // CD = 0.2160...
    let expected_cd = 0.216;
    assert!(
        (result.centered - expected_cd).abs() < 0.01,
        "CD = {}, expected ~{expected_cd}",
        result.centered,
    );
}

// ---------------------------------------------------------------------------
// Scenario 2: Sobol vs random -- Sobol should have lower CD
// ---------------------------------------------------------------------------

#[test]
fn sobol_lower_cd_than_random() {
    let sobol = sobol_sample(256, 3);
    let lhs = lhs_sample(256, 3);

    let r_sobol = compute_discrepancy(&sobol).unwrap();
    let r_lhs = compute_discrepancy(&lhs).unwrap();

    assert!(
        r_sobol.centered < r_lhs.centered,
        "Sobol CD ({}) should be less than LHS CD ({})",
        r_sobol.centered,
        r_lhs.centered
    );
}

// ---------------------------------------------------------------------------
// Scenario 3: Discrepancy decreases with N for Sobol
// ---------------------------------------------------------------------------

#[test]
fn discrepancy_decreases_with_n_sobol() {
    let sobol_64 = sobol_sample(64, 3);
    let sobol_256 = sobol_sample(256, 3);

    let r_64 = compute_discrepancy(&sobol_64).unwrap();
    let r_256 = compute_discrepancy(&sobol_256).unwrap();

    assert!(
        r_256.centered < r_64.centered,
        "CD at N=256 ({}) should be less than at N=64 ({})",
        r_256.centered,
        r_64.centered
    );
}

// ---------------------------------------------------------------------------
// Scenario 4: All discrepancy values are non-negative
// ---------------------------------------------------------------------------

#[test]
fn all_discrepancies_non_negative() {
    let samples: Vec<Array2<f64>> = vec![
        regular_grid_2d(),
        sobol_sample(32, 2),
        lhs_sample(16, 4),
        array![[0.0, 0.0], [1.0, 1.0]],
        array![[0.5, 0.5]],
    ];
    for (idx, sample) in samples.iter().enumerate() {
        let r = compute_discrepancy(sample).unwrap();
        assert!(r.centered >= 0.0, "sample {idx}: CD = {} < 0", r.centered);
        assert!(
            r.wrap_around >= 0.0,
            "sample {idx}: WD = {} < 0",
            r.wrap_around
        );
        assert!(r.modified >= 0.0, "sample {idx}: MD = {} < 0", r.modified);
        assert!(r.l2_star >= 0.0, "sample {idx}: L2* = {} < 0", r.l2_star);
    }
}

// ---------------------------------------------------------------------------
// Error paths
// ---------------------------------------------------------------------------

#[test]
fn error_empty_matrix() {
    let sample = Array2::<f64>::zeros((0, 3));
    assert!(matches!(
        compute_discrepancy(&sample),
        Err(DiscrepancyError::EmptyMatrix)
    ));
}

#[test]
fn error_out_of_range() {
    let sample = array![[0.5, -0.1]];
    assert!(matches!(
        compute_discrepancy(&sample),
        Err(DiscrepancyError::NotUnitInterval(v)) if v < 0.0
    ));

    let sample2 = array![[0.5, 1.01]];
    assert!(matches!(
        compute_discrepancy(&sample2),
        Err(DiscrepancyError::NotUnitInterval(v)) if v > 1.0
    ));
}

// ---------------------------------------------------------------------------
// Gherkin runner
// ---------------------------------------------------------------------------

#[derive(Default)]
struct World {
    grid_result: Option<DiscrepancyResult>,
    sobol_result: Option<DiscrepancyResult>,
    lhs_result: Option<DiscrepancyResult>,
    sobol_64_result: Option<DiscrepancyResult>,
    sobol_256_result: Option<DiscrepancyResult>,
}

#[allow(clippy::too_many_lines)]
#[test]
fn gherkin_discrepancy_feature_runs() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let feature = parse_feature(&content, "discrepancy.feature").expect("parses cleanly");

    let runner = SyncRunner::new(World::default)
        // ---- Scenario 1: Regular grid ----
        .step(
            "a 2D regular grid of 4 points in [0,1]^2",
            |_w: &mut World, _| Ok(()),
        )
        .step("I compute discrepancy", |w: &mut World, _| {
            // For the regular-grid scenario and the non-negative scenario.
            // Compute whichever grid is available.
            let sample = if w.grid_result.is_none() {
                regular_grid_2d()
            } else {
                // Second call (non-negative scenario) uses a Sobol sample.
                sobol_sample(32, 2)
            };
            let r = compute_discrepancy(&sample).map_err(|e| StepError::new(format!("{e}")))?;
            w.grid_result = Some(r);
            Ok(())
        })
        .step(
            "centered_discrepancy is within 0.01 of the analytic value",
            |w: &mut World, _| {
                let r = w
                    .grid_result
                    .as_ref()
                    .ok_or_else(|| StepError::new("no result"))?;
                let expected = 0.216;
                if (r.centered - expected).abs() >= 0.01 {
                    return Err(StepError::new(format!(
                        "CD = {}, expected ~{expected} (tol 0.01)",
                        r.centered,
                    )));
                }
                Ok(())
            },
        )
        // ---- Scenario 2: Sobol vs random ----
        .step("a Sobol sample of N=256 in d=3", |w: &mut World, _| {
            let sample = sobol_sample(256, 3);
            w.sobol_result = Some(compute_discrepancy(&sample).unwrap());
            Ok(())
        })
        .step("a random sample of N=256 in d=3", |w: &mut World, _| {
            let sample = lhs_sample(256, 3);
            w.lhs_result = Some(compute_discrepancy(&sample).unwrap());
            Ok(())
        })
        .step("I compute discrepancy for both", |_w: &mut World, _| {
            // Already computed in Given steps.
            Ok(())
        })
        .step(
            "the Sobol centered_discrepancy is less than the random centered_discrepancy",
            |w: &mut World, _| {
                let s = w
                    .sobol_result
                    .as_ref()
                    .ok_or_else(|| StepError::new("no sobol result"))?;
                let r = w
                    .lhs_result
                    .as_ref()
                    .ok_or_else(|| StepError::new("no lhs result"))?;
                if s.centered >= r.centered {
                    return Err(StepError::new(format!(
                        "Sobol CD ({}) >= LHS CD ({})",
                        s.centered, r.centered
                    )));
                }
                Ok(())
            },
        )
        // ---- Scenario 3: Monotone N ----
        .step(
            "Sobol samples at N=64 and N=256 in d=3",
            |w: &mut World, _| {
                let s64 = sobol_sample(64, 3);
                let s256 = sobol_sample(256, 3);
                w.sobol_64_result = Some(compute_discrepancy(&s64).unwrap());
                w.sobol_256_result = Some(compute_discrepancy(&s256).unwrap());
                Ok(())
            },
        )
        .step(
            "centered_discrepancy at N=256 is less than at N=64",
            |w: &mut World, _| {
                let r64 = w
                    .sobol_64_result
                    .as_ref()
                    .ok_or_else(|| StepError::new("no N=64 result"))?;
                let r256 = w
                    .sobol_256_result
                    .as_ref()
                    .ok_or_else(|| StepError::new("no N=256 result"))?;
                if r256.centered >= r64.centered {
                    return Err(StepError::new(format!(
                        "CD@256 ({}) >= CD@64 ({})",
                        r256.centered, r64.centered
                    )));
                }
                Ok(())
            },
        )
        // ---- Scenario 4: Non-negative ----
        .step("any sample matrix in [0,1]^d", |_w: &mut World, _| {
            // Will be computed in the "I compute discrepancy" step.
            Ok(())
        })
        .step(
            "centered, wrap_around, modified, and l2_star are all non-negative",
            |w: &mut World, _| {
                let r = w
                    .grid_result
                    .as_ref()
                    .ok_or_else(|| StepError::new("no result"))?;
                if r.centered < 0.0 || r.wrap_around < 0.0 || r.modified < 0.0 || r.l2_star < 0.0 {
                    return Err(StepError::new(format!(
                        "negative: CD={}, WD={}, MD={}, L2*={}",
                        r.centered, r.wrap_around, r.modified, r.l2_star
                    )));
                }
                Ok(())
            },
        );

    let report = runner.run(&feature);
    report.assert_all_passed();
}
