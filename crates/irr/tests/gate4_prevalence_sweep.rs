use irr::cohen;
use irr::fleiss;
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

/// Gate 4: Krippendorff alpha is monotone increasing in agreement probability.
#[test]
fn krippendorff_monotone_in_agreement() {
    let n_items = 200;
    let n_raters = 4;
    let n_cats = 3;
    let agreement_probs = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0];
    let mut prev_alpha = f64::NEG_INFINITY;

    for (i, &p) in agreement_probs.iter().enumerate() {
        let mut rng = StdRng::seed_from_u64(100 + i as u64);
        let data = generate_ratings(&mut rng, n_items, n_raters, n_cats, p);
        let matrix = make_matrix(data, n_raters);
        let result = krippendorff::alpha(&matrix, Some(MetricLevel::Nominal));
        match result {
            Ok(r) => {
                assert!(
                    r.value >= prev_alpha - 0.05,
                    "alpha not monotone: p={p}, alpha={}, prev={}",
                    r.value,
                    prev_alpha
                );
                prev_alpha = r.value;
                eprintln!("Krippendorff alpha at p={p}: {:.4}", r.value);
            }
            Err(e) => {
                if p == 1.0 {
                    eprintln!("p=1.0 degenerate (expected): {e}");
                } else {
                    panic!("unexpected error at p={p}: {e}");
                }
            }
        }
    }
}

/// Gate 4: Fleiss kappa is monotone increasing in agreement probability.
#[test]
fn fleiss_monotone_in_agreement() {
    let n_items = 200;
    let n_raters = 4;
    let n_cats = 3;
    let agreement_probs = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0];
    let mut prev_kappa = f64::NEG_INFINITY;

    for (i, &p) in agreement_probs.iter().enumerate() {
        let mut rng = StdRng::seed_from_u64(200 + i as u64);
        let data = generate_ratings(&mut rng, n_items, n_raters, n_cats, p);
        let matrix = make_matrix(data, n_raters);
        let result = fleiss::kappa(&matrix);
        match result {
            Ok(r) => {
                assert!(
                    r.value >= prev_kappa - 0.05,
                    "kappa not monotone: p={p}, kappa={}, prev={}",
                    r.value,
                    prev_kappa
                );
                prev_kappa = r.value;
                eprintln!("Fleiss kappa at p={p}: {:.4}", r.value);
            }
            Err(e) => {
                if p == 1.0 {
                    eprintln!("p=1.0 degenerate (expected): {e}");
                } else {
                    panic!("unexpected error at p={p}: {e}");
                }
            }
        }
    }
}

/// Gate 4: Cohen kappa is monotone increasing in agreement probability.
#[test]
fn cohen_monotone_in_agreement() {
    let n_items = 200;
    let n_cats = 3u32;
    let agreement_probs = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0];
    let mut prev_kappa = f64::NEG_INFINITY;

    for (i, &p) in agreement_probs.iter().enumerate() {
        let mut rng = StdRng::seed_from_u64(300 + i as u64);
        let r1: Vec<u32> = (0..n_items).map(|_| rng.random_range(0..n_cats)).collect();
        let r2: Vec<u32> = r1
            .iter()
            .map(|&v| {
                if rng.random_bool(p) {
                    v
                } else {
                    rng.random_range(0..n_cats)
                }
            })
            .collect();
        let result = cohen::kappa(&r1, &r2);
        match result {
            Ok(r) => {
                assert!(
                    r.value >= prev_kappa - 0.05,
                    "kappa not monotone: p={p}, kappa={}, prev={}",
                    r.value,
                    prev_kappa
                );
                prev_kappa = r.value;
                eprintln!("Cohen kappa at p={p}: {:.4}", r.value);
            }
            Err(e) => {
                if p == 1.0 {
                    eprintln!("p=1.0 degenerate (expected): {e}");
                } else {
                    panic!("unexpected error at p={p}: {e}");
                }
            }
        }
    }
}

/// Gate 4: All IRR statistics agree on direction for same data.
#[test]
fn irr_statistics_directional_consistency() {
    let n_items = 100;
    let n_cats = 3u32;

    for &agreement in &[0.3, 0.5, 0.7, 0.9] {
        let mut rng = StdRng::seed_from_u64((agreement * 1000.0) as u64);
        let data = generate_ratings(&mut rng, n_items, 2, n_cats, agreement);
        let matrix = make_matrix(data.clone(), 2);

        let r1: Vec<u32> = data.iter().map(|row| row[0].unwrap()).collect();
        let r2: Vec<u32> = data.iter().map(|row| row[1].unwrap()).collect();

        let ka = krippendorff::alpha(&matrix, Some(MetricLevel::Nominal))
            .map(|r| r.value)
            .unwrap_or(f64::NAN);
        let ck = cohen::kappa(&r1, &r2).map(|r| r.value).unwrap_or(f64::NAN);
        let fk = fleiss::kappa(&matrix).map(|r| r.value).unwrap_or(f64::NAN);

        eprintln!("p={agreement}: Krippendorff={ka:.4}, Cohen={ck:.4}, Fleiss={fk:.4}");

        if ka.is_finite() && ck.is_finite() {
            assert!(
                (ka - ck).abs() < 0.15,
                "Krippendorff ({ka:.4}) and Cohen ({ck:.4}) diverge too much at p={agreement}"
            );
        }
        if ck.is_finite() && fk.is_finite() {
            assert!(
                (ck - fk).abs() < 0.15,
                "Cohen ({ck:.4}) and Fleiss ({fk:.4}) diverge too much at p={agreement}"
            );
        }
    }
}
