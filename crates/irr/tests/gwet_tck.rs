use cucumber::{given, then, when, World};
use irr::categorical_agreement_weights::{WeightMatrix, WeightScheme};
use irr::cohen;
use irr::gwet;
use irr::types::RatingMatrix;

#[derive(Debug, Default, World)]
pub struct GwetWorld {
    matrix: Option<RatingMatrix>,
    relabeled_matrix: Option<RatingMatrix>,
    custom_weights: Option<WeightMatrix>,
    ac1_result: Option<f64>,
    ac2_result: Option<f64>,
    ac1_relabeled: Option<f64>,
    ac3_result: Option<f64>,
    cohen_kappa: Option<f64>,
    error: Option<String>,
}

/// Build a RatingMatrix from a confusion matrix (2-rater, expand cells to item rows).
fn confusion_to_rating_matrix(confusion: &[Vec<u64>]) -> RatingMatrix {
    let mut ratings = Vec::new();
    let mut items = Vec::new();
    let mut idx = 0usize;
    for (i, row) in confusion.iter().enumerate() {
        for (j, &count) in row.iter().enumerate() {
            for _ in 0..count {
                items.push(format!("item-{idx}"));
                ratings.push(vec![Some(i as u32), Some(j as u32)]);
                idx += 1;
            }
        }
    }
    RatingMatrix {
        items,
        raters: vec!["r0".to_string(), "r1".to_string()],
        ratings,
    }
}

/// Build a RatingMatrix from the Krippendorff 2011 data (items x raters, with nulls).
fn krippendorff_data_to_matrix(data: &[Vec<Option<u32>>]) -> RatingMatrix {
    let n_items = data.len();
    let n_raters = if n_items > 0 { data[0].len() } else { 0 };
    RatingMatrix {
        items: (0..n_items).map(|i| format!("item-{i}")).collect(),
        raters: (0..n_raters).map(|i| format!("r{i}")).collect(),
        ratings: data.to_vec(),
    }
}

// ======================== Given steps ========================

#[given("the Gwet 2014 Table 4.1 rating matrix")]
fn given_gwet_2014(world: &mut GwetWorld) {
    let path = format!(
        "{}/tests/golden/gwet_2014_table4_1.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let confusion: Vec<Vec<u64>> = json["confusion_matrix"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row.as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap())
                .collect()
        })
        .collect();
    world.matrix = Some(confusion_to_rating_matrix(&confusion));
}

#[given("the Krippendorff 2011 reliability data for Gwet")]
fn given_krippendorff_2011(world: &mut GwetWorld) {
    // The expected AC1 = 0.77544 comes from irrCAC's cac.raw4raters dataset,
    // which is the Krippendorff 2011 data with a 4th rater added.
    let path = format!(
        "{}/tests/golden/cac_raw4raters.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let data: Vec<Vec<Option<u32>>> = json["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row.as_array()
                .unwrap()
                .iter()
                .map(|v| {
                    if v.is_null() {
                        None
                    } else {
                        Some(v.as_u64().unwrap() as u32)
                    }
                })
                .collect()
        })
        .collect();
    world.matrix = Some(krippendorff_data_to_matrix(&data));
}

#[given(expr = "a rating matrix where all raters agree on {int} items across {int} categories")]
fn given_perfect_agreement(world: &mut GwetWorld, n_items: usize, n_cats: u32) {
    let n_raters = 3;
    let ratings: Vec<Vec<Option<u32>>> = (0..n_items)
        .map(|i| {
            let cat = (i as u32) % n_cats;
            vec![Some(cat); n_raters]
        })
        .collect();
    world.matrix = Some(RatingMatrix {
        items: (0..n_items).map(|i| format!("item-{i}")).collect(),
        raters: (0..n_raters).map(|i| format!("r{i}")).collect(),
        ratings,
    });
}

#[given(expr = "a high-prevalence 2-rater matrix with 90% category 0 seeded at {int}")]
fn given_high_prevalence(world: &mut GwetWorld, seed: u64) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    let n_items = 100;
    let mut ratings = Vec::with_capacity(n_items);
    for _ in 0..n_items {
        let cat: u32 = if rng.random_bool(0.9) { 0 } else { 1 };
        // Second rater agrees most of the time, slight noise
        let cat2: u32 = if rng.random_bool(0.85) { cat } else { 1 - cat };
        ratings.push(vec![Some(cat), Some(cat2)]);
    }
    world.matrix = Some(RatingMatrix {
        items: (0..n_items).map(|i| format!("item-{i}")).collect(),
        raters: vec!["r0".to_string(), "r1".to_string()],
        ratings,
    });
}

