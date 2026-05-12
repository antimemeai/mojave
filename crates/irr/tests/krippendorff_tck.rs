use cucumber::{given, then, when, World};
use irr::krippendorff;
use irr::types::{IrrResult, MetricLevel, RatingMatrix};

#[derive(Debug, Default, World)]
pub struct KrippendorffWorld {
    matrix: Option<RatingMatrix>,
    result: Option<IrrResult>,
    error: Option<String>,
    alpha_values: Vec<f64>,
}

fn krippendorff_2011_data() -> Vec<Vec<Option<u32>>> {
    vec![
        vec![Some(1), Some(1), None],
        vec![Some(2), Some(2), Some(3)],
        vec![Some(3), Some(3), Some(3)],
        vec![Some(3), Some(3), Some(3)],
        vec![Some(2), Some(2), Some(2)],
        vec![Some(1), Some(2), Some(3)],
        vec![Some(4), Some(4), Some(4)],
        vec![Some(1), Some(1), Some(2)],
        vec![Some(2), Some(2), Some(2)],
        vec![None, Some(5), Some(5)],
        vec![None, None, Some(1)],
        vec![None, None, Some(3)],
    ]
}

fn make_matrix(data: Vec<Vec<Option<u32>>>) -> RatingMatrix {
    let n_items = data.len();
    let n_raters = data.first().map_or(0, |r| r.len());
    RatingMatrix {
        items: (0..n_items).map(|i| format!("item-{i}")).collect(),
        raters: (0..n_raters).map(|i| format!("r{i}")).collect(),
        ratings: data,
    }
}

#[given("the Krippendorff 2011 nominal dataset")]
fn given_krippendorff_2011(world: &mut KrippendorffWorld) {
    world.matrix = Some(make_matrix(krippendorff_2011_data()));
}

#[given("a rating matrix where all raters agree perfectly on 3 categories")]
fn given_perfect_agreement(world: &mut KrippendorffWorld) {
    let data: Vec<Vec<Option<u32>>> = (0..10)
        .map(|i| vec![Some(i % 3), Some(i % 3), Some(i % 3)])
        .collect();
    world.matrix = Some(make_matrix(data));
}

#[given(
    expr = "a {int}-item {int}-rater matrix with random labels from {int} categories seeded at {int}"
)]
fn given_random(
    world: &mut KrippendorffWorld,
    n_items: usize,
    n_raters: usize,
    n_cats: u32,
    seed: u64,
) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    let data: Vec<Vec<Option<u32>>> = (0..n_items)
        .map(|_| {
            (0..n_raters)
                .map(|_| Some(rng.random_range(0..n_cats)))
                .collect()
        })
        .collect();
    world.matrix = Some(make_matrix(data));
}

#[given("an empty rating matrix")]
fn given_empty(world: &mut KrippendorffWorld) {
    world.matrix = Some(RatingMatrix {
        items: vec![],
        raters: vec![],
        ratings: vec![],
    });
}

#[given(expr = "a rating matrix with {int} item and {int} raters all rating {int}")]
fn given_single_item(world: &mut KrippendorffWorld, _n: usize, n_raters: usize, val: u32) {
    world.matrix = Some(RatingMatrix {
        items: vec!["item-0".into()],
        raters: (0..n_raters).map(|i| format!("r{i}")).collect(),
        ratings: vec![vec![Some(val); n_raters]],
    });
}

#[given("a rating matrix where each item has only 1 rater")]
fn given_unpaired(world: &mut KrippendorffWorld) {
    let data = vec![
        vec![Some(1), None, None],
        vec![None, Some(2), None],
        vec![None, None, Some(3)],
    ];
    world.matrix = Some(make_matrix(data));
}

