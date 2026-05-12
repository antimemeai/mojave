use irr::cohen;
use irr::gwet;
use irr::types::RatingMatrix;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Build a `RatingMatrix` from two rater vectors (2-rater, no missing data).
fn make_2rater_matrix(r1: &[u32], r2: &[u32]) -> RatingMatrix {
    assert_eq!(r1.len(), r2.len());
    let n = r1.len();
    RatingMatrix {
        items: (0..n).map(|i| format!("item-{i}")).collect(),
        raters: vec!["r0".to_string(), "r1".to_string()],
        ratings: (0..n).map(|i| vec![Some(r1[i]), Some(r2[i])]).collect(),
    }
}

/// Generate binary 2-rater data with controlled prevalence and agreement.
///
/// `prevalence` = P(truth = 0).  Each rater independently echoes the truth with
/// probability `agreement_prob`, otherwise flips.
fn generate_prevalence_data(
    rng: &mut StdRng,
    n_items: usize,
    prevalence: f64,
    agreement_prob: f64,
) -> (Vec<u32>, Vec<u32>) {
    let mut r1 = Vec::with_capacity(n_items);
    let mut r2 = Vec::with_capacity(n_items);
    for _ in 0..n_items {
        let truth: u32 = if rng.random_bool(prevalence) { 0 } else { 1 };
        let v1 = if rng.random_bool(agreement_prob) {
            truth
        } else {
            1 - truth
        };
        let v2 = if rng.random_bool(agreement_prob) {
            truth
        } else {
            1 - truth
        };
        r1.push(v1);
        r2.push(v2);
    }
    (r1, r2)
}

/// Gate 4 — Kappa paradox prevalence sweep.
///
/// The marquee demonstration of Gwet AC1 vs Cohen kappa: as prevalence becomes
/// imbalanced (holding true agreement fixed), kappa collapses while AC1 stays
/// stable.  This directly validates Gwet (2008) §4 and Feinstein & Cicchetti
/// (1990).
#[test]
fn kappa_paradox_prevalence_sweep() {
    let prevalence_levels = [0.5, 0.6, 0.7, 0.8, 0.9, 0.95];
    let agreement_prob = 0.85;
    let n_items = 200;
    let n_trials = 50;

    let mut mean_ac1s = Vec::new();
    let mut mean_kappas = Vec::new();

    for (pi, &prev) in prevalence_levels.iter().enumerate() {
        let mut ac1_sum = 0.0_f64;
        let mut kappa_sum = 0.0_f64;

        for trial in 0..n_trials {
            let seed = 1000 + (pi as u64) * 100 + trial as u64;
            let mut rng = StdRng::seed_from_u64(seed);
            let (r1, r2) = generate_prevalence_data(&mut rng, n_items, prev, agreement_prob);

            let matrix = make_2rater_matrix(&r1, &r2);
            let ac1 = gwet::ac(&matrix, None).expect("AC1 should not fail").value;
            let ck = cohen::kappa(&r1, &r2)
                .expect("Cohen kappa should not fail")
                .value;

            ac1_sum += ac1;
            kappa_sum += ck;
        }

        let mean_ac1 = ac1_sum / n_trials as f64;
        let mean_kappa = kappa_sum / n_trials as f64;
        eprintln!("prev={prev:.2}: mean AC1={mean_ac1:.4}, mean Cohen kappa={mean_kappa:.4}");
        mean_ac1s.push(mean_ac1);
        mean_kappas.push(mean_kappa);
    }

    // AC1 should be stable across prevalence levels
    let ac1_min = mean_ac1s.iter().copied().fold(f64::INFINITY, f64::min);
    let ac1_max = mean_ac1s.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let ac1_range = ac1_max - ac1_min;

    // Kappa should vary much more
    let kappa_min = mean_kappas.iter().copied().fold(f64::INFINITY, f64::min);
    let kappa_max = mean_kappas
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let kappa_range = kappa_max - kappa_min;

    eprintln!("AC1 range: {ac1_range:.4}  |  Kappa range: {kappa_range:.4}");

    assert!(
        ac1_range < 0.15,
        "AC1 should be stable across prevalence: range={ac1_range:.4} (want < 0.15)"
    );
    assert!(
        kappa_range > ac1_range,
        "Kappa should vary more than AC1: kappa_range={kappa_range:.4}, ac1_range={ac1_range:.4}"
    );
}

/// Gate 4 — AC1 is monotone increasing in agreement probability.
///
/// With prevalence held at uniform (3 categories, 4 raters), increasing the
/// agreement probability should produce monotonically increasing AC1 values.
#[test]
fn ac1_monotone_in_agreement() {
    let n_items = 200;
    let n_raters = 4;
    let n_cats = 3u32;
    let agreement_probs = [0.0, 0.2, 0.4, 0.6, 0.8];
    let mut prev_ac1 = f64::NEG_INFINITY;

    for (i, &p) in agreement_probs.iter().enumerate() {
        let mut rng = StdRng::seed_from_u64(2000 + i as u64);

        // Generate n_raters columns of ratings
        let data: Vec<Vec<Option<u32>>> = (0..n_items)
            .map(|_| {
                let truth: u32 = rng.random_range(0..n_cats);
                (0..n_raters)
                    .map(|_| {
                        if rng.random_bool(p) {
                            Some(truth)
                        } else {
                            Some(rng.random_range(0..n_cats))
                        }
                    })
                    .collect()
            })
            .collect();

        let matrix = RatingMatrix {
            items: (0..n_items).map(|j| format!("item-{j}")).collect(),
            raters: (0..n_raters).map(|j| format!("r{j}")).collect(),
            ratings: data,
        };

        let result = gwet::ac(&matrix, None);
        match result {
            Ok(r) => {
                eprintln!("AC1 at agreement={p:.1}: {:.4}", r.value);
                assert!(
                    r.value >= prev_ac1 - 0.05,
                    "AC1 not monotone: p={p}, ac1={}, prev={prev_ac1}",
                    r.value
                );
                prev_ac1 = r.value;
            }
            Err(e) => {
                panic!("unexpected error at p={p}: {e}");
            }
        }
    }
}
