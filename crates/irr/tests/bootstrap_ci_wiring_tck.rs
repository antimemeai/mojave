use cucumber::{given, then, when, World};
use irr::cohen;
use irr::fleiss;
use irr::gwet;
use irr::krippendorff;
use irr::types::{IrrResult, MetricLevel, RatingMatrix};

#[derive(Debug, Default, World)]
pub struct CiWiringWorld {
    matrix: Option<RatingMatrix>,
    result: Option<IrrResult>,
    ci_90: Option<IrrResult>,
    ci_99: Option<IrrResult>,
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

fn make_two_rater_moderate(seed: u64) -> RatingMatrix {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    let data: Vec<Vec<Option<u32>>> = (0..30)
        .map(|_| {
            let base: u32 = rng.random_range(0..3);
            let r2 = if rng.random_bool(0.7) {
                base
            } else {
                rng.random_range(0..3)
            };
            vec![Some(base), Some(r2)]
        })
        .collect();
    make_matrix(data)
}

fn make_multi_rater_moderate(n_raters: usize, seed: u64) -> RatingMatrix {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    let data: Vec<Vec<Option<u32>>> = (0..30)
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
    make_matrix(data)
}

// --- Given steps ---

#[given(expr = "a two-rater rating matrix with moderate agreement seeded at {int}")]
fn given_two_rater(world: &mut CiWiringWorld, seed: u64) {
    world.matrix = Some(make_two_rater_moderate(seed));
}

#[given(
    expr = "a multi-rater rating matrix with {int} raters and moderate agreement seeded at {int}"
)]
fn given_multi_rater(world: &mut CiWiringWorld, n_raters: usize, seed: u64) {
    world.matrix = Some(make_multi_rater_moderate(n_raters, seed));
}

// --- When steps ---

#[when(
    expr = "I compute Cohen kappa with bootstrap CIs using {int} resamples at {int}% confidence seeded at {int}"
)]
fn when_cohen_ci(world: &mut CiWiringWorld, n_resamples: usize, confidence: usize, seed: u64) {
    let m = world.matrix.as_ref().unwrap();
    let conf = confidence as f64 / 100.0;
    world.result = Some(cohen::kappa_with_ci(m, n_resamples, conf, seed).unwrap());
}

#[when(
    expr = "I compute Fleiss kappa with bootstrap CIs using {int} resamples at {int}% confidence seeded at {int}"
)]
fn when_fleiss_ci(world: &mut CiWiringWorld, n_resamples: usize, confidence: usize, seed: u64) {
    let m = world.matrix.as_ref().unwrap();
    let conf = confidence as f64 / 100.0;
    world.result = Some(fleiss::kappa_with_ci(m, n_resamples, conf, seed).unwrap());
}

#[when(
    expr = "I compute Krippendorff alpha with bootstrap CIs using {int} resamples at {int}% confidence seeded at {int}"
)]
fn when_krippendorff_ci(
    world: &mut CiWiringWorld,
    n_resamples: usize,
    confidence: usize,
    seed: u64,
) {
    let m = world.matrix.as_ref().unwrap();
    let conf = confidence as f64 / 100.0;
    world.result = Some(
        krippendorff::alpha_with_ci(m, MetricLevel::Nominal, n_resamples, conf, seed).unwrap(),
    );
}

#[when(
    expr = "I compute Gwet AC1 with bootstrap CIs using {int} resamples at {int}% confidence seeded at {int}"
)]
fn when_gwet_ci(world: &mut CiWiringWorld, n_resamples: usize, confidence: usize, seed: u64) {
    let m = world.matrix.as_ref().unwrap();
    let conf = confidence as f64 / 100.0;
    world.result = Some(gwet::ac_with_ci(m, None, n_resamples, conf, seed).unwrap());
}

#[when(
    expr = "I compute Cohen kappa CIs at {int}% and {int}% confidence with {int} resamples seeded at {int}"
)]
fn when_cohen_compare(
    world: &mut CiWiringWorld,
    lo_conf: usize,
    hi_conf: usize,
    n_resamples: usize,
    seed: u64,
) {
    let m = world.matrix.as_ref().unwrap();
    let lo = lo_conf as f64 / 100.0;
    let hi = hi_conf as f64 / 100.0;
    world.ci_90 = Some(cohen::kappa_with_ci(m, n_resamples, lo, seed).unwrap());
    world.ci_99 = Some(cohen::kappa_with_ci(m, n_resamples, hi, seed).unwrap());
}

// --- Then steps ---

#[then("ci_lower is not None")]
fn then_ci_lower_some(world: &mut CiWiringWorld) {
    let r = world.result.as_ref().expect("no result");
    assert!(r.ci_lower.is_some(), "ci_lower is None");
}

#[then("ci_upper is not None")]
fn then_ci_upper_some(world: &mut CiWiringWorld) {
    let r = world.result.as_ref().expect("no result");
    assert!(r.ci_upper.is_some(), "ci_upper is None");
}

#[then("ci_lower <= kappa <= ci_upper")]
fn then_ci_brackets(world: &mut CiWiringWorld) {
    let r = world.result.as_ref().expect("no result");
    let lo = r.ci_lower.unwrap();
    let hi = r.ci_upper.unwrap();
    assert!(
        lo <= r.value + 1e-10 && r.value <= hi + 1e-10,
        "point estimate {} not in CI [{}, {}]",
        r.value,
        lo,
        hi
    );
}

#[then("the 99% CI is at least as wide as the 90% CI")]
fn then_wider_ci(world: &mut CiWiringWorld) {
    let r90 = world.ci_90.as_ref().expect("no 90% CI");
    let r99 = world.ci_99.as_ref().expect("no 99% CI");
    let w90 = r90.ci_upper.unwrap() - r90.ci_lower.unwrap();
    let w99 = r99.ci_upper.unwrap() - r99.ci_lower.unwrap();
    assert!(
        w99 >= w90 - 1e-10,
        "99% width {} < 90% width {}",
        w99,
        w90
    );
}

fn main() {
    let runner =
        CiWiringWorld::run(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tck/irr"));
    futures::executor::block_on(runner);
}