fn compute_alpha(world: &mut KrippendorffWorld, level: Option<MetricLevel>) {
    match krippendorff::alpha(world.matrix.as_ref().unwrap(), level) {
        Ok(r) => {
            world.alpha_values.push(r.value);
            world.result = Some(r);
        }
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute alpha with level nominal")]
fn compute_nominal(world: &mut KrippendorffWorld) {
    compute_alpha(world, Some(MetricLevel::Nominal));
}

#[when("I compute alpha with level interval")]
fn compute_interval(world: &mut KrippendorffWorld) {
    compute_alpha(world, Some(MetricLevel::Interval));
}

#[when("I compute alpha with level ordinal")]
fn compute_ordinal(world: &mut KrippendorffWorld) {
    compute_alpha(world, Some(MetricLevel::Ordinal));
}

#[when("I compute alpha with level ratio")]
fn compute_ratio(world: &mut KrippendorffWorld) {
    compute_alpha(world, Some(MetricLevel::Ratio));
}

#[when("I compute alpha without specifying a level")]
fn compute_no_level(world: &mut KrippendorffWorld) {
    compute_alpha(world, None);
}

#[when("I permute the rater columns and compute again")]
fn permute_raters_and_compute(world: &mut KrippendorffWorld) {
    let m = world.matrix.as_mut().unwrap();
    m.raters.reverse();
    for row in &mut m.ratings {
        row.reverse();
    }
    compute_alpha(world, Some(MetricLevel::Nominal));
}

#[when("I permute the item rows and compute again")]
fn permute_items_and_compute(world: &mut KrippendorffWorld) {
    let m = world.matrix.as_mut().unwrap();
    m.items.reverse();
    m.ratings.reverse();
    compute_alpha(world, Some(MetricLevel::Nominal));
}

#[then(expr = "alpha is approximately {float} with tolerance {float}")]
fn assert_approx(world: &mut KrippendorffWorld, expected: f64, tol: f64) {
    let result = world.result.as_ref().expect("no result");
    assert!(
        (result.value - expected).abs() < tol,
        "alpha = {}, expected {} ± {}",
        result.value,
        expected,
        tol
    );
}

#[then(expr = "alpha is between {float} and {float}")]
fn assert_range(world: &mut KrippendorffWorld, lo: f64, hi: f64) {
    let result = world.result.as_ref().expect("no result");
    assert!(
        result.value >= lo && result.value <= hi,
        "alpha = {}, expected in [{}, {}]",
        result.value,
        lo,
        hi
    );
}

#[then("both alpha values are identical")]
fn assert_identical(world: &mut KrippendorffWorld) {
    assert!(world.alpha_values.len() >= 2);
    let last = world.alpha_values.len();
    assert!(
        (world.alpha_values[last - 2] - world.alpha_values[last - 1]).abs() < 1e-12,
        "alpha values differ: {} vs {}",
        world.alpha_values[last - 2],
        world.alpha_values[last - 1]
    );
}

#[then("alpha is at most 1.0")]
fn assert_at_most_one(world: &mut KrippendorffWorld) {
    let result = world.result.as_ref().expect("no result");
    assert!(
        result.value <= 1.0 + 1e-12,
        "alpha = {} exceeds 1.0",
        result.value
    );
}

#[then("alpha is a finite number")]
fn assert_finite(world: &mut KrippendorffWorld) {
    let result = world.result.as_ref().expect("no result");
    assert!(
        result.value.is_finite(),
        "alpha = {} is not finite",
        result.value
    );
}

#[then("I get an error requiring metric level")]
fn assert_level_error(world: &mut KrippendorffWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("metric level"), "error: {err}");
}

#[then("I get an error about empty data")]
fn assert_empty_error(world: &mut KrippendorffWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("empty"), "error: {err}");
}

#[then("I get an error about degenerate data")]
fn assert_degenerate(world: &mut KrippendorffWorld) {
    let err = world.error.as_ref().expect("expected degenerate error");
    assert!(
        err.contains("degenerate") || err.contains("pairable"),
        "error: {err}"
    );
}

fn main() {
    let runner = KrippendorffWorld::run("../../tck/irr");
    futures::executor::block_on(runner);
}
