use cucumber::{given, then, when, World};
use irr::mandel;

#[derive(Debug, Default, World)]
pub struct MandelWorld {
    configs: Option<Vec<Vec<f64>>>,
    result: Option<mandel::MandelStatistics>,
    alpha: Option<f64>,
}

// --- Given steps ---

#[given("an interlaboratory dataset with 5 labs and one outlier lab")]
fn given_outlier_lab(world: &mut MandelWorld) {
    // 5 labs, each with 4 replicates. Lab 3 (index 2) has a shifted mean.
    world.configs = Some(vec![
        vec![10.1, 10.3, 10.2, 10.0],
        vec![10.0, 10.2, 10.1, 10.3],
        vec![14.0, 14.2, 14.1, 13.9], // outlier: shifted mean
        vec![10.2, 10.0, 10.1, 10.3],
        vec![10.1, 10.2, 10.0, 10.3],
    ]);
}

#[given("an interlaboratory dataset with 5 labs and one high-variability lab")]
fn given_high_var_lab(world: &mut MandelWorld) {
    // 5 labs, each with 4 replicates. Lab 2 (index 1) has high within-lab variability.
    world.configs = Some(vec![
        vec![10.1, 10.0, 10.2, 10.1],
        vec![5.0, 15.0, 7.0, 13.0], // high variability
        vec![10.0, 10.1, 10.2, 10.0],
        vec![10.2, 10.1, 10.0, 10.2],
        vec![10.1, 10.0, 10.1, 10.2],
    ]);
}

#[given("an interlaboratory dataset with 5 consistent labs")]
fn given_consistent_labs(world: &mut MandelWorld) {
    // 5 labs, all consistent and similar
    world.configs = Some(vec![
        vec![10.1, 10.0, 10.2, 10.1],
        vec![10.2, 10.1, 10.0, 10.2],
        vec![10.0, 10.1, 10.2, 10.0],
        vec![10.1, 10.2, 10.0, 10.1],
        vec![10.2, 10.0, 10.1, 10.2],
    ]);
}

// --- When steps ---

#[when(expr = "I compute Mandel h statistics at alpha {float}")]
fn when_mandel_h(world: &mut MandelWorld, alpha: f64) {
    let configs = world.configs.as_ref().unwrap();
    let refs: Vec<&[f64]> = configs.iter().map(|c| c.as_slice()).collect();
    world.result = Some(mandel::mandel_hk(&refs, alpha).unwrap());
    world.alpha = Some(alpha);
}

#[when(expr = "I compute Mandel k statistics at alpha {float}")]
fn when_mandel_k(world: &mut MandelWorld, alpha: f64) {
    let configs = world.configs.as_ref().unwrap();
    let refs: Vec<&[f64]> = configs.iter().map(|c| c.as_slice()).collect();
    world.result = Some(mandel::mandel_hk(&refs, alpha).unwrap());
    world.alpha = Some(alpha);
}

#[when(expr = "I compute Mandel h and k statistics at alpha {float}")]
fn when_mandel_hk(world: &mut MandelWorld, alpha: f64) {
    let configs = world.configs.as_ref().unwrap();
    let refs: Vec<&[f64]> = configs.iter().map(|c| c.as_slice()).collect();
    world.result = Some(mandel::mandel_hk(&refs, alpha).unwrap());
    world.alpha = Some(alpha);
}

// --- Then steps ---

#[then("the outlier lab has absolute h exceeding the critical value")]
fn then_outlier_h(world: &mut MandelWorld) {
    let r = world.result.as_ref().expect("no result");
    // Lab 3 (index 2) should be the outlier
    assert!(
        r.h[2].abs() > r.h_critical,
        "outlier lab h={} should exceed h_critical={}",
        r.h[2].abs(),
        r.h_critical
    );
    assert!(
        r.h_outliers.contains(&2),
        "lab 2 should be in h_outliers: {:?}",
        r.h_outliers
    );
}

#[then("non-outlier labs have absolute h below the critical value")]
fn then_non_outlier_h(world: &mut MandelWorld) {
    let r = world.result.as_ref().expect("no result");
    for i in [0, 1, 3, 4] {
        assert!(
            r.h[i].abs() <= r.h_critical,
            "non-outlier lab {} h={} should not exceed h_critical={}",
            i,
            r.h[i].abs(),
            r.h_critical
        );
    }
}

#[then("the high-variability lab has k exceeding the critical value")]
fn then_high_var_k(world: &mut MandelWorld) {
    let r = world.result.as_ref().expect("no result");
    // Lab 2 (index 1) should be the high-variability outlier
    assert!(
        r.k[1] > r.k_critical,
        "high-var lab k={} should exceed k_critical={}",
        r.k[1],
        r.k_critical
    );
    assert!(
        r.k_outliers.contains(&1),
        "lab 1 should be in k_outliers: {:?}",
        r.k_outliers
    );
}

#[then("other labs have k below the critical value")]
fn then_other_k(world: &mut MandelWorld) {
    let r = world.result.as_ref().expect("no result");
    for i in [0, 2, 3, 4] {
        assert!(
            r.k[i] <= r.k_critical,
            "non-outlier lab {} k={} should not exceed k_critical={}",
            i,
            r.k[i],
            r.k_critical
        );
    }
}

#[then("no labs are flagged as h outliers")]
fn then_no_h_outliers(world: &mut MandelWorld) {
    let r = world.result.as_ref().expect("no result");
    assert!(
        r.h_outliers.is_empty(),
        "expected no h outliers, got: {:?}",
        r.h_outliers
    );
}

#[then("no labs are flagged as k outliers")]
fn then_no_k_outliers(world: &mut MandelWorld) {
    let r = world.result.as_ref().expect("no result");
    assert!(
        r.k_outliers.is_empty(),
        "expected no k outliers, got: {:?}",
        r.k_outliers
    );
}

fn main() {
    let runner = MandelWorld::run(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tck/irr"));
    futures::executor::block_on(runner);
}
