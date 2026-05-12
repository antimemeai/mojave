use irr::bland_altman;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rand_distr::{Distribution, Normal};

/// Gate 4: Bland-Altman mean_diff converges to true offset as n grows.
///
/// Generates paired data with known offset and SD, verifies that
/// the recovered mean_diff error decreases with sample size and
/// is < 0.5 at n = 2000.
#[test]
fn convergence_to_true_parameters() {
    let true_offset = 3.0;
    let true_sd = 5.0;
    let sample_sizes = [20, 100, 500, 2000];
    let noise = Normal::new(0.0, true_sd).unwrap();

    let mut errors = Vec::new();

    for &n in &sample_sizes {
        let mut rng = StdRng::seed_from_u64(3000 + n as u64);

        let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.1).collect();
        let y: Vec<f64> = x
            .iter()
            .map(|&xi| xi - true_offset + noise.sample(&mut rng))
            .collect();

        let result = bland_altman::agreement(&x, &y).expect("agreement should succeed");
        let error = (result.mean_diff - true_offset).abs();

        eprintln!(
            "n={n:>4}: mean_diff={:.4}, error={:.4}",
            result.mean_diff, error
        );

        errors.push((n, error));
    }

    // Overall trend: error at largest n should be less than error at smallest n
    let first_error = errors.first().unwrap().1;
    let last_error = errors.last().unwrap().1;
    assert!(
        last_error < first_error,
        "error should decrease overall: n=20 error={first_error:.4}, n=2000 error={last_error:.4}"
    );

    // At n=2000, mean_diff should be within 0.5 of the true offset
    assert!(
        last_error < 0.5,
        "at n=2000, mean_diff error ({last_error:.4}) should be < 0.5"
    );
}

/// Gate 4: ~95% of differences fall within the computed LoA for normal data.
///
/// Runs 200 Monte Carlo trials of 100 observations each, computes coverage
/// of the LoA, and checks mean coverage is in [0.90, 1.0].
#[test]
fn loa_coverage_95_percent() {
    let n_trials = 200;
    let n_obs = 100;
    let true_offset = 2.0;
    let true_sd = 4.0;

    let mut rng = StdRng::seed_from_u64(4000);
    let diff_dist = Normal::new(true_offset, true_sd).unwrap();

    let mut coverages = Vec::with_capacity(n_trials);

    for trial in 0..n_trials {
        let x: Vec<f64> = (0..n_obs).map(|_| rng.random::<f64>() * 100.0).collect();
        let diffs: Vec<f64> = (0..n_obs).map(|_| diff_dist.sample(&mut rng)).collect();
        let y: Vec<f64> = x
            .iter()
            .zip(diffs.iter())
            .map(|(&xi, &di)| xi - di)
            .collect();

        let result = bland_altman::agreement(&x, &y).expect("agreement should succeed");

        // Count what fraction of actual differences fall within the computed LoA
        let actual_diffs: Vec<f64> = x.iter().zip(y.iter()).map(|(&xi, &yi)| xi - yi).collect();
        let within = actual_diffs
            .iter()
            .filter(|&&d| d >= result.lower_loa && d <= result.upper_loa)
            .count();
        let coverage = within as f64 / n_obs as f64;
        coverages.push(coverage);

        if trial < 5 {
            eprintln!(
                "trial {trial}: mean_diff={:.3}, sd={:.3}, coverage={:.3}",
                result.mean_diff, result.sd_diff, coverage
            );
        }
    }

    let mean_coverage: f64 = coverages.iter().sum::<f64>() / n_trials as f64;
    eprintln!("mean coverage across {n_trials} trials: {mean_coverage:.4}");

    assert!(
        (0.90..=1.0).contains(&mean_coverage),
        "mean coverage ({mean_coverage:.4}) should be between 0.90 and 1.0"
    );
}
