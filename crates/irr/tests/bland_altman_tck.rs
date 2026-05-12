use cucumber::{given, then, when, World};
use irr::bland_altman::{self, BlandAltmanResult};

#[derive(Debug, Default, World)]
pub struct BlandAltmanWorld {
    x: Vec<f64>,
    y: Vec<f64>,
    result_xy: Option<BlandAltmanResult>,
    result_yx: Option<BlandAltmanResult>,
    error: Option<String>,
}

// ======================== Given steps ========================

#[given("the Bland-Altman 1986 PEFR data")]
fn given_pefr_data(world: &mut BlandAltmanWorld) {
    let path = format!(
        "{}/tests/golden/bland_altman_1986_pefr.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

    world.x = json["wright"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    world.y = json["mini_wright"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
}

#[given(expr = "measurements x = [{float}, {float}, {float}, {float}, {float}]")]
fn given_x_5(world: &mut BlandAltmanWorld, a: f64, b: f64, c: f64, d: f64, e: f64) {
    world.x = vec![a, b, c, d, e];
}

#[given(expr = "measurements y = [{float}, {float}, {float}, {float}, {float}]")]
fn given_y_5(world: &mut BlandAltmanWorld, a: f64, b: f64, c: f64, d: f64, e: f64) {
    world.y = vec![a, b, c, d, e];
}

#[given(expr = "measurements x = [{float}, {float}, {float}]")]
fn given_x_3(world: &mut BlandAltmanWorld, a: f64, b: f64, c: f64) {
    world.x = vec![a, b, c];
}

#[given(expr = "measurements y = [{float}, {float}, {float}]")]
fn given_y_3(world: &mut BlandAltmanWorld, a: f64, b: f64, c: f64) {
    world.y = vec![a, b, c];
}

#[given(expr = "measurements x with {int} values and y with {int} values")]
fn given_mismatched(world: &mut BlandAltmanWorld, nx: usize, ny: usize) {
    world.x = (0..nx).map(|i| i as f64).collect();
    world.y = (0..ny).map(|i| i as f64).collect();
}

#[given(expr = "measurements x = [{float}] and y = [{float}]")]
fn given_single(world: &mut BlandAltmanWorld, xv: f64, yv: f64) {
    world.x = vec![xv];
    world.y = vec![yv];
}

// ======================== When steps ========================

#[when("I compute Bland-Altman agreement")]
fn when_compute(world: &mut BlandAltmanWorld) {
    match bland_altman::agreement(&world.x, &world.y) {
        Ok(r) => world.result_xy = Some(r),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I attempt Bland-Altman agreement")]
fn when_attempt(world: &mut BlandAltmanWorld) {
    match bland_altman::agreement(&world.x, &world.y) {
        Ok(r) => world.result_xy = Some(r),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute Bland-Altman agreement for x and y")]
fn when_compute_xy(world: &mut BlandAltmanWorld) {
    match bland_altman::agreement(&world.x, &world.y) {
        Ok(r) => world.result_xy = Some(r),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute Bland-Altman agreement for y and x")]
fn when_compute_yx(world: &mut BlandAltmanWorld) {
    match bland_altman::agreement(&world.y, &world.x) {
        Ok(r) => world.result_yx = Some(r),
        Err(e) => world.error = Some(e.to_string()),
    }
}

// ======================== Then steps ========================

#[then(expr = "mean difference is {float} within {float}")]
fn then_mean_diff(world: &mut BlandAltmanWorld, expected: f64, tol: f64) {
    let r = world.result_xy.as_ref().expect("no result computed");
    assert!(
        (r.mean_diff - expected).abs() < tol,
        "mean_diff = {}, expected {} +/- {}",
        r.mean_diff,
        expected,
        tol
    );
}

#[then(expr = "SD of differences is {float} within {float}")]
fn then_sd_diff(world: &mut BlandAltmanWorld, expected: f64, tol: f64) {
    let r = world.result_xy.as_ref().expect("no result computed");
    assert!(
        (r.sd_diff - expected).abs() < tol,
        "sd_diff = {}, expected {} +/- {}",
        r.sd_diff,
        expected,
        tol
    );
}

#[then(expr = "lower LoA is approximately {float} within {float}")]
fn then_lower_loa(world: &mut BlandAltmanWorld, expected: f64, tol: f64) {
    let r = world.result_xy.as_ref().expect("no result computed");
    assert!(
        (r.lower_loa - expected).abs() < tol,
        "lower_loa = {}, expected {} +/- {}",
        r.lower_loa,
        expected,
        tol
    );
}

#[then(expr = "upper LoA is approximately {float} within {float}")]
fn then_upper_loa(world: &mut BlandAltmanWorld, expected: f64, tol: f64) {
    let r = world.result_xy.as_ref().expect("no result computed");
    assert!(
        (r.upper_loa - expected).abs() < tol,
        "upper_loa = {}, expected {} +/- {}",
        r.upper_loa,
        expected,
        tol
    );
}

#[then(expr = "I get a Bland-Altman error containing {string}")]
fn then_error_contains(world: &mut BlandAltmanWorld, substring: String) {
    let err = world
        .error
        .as_ref()
        .expect("expected an error but got none");
    assert!(
        err.to_lowercase().contains(&substring.to_lowercase()),
        "error '{}' does not contain '{}'",
        err,
        substring
    );
}

#[then(expr = "the mean differences are negations within {float}")]
fn then_mean_negation(world: &mut BlandAltmanWorld, tol: f64) {
    let rxy = world.result_xy.as_ref().expect("no xy result");
    let ryx = world.result_yx.as_ref().expect("no yx result");
    assert!(
        (rxy.mean_diff + ryx.mean_diff).abs() < tol,
        "mean_diff(xy)={} and mean_diff(yx)={} are not negations (sum={})",
        rxy.mean_diff,
        ryx.mean_diff,
        rxy.mean_diff + ryx.mean_diff
    );
}

#[then(expr = "the SD values are equal within {float}")]
fn then_sd_equal(world: &mut BlandAltmanWorld, tol: f64) {
    let rxy = world.result_xy.as_ref().expect("no xy result");
    let ryx = world.result_yx.as_ref().expect("no yx result");
    assert!(
        (rxy.sd_diff - ryx.sd_diff).abs() < tol,
        "sd_diff(xy)={} and sd_diff(yx)={} differ by {}",
        rxy.sd_diff,
        ryx.sd_diff,
        (rxy.sd_diff - ryx.sd_diff).abs()
    );
}

fn main() {
    let runner = BlandAltmanWorld::run(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tck/irr/bland_altman.feature"
    ));
    futures::executor::block_on(runner);
}
