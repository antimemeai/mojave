use cucumber::{given, then, when, World};
use irr::bootstrap::{self, BootstrapCiResult};
use irr::krippendorff;
use irr::types::{MetricLevel, RatingMatrix};

#[derive(Debug, Default, World)]
pub struct BootstrapWorld {
    matrix: Option<RatingMatrix>,
    result: Option<BootstrapCiResult>,
    result2: Option<BootstrapCiResult>,
    ci_90: Option<BootstrapCiResult>,
    ci_99: Option<BootstrapCiResult>,
    error: Option<String>,
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

fn kripp_alpha_fn(m: &RatingMatrix) -> Result<f64, String> {
    krippendorff::alpha(m, Some(MetricLevel::Nominal))
        .map(|r| r.value)
        .map_err(|e| e.to_string())
}

// --- Given steps ---

#[given(
    expr = "a rating matrix with mixed agreement on {int} items and {int} raters seeded at {int}"
)]
fn given_mixed(world: &mut BootstrapWorld, n_items: usize, n_raters: usize, seed: u64) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    let data: Vec<Vec<Option<u32>>> = (0..n_items)
        .map(|_| {
            let base: u32 = rng.random_range(0..3);
            (0..n_raters)
                .map(|_| {
                    if rng.random_bool(0.7) {
                        Some(base)
                    } else {
                        Some(rng.random_range(0..3))
                    }
                })
                .collect()
        })
        .collect();
    world.matrix = Some(make_matrix(data));
}

#[given(
    expr = "a rating matrix where {int} raters agree perfectly on {int} items across {int} categories"
)]
fn given_perfect(world: &mut BootstrapWorld, n_raters: usize, n_items: usize, n_cats: u32) {
    let data: Vec<Vec<Option<u32>>> = (0..n_items)
        .map(|i| vec![Some(i as u32 % n_cats); n_raters])
        .collect();
    world.matrix = Some(make_matrix(data));
}

#[given("an empty rating matrix for bootstrap")]
fn given_empty(world: &mut BootstrapWorld) {
    world.matrix = Some(RatingMatrix {
        items: vec![],
        raters: vec![],
        ratings: vec![],
    });
}

#[given(expr = "a rating matrix with {int} item rated by {int} raters")]
fn given_single_item(world: &mut BootstrapWorld, n_items: usize, n_raters: usize) {
    let data: Vec<Vec<Option<u32>>> = (0..n_items).map(|_| vec![Some(1); n_raters]).collect();
    world.matrix = Some(make_matrix(data));
}

// --- When steps ---

