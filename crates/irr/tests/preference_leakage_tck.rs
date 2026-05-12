use cucumber::{given, then, when, World};
use irr::preference_leakage;
use irr::types::{PreferenceLeakageResult, RelatednessRegime};
use std::collections::BTreeMap;

#[derive(Debug, Default, World)]
pub struct PlsWorld {
    models: Vec<String>,
    win_rates: Vec<Vec<f64>>,
    family_map: BTreeMap<String, String>,
    result: Option<PreferenceLeakageResult>,
    error: Option<String>,
    golden_expected: Vec<GoldenExpectedPair>,
    golden_tolerance: f64,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct GoldenExpectedPair {
    i: String,
    j: String,
    pls: f64,
    regime: String,
}

#[derive(Debug, serde::Deserialize)]
struct GoldenExample {
    name: String,
    models: Vec<String>,
    win_rates: Vec<Vec<f64>>,
    family_map: BTreeMap<String, String>,
    expected_pairs: Vec<GoldenExpectedPair>,
    tolerance: f64,
}

#[derive(Debug, serde::Deserialize)]
struct GoldenDataset {
    examples: Vec<GoldenExample>,
}

fn run_pls(world: &mut PlsWorld) {
    match preference_leakage::compute_pls(&world.models, &world.win_rates, &world.family_map) {
        Ok(r) => world.result = Some(r),
        Err(e) => world.error = Some(e.to_string()),
    }
}

// --- Given steps ---

#[given(expr = "PLS golden example {string}")]
fn given_golden(world: &mut PlsWorld, name: String) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/pls_li2025.json");
    let data: GoldenDataset =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let example = data
        .examples
        .into_iter()
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("golden example {name:?} not found"));

    world.models = example.models;
    world.win_rates = example.win_rates;
    world.family_map = example.family_map;
    world.golden_expected = example.expected_pairs;
    world.golden_tolerance = example.tolerance;
}

#[given(expr = "{int} models with uniform win rates of {float}")]
fn given_uniform(world: &mut PlsWorld, n: usize, rate: f64) {
    world.models = (0..n).map(|i| format!("model-{i}")).collect();
    world.win_rates = vec![vec![rate; n]; n];
}

#[given("all models are cross-family")]
fn given_cross_family(world: &mut PlsWorld) {
    for (i, m) in world.models.iter().enumerate() {
        world.family_map.insert(m.clone(), format!("fam-{i}"));
    }
}

#[given(expr = "{int} models in {int} families of {int} with uniform win rates of {float}")]
fn given_families(
    world: &mut PlsWorld,
    n_models: usize,
    n_families: usize,
    per_family: usize,
    rate: f64,
) {
    assert_eq!(n_models, n_families * per_family);
    world.models = (0..n_models).map(|i| format!("model-{i}")).collect();
    world.win_rates = vec![vec![rate; n_models]; n_models];
    for (i, m) in world.models.iter().enumerate() {
        let family_idx = i / per_family;
        world
            .family_map
            .insert(m.clone(), format!("family-{family_idx}"));
    }
}

#[given("an empty win-rate matrix")]
fn given_empty(world: &mut PlsWorld) {
    world.models.clear();
    world.win_rates.clear();
    world.family_map.clear();
}

#[given(expr = "{int} model with win rate {float}")]
fn given_single(world: &mut PlsWorld, n: usize, rate: f64) {
    world.models = (0..n).map(|i| format!("model-{i}")).collect();
    world.win_rates = vec![vec![rate; n]; n];
}

#[given("a non-square win-rate matrix")]
fn given_nonsquare(world: &mut PlsWorld) {
    world.models = vec!["A".to_string(), "B".to_string()];
    world.win_rates = vec![vec![0.5, 0.5, 0.5]]; // 1x3 but 2 models
    world.family_map.insert("A".to_string(), "f".to_string());
    world.family_map.insert("B".to_string(), "f".to_string());
}

#[given("a win-rate matrix with values outside 0 to 1")]
fn given_invalid_wr(world: &mut PlsWorld) {
    world.models = vec!["A".to_string(), "B".to_string()];
    world.win_rates = vec![vec![1.5, 0.5], vec![0.5, 0.5]];
    world.family_map.insert("A".to_string(), "f".to_string());
    world.family_map.insert("B".to_string(), "f".to_string());
}

