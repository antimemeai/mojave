#![allow(clippy::needless_range_loop)]

use cucumber::{given, then, when, World};
use irr::dawid_skene::{self, DawidSkeneConfig};
use irr::types::{AnnotationTriple, DawidSkeneResult};

#[derive(Debug, Default, World)]
pub struct DsWorld {
    triples: Vec<AnnotationTriple>,
    true_labels: Vec<u32>,
    result: Option<DawidSkeneResult>,
    error: Option<String>,
    n_items: usize,
    n_classes: u32,
}

fn make_triple(item: usize, annotator: usize, label: u32) -> AnnotationTriple {
    AnnotationTriple {
        item_id: format!("item-{item}"),
        annotator_id: format!("ann-{annotator}"),
        label,
    }
}

#[given(expr = "{int} annotators who all agree perfectly on {int} items with {int} classes")]
fn given_perfect(world: &mut DsWorld, n_ann: usize, n_items: usize, n_classes: u32) {
    world.n_items = n_items;
    world.n_classes = n_classes;
    world.true_labels = (0..n_items).map(|i| i as u32 % n_classes).collect();
    for i in 0..n_items {
        let label = world.true_labels[i];
        for j in 0..n_ann {
            world.triples.push(make_triple(i, j, label));
        }
    }
}

#[given(expr = "{int} annotators on {int} items with {int} classes")]
fn given_base(world: &mut DsWorld, _n_ann: usize, n_items: usize, n_classes: u32) {
    world.n_items = n_items;
    world.n_classes = n_classes;
    world.true_labels = (0..n_items).map(|i| i as u32 % n_classes).collect();
}

#[given("annotator 0 and 1 are perfect")]
fn given_perfect_annotators(world: &mut DsWorld) {
    for (i, &label) in world.true_labels.iter().enumerate() {
        world.triples.push(make_triple(i, 0, label));
        world.triples.push(make_triple(i, 1, label));
    }
}

#[given(expr = "annotator 2 flips labels {int}% of the time seeded at {int}")]
fn given_bad_annotator(world: &mut DsWorld, flip_pct: usize, seed: u64) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    let flip_rate = flip_pct as f64 / 100.0;
    for (i, &label) in world.true_labels.iter().enumerate() {
        let assigned = if rng.random_bool(flip_rate) {
            (label + 1) % world.n_classes
        } else {
            label
        };
        world.triples.push(make_triple(i, 2, assigned));
    }
}

#[given(expr = "{int}% of annotations are missing at random seeded at {int}")]
fn given_missing(world: &mut DsWorld, miss_pct: usize, seed: u64) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    let n_ann = 3usize;
    world.true_labels = (0..world.n_items)
        .map(|i| i as u32 % world.n_classes)
        .collect();
    let miss_rate = miss_pct as f64 / 100.0;
    for i in 0..world.n_items {
        let label = world.true_labels[i];
        for j in 0..n_ann {
            if !rng.random_bool(miss_rate) {
                world.triples.push(make_triple(i, j, label));
            }
        }
    }
}

#[given(expr = "{int} perfect annotators on {int} items with {int} classes evenly distributed")]
fn given_perfect_even(world: &mut DsWorld, n_ann: usize, n_items: usize, n_classes: u32) {
    world.n_items = n_items;
    world.n_classes = n_classes;
    world.true_labels = (0..n_items).map(|i| i as u32 % n_classes).collect();
    for i in 0..n_items {
        let label = world.true_labels[i];
        for j in 0..n_ann {
            world.triples.push(make_triple(i, j, label));
        }
    }
}

#[given("no annotation triples")]
fn given_empty(world: &mut DsWorld) {
    world.triples.clear();
}

#[given(expr = "{int} annotators on {int} items all labeled class {int}")]
fn given_single_class(world: &mut DsWorld, n_ann: usize, n_items: usize, class: u32) {
    world.n_items = n_items;
    world.n_classes = 1;
    world.true_labels = vec![class; n_items];
    for i in 0..n_items {
        for j in 0..n_ann {
            world.triples.push(make_triple(i, j, class));
        }
    }
}