#[when(
    expr = "I bootstrap Krippendorff alpha with {int} resamples at {int}% confidence seeded at {int}"
)]
fn when_bootstrap(world: &mut BootstrapWorld, n_resamples: usize, confidence: usize, seed: u64) {
    let m = world.matrix.as_ref().unwrap();
    let conf = confidence as f64 / 100.0;
    match bootstrap::bootstrap_ci(m, kripp_alpha_fn, n_resamples, conf, seed) {
        Ok(r) => {
            if world.result.is_none() {
                world.result = Some(r);
            } else {
                world.result2 = Some(r);
            }
        }
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when(
    expr = "I bootstrap Krippendorff alpha with {int} resamples at {int}% confidence seeded at {int} twice"
)]
fn when_bootstrap_twice(
    world: &mut BootstrapWorld,
    n_resamples: usize,
    confidence: usize,
    seed: u64,
) {
    let m = world.matrix.as_ref().unwrap();
    let conf = confidence as f64 / 100.0;
    match bootstrap::bootstrap_ci(m, kripp_alpha_fn, n_resamples, conf, seed) {
        Ok(r) => world.result = Some(r),
        Err(e) => {
            world.error = Some(e.to_string());
            return;
        }
    }
    match bootstrap::bootstrap_ci(m, kripp_alpha_fn, n_resamples, conf, seed) {
        Ok(r) => world.result2 = Some(r),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when(
    expr = "I compare bootstrap CIs at {int}% and {int}% confidence with {int} resamples seeded at {int}"
)]
fn when_compare_ci(
    world: &mut BootstrapWorld,
    lo_conf: usize,
    hi_conf: usize,
    n_resamples: usize,
    seed: u64,
) {
    let m = world.matrix.as_ref().unwrap();
    let lo = lo_conf as f64 / 100.0;
    let hi = hi_conf as f64 / 100.0;
    match bootstrap::bootstrap_ci(m, kripp_alpha_fn, n_resamples, lo, seed) {
        Ok(r) => world.ci_90 = Some(r),
        Err(e) => {
            world.error = Some(e.to_string());
            return;
        }
    }
    match bootstrap::bootstrap_ci(m, kripp_alpha_fn, n_resamples, hi, seed) {
        Ok(r) => world.ci_99 = Some(r),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I attempt bootstrap CI computation")]
fn when_attempt(world: &mut BootstrapWorld) {
    let m = world.matrix.as_ref().unwrap();
    match bootstrap::bootstrap_ci(m, kripp_alpha_fn, 100, 0.95, 1) {
        Ok(r) => world.result = Some(r),
        Err(e) => world.error = Some(e.to_string()),
    }
}

// --- Then steps ---

#[then("the CI lower bound is at most the upper bound")]
fn then_ordered(world: &mut BootstrapWorld) {
    let r = world.result.as_ref().expect("no result");
    assert!(
        r.ci_lower <= r.ci_upper,
        "CI not ordered: [{}, {}]",
        r.ci_lower,
        r.ci_upper
    );
}

#[then(expr = "the CI lower bound is greater than {float}")]
fn then_lower_gt(world: &mut BootstrapWorld, threshold: f64) {
    let r = world.result.as_ref().expect("no result");
    assert!(
        r.ci_lower > threshold,
        "CI lower = {}, expected > {}",
        r.ci_lower,
        threshold
    );
}

#[then(expr = "the CI upper bound is at most {float}")]
fn then_upper_leq(world: &mut BootstrapWorld, threshold: f64) {
    let r = world.result.as_ref().expect("no result");
    assert!(
        r.ci_upper <= threshold + 1e-10,
        "CI upper = {}, expected <= {}",
        r.ci_upper,
        threshold
    );
}

#[then("the Krippendorff alpha point estimate falls within the CI")]
fn then_point_in_ci(world: &mut BootstrapWorld) {
    let r = world.result.as_ref().expect("no result");
    assert!(
        r.point_estimate >= r.ci_lower - 1e-10 && r.point_estimate <= r.ci_upper + 1e-10,
        "point estimate {} not in CI [{}, {}]",
        r.point_estimate,
        r.ci_lower,
        r.ci_upper
    );
}

#[then("the 99% CI width is at least the 90% CI width")]
fn then_monotone(world: &mut BootstrapWorld) {
    let ci_90 = world.ci_90.as_ref().expect("no 90% CI");
    let ci_99 = world.ci_99.as_ref().expect("no 99% CI");
    let width_90 = ci_90.ci_upper - ci_90.ci_lower;
    let width_99 = ci_99.ci_upper - ci_99.ci_lower;
    assert!(
        width_99 >= width_90 - 1e-10,
        "99% width {} < 90% width {}",
        width_99,
        width_90
    );
}

#[then("both runs produce identical CIs")]
fn then_reproducible(world: &mut BootstrapWorld) {
    let r1 = world.result.as_ref().expect("no first result");
    let r2 = world.result2.as_ref().expect("no second result");
    assert!(
        (r1.ci_lower - r2.ci_lower).abs() < 1e-12,
        "lower bounds differ: {} vs {}",
        r1.ci_lower,
        r2.ci_lower
    );
    assert!(
        (r1.ci_upper - r2.ci_upper).abs() < 1e-12,
        "upper bounds differ: {} vs {}",
        r1.ci_upper,
        r2.ci_upper
    );
}

#[then("I get a bootstrap error about empty data")]
fn then_error_empty(world: &mut BootstrapWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("empty"), "error: {err}");
}

#[then("I get a bootstrap error about statistic failure")]
fn then_error_stat(world: &mut BootstrapWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(
        err.contains("statistic") || err.contains("failed"),
        "error: {err}"
    );
}

#[when(expr = "I attempt bootstrap with confidence {float}")]
fn when_invalid_confidence(world: &mut BootstrapWorld, conf: f64) {
    let m = world.matrix.as_ref().unwrap();
    match bootstrap::bootstrap_ci(m, kripp_alpha_fn, 100, conf, 1) {
        Ok(r) => world.result = Some(r),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when(expr = "I attempt bootstrap with {int} resamples")]
fn when_zero_resamples(world: &mut BootstrapWorld, n: usize) {
    let m = world.matrix.as_ref().unwrap();
    match bootstrap::bootstrap_ci(m, kripp_alpha_fn, n, 0.95, 1) {
        Ok(r) => world.result = Some(r),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[then("I get a bootstrap error about invalid confidence")]
fn then_error_confidence(world: &mut BootstrapWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("confidence"), "error: {err}");
}

#[then("I get a bootstrap error about invalid resamples")]
fn then_error_resamples(world: &mut BootstrapWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("n_resamples"), "error: {err}");
}

fn main() {
    let runner = BootstrapWorld::run(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tck/irr"));
    futures::executor::block_on(runner);
}
