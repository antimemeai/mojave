use cucumber::{given, then, when, World};
use irr::cohen;
use irr::types::IrrResult;

#[derive(Debug, Default, World)]
pub struct CohenWorld {
    rater1: Vec<u32>,
    rater2: Vec<u32>,
    result: Option<IrrResult>,
    weighted_result: Option<IrrResult>,
    error: Option<String>,
    kappa_values: Vec<f64>,
}

#[given(expr = "two raters who agree perfectly on {int} items across {int} categories")]
fn given_perfect(world: &mut CohenWorld, n_items: usize, n_cats: u32) {
    world.rater1 = (0..n_items).map(|i| i as u32 % n_cats).collect();
    world.rater2 = world.rater1.clone();
}

#[given(expr = "two random raters on {int} items from {int} categories seeded at {int}")]
fn given_random(world: &mut CohenWorld, n_items: usize, n_cats: u32, seed: u64) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    world.rater1 = (0..n_items).map(|_| rng.random_range(0..n_cats)).collect();
    world.rater2 = (0..n_items).map(|_| rng.random_range(0..n_cats)).collect();
}

#[given(expr = "two raters with mixed agreement on {int} items seeded at {int}")]
fn given_mixed(world: &mut CohenWorld, n_items: usize, seed: u64) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    world.rater1 = (0..n_items).map(|_| rng.random_range(0..3u32)).collect();
    world.rater2 = world
        .rater1
        .iter()
        .map(|&v| {
            if rng.random_bool(0.7) {
                v
            } else {
                rng.random_range(0..3u32)
            }
        })
        .collect();
}

#[given(expr = "two raters on a {int}-point scale with {int} items seeded at {int}")]
fn given_ordinal(world: &mut CohenWorld, scale: u32, n_items: usize, seed: u64) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    world.rater1 = (0..n_items).map(|_| rng.random_range(0..scale)).collect();
    world.rater2 = world
        .rater1
        .iter()
        .map(|&v| {
            let noise: i32 = rng.random_range(-1..=1);
            (v as i32 + noise).clamp(0, scale as i32 - 1) as u32
        })
        .collect();
}

#[given("two empty rater vectors")]
fn given_empty(world: &mut CohenWorld) {
    world.rater1 = vec![];
    world.rater2 = vec![];
}

#[given(expr = "rater1 with {int} items and rater2 with {int} items")]
fn given_unequal(world: &mut CohenWorld, n1: usize, n2: usize) {
    world.rater1 = vec![0; n1];
    world.rater2 = vec![0; n2];
}

#[when("I compute Cohen kappa")]
fn when_cohen(world: &mut CohenWorld) {
    match cohen::kappa(&world.rater1, &world.rater2) {
        Ok(r) => {
            world.kappa_values.push(r.value);
            world.result = Some(r);
        }
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I swap the raters and compute Cohen kappa again")]
fn when_swap(world: &mut CohenWorld) {
    match cohen::kappa(&world.rater2, &world.rater1) {
        Ok(r) => {
            world.kappa_values.push(r.value);
            world.result = Some(r);
        }
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute Cohen weighted kappa with linear weights")]
fn when_weighted_linear(world: &mut CohenWorld) {
    match cohen::weighted_kappa(&world.rater1, &world.rater2, cohen::linear_weight) {
        Ok(r) => {
            world.kappa_values.push(r.value);
            world.weighted_result = Some(r);
        }
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute Cohen weighted kappa with quadratic weights")]
fn when_weighted_quadratic(world: &mut CohenWorld) {
    match cohen::weighted_kappa(&world.rater1, &world.rater2, cohen::quadratic_weight) {
        Ok(r) => {
            world.kappa_values.push(r.value);
            world.weighted_result = Some(r);
        }
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[then(expr = "Cohen kappa is approximately {float} with tolerance {float}")]
fn assert_approx(world: &mut CohenWorld, expected: f64, tol: f64) {
    let result = world.result.as_ref().expect("no result");
    assert!(
        (result.value - expected).abs() < tol,
        "kappa = {}, expected {} ± {}",
        result.value,
        expected,
        tol
    );
}

#[then(expr = "Cohen kappa is between {float} and {float}")]
fn assert_range(world: &mut CohenWorld, lo: f64, hi: f64) {
    let result = world.result.as_ref().expect("no result");
    assert!(
        result.value >= lo && result.value <= hi,
        "kappa = {}, expected in [{}, {}]",
        result.value,
        lo,
        hi
    );
}

#[then("both Cohen kappa values are identical")]
fn assert_identical(world: &mut CohenWorld) {
    assert!(world.kappa_values.len() >= 2);
    let last = world.kappa_values.len();
    assert!(
        (world.kappa_values[last - 2] - world.kappa_values[last - 1]).abs() < 1e-12,
        "kappa values differ: {} vs {}",
        world.kappa_values[last - 2],
        world.kappa_values[last - 1]
    );
}

#[then("Cohen kappa is a finite number")]
fn assert_finite(world: &mut CohenWorld) {
    let result = world
        .weighted_result
        .as_ref()
        .or(world.result.as_ref())
        .expect("no result");
    assert!(
        result.value.is_finite(),
        "kappa = {} is not finite",
        result.value
    );
}

#[then("weighted kappa is greater than or equal to unweighted kappa")]
fn assert_weighted_ge_unweighted(world: &mut CohenWorld) {
    let unweighted = world.result.as_ref().expect("no unweighted result").value;
    let weighted = world
        .weighted_result
        .as_ref()
        .expect("no weighted result")
        .value;
    assert!(
        weighted >= unweighted - 1e-10,
        "weighted kappa ({weighted}) < unweighted kappa ({unweighted})"
    );
}

#[then("I get a Cohen error about empty data")]
fn assert_empty(world: &mut CohenWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("empty"), "error: {err}");
}

#[then("I get a Cohen error about unequal length")]
fn assert_unequal(world: &mut CohenWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("equal length"), "error: {err}");
}

fn main() {
    let runner = CohenWorld::run("../../tck/irr");
    futures::executor::block_on(runner);
}