#[when(expr = "I fit Dawid-Skene with max {int} EM iterations")]
fn when_fit(world: &mut DsWorld, max_iter: usize) {
    let config = DawidSkeneConfig {
        max_iterations: max_iter,
        tolerance: 1e-6,
    };
    match dawid_skene::fit(&world.triples, &config) {
        Ok(r) => world.result = Some(r),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I attempt Dawid-Skene fitting")]
fn when_attempt(world: &mut DsWorld) {
    let config = DawidSkeneConfig::default();
    match dawid_skene::fit(&world.triples, &config) {
        Ok(r) => world.result = Some(r),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[then("the model converged")]
fn then_converged(world: &mut DsWorld) {
    let r = world.result.as_ref().expect("no result");
    assert!(
        r.converged,
        "model did not converge after {} iterations",
        r.n_iterations
    );
}

#[then("all confusion matrices are approximately identity")]
fn then_identity_matrices(world: &mut DsWorld) {
    let r = world.result.as_ref().expect("no result");
    for (j, cm) in r.confusion_matrices.iter().enumerate() {
        for k in 0..cm.len() {
            for l in 0..cm[k].len() {
                let expected = if k == l { 1.0 } else { 0.0 };
                assert!(
                    (cm[k][l] - expected).abs() < 0.05,
                    "annotator {j} confusion[{k}][{l}] = {}, expected {expected}",
                    cm[k][l]
                );
            }
        }
    }
}

#[then("the estimated labels match the input labels")]
fn then_labels_match(world: &mut DsWorld) {
    let r = world.result.as_ref().expect("no result");
    assert_eq!(
        r.estimated_labels, world.true_labels,
        "estimated labels do not match true labels"
    );
}

#[then(expr = "annotator 2 has off-diagonal mass > {float}")]
fn then_bad_annotator(world: &mut DsWorld, threshold: f64) {
    let r = world.result.as_ref().expect("no result");
    let cm = &r.confusion_matrices[2];
    let mut off_diag = 0.0;
    let mut total = 0.0;
    for k in 0..cm.len() {
        for l in 0..cm[k].len() {
            total += cm[k][l];
            if k != l {
                off_diag += cm[k][l];
            }
        }
    }
    let off_diag_frac = off_diag / total;
    assert!(
        off_diag_frac > threshold,
        "annotator 2 off-diagonal fraction = {off_diag_frac}, expected > {threshold}"
    );
}

#[then(expr = "annotator 0 and 1 have off-diagonal mass < {float}")]
fn then_good_annotators(world: &mut DsWorld, threshold: f64) {
    let r = world.result.as_ref().expect("no result");
    for ann in [0, 1] {
        let cm = &r.confusion_matrices[ann];
        let mut off_diag = 0.0;
        let mut total = 0.0;
        for k in 0..cm.len() {
            for l in 0..cm[k].len() {
                total += cm[k][l];
                if k != l {
                    off_diag += cm[k][l];
                }
            }
        }
        let off_diag_frac = off_diag / total;
        assert!(
            off_diag_frac < threshold,
            "annotator {ann} off-diagonal fraction = {off_diag_frac}, expected < {threshold}"
        );
    }
}

#[then("the estimated labels mostly match the true labels")]
fn then_mostly_match(world: &mut DsWorld) {
    let r = world.result.as_ref().expect("no result");
    let correct = r
        .estimated_labels
        .iter()
        .zip(world.true_labels.iter())
        .filter(|(a, b)| a == b)
        .count();
    let accuracy = correct as f64 / world.true_labels.len() as f64;
    assert!(accuracy > 0.9, "accuracy = {accuracy}, expected > 0.9");
}

#[then(expr = "the estimated labels have > {int}% accuracy vs true labels")]
fn then_accuracy(world: &mut DsWorld, min_pct: usize) {
    let r = world.result.as_ref().expect("no result");
    let correct = r
        .estimated_labels
        .iter()
        .zip(world.true_labels.iter())
        .filter(|(a, b)| a == b)
        .count();
    let accuracy = correct as f64 / world.true_labels.len() as f64;
    let threshold = min_pct as f64 / 100.0;
    assert!(
        accuracy > threshold,
        "accuracy = {accuracy}, expected > {threshold}"
    );
}

#[then(expr = "each class prior is approximately {float} with tolerance {float}")]
fn then_priors(world: &mut DsWorld, expected: f64, tol: f64) {
    let r = world.result.as_ref().expect("no result");
    for (k, &prior) in r.class_priors.iter().enumerate() {
        assert!(
            (prior - expected).abs() < tol,
            "class prior[{k}] = {prior}, expected {expected} ± {tol}"
        );
    }
}

#[then("I get a Dawid-Skene error about empty data")]
fn then_empty_error(world: &mut DsWorld) {
    let err = world.error.as_ref().expect("expected error");
    assert!(err.contains("empty"), "error: {err}");
}

#[then("all estimated labels are class 0")]
fn then_all_class_zero(world: &mut DsWorld) {
    let r = world.result.as_ref().expect("no result");
    assert!(
        r.estimated_labels.iter().all(|&l| l == 0),
        "not all labels are 0: {:?}",
        r.estimated_labels
    );
}

fn main() {
    let runner = DsWorld::run(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tck/irr"));
    futures::executor::block_on(runner);
}
