use irr::bootstrap;
use irr::krippendorff;
use irr::types::{MetricLevel, RatingMatrix};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

fn make_matrix(data: Vec<Vec<Option<u32>>>, n_raters: usize) -> RatingMatrix {
    let n_items = data.len();
    RatingMatrix {
        items: (0..n_items).map(|i| format!("item-{i}")).collect(),
        raters: (0..n_raters).map(|i| format!("r{i}")).collect(),
        ratings: data,
    }
}

fn kripp_alpha_fn(m: &RatingMatrix) -> Result<f64, String> {
    krippendorff::alpha(m, Some(MetricLevel::Nominal))
        .map(|r| r.value)
        .map_err(|e| e.to_string())
}

fn generate_ratings(
    rng: &mut StdRng,
    n_items: usize,
    n_raters: usize,
    n_cats: u32,
    agreement_prob: f64,
) -> Vec<Vec<Option<u32>>> {
    (0..n_items)
        .map(|_| {
            let truth: u32 = rng.random_range(0..n_cats);
            (0..n_raters)
                .map(|_| {
                    if rng.random_bool(agreement_prob) {
                        Some(truth)
                    } else {
                        Some(rng.random_range(0..n_cats))
                    }
                })
                .collect()
        })
        .collect()
}

/// Gate 4: Monte Carlo coverage calibration for 95% bootstrap CI.
///
/// Generates a large population, computes alpha_true, then repeatedly
/// subsamples and checks how often the 95% CI covers the truth.
#[test]
fn coverage_calibration_95() {
    let mut pop_rng = StdRng::seed_from_u64(2025);
    let n_raters = 4;
    let n_cats = 3;
    let agreement_prob = 0.7;

    let pop_data = generate_ratings(&mut pop_rng, 5000, n_raters, n_cats, agreement_prob);
    let pop_matrix = make_matrix(pop_data, n_raters);
    let alpha_true = kripp_alpha_fn(&pop_matrix).expect("pop alpha failed");

    let n_trials = 200;
    let subsample_size = 30;
    let n_resamples = 200;
    let confidence = 0.95;
    let mut covers = 0usize;

    let mut trial_rng = StdRng::seed_from_u64(42);
    for trial in 0..n_trials {
        let sample_data = generate_ratings(
            &mut trial_rng,
            subsample_size,
            n_raters,
            n_cats,
            agreement_prob,
        );
        let sample_matrix = make_matrix(sample_data, n_raters);

        let boot_seed = trial as u64;
        match bootstrap::bootstrap_ci(
            &sample_matrix,
            kripp_alpha_fn,
            n_resamples,
            confidence,
            boot_seed,
        ) {
            Ok(ci) => {
                if ci.ci_lower <= alpha_true && alpha_true <= ci.ci_upper {
                    covers += 1;
                }
            }
            Err(_) => continue,
        }
    }

    let coverage = covers as f64 / n_trials as f64;
    assert!(
        coverage >= 0.85,
        "95% CI coverage = {coverage:.3} ({covers}/{n_trials}), expected >= 0.85"
    );
    assert!(
        coverage <= 1.0,
        "95% CI coverage = {coverage:.3}, should not exceed 1.0"
    );
    eprintln!(
        "Gate 4 coverage calibration: {coverage:.3} ({covers}/{n_trials}) for alpha_true={alpha_true:.4}"
    );
}

/// Gate 4: Type-I error rate — random data should have alpha near 0,
/// and the CI should include 0 at the nominal rate.
#[test]
fn type_i_error_rate() {
    let n_trials = 200;
    let n_items = 30;
    let n_raters = 4;
    let n_cats = 3;
    let n_resamples = 200;
    let confidence = 0.95;
    let mut includes_zero = 0usize;

    let mut rng = StdRng::seed_from_u64(99);
    for trial in 0..n_trials {
        let data: Vec<Vec<Option<u32>>> = (0..n_items)
            .map(|_| {
                (0..n_raters)
                    .map(|_| Some(rng.random_range(0..n_cats)))
                    .collect()
            })
            .collect();
        let matrix = make_matrix(data, n_raters);

        let boot_seed = trial as u64 + 1000;
        match bootstrap::bootstrap_ci(&matrix, kripp_alpha_fn, n_resamples, confidence, boot_seed) {
            Ok(ci) => {
                if ci.ci_lower <= 0.0 && 0.0 <= ci.ci_upper {
                    includes_zero += 1;
                }
            }
            Err(_) => continue,
        }
    }

    let inclusion_rate = includes_zero as f64 / n_trials as f64;
    assert!(
        inclusion_rate >= 0.85,
        "Type-I: CI includes 0 at rate {inclusion_rate:.3} ({includes_zero}/{n_trials}), expected >= 0.85"
    );
    eprintln!(
        "Gate 4 Type-I: CI includes 0 at rate {inclusion_rate:.3} ({includes_zero}/{n_trials})"
    );
}