#[given("a win-rate matrix where AVG equals zero")]
fn given_degenerate_avg(world: &mut PlsWorld) {
    world.models = vec!["A".to_string(), "B".to_string()];
    world.win_rates = vec![vec![0.0, 0.0], vec![0.5, 0.5]];
    world.family_map.insert("A".to_string(), "fa".to_string());
    world.family_map.insert("B".to_string(), "fb".to_string());
}

// --- When steps ---

#[when("I compute PLS")]
fn when_compute(world: &mut PlsWorld) {
    run_pls(world);
}

#[when("I attempt PLS computation")]
fn when_attempt(world: &mut PlsWorld) {
    run_pls(world);
}

// --- Then steps ---

#[then("each PLS value matches the golden expected value")]
fn then_golden_match(world: &mut PlsWorld) {
    let r = world.result.as_ref().expect("no result");
    let tol = world.golden_tolerance;
    for expected in &world.golden_expected {
        let pair = r
            .pls_scores
            .iter()
            .find(|p| {
                (p.model_i == expected.i && p.model_j == expected.j)
                    || (p.model_i == expected.j && p.model_j == expected.i)
            })
            .unwrap_or_else(|| {
                panic!("pair ({}, {}) not found in results", expected.i, expected.j)
            });
        assert!(
            (pair.pls - expected.pls).abs() < tol,
            "PLS({}, {}) = {}, expected {} ± {tol}",
            expected.i,
            expected.j,
            pair.pls,
            expected.pls
        );
        let expected_regime = match expected.regime.as_str() {
            "SameFamily" => RelatednessRegime::SameFamily,
            "CrossFamily" => RelatednessRegime::CrossFamily,
            other => panic!("unknown regime: {other}"),
        };
        assert_eq!(
            pair.regime, expected_regime,
            "pair ({}, {}) regime mismatch",
            expected.i, expected.j
        );
    }
}

#[then("all pairwise PLS values are 0.0")]
fn then_all_zero(world: &mut PlsWorld) {
    let r = world.result.as_ref().expect("no result");
    for pair in &r.pls_scores {
        assert!(
            pair.pls.abs() < 1e-10,
            "PLS({}, {}) = {}, expected 0.0",
            pair.model_i,
            pair.model_j,
            pair.pls
        );
    }
}

#[then(expr = "there are {int} SameFamily pairs and {int} CrossFamily pairs")]
fn then_regime_counts(world: &mut PlsWorld, same: usize, cross: usize) {
    let r = world.result.as_ref().expect("no result");
    let same_count = r
        .pls_scores
        .iter()
        .filter(|p| p.regime == RelatednessRegime::SameFamily)
        .count();
    let cross_count = r
        .pls_scores
        .iter()
        .filter(|p| p.regime == RelatednessRegime::CrossFamily)
        .count();
    assert_eq!(
        same_count, same,
        "SameFamily pairs: {same_count}, expected {same}"
    );
    assert_eq!(
        cross_count, cross,
        "CrossFamily pairs: {cross_count}, expected {cross}"
    );
}

#[then("I get a PLS error about empty data")]
fn then_error_empty(world: &mut PlsWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("empty"), "error: {err}");
}

#[then(expr = "there are {int} pairwise PLS values")]
fn then_pair_count(world: &mut PlsWorld, count: usize) {
    let r = world.result.as_ref().expect("no result");
    assert_eq!(
        r.pls_scores.len(),
        count,
        "pair count: {}, expected {count}",
        r.pls_scores.len()
    );
}

#[then("I get a PLS error about non-square matrix")]
fn then_error_nonsquare(world: &mut PlsWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("square"), "error: {err}");
}

#[then("I get a PLS error about invalid win rate")]
fn then_error_invalid(world: &mut PlsWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("not a finite value in [0, 1]"), "error: {err}");
}

#[then("I get a PLS error about degenerate average")]
fn then_error_degenerate(world: &mut PlsWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("degenerate AVG"), "error: {err}");
}

fn main() {
    let runner = PlsWorld::run(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tck/irr"));
    futures::executor::block_on(runner);
}
