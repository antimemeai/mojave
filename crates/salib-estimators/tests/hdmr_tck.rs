#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

//! TCK integration tests for RS-HDMR via PCE decomposition.
//!
//! Implements the scenarios from `tck/salib/hdmr/features/hdmr.feature`.

use ndarray::Array2;
use salib_core::RngState;
use salib_estimators::{estimate_hdmr, HdmrResult};
use salib_samplers::{Sampler, SobolSampler};
use salib_surrogate::sobol_indices_from_pce;
use salib_validation::ishigami;

const FIXTURE_SEED: [u8; 32] = [0; 32];

/// Sample N points from Ishigami's Uniform(-π, π) distribution using Sobol.
fn ishigami_sample(n: usize) -> (Array2<f64>, Vec<f64>) {
    let problem = ishigami::input_distribution();
    let sampler = SobolSampler::standard(3);
    let mut rng = RngState::from_seed(FIXTURE_SEED);
    let unit = sampler.unit_sample(n, &mut rng);

    // Map [0,1] → [-π, π]
    let mut x = Array2::<f64>::zeros((n, 3));
    for i in 0..n {
        for j in 0..3 {
            let (lo, hi) = problem.factors()[j].distribution.support();
            x[[i, j]] = lo + unit[[i, j]] * (hi - lo);
        }
    }

    let y: Vec<f64> = x
        .rows()
        .into_iter()
        .map(|row| {
            let slice = row.as_slice().expect("contiguous");
            ishigami::ishigami(slice)
        })
        .collect();

    (x, y)
}

// ---------------------------------------------------------------------------
// Scenario 1: HDMR on Ishigami recovers first-order indices
// ---------------------------------------------------------------------------

#[test]
fn hdmr_ishigami_first_order() {
    let (x, y) = ishigami_sample(4096);
    let problem = ishigami::input_distribution();
    let analytic = ishigami::analytic_indices(7.0, 0.1);

    let result: HdmrResult = estimate_hdmr(&x, &y, &problem, 2, 6).unwrap();

    let tol = 0.05;
    assert!(
        (result.first_order[0] - analytic.first_order[0]).abs() < tol,
        "S_1: got {}, want {} ± {tol}",
        result.first_order[0],
        analytic.first_order[0]
    );
    assert!(
        (result.first_order[1] - analytic.first_order[1]).abs() < tol,
        "S_2: got {}, want {} ± {tol}",
        result.first_order[1],
        analytic.first_order[1]
    );
    assert!(
        (result.first_order[2] - analytic.first_order[2]).abs() < tol,
        "S_3: got {}, want {} ± {tol}",
        result.first_order[2],
        analytic.first_order[2]
    );
}

// ---------------------------------------------------------------------------
// Scenario 2: HDMR second-order matches Ishigami S2_13
// ---------------------------------------------------------------------------

#[test]
fn hdmr_ishigami_second_order() {
    let (x, y) = ishigami_sample(4096);
    let problem = ishigami::input_distribution();
    let analytic = ishigami::analytic_indices(7.0, 0.1);
    let analytic_s2 = analytic.second_order.as_ref().unwrap();
    // S2_{0,2} = analytic_s2[0][1] ≈ 0.244
    let expected_s2_02 = analytic_s2[0][1];

    let result = estimate_hdmr(&x, &y, &problem, 2, 6).unwrap();

    let tol = 0.05;
    // result.second_order[0][1] = S2_{0, 0+1+1} = S2_{0,2}
    assert!(
        (result.second_order[0][1] - expected_s2_02).abs() < tol,
        "S2_{{0,2}}: got {}, want {} ± {tol}",
        result.second_order[0][1],
        expected_s2_02
    );
}

// ---------------------------------------------------------------------------
// Scenario 3: HDMR component variances sum to ~1
// ---------------------------------------------------------------------------

#[test]
fn hdmr_order_variances_sum_to_one() {
    let (x, y) = ishigami_sample(1024);
    let problem = ishigami::input_distribution();

    let result = estimate_hdmr(&x, &y, &problem, 2, 4).unwrap();

    let sum: f64 = result.order_variance.iter().sum();
    // Sum of normalized order variances should be close to 1
    // (may not be exactly 1 if higher-order interactions exist beyond max_order)
    assert!(
        sum > 0.9 && sum <= 1.0 + 1e-10,
        "order_variance sum = {sum}, expected close to 1.0"
    );
}

// ---------------------------------------------------------------------------
// Scenario 4: HDMR agrees with PCE Sobol indices
// ---------------------------------------------------------------------------

#[test]
fn hdmr_agrees_with_pce_sobol() {
    let (x, y) = ishigami_sample(4096);
    let problem = ishigami::input_distribution();

    let hdmr = estimate_hdmr(&x, &y, &problem, 2, 6).unwrap();
    let pce_sobol = sobol_indices_from_pce(&hdmr.pce).unwrap();

    let tol = 0.001;
    for i in 0..3 {
        assert!(
            (hdmr.first_order[i] - pce_sobol.first_order[i]).abs() < tol,
            "S_{i}: HDMR={}, PCE={}",
            hdmr.first_order[i],
            pce_sobol.first_order[i]
        );
        assert!(
            (hdmr.total_order[i] - pce_sobol.total_order[i]).abs() < tol,
            "ST_{i}: HDMR={}, PCE={}",
            hdmr.total_order[i],
            pce_sobol.total_order[i]
        );
    }
}
