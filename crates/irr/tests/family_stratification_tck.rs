use cucumber::{given, then, when, World};
use irr::family_stratification::{self, StratifiedAlphaResult};
use irr::krippendorff;
use irr::types::{MetricLevel, RatingMatrix};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::BTreeMap;

#[derive(Debug, Default, World)]
pub struct StratWorld {
    matrix: Option<RatingMatrix>,
    rater_families: BTreeMap<String, String>,
    result: Option<StratifiedAlphaResult>,
    error: Option<String>,
    n_items: usize,
    n_raters: usize,
    within_agreement: f64,
    between_agreement: f64,
    uniform_agreement: f64,
    seed: u64,
    biased: bool,
}

fn make_matrix(data: Vec<Vec<Option<u32>>>, n_raters: usize) -> RatingMatrix {
    let n_items = data.len();
    RatingMatrix {
        items: (0..n_items).map(|i| format!("item-{i}")).collect(),
        raters: (0..n_raters).map(|i| format!("r{i}")).collect(),
        ratings: data,
    }
}

fn generate_biased_ratings(
    rng: &mut StdRng,
    n_items: usize,
    n_raters: usize,
    n_cats: u32,
    rater_families: &BTreeMap<String, String>,
    within_agreement: f64,
    between_agreement: f64,
) -> Vec<Vec<Option<u32>>> {
    let mut family_groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for i in 0..n_raters {
        let rater_name = format!("r{i}");
        let family = rater_families.get(&rater_name).cloned().unwrap_or_default();
        family_groups.entry(family).or_default().push(i);
    }

    let families: Vec<(String, Vec<usize>)> = family_groups.into_iter().collect();

    (0..n_items)
        .map(|_| {
            let truth: u32 = rng.random_range(0..n_cats);
            let mut family_truths: BTreeMap<String, u32> = BTreeMap::new();
            for (fam, _) in &families {
                if rng.random_bool(between_agreement) {
                    family_truths.insert(fam.clone(), truth);
                } else {
                    family_truths.insert(fam.clone(), rng.random_range(0..n_cats));
                }
            }

            let mut row = vec![None; n_raters];
            for (fam, indices) in &families {
                let fam_truth = family_truths[fam];
                for &idx in indices {
                    if rng.random_bool(within_agreement) {
                        row[idx] = Some(fam_truth);
                    } else {
                        row[idx] = Some(rng.random_range(0..n_cats));
                    }
                }
            }
            row
        })
        .collect()
}

fn generate_uniform_ratings(
    rng: &mut StdRng,
    n_items: usize,
    n_raters: usize,
    n_cats: u32,
    agreement: f64,
) -> Vec<Vec<Option<u32>>> {
    (0..n_items)
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
        .collect()
}

// --- Given steps ---

#[given(expr = "a {int}-item rating matrix with {int} raters")]
fn given_matrix(world: &mut StratWorld, n_items: usize, n_raters: usize) {
    world.n_items = n_items;
    world.n_raters = n_raters;
}

#[given(expr = "raters r0,r1,r2 belong to family {string}")]
fn given_family_012(world: &mut StratWorld, family: String) {
    for i in 0..3 {
        world.rater_families.insert(format!("r{i}"), family.clone());
    }
}

#[given(expr = "raters r3,r4,r5 belong to family {string}")]
fn given_family_345(world: &mut StratWorld, family: String) {
    for i in 3..6 {
        world.rater_families.insert(format!("r{i}"), family.clone());
    }
}

#[given(expr = "raters r0,r1 belong to family {string}")]
fn given_family_01(world: &mut StratWorld, family: String) {
    for i in 0..2 {
        world.rater_families.insert(format!("r{i}"), family.clone());
    }
}

#[given(expr = "raters r2,r3 belong to family {string}")]
fn given_family_23(world: &mut StratWorld, family: String) {
    for i in 2..4 {
        world.rater_families.insert(format!("r{i}"), family.clone());
    }
}

#[given(expr = "raters r4,r5 belong to family {string}")]
fn given_family_45(world: &mut StratWorld, family: String) {
    for i in 4..6 {
        world.rater_families.insert(format!("r{i}"), family.clone());
    }
}

#[given(expr = "rater r3 belongs to family {string}")]
fn given_family_3(world: &mut StratWorld, family: String) {
    world.rater_families.insert("r3".to_string(), family);
}

#[given(expr = "all raters belong to family {string}")]
fn given_all_same_family(world: &mut StratWorld, family: String) {
    for i in 0..world.n_raters {
        world.rater_families.insert(format!("r{i}"), family.clone());
    }
}

#[given(expr = "within-family agreement is {float} and between-family agreement is {float}")]
fn given_biased_agreement(world: &mut StratWorld, within: f64, between: f64) {
    world.within_agreement = within;
    world.between_agreement = between;
    world.biased = true;
}

#[given(expr = "all raters agree with probability {float} regardless of family")]
fn given_uniform_agreement(world: &mut StratWorld, prob: f64) {
    world.uniform_agreement = prob;
    world.biased = false;
}

