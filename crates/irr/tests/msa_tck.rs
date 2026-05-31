use cucumber::{given, then, when, World};
use irr::msa;

#[derive(Debug, Default, World)]
pub struct MsaWorld {
    sigma_parts: Option<f64>,
    sigma_gauge_rr: Option<f64>,
    tolerance: Option<f64>,
    ndc_result: Option<msa::MsaDiagnostics>,
    pt_result: Option<msa::PtRatioDiagnostics>,
}

// --- Given steps ---

#[given(expr = "a rating study with sigma_parts {float} and sigma_gauge_rr {float}")]
fn given_sigma_parts_and_gauge(world: &mut MsaWorld, sigma_parts: f64, sigma_gauge_rr: f64) {
    world.sigma_parts = Some(sigma_parts);
    world.sigma_gauge_rr = Some(sigma_gauge_rr);
}

#[given(expr = "a rating study with sigma_gauge_rr {float} and tolerance {float}")]
fn given_sigma_gauge_and_tolerance(world: &mut MsaWorld, sigma_gauge_rr: f64, tolerance: f64) {
    world.sigma_gauge_rr = Some(sigma_gauge_rr);
    world.tolerance = Some(tolerance);
}

// --- When steps ---

#[when("I compute ndc")]
fn when_compute_ndc(world: &mut MsaWorld) {
    let sp = world.sigma_parts.unwrap();
    let sg = world.sigma_gauge_rr.unwrap();
    world.ndc_result = Some(msa::ndc(sp, sg).unwrap());
}

#[when("I compute the P-T ratio")]
fn when_compute_pt(world: &mut MsaWorld) {
    let sg = world.sigma_gauge_rr.unwrap();
    let tol = world.tolerance.unwrap();
    world.pt_result = Some(msa::pt_ratio(sg, tol).unwrap());
}

// --- Then steps ---

#[then(expr = "ndc equals {int}")]
fn then_ndc_equals(world: &mut MsaWorld, expected: usize) {
    let result = world.ndc_result.as_ref().expect("no ndc result");
    assert_eq!(
        result.ndc, expected,
        "ndc = {}, expected {}",
        result.ndc, expected
    );
}

#[then("ndc is flagged as inadequate because AIAG requires ndc >= 5")]
fn then_ndc_inadequate(world: &mut MsaWorld) {
    let result = world.ndc_result.as_ref().expect("no ndc result");
    assert!(
        !result.ndc_adequate,
        "ndc {} should be flagged as inadequate (< 5)",
        result.ndc
    );
}

#[then("ndc is flagged as adequate")]
fn then_ndc_adequate(world: &mut MsaWorld) {
    let result = world.ndc_result.as_ref().expect("no ndc result");
    assert!(
        result.ndc_adequate,
        "ndc {} should be flagged as adequate (>= 5)",
        result.ndc
    );
}

#[then(expr = "the P-T ratio equals {float}")]
fn then_pt_equals(world: &mut MsaWorld, expected: f64) {
    let result = world.pt_result.as_ref().expect("no P/T result");
    assert!(
        (result.p_t_ratio - expected).abs() < 1e-10,
        "P/T ratio = {}, expected {}",
        result.p_t_ratio,
        expected
    );
}

#[then("the gauge P-T is flagged as inadequate")]
fn then_pt_inadequate(world: &mut MsaWorld) {
    let result = world.pt_result.as_ref().expect("no P/T result");
    assert!(
        !result.pt_adequate,
        "P/T {} should be flagged as inadequate (>= 0.30)",
        result.p_t_ratio
    );
}

#[then("the gauge P-T is flagged as adequate")]
fn then_pt_adequate(world: &mut MsaWorld) {
    let result = world.pt_result.as_ref().expect("no P/T result");
    assert!(
        result.pt_adequate,
        "P/T {} should be flagged as adequate (< 0.30)",
        result.p_t_ratio
    );
}

fn main() {
    let runner = MsaWorld::run(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tck/irr"));
    futures::executor::block_on(runner);
}