#[given(expr = "a mixed-agreement 2-rater matrix seeded at {int}")]
fn given_mixed_agreement(world: &mut GwetWorld, seed: u64) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    let n_items = 50;
    let n_cats = 3u32;
    let mut ratings = Vec::with_capacity(n_items);
    for _ in 0..n_items {
        let c1: u32 = rng.random_range(0..n_cats);
        let c2: u32 = if rng.random_bool(0.7) {
            c1
        } else {
            rng.random_range(0..n_cats)
        };
        ratings.push(vec![Some(c1), Some(c2)]);
    }
    world.matrix = Some(RatingMatrix {
        items: (0..n_items).map(|i| format!("item-{i}")).collect(),
        raters: vec!["r0".to_string(), "r1".to_string()],
        ratings,
    });
}

#[given(expr = "the same data relabeled from {int},{int},{int} to {int},{int},{int}")]
fn given_relabeled(
    world: &mut GwetWorld,
    from0: u32,
    from1: u32,
    from2: u32,
    to0: u32,
    to1: u32,
    to2: u32,
) {
    let m = world.matrix.as_ref().expect("no matrix to relabel");
    let mapping: std::collections::BTreeMap<u32, u32> = [(from0, to0), (from1, to1), (from2, to2)]
        .into_iter()
        .collect();
    let ratings: Vec<Vec<Option<u32>>> = m
        .ratings
        .iter()
        .map(|row| {
            row.iter()
                .map(|v| v.map(|c| *mapping.get(&c).unwrap_or(&c)))
                .collect()
        })
        .collect();
    world.relabeled_matrix = Some(RatingMatrix {
        items: m.items.clone(),
        raters: m.raters.clone(),
        ratings,
    });
}

#[given("an empty rating matrix for Gwet")]
fn given_empty(world: &mut GwetWorld) {
    world.matrix = Some(RatingMatrix {
        items: vec![],
        raters: vec![],
        ratings: vec![],
    });
}

#[given(expr = "a single-rater matrix with {int} items")]
fn given_single_rater(world: &mut GwetWorld, n_items: usize) {
    let ratings: Vec<Vec<Option<u32>>> = (0..n_items).map(|i| vec![Some(i as u32 % 3)]).collect();
    world.matrix = Some(RatingMatrix {
        items: (0..n_items).map(|i| format!("item-{i}")).collect(),
        raters: vec!["r0".to_string()],
        ratings,
    });
}

#[given(expr = "a matrix where all {int} items are category {int} by {int} raters")]
fn given_all_same(world: &mut GwetWorld, n_items: usize, cat: u32, n_raters: usize) {
    let ratings: Vec<Vec<Option<u32>>> = (0..n_items).map(|_| vec![Some(cat); n_raters]).collect();
    world.matrix = Some(RatingMatrix {
        items: (0..n_items).map(|i| format!("item-{i}")).collect(),
        raters: (0..n_raters).map(|i| format!("r{i}")).collect(),
        ratings,
    });
}

#[given("a weight matrix for only categories 0 and 1")]
fn given_partial_weights(world: &mut GwetWorld) {
    world.custom_weights = Some(WeightMatrix::from_scheme(&[0, 1], WeightScheme::Quadratic));
}

#[given("a custom 3x3 weight matrix")]
fn given_custom_weights(world: &mut GwetWorld) {
    let w = vec![
        vec![1.0, 0.5, 0.0],
        vec![0.5, 1.0, 0.5],
        vec![0.0, 0.5, 1.0],
    ];
    world.custom_weights =
        Some(WeightMatrix::custom(&[0, 1, 2], w).expect("custom weight matrix should be valid"));
}

// ======================== When steps ========================