#[given(expr = "the data is seeded at {int}")]
fn given_seed(world: &mut StratWorld, seed: u64) {
    world.seed = seed;
    let mut rng = StdRng::seed_from_u64(seed);
    let n_cats = 3u32;

    let data = if world.biased {
        generate_biased_ratings(
            &mut rng,
            world.n_items,
            world.n_raters,
            n_cats,
            &world.rater_families,
            world.within_agreement,
            world.between_agreement,
        )
    } else {
        generate_uniform_ratings(
            &mut rng,
            world.n_items,
            world.n_raters,
            n_cats,
            world.uniform_agreement,
        )
    };
    world.matrix = Some(make_matrix(data, world.n_raters));
}

#[given("an empty rating matrix for stratification")]
fn given_empty(world: &mut StratWorld) {
    world.matrix = Some(RatingMatrix {
        items: vec![],
        raters: vec![],
        ratings: vec![],
    });
}

// --- When steps ---

#[when("I compute family-stratified alpha with level nominal")]
fn when_compute(world: &mut StratWorld) {
    let m = world.matrix.as_ref().unwrap();
    match family_stratification::stratified_alpha(m, &world.rater_families, MetricLevel::Nominal) {
        Ok(r) => world.result = Some(r),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I attempt family-stratified alpha with level nominal")]
fn when_attempt(world: &mut StratWorld) {
    let m = world.matrix.as_ref().unwrap();
    match family_stratification::stratified_alpha(m, &world.rater_families, MetricLevel::Nominal) {
        Ok(r) => world.result = Some(r),
        Err(e) => world.error = Some(e.to_string()),
    }
}

// --- Then steps ---

#[then(expr = "within-family alpha for {string} is greater than {float}")]
fn then_within_gt(world: &mut StratWorld, family: String, threshold: f64) {
    let r = world.result.as_ref().expect("no result");
    let val = r
        .within_family
        .get(&family)
        .unwrap_or_else(|| panic!("no within-family alpha for {family}"));
    assert!(
        *val > threshold,
        "within-family alpha for {family} = {val}, expected > {threshold}"
    );
}

#[then(expr = "between-family alpha is less than {float}")]
fn then_between_lt(world: &mut StratWorld, threshold: f64) {
    let r = world.result.as_ref().expect("no result");
    assert!(
        r.between_family_alpha < threshold,
        "between-family alpha = {}, expected < {}",
        r.between_family_alpha,
        threshold
    );
}

#[then(expr = "bias-burden is greater than {float}")]
fn then_bias_gt(world: &mut StratWorld, threshold: f64) {
    let r = world.result.as_ref().expect("no result");
    assert!(
        r.bias_burden > threshold,
        "bias-burden = {}, expected > {}",
        r.bias_burden,
        threshold
    );
}

#[then(expr = "bias-burden is between {float} and {float}")]
fn then_bias_between(world: &mut StratWorld, lo: f64, hi: f64) {
    let r = world.result.as_ref().expect("no result");
    assert!(
        r.bias_burden >= lo && r.bias_burden <= hi,
        "bias-burden = {}, expected in [{}, {}]",
        r.bias_burden,
        lo,
        hi
    );
}

#[then("overall alpha matches a direct Krippendorff alpha computation within 0.001")]
fn then_overall_matches(world: &mut StratWorld) {
    let r = world.result.as_ref().expect("no result");
    let m = world.matrix.as_ref().unwrap();
    let direct = krippendorff::alpha(m, Some(MetricLevel::Nominal))
        .expect("direct alpha failed")
        .value;
    assert!(
        (r.overall_alpha - direct).abs() < 0.001,
        "overall_alpha = {}, direct = {}",
        r.overall_alpha,
        direct
    );
}

#[then(expr = "I get a stratification error about too few families")]
fn then_error_families(world: &mut StratWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("2 families"), "error: {err}");
}

#[then(expr = "within-family alpha for {string} is defined")]
fn then_within_defined(world: &mut StratWorld, family: String) {
    let r = world.result.as_ref().expect("no result");
    assert!(
        r.within_family.contains_key(&family),
        "within-family alpha for {family} not defined"
    );
}

#[then(expr = "within-family alpha for {string} is not defined")]
fn then_within_not_defined(world: &mut StratWorld, family: String) {
    let r = world.result.as_ref().expect("no result");
    assert!(
        !r.within_family.contains_key(&family),
        "within-family alpha for {family} should not be defined (single rater)"
    );
}

#[then("between-family alpha is defined")]
fn then_between_defined(world: &mut StratWorld) {
    let r = world.result.as_ref().expect("no result");
    assert!(
        r.between_family_alpha.is_finite(),
        "between-family alpha is not finite: {}",
        r.between_family_alpha
    );
}

#[then("I get a stratification error about unmapped rater")]
fn then_error_unmapped(world: &mut StratWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("not found in family map"), "error: {err}");
}

#[then("I get a stratification error about empty data")]
fn then_error_empty(world: &mut StratWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("empty"), "error: {err}");
}

fn main() {
    let runner = StratWorld::run(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tck/irr/family_stratification.feature"
    ));
    futures::executor::block_on(runner);
}
