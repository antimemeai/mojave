use cucumber::{given, then, when, World};
use irr::fleiss;
use irr::types::{IrrResult, RatingMatrix};

#[derive(Debug, Default, World)]
pub struct FleissWorld {
    matrix: Option<RatingMatrix>,
    result: Option<IrrResult>,
    error: Option<String>,
    kappa_values: Vec<f64>,
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

#[given(expr = "the Fleiss golden dataset {string}")]
fn given_golden(world: &mut FleissWorld, filename: String) {
    let path = format!("{}/tests/golden/{filename}", env!("CARGO_MANIFEST_DIR"));
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

    let counts = json["category_counts"].as_array().unwrap();
    let n_raters = json["n_raters"].as_u64().unwrap() as usize;

    let mut data: Vec<Vec<Option<u32>>> = Vec::new();
    for row in counts {
        let cat_counts: Vec<usize> = row
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        let mut ratings = Vec::with_capacity(n_raters);
        for (cat, &count) in cat_counts.iter().enumerate() {
            for _ in 0..count {
                ratings.push(Some(cat as u32));
            }
        }
        assert_eq!(ratings.len(), n_raters);
        data.push(ratings);
    }
    world.matrix = Some(make_matrix(data));
}

#[given(
    expr = "a Fleiss matrix where all {int} raters agree on each of {int} items across {int} categories"
)]
fn given_perfect(world: &mut FleissWorld, n_raters: usize, n_items: usize, n_cats: u32) {
    let data: Vec<Vec<Option<u32>>> = (0..n_items)
        .map(|i| vec![Some(i as u32 % n_cats); n_raters])
        .collect();
    world.matrix = Some(make_matrix(data));
}

#[given(expr = "a Fleiss matrix with {int} items {int} raters {int} categories seeded at {int}")]
fn given_random(world: &mut FleissWorld, n_items: usize, n_raters: usize, n_cats: u32, seed: u64) {
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

#[given("an empty Fleiss matrix")]
fn given_empty(world: &mut FleissWorld) {
    world.matrix = Some(RatingMatrix {
        items: vec![],
        raters: vec![],
        ratings: vec![],
    });
}

#[given("a Fleiss matrix with missing values")]
fn given_missing(world: &mut FleissWorld) {
    let data = vec![
        vec![Some(1), Some(2), None],
        vec![Some(2), Some(2), Some(1)],
    ];
    world.matrix = Some(make_matrix(data));
}

#[given("a Fleiss matrix with 1 rater")]
fn given_single_rater(world: &mut FleissWorld) {
    let data = vec![vec![Some(0)], vec![Some(1)], vec![Some(2)]];
    world.matrix = Some(make_matrix(data));
}

#[given("a Fleiss matrix where all raters assign the same category")]
fn given_degenerate(world: &mut FleissWorld) {
    let data = vec![
        vec![Some(0), Some(0), Some(0)],
        vec![Some(0), Some(0), Some(0)],
        vec![Some(0), Some(0), Some(0)],
    ];
    world.matrix = Some(make_matrix(data));
}

fn compute_fleiss(world: &mut FleissWorld) {
    match fleiss::kappa(world.matrix.as_ref().unwrap()) {
        Ok(r) => {
            world.kappa_values.push(r.value);
            world.result = Some(r);
        }
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute Fleiss kappa")]
fn when_compute(world: &mut FleissWorld) {
    compute_fleiss(world);
}

#[when("I permute the rater columns and compute Fleiss kappa again")]
fn when_permute(world: &mut FleissWorld) {
    let m = world.matrix.as_mut().unwrap();
    m.raters.reverse();
    for row in &mut m.ratings {
        row.reverse();
    }
    compute_fleiss(world);
}

#[then(expr = "kappa is approximately {float} with tolerance {float}")]
fn assert_approx(world: &mut FleissWorld, expected: f64, tol: f64) {
    let result = world.result.as_ref().expect("no result");
    assert!(
        (result.value - expected).abs() < tol,
        "kappa = {}, expected {} ± {}",
        result.value,
        expected,
        tol
    );
}

#[then(expr = "kappa is between {float} and {float}")]
fn assert_range(world: &mut FleissWorld, lo: f64, hi: f64) {
    let result = world.result.as_ref().expect("no result");
    assert!(
        result.value >= lo && result.value <= hi,
        "kappa = {}, expected in [{}, {}]",
        result.value,
        lo,
        hi
    );
}

#[then("both Fleiss kappa values are identical")]
fn assert_identical(world: &mut FleissWorld) {
    assert!(world.kappa_values.len() >= 2);
    let last = world.kappa_values.len();
    assert!(
        (world.kappa_values[last - 2] - world.kappa_values[last - 1]).abs() < 1e-12,
        "kappa values differ: {} vs {}",
        world.kappa_values[last - 2],
        world.kappa_values[last - 1]
    );
}

#[then("I get a Fleiss error about empty data")]
fn assert_empty_error(world: &mut FleissWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("empty"), "error: {err}");
}

#[then("I get a Fleiss error about missing data")]
fn assert_missing_error(world: &mut FleissWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("missing"), "error: {err}");
}

#[then("I get a Fleiss error about insufficient raters")]
fn assert_insufficient_raters(world: &mut FleissWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("at least 2 raters"), "error: {err}");
}

#[then("I get a Fleiss error about degenerate data")]
fn assert_degenerate(world: &mut FleissWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("degenerate"), "error: {err}");
}

fn main() {
    let runner = FleissWorld::run(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tck/irr"));
    futures::executor::block_on(runner);
}
