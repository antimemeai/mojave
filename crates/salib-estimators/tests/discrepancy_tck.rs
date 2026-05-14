#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use ndarray::{array, Array2};
use salib_core::RngState;
use salib_estimators::{compute_discrepancy, DiscrepancyError};
use salib_samplers::{LhsSampler, Sampler, SobolSampler};

const FIXTURE_SEED: [u8; 32] = [0; 32];

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
