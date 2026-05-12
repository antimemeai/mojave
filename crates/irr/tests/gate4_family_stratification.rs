use irr::family_stratification;
use irr::types::{MetricLevel, RatingMatrix};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::BTreeMap;

fn make_matrix(data: Vec<Vec<Option<u32>>>, n_raters: usize) -> RatingMatrix {
    let n_items = data.len();
    RatingMatrix {
        items: (0..n_items).map(|i| format!("item-{i}")).collect(),
        raters: (0..n_raters).map(|i| format!("r{i}")).collect(),
        ratings: data,
    }
}

fn make_family_map(
    n_raters: usize,
    families: &[(&str, std::ops::Range<usize>)],
) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for (fam, range) in families {
        for i in range.clone() {
            map.insert(format!("r{i}"), fam.to_string());
        }
    }
    assert_eq!(map.len(), n_raters);
    map
}

fn generate_biased_data(
    rng: &mut StdRng,
    n_items: usize,
    n_raters: usize,
    n_cats: u32,
    families: &BTreeMap<String, String>,
    within_prob: f64,
    between_prob: f64,
) -> Vec<Vec<Option<u32>>> {
    let mut family_groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for i in 0..n_raters {
        let fam = families.get(&format!("r{i}")).unwrap().clone();
        family_groups.entry(fam).or_default().push(i);
    }
    let fams: Vec<(String, Vec<usize>)> = family_groups.into_iter().collect();

    (0..n_items)
        .map(|_| {
            let truth: u32 = rng.random_range(0..n_cats);
            let mut fam_truths: BTreeMap<String, u32> = BTreeMap::new();
            for (f, _) in &fams {
                if rng.random_bool(between_prob) {
                    fam_truths.insert(f.clone(), truth);
                } else {
                    fam_truths.insert(f.clone(), rng.random_range(0..n_cats));
                }
            }
            let mut row = vec![None; n_raters];
            for (f, indices) in &fams {
                let ft = fam_truths[f];
                for &idx in indices {
                    if rng.random_bool(within_prob) {
                        row[idx] = Some(ft);
                    } else {
                        row[idx] = Some(rng.random_range(0..n_cats));
                    }
                }
            }
            row
        })
        .collect()
}

/// Gate 4: Bias-burden is monotone in the gap between within and between agreement.
#[test]
fn bias_burden_monotone_in_gap() {
    let n_items = 100;
    let n_raters = 6;
    let n_cats = 3;
    let families = make_family_map(n_raters, &[("A", 0..3), ("B", 3..6)]);

    let gaps = [(0.5, 0.5), (0.6, 0.4), (0.7, 0.3), (0.8, 0.2), (0.9, 0.1)];
    let mut prev_burden = f64::NEG_INFINITY;

    for (i, &(within, between)) in gaps.iter().enumerate() {
        let mut rng = StdRng::seed_from_u64(500 + i as u64);
        let data = generate_biased_data(
            &mut rng, n_items, n_raters, n_cats, &families, within, between,
        );
        let matrix = make_matrix(data, n_raters);
        let result =
            family_stratification::stratified_alpha(&matrix, &families, MetricLevel::Nominal);
        match result {
            Ok(r) => {
                eprintln!(
                    "within={within}, between={between}: burden={:.4}, within_mean={:.4}, between_alpha={:.4}",
                    r.bias_burden,
                    r.within_family.values().sum::<f64>() / r.within_family.len() as f64,
                    r.between_family_alpha
                );
                assert!(
                    r.bias_burden >= prev_burden - 0.15,
                    "burden not monotone: gap=({within},{between}), burden={}, prev={prev_burden}",
                    r.bias_burden
                );
                prev_burden = r.bias_burden;
            }
            Err(e) => panic!("unexpected error at within={within}, between={between}: {e}"),
        }
    }
}

/// Gate 4: Under no family structure, bias-burden is near zero across many trials.
///
/// Uses flat data generation (all raters independent from truth) so there is
/// no shared intermediate "family truth" to inflate within-family agreement.
#[test]
fn unbiased_burden_near_zero() {
    let n_trials = 100;
    let n_items = 50;
    let n_raters = 6;
    let n_cats = 3u32;
    let families = make_family_map(n_raters, &[("A", 0..3), ("B", 3..6)]);
    let agreement = 0.7;

    let mut burdens = Vec::new();
    let mut rng = StdRng::seed_from_u64(700);
    for _ in 0..n_trials {
        let data: Vec<Vec<Option<u32>>> = (0..n_items)
            .map(|_| {
                let truth: u32 = rng.random_range(0..n_cats);
                (0..n_raters)
                    .map(|_| {
                        if rng.random_bool(agreement) {
                            Some(truth)
                        } else {
                            Some(rng.random_range(0..n_cats))
                        }
                    })
                    .collect()
            })
            .collect();
        let matrix = make_matrix(data, n_raters);
        match family_stratification::stratified_alpha(&matrix, &families, MetricLevel::Nominal) {
            Ok(r) => burdens.push(r.bias_burden),
            Err(_) => continue,
        }
    }

    let mean_burden = burdens.iter().sum::<f64>() / burdens.len() as f64;
    eprintln!(
        "Unbiased MC: mean burden={mean_burden:.4}, n_successful={}/{}",
        burdens.len(),
        n_trials
    );
    assert!(
        mean_burden.abs() < 0.10,
        "mean burden under no bias = {mean_burden:.4}, expected near 0"
    );
}

/// Gate 4: Under strong bias, burden is reliably positive.
#[test]
fn biased_burden_reliably_positive() {
    let n_trials = 100;
    let n_items = 50;
    let n_raters = 6;
    let n_cats = 3;
    let families = make_family_map(n_raters, &[("A", 0..3), ("B", 3..6)]);

    let mut positive_count = 0;
    let mut rng = StdRng::seed_from_u64(800);
    for _ in 0..n_trials {
        let data = generate_biased_data(&mut rng, n_items, n_raters, n_cats, &families, 0.9, 0.3);
        let matrix = make_matrix(data, n_raters);
        match family_stratification::stratified_alpha(&matrix, &families, MetricLevel::Nominal) {
            Ok(r) => {
                if r.bias_burden > 0.0 {
                    positive_count += 1;
                }
            }
            Err(_) => continue,
        }
    }

    let positive_rate = positive_count as f64 / n_trials as f64;
    eprintln!("Biased MC: positive burden rate={positive_rate:.3} ({positive_count}/{n_trials})");
    assert!(
        positive_rate >= 0.90,
        "bias detection rate = {positive_rate:.3}, expected >= 0.90"
    );
}