#[when("I compute Gwet AC1")]
fn when_ac1(world: &mut GwetWorld) {
    let m = world.matrix.as_ref().expect("no matrix");
    match gwet::ac(m, None) {
        Ok(r) => world.ac1_result = Some(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute Gwet AC2 with quadratic weights")]
fn when_ac2_quadratic(world: &mut GwetWorld) {
    let m = world.matrix.as_ref().expect("no matrix");
    // Discover categories from data for weight matrix
    let cats = discover_categories(m);
    let wm = WeightMatrix::from_scheme(&cats, WeightScheme::Quadratic);
    match gwet::ac(m, Some(&wm)) {
        Ok(r) => world.ac2_result = Some(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute Cohen kappa on the same data")]
fn when_cohen_kappa(world: &mut GwetWorld) {
    let m = world.matrix.as_ref().expect("no matrix");
    match cohen::kappa_from_matrix(m) {
        Ok(r) => world.cohen_kappa = Some(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute Gwet AC2 with identity weights")]
fn when_ac2_identity(world: &mut GwetWorld) {
    let m = world.matrix.as_ref().expect("no matrix");
    let cats = discover_categories(m);
    let wm = WeightMatrix::from_scheme(&cats, WeightScheme::Identity);
    match gwet::ac(m, Some(&wm)) {
        Ok(r) => world.ac2_result = Some(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute Gwet AC1 on original")]
fn when_ac1_original(world: &mut GwetWorld) {
    let m = world.matrix.as_ref().expect("no matrix");
    match gwet::ac(m, None) {
        Ok(r) => world.ac1_result = Some(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute Gwet AC1 on relabeled")]
fn when_ac1_relabeled(world: &mut GwetWorld) {
    let m = world
        .relabeled_matrix
        .as_ref()
        .expect("no relabeled matrix");
    match gwet::ac(m, None) {
        Ok(r) => world.ac1_relabeled = Some(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I attempt Gwet AC1")]
fn when_attempt_ac1(world: &mut GwetWorld) {
    let m = world.matrix.as_ref().expect("no matrix");
    match gwet::ac(m, None) {
        Ok(r) => world.ac1_result = Some(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I attempt Gwet AC2 with the partial weights")]
fn when_attempt_ac2_partial(world: &mut GwetWorld) {
    let m = world.matrix.as_ref().expect("no matrix");
    let wm = world.custom_weights.as_ref().expect("no partial weights");
    match gwet::ac(m, Some(wm)) {
        Ok(r) => world.ac2_result = Some(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute Gwet AC3 with the custom weights")]
fn when_ac3(world: &mut GwetWorld) {
    let m = world.matrix.as_ref().expect("no matrix");
    let wm = world.custom_weights.as_ref().expect("no custom weights");
    match gwet::ac(m, Some(wm)) {
        Ok(r) => world.ac3_result = Some(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

// ======================== Then steps ========================

#[then(expr = "the result is {float} within {float}")]
fn then_result_within(world: &mut GwetWorld, expected: f64, tol: f64) {
    // Check ac2 first (for quadratic scenario), then ac1
    let actual = world
        .ac2_result
        .or(world.ac1_result)
        .expect("no AC result computed");
    assert!(
        (actual - expected).abs() < tol,
        "AC = {actual}, expected {expected} +/- {tol}"
    );
}

#[then("AC1 is greater than or equal to kappa")]
fn then_ac1_gte_kappa(world: &mut GwetWorld) {
    let ac1 = world.ac1_result.expect("no AC1 result");
    let kappa = world.cohen_kappa.expect("no Cohen kappa result");
    assert!(
        ac1 >= kappa - 1e-10,
        "AC1 ({ac1}) should be >= kappa ({kappa})"
    );
}

#[then(expr = "AC1 and AC2-identity match within {float}")]
fn then_ac1_ac2_identity_match(world: &mut GwetWorld, tol: f64) {
    let ac1 = world.ac1_result.expect("no AC1 result");
    let ac2 = world.ac2_result.expect("no AC2 result");
    assert!(
        (ac1 - ac2).abs() < tol,
        "AC1 ({ac1}) and AC2-identity ({ac2}) differ by more than {tol}"
    );
}

#[then(expr = "both AC1 values match within {float}")]
fn then_both_ac1_match(world: &mut GwetWorld, tol: f64) {
    let ac1 = world.ac1_result.expect("no AC1 result on original");
    let ac1_re = world.ac1_relabeled.expect("no AC1 result on relabeled");
    assert!(
        (ac1 - ac1_re).abs() < tol,
        "AC1 original ({ac1}) and relabeled ({ac1_re}) differ by more than {tol}"
    );
}

#[then(expr = "I get a Gwet error containing {string}")]
fn then_gwet_error(world: &mut GwetWorld, substring: String) {
    let err = world
        .error
        .as_ref()
        .expect("expected an error but got none");
    assert!(
        err.to_lowercase().contains(&substring.to_lowercase()),
        "error '{err}' does not contain '{substring}'"
    );
}

#[then("the result is a finite number between -1 and 1")]
fn then_finite_in_range(world: &mut GwetWorld) {
    let val = world.ac3_result.expect("no AC3 result");
    assert!(
        val.is_finite() && (-1.0..=1.0).contains(&val),
        "AC3 = {val}, expected finite in [-1, 1]"
    );
}

// ======================== Helpers ========================

fn discover_categories(m: &RatingMatrix) -> Vec<u32> {
    let mut cats: Vec<u32> = m
        .ratings
        .iter()
        .flat_map(|row| row.iter().filter_map(|&v| v))
        .collect();
    cats.sort_unstable();
    cats.dedup();
    cats
}

fn main() {
    let runner = GwetWorld::run(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tck/irr/gwet.feature"
    ));
    futures::executor::block_on(runner);
}
