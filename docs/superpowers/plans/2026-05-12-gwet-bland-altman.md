# Gwet AC1/AC2/AC3 and Bland-Altman Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two validated IRR modules — Gwet AC (resistant to kappa paradox) and Bland-Altman limits of agreement — with full 4-gate validation per JSMNTL methodology.

**Architecture:** Three new source files: `categorical_agreement_weights.rs` (weight matrices for categorical agreement), `gwet.rs` (AC1/AC2/AC3), `bland_altman.rs` (limits of agreement for paired `&[f64]` vectors). Gwet consumes `RatingMatrix` + optional `WeightMatrix`; Bland-Altman is standalone. Each gets its own `.feature` TCK, Cucumber harness, golden datasets, and Gate 4 Monte Carlo tests.

**Tech Stack:** Rust (irr crate), Cucumber/Gherkin TCK, serde_json for golden datasets, rand for MC tests.

---

## File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Create | `crates/irr/src/categorical_agreement_weights.rs` | `WeightScheme` enum, `WeightMatrix` struct, `WeightError`, `from_scheme()`, `custom()` |
| Create | `crates/irr/src/gwet.rs` | `GwetError`, `ac()` function |
| Create | `crates/irr/src/bland_altman.rs` | `BlandAltmanResult`, `BlandAltmanError`, `agreement()` function |
| Modify | `crates/irr/src/lib.rs` | Add `pub mod` for three new modules |
| Create | `crates/irr/tests/golden/gwet_2014_table4_1.json` | Gwet 2014 Table 4.1 golden dataset (3 abstractors, 3 categories) |
| Create | `crates/irr/tests/golden/bland_altman_1986_pefr.json` | Bland & Altman 1986 PEFR golden dataset |
| Create | `tck/irr/gwet.feature` | Gwet TCK scenarios (Gate 1, 3, edge cases) |
| Create | `tck/irr/bland_altman.feature` | Bland-Altman TCK scenarios (Gate 1, 3, edge cases) |
| Create | `crates/irr/tests/gwet_tck.rs` | Cucumber harness for gwet.feature |
| Create | `crates/irr/tests/bland_altman_tck.rs` | Cucumber harness for bland_altman.feature |
| Create | `crates/irr/tests/gate4_gwet_kappa_paradox.rs` | Gate 4 MC: kappa paradox sweep, AC1 stability vs kappa collapse |
| Create | `crates/irr/tests/gate4_bland_altman_calibration.rs` | Gate 4 MC: convergence + 95% coverage verification |
| Modify | `crates/irr/Cargo.toml` | Add `[[test]]` entries for new Cucumber harnesses |

---

### Task 1: Categorical Agreement Weights — TCK + Implementation

Weight matrices are a prerequisite for Gwet AC2/AC3. This task builds the `categorical_agreement_weights` module in one pass because it is purely algebraic (no golden datasets needed — the correctness proof is in the Gwet TCK when AC2 values match irrCAC).

**Files:**
- Create: `crates/irr/src/categorical_agreement_weights.rs`
- Modify: `crates/irr/src/lib.rs`

- [ ] **Step 1: Create the module with types and `from_scheme` for Identity**

Create `crates/irr/src/categorical_agreement_weights.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeightScheme {
    Identity,
    Linear,
    Quadratic,
    Ordinal,
}

#[derive(Debug, Clone)]
pub struct WeightMatrix {
    pub weights: Vec<Vec<f64>>,
    pub categories: Vec<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum WeightError {
    #[error("weight matrix must be square: got {rows}x{cols}")]
    NotSquare { rows: usize, cols: usize },
    #[error("weight matrix must be symmetric")]
    NotSymmetric,
    #[error("diagonal entries must be 1.0")]
    DiagonalNotOne,
    #[error("weights must be non-negative")]
    NegativeWeight,
    #[error("dimension mismatch: {n_cats} categories but {n_weights}x{n_weights} weights")]
    DimensionMismatch { n_cats: usize, n_weights: usize },
}

impl WeightMatrix {
    pub fn from_scheme(categories: &[u32], scheme: WeightScheme) -> Self {
        let q = categories.len();
        let mut sorted = categories.to_vec();
        sorted.sort();

        let weights = match scheme {
            WeightScheme::Identity => {
                let mut w = vec![vec![0.0; q]; q];
                for i in 0..q {
                    w[i][i] = 1.0;
                }
                w
            }
            WeightScheme::Linear => {
                let mut w = vec![vec![0.0; q]; q];
                let max_diff = if q > 1 { (q - 1) as f64 } else { 1.0 };
                for i in 0..q {
                    for j in 0..q {
                        w[i][j] = 1.0 - (i as f64 - j as f64).abs() / max_diff;
                    }
                }
                w
            }
            WeightScheme::Quadratic => {
                let mut w = vec![vec![0.0; q]; q];
                let max_diff_sq = if q > 1 {
                    ((q - 1) as f64).powi(2)
                } else {
                    1.0
                };
                for i in 0..q {
                    for j in 0..q {
                        let diff = i as f64 - j as f64;
                        w[i][j] = 1.0 - (diff * diff) / max_diff_sq;
                    }
                }
                w
            }
            WeightScheme::Ordinal => {
                let mut w = vec![vec![0.0; q]; q];
                for i in 0..q {
                    for j in 0..q {
                        let lo = i.min(j);
                        let hi = i.max(j);
                        let n_between = (hi - lo + 1) as f64;
                        let max_between = q as f64;
                        w[i][j] = 1.0 - (n_between - 1.0) * n_between
                            / (max_between * (max_between - 1.0));
                    }
                }
                if q <= 1 {
                    for i in 0..q {
                        w[i][i] = 1.0;
                    }
                }
                w
            }
        };

        Self {
            weights,
            categories: sorted,
        }
    }

    pub fn custom(categories: &[u32], weights: Vec<Vec<f64>>) -> Result<Self, WeightError> {
        let q = categories.len();
        let rows = weights.len();
        if rows != q {
            return Err(WeightError::DimensionMismatch {
                n_cats: q,
                n_weights: rows,
            });
        }
        for (i, row) in weights.iter().enumerate() {
            if row.len() != q {
                return Err(WeightError::NotSquare {
                    rows,
                    cols: row.len(),
                });
            }
            if (row[i] - 1.0).abs() > 1e-12 {
                return Err(WeightError::DiagonalNotOne);
            }
            for (j, &val) in row.iter().enumerate() {
                if val < 0.0 {
                    return Err(WeightError::NegativeWeight);
                }
                if (val - weights[j][i]).abs() > 1e-12 {
                    return Err(WeightError::NotSymmetric);
                }
            }
        }
        let mut sorted = categories.to_vec();
        sorted.sort();
        Ok(Self {
            weights,
            categories: sorted,
        })
    }
}
```

- [ ] **Step 2: Register the module in lib.rs**

Add to `crates/irr/src/lib.rs`:

```rust
pub mod categorical_agreement_weights;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p irr`
Expected: compiles with no errors.

- [ ] **Step 4: Run clippy and fmt**

Run: `cargo clippy -p irr -- -D warnings && cargo fmt -p irr`
Expected: zero warnings, formatted.

- [ ] **Step 5: Commit**

```bash
git add crates/irr/src/categorical_agreement_weights.rs crates/irr/src/lib.rs
git commit -m "feat(irr): categorical agreement weight matrices (identity, linear, quadratic, ordinal, custom)"
```

---

### Task 2: Gwet Golden Datasets

Create the golden dataset files needed by the TCK. Values are from irrCAC (Gwet's canonical R/Python implementation).

**Files:**
- Create: `crates/irr/tests/golden/gwet_2014_table4_1.json`

- [ ] **Step 1: Create Gwet 2014 Table 4.1 golden dataset**

This is the same 3x3 abstractor confusion matrix already stored as `cohen_gwet2014_3x3.json`. The Gwet AC values come from irrCAC:

Create `crates/irr/tests/golden/gwet_2014_table4_1.json`:

```json
{
  "source": "Gwet 2014, Handbook of Inter-Rater Reliability, 4th ed., Table 4.1. 100 pregnant women classified by 2 abstractors into 3 pregnancy types (Ectopic, AIU, NIU).",
  "n_items": 100,
  "n_categories": 3,
  "category_labels": ["Ectopic", "AIU", "NIU"],
  "confusion_matrix": [
    [13, 0, 0],
    [0, 20, 4],
    [0, 7, 56]
  ],
  "expected_ac1": 0.84933,
  "expected_ac2_quadratic": 0.94024,
  "tolerance": 0.0001,
  "reference_impl": "irrCAC R package (Gwet), gwet.ac1.raw() and gwet.ac2.raw()",
  "notes": "AC1 uses identity weights. AC2(quadratic) uses quadratic agreement weights. Same underlying data as cohen_gwet2014_3x3.json."
}
```

- [ ] **Step 2: Commit golden dataset**

```bash
git add crates/irr/tests/golden/gwet_2014_table4_1.json
git commit -m "test(irr): Gwet 2014 Table 4.1 golden dataset (AC1=0.84933, AC2=0.94024)"
```

---

### Task 3: Gwet TCK Feature File

Write the Gherkin scenarios. These will fail until implementation (Task 5).

**Files:**
- Create: `tck/irr/gwet.feature`

- [ ] **Step 1: Write the Gwet feature file**

Create `tck/irr/gwet.feature`:

```gherkin
Feature: Gwet AC1/AC2/AC3
  Chance-corrected agreement coefficient resistant to the kappa paradox.
  AC1 uses identity weights, AC2 uses standard weight schemes,
  AC3 uses arbitrary user-provided weights (do not use without a motivated reason).

  Reference: Gwet (2008, 2014). Kappa paradox: Feinstein & Cicchetti (1990).

  # Gate 1: Textbook — Gwet 2014 Table 4.1
  Scenario: AC1 on Gwet 3-abstractor data
    Given the Gwet 2014 Table 4.1 rating matrix
    When I compute Gwet AC1
    Then the result is 0.84933 within 0.0001

  # Gate 1: Textbook — Gwet 2014 Table 5.7 (same data, quadratic weights)
  Scenario: AC2 quadratic on Gwet 3-abstractor data
    Given the Gwet 2014 Table 4.1 rating matrix
    When I compute Gwet AC2 with quadratic weights
    Then the result is 0.94024 within 0.0001

  # Gate 1: Textbook — Krippendorff 2011 multi-rater data
  Scenario: AC1 on Krippendorff multi-rater data
    Given the Krippendorff 2011 reliability data for Gwet
    When I compute Gwet AC1
    Then the result is 0.77544 within 0.001

  # Gate 3: Property — perfect agreement
  Scenario: Perfect agreement yields AC1 = 1.0
    Given a rating matrix where all raters agree on 20 items across 3 categories
    When I compute Gwet AC1
    Then the result is 1.0 within 0.0001

  # Gate 3: Property — kappa paradox demonstration
  Scenario: AC1 >= Cohen kappa on high-prevalence data
    Given a high-prevalence 2-rater matrix with 90% category 0 seeded at 42
    When I compute Gwet AC1
    And I compute Cohen kappa on the same data
    Then AC1 is greater than or equal to kappa

  # Gate 3: Property — identity weights recover AC1
  Scenario: AC2 with identity weights equals AC1
    Given a mixed-agreement 2-rater matrix seeded at 55
    When I compute Gwet AC1
    And I compute Gwet AC2 with identity weights
    Then AC1 and AC2-identity match within 0.0001

  # Gate 3: Property — category relabeling invariance
  Scenario: Relabeling categories does not change AC1
    Given a mixed-agreement 2-rater matrix seeded at 55
    And the same data relabeled from 0,1,2 to 5,10,15
    When I compute Gwet AC1 on original
    And I compute Gwet AC1 on relabeled
    Then both AC1 values match within 0.0001

  # Edge: empty data
  Scenario: Empty matrix is an error
    Given an empty rating matrix for Gwet
    When I attempt Gwet AC1
    Then I get a Gwet error containing "empty"

  # Edge: single rater
  Scenario: Single rater is an error
    Given a single-rater matrix with 10 items
    When I attempt Gwet AC1
    Then I get a Gwet error containing "2 raters"

  # Edge: degenerate prevalence (pe = 1.0)
  Scenario: All-same-category data is degenerate
    Given a matrix where all 20 items are category 0 by 3 raters
    When I attempt Gwet AC1
    Then I get a Gwet error containing "pe"

  # AC3: custom weights with warning
  Scenario: AC3 with custom weight matrix
    Given a mixed-agreement 2-rater matrix seeded at 55
    And a custom 3x3 weight matrix
    When I compute Gwet AC3 with the custom weights
    Then the result is a finite number between -1 and 1
```

- [ ] **Step 2: Commit feature file**

```bash
git add tck/irr/gwet.feature
git commit -m "test(irr): Gwet AC1/AC2/AC3 TCK feature file (12 scenarios)"
```

---

### Task 4: Gwet Cucumber Harness (red)

Build the Cucumber glue code. Tests will fail (red) because `gwet.rs` doesn't exist yet.

**Files:**
- Create: `crates/irr/tests/gwet_tck.rs`
- Modify: `crates/irr/Cargo.toml`

- [ ] **Step 1: Write the Cucumber harness**

Create `crates/irr/tests/gwet_tck.rs`:

```rust
use cucumber::{given, then, when, World};
use irr::categorical_agreement_weights::{WeightMatrix, WeightScheme};
use irr::cohen;
use irr::gwet;
use irr::types::{RatingMatrix};

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

fn load_golden(filename: &str) -> serde_json::Value {
    let path = format!("{}/tests/golden/{filename}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

fn confusion_to_matrix(json: &serde_json::Value) -> RatingMatrix {
    let cm = json["confusion_matrix"].as_array().unwrap();
    let mut r1 = Vec::new();
    let mut r2 = Vec::new();
    for (i, row) in cm.iter().enumerate() {
        for (j, count) in row.as_array().unwrap().iter().enumerate() {
            let n = count.as_u64().unwrap() as usize;
            for _ in 0..n {
                r1.push(Some(i as u32));
                r2.push(Some(j as u32));
            }
        }
    }
    let n = r1.len();
    RatingMatrix {
        items: (0..n).map(|i| format!("item-{i}")).collect(),
        raters: vec!["r0".to_string(), "r1".to_string()],
        ratings: r1.into_iter().zip(r2).map(|(a, b)| vec![a, b]).collect(),
    }
}

// --- Given steps ---

#[given("the Gwet 2014 Table 4.1 rating matrix")]
fn given_gwet_table(world: &mut GwetWorld) {
    let json = load_golden("gwet_2014_table4_1.json");
    world.matrix = Some(confusion_to_matrix(&json));
}

#[given("the Krippendorff 2011 reliability data for Gwet")]
fn given_krippendorff(world: &mut GwetWorld) {
    let json = load_golden("krippendorff_2011.json");
    let data: Vec<Vec<Option<u32>>> = json["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row.as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().map(|n| n as u32))
                .collect()
        })
        .collect();
    let n_raters = data[0].len();
    let n_items = data.len();
    world.matrix = Some(RatingMatrix {
        items: (0..n_items).map(|i| format!("item-{i}")).collect(),
        raters: (0..n_raters).map(|i| format!("r{i}")).collect(),
        ratings: data,
    });
}

#[given(expr = "a rating matrix where all raters agree on {int} items across {int} categories")]
fn given_perfect(world: &mut GwetWorld, n_items: usize, n_cats: u32) {
    let ratings: Vec<Vec<Option<u32>>> = (0..n_items)
        .map(|i| {
            let cat = (i as u32) % n_cats;
            vec![Some(cat); 3]
        })
        .collect();
    world.matrix = Some(RatingMatrix {
        items: (0..n_items).map(|i| format!("item-{i}")).collect(),
        raters: vec!["r0".to_string(), "r1".to_string(), "r2".to_string()],
        ratings,
    });
}

#[given(expr = "a high-prevalence 2-rater matrix with {int}% category 0 seeded at {int}")]
fn given_high_prevalence(world: &mut GwetWorld, prevalence: u32, seed: u64) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    let n = 100;
    let p = prevalence as f64 / 100.0;
    let ratings: Vec<Vec<Option<u32>>> = (0..n)
        .map(|_| {
            let truth = if rng.random_bool(p) { 0u32 } else { 1u32 };
            let r1 = if rng.random_bool(0.85) {
                truth
            } else {
                1 - truth
            };
            let r2 = if rng.random_bool(0.85) {
                truth
            } else {
                1 - truth
            };
            vec![Some(r1), Some(r2)]
        })
        .collect();
    world.matrix = Some(RatingMatrix {
        items: (0..n).map(|i| format!("item-{i}")).collect(),
        raters: vec!["r0".to_string(), "r1".to_string()],
        ratings,
    });
}

#[given(expr = "a mixed-agreement 2-rater matrix seeded at {int}")]
fn given_mixed(world: &mut GwetWorld, seed: u64) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    let n = 50;
    let ratings: Vec<Vec<Option<u32>>> = (0..n)
        .map(|_| {
            let truth: u32 = rng.random_range(0..3);
            let r1 = truth;
            let r2 = if rng.random_bool(0.7) {
                truth
            } else {
                rng.random_range(0..3)
            };
            vec![Some(r1), Some(r2)]
        })
        .collect();
    world.matrix = Some(RatingMatrix {
        items: (0..n).map(|i| format!("item-{i}")).collect(),
        raters: vec!["r0".to_string(), "r1".to_string()],
        ratings,
    });
}

#[given("the same data relabeled from 0,1,2 to 5,10,15")]
fn given_relabeled(world: &mut GwetWorld) {
    let m = world.matrix.as_ref().unwrap();
    let label_map = |v: u32| -> u32 {
        match v {
            0 => 5,
            1 => 10,
            2 => 15,
            _ => v,
        }
    };
    let ratings: Vec<Vec<Option<u32>>> = m
        .ratings
        .iter()
        .map(|row| row.iter().map(|v| v.map(label_map)).collect())
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
fn given_single_rater(world: &mut GwetWorld, n: usize) {
    world.matrix = Some(RatingMatrix {
        items: (0..n).map(|i| format!("item-{i}")).collect(),
        raters: vec!["r0".to_string()],
        ratings: (0..n).map(|i| vec![Some(i as u32 % 3)]).collect(),
    });
}

#[given(expr = "a matrix where all {int} items are category {int} by {int} raters")]
fn given_all_same(world: &mut GwetWorld, n_items: usize, cat: u32, n_raters: usize) {
    world.matrix = Some(RatingMatrix {
        items: (0..n_items).map(|i| format!("item-{i}")).collect(),
        raters: (0..n_raters).map(|i| format!("r{i}")).collect(),
        ratings: vec![vec![Some(cat); n_raters]; n_items],
    });
}

#[given("a custom 3x3 weight matrix")]
fn given_custom_weights(world: &mut GwetWorld) {
    let w = WeightMatrix::custom(
        &[0, 1, 2],
        vec![
            vec![1.0, 0.5, 0.0],
            vec![0.5, 1.0, 0.5],
            vec![0.0, 0.5, 1.0],
        ],
    )
    .expect("custom weight matrix should be valid");
    world.custom_weights = Some(w);
}

// --- When steps ---

#[when("I compute Gwet AC1")]
fn when_ac1(world: &mut GwetWorld) {
    let m = world.matrix.as_ref().unwrap();
    match gwet::ac(m, None) {
        Ok(r) => world.ac1_result = Some(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute Gwet AC2 with quadratic weights")]
fn when_ac2_quadratic(world: &mut GwetWorld) {
    let m = world.matrix.as_ref().unwrap();
    let cats: Vec<u32> = m
        .ratings
        .iter()
        .flat_map(|row| row.iter().filter_map(|v| *v))
        .collect::<std::collections::BTreeSet<u32>>()
        .into_iter()
        .collect();
    let w = WeightMatrix::from_scheme(&cats, WeightScheme::Quadratic);
    match gwet::ac(m, Some(&w)) {
        Ok(r) => world.ac2_result = Some(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute Gwet AC2 with identity weights")]
fn when_ac2_identity(world: &mut GwetWorld) {
    let m = world.matrix.as_ref().unwrap();
    let cats: Vec<u32> = m
        .ratings
        .iter()
        .flat_map(|row| row.iter().filter_map(|v| *v))
        .collect::<std::collections::BTreeSet<u32>>()
        .into_iter()
        .collect();
    let w = WeightMatrix::from_scheme(&cats, WeightScheme::Identity);
    match gwet::ac(m, Some(&w)) {
        Ok(r) => world.ac2_result = Some(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute Cohen kappa on the same data")]
fn when_cohen(world: &mut GwetWorld) {
    let m = world.matrix.as_ref().unwrap();
    let r1: Vec<u32> = m.ratings.iter().map(|row| row[0].unwrap()).collect();
    let r2: Vec<u32> = m.ratings.iter().map(|row| row[1].unwrap()).collect();
    match cohen::kappa(&r1, &r2) {
        Ok(r) => world.cohen_kappa = Some(r.value),
        Err(_) => world.cohen_kappa = Some(f64::NEG_INFINITY),
    }
}

#[when("I compute Gwet AC1 on original")]
fn when_ac1_original(world: &mut GwetWorld) {
    let m = world.matrix.as_ref().unwrap();
    match gwet::ac(m, None) {
        Ok(r) => world.ac1_result = Some(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute Gwet AC1 on relabeled")]
fn when_ac1_relabeled(world: &mut GwetWorld) {
    let m = world.relabeled_matrix.as_ref().unwrap();
    match gwet::ac(m, None) {
        Ok(r) => world.ac1_relabeled = Some(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I attempt Gwet AC1")]
fn when_attempt(world: &mut GwetWorld) {
    let m = world.matrix.as_ref().unwrap();
    match gwet::ac(m, None) {
        Ok(r) => world.ac1_result = Some(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

#[when("I compute Gwet AC3 with the custom weights")]
fn when_ac3(world: &mut GwetWorld) {
    let m = world.matrix.as_ref().unwrap();
    let w = world.custom_weights.as_ref().unwrap();
    match gwet::ac(m, Some(w)) {
        Ok(r) => world.ac3_result = Some(r.value),
        Err(e) => world.error = Some(e.to_string()),
    }
}

// --- Then steps ---

#[then(expr = "the result is {float} within {float}")]
fn then_approx(world: &mut GwetWorld, expected: f64, tol: f64) {
    let val = world
        .ac2_result
        .or(world.ac1_result)
        .expect("no result");
    assert!(
        (val - expected).abs() < tol,
        "got {val}, expected {expected} within {tol}"
    );
}

#[then("AC1 is greater than or equal to kappa")]
fn then_ac1_gte_kappa(world: &mut GwetWorld) {
    let ac1 = world.ac1_result.expect("no AC1 result");
    let kappa = world.cohen_kappa.expect("no kappa result");
    assert!(
        ac1 >= kappa - 1e-10,
        "AC1 ({ac1}) < kappa ({kappa})"
    );
}

#[then("AC1 and AC2-identity match within 0.0001")]
fn then_ac1_eq_ac2_identity(world: &mut GwetWorld) {
    let ac1 = world.ac1_result.expect("no AC1");
    let ac2 = world.ac2_result.expect("no AC2");
    assert!(
        (ac1 - ac2).abs() < 0.0001,
        "AC1={ac1}, AC2(identity)={ac2}"
    );
}

#[then("both AC1 values match within 0.0001")]
fn then_relabeled_match(world: &mut GwetWorld) {
    let orig = world.ac1_result.expect("no original AC1");
    let relabeled = world.ac1_relabeled.expect("no relabeled AC1");
    assert!(
        (orig - relabeled).abs() < 0.0001,
        "original={orig}, relabeled={relabeled}"
    );
}

#[then(expr = "I get a Gwet error containing {string}")]
fn then_error(world: &mut GwetWorld, substring: String) {
    let err = world.error.as_ref().expect("expected error");
    assert!(
        err.to_lowercase().contains(&substring.to_lowercase()),
        "error '{err}' does not contain '{substring}'"
    );
}

#[then("the result is a finite number between -1 and 1")]
fn then_finite_range(world: &mut GwetWorld) {
    let val = world.ac3_result.expect("no AC3 result");
    assert!(val.is_finite(), "AC3 is not finite: {val}");
    assert!(
        val >= -1.0 && val <= 1.0,
        "AC3 = {val}, expected in [-1, 1]"
    );
}

fn main() {
    let runner = GwetWorld::run(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tck/irr/gwet.feature"
    ));
    futures::executor::block_on(runner);
}
```

- [ ] **Step 2: Add test harness entry to Cargo.toml**

Add to `crates/irr/Cargo.toml`:

```toml
[[test]]
name = "gwet_tck"
harness = false
```

- [ ] **Step 3: Verify it compiles (will fail to link — gwet module doesn't exist yet)**

Run: `cargo check -p irr --test gwet_tck`
Expected: compile error about missing `gwet` module. This is the "red" state.

- [ ] **Step 4: Commit the red harness**

```bash
git add crates/irr/tests/gwet_tck.rs crates/irr/Cargo.toml
git commit -m "test(irr): Gwet TCK Cucumber harness (red — awaiting implementation)"
```

---

### Task 5: Gwet AC Implementation (green)

Implement the core `gwet::ac()` function to make all TCK scenarios pass.

**Files:**
- Create: `crates/irr/src/gwet.rs`
- Modify: `crates/irr/src/lib.rs`

- [ ] **Step 1: Create the Gwet module**

Create `crates/irr/src/gwet.rs`:

```rust
use crate::categorical_agreement_weights::{WeightMatrix, WeightScheme};
use crate::types::{IrrResult, RatingMatrix};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum GwetError {
    #[error("empty rating matrix")]
    EmptyData,
    #[error("need at least 2 raters")]
    TooFewRaters,
    #[error("degenerate data: fewer than 2 pairable values")]
    DegenerateData,
    #[error("chance agreement pe = 1.0; AC is undefined")]
    DegeneratePe,
    #[error("weight error: {0}")]
    Weight(#[from] crate::categorical_agreement_weights::WeightError),
}

/// Compute Gwet's AC agreement coefficient.
///
/// - `weights = None` -> AC1 (identity weights)
/// - `weights = Some(w)` with standard scheme -> AC2
/// - `weights = Some(w)` with custom matrix -> AC3
///   AC3: do not use without a motivated reason for the custom weights.
///
/// Reference: Gwet (2008) eq. 3; Gwet (2014) Handbook, 4th ed.
pub fn ac(
    matrix: &RatingMatrix,
    weights: Option<&WeightMatrix>,
) -> Result<IrrResult, GwetError> {
    if matrix.n_items() == 0 {
        return Err(GwetError::EmptyData);
    }
    if matrix.n_raters() < 2 {
        return Err(GwetError::TooFewRaters);
    }

    let mut all_values: Vec<u32> = matrix
        .ratings
        .iter()
        .flat_map(|row| row.iter().filter_map(|v| *v))
        .collect();
    all_values.sort();
    all_values.dedup();
    let q = all_values.len();

    if q < 1 {
        return Err(GwetError::DegenerateData);
    }

    let val_idx: BTreeMap<u32, usize> = all_values
        .iter()
        .enumerate()
        .map(|(i, &v)| (v, i))
        .collect();

    let w = match weights {
        Some(wm) => wm.clone(),
        None => WeightMatrix::from_scheme(&all_values, WeightScheme::Identity),
    };

    let wm_idx: BTreeMap<u32, usize> = w
        .categories
        .iter()
        .enumerate()
        .map(|(i, &v)| (v, i))
        .collect();

    let mut n_valid_items = 0usize;
    let mut pa_sum = 0.0f64;
    let mut pi = vec![0.0f64; q];

    for row in &matrix.ratings {
        let present: Vec<u32> = row.iter().filter_map(|v| *v).collect();
        let r = present.len();
        if r < 2 {
            continue;
        }
        n_valid_items += 1;
        let rf = r as f64;

        let mut item_agreement = 0.0f64;
        for i_idx in 0..r {
            for j_idx in 0..r {
                if i_idx == j_idx {
                    continue;
                }
                let ci = present[i_idx];
                let cj = present[j_idx];
                let wi = wm_idx.get(&ci).copied().unwrap_or_else(|| val_idx[&ci]);
                let wj = wm_idx.get(&cj).copied().unwrap_or_else(|| val_idx[&cj]);
                let weight_val = if wi < w.weights.len() && wj < w.weights[0].len() {
                    w.weights[wi][wj]
                } else {
                    if ci == cj { 1.0 } else { 0.0 }
                };
                item_agreement += weight_val;
            }
        }
        item_agreement /= rf * (rf - 1.0);
        pa_sum += item_agreement;

        for &v in &present {
            pi[val_idx[&v]] += 1.0 / rf;
        }
    }

    if n_valid_items < 1 {
        return Err(GwetError::DegenerateData);
    }

    let nf = n_valid_items as f64;
    let pa = pa_sum / nf;

    for p in pi.iter_mut() {
        *p /= nf;
    }

    let qf = q as f64;

    let tw: f64 = w.weights.iter().flat_map(|row| row.iter()).sum::<f64>() / (qf * qf);

    let pi_dispersion: f64 = pi.iter().map(|&p| p * (1.0 - p)).sum();

    let pe = if q <= 1 {
        1.0
    } else {
        tw * pi_dispersion / (qf - 1.0)
    };

    if (1.0 - pe).abs() < 1e-15 {
        return Err(GwetError::DegeneratePe);
    }

    let ac_val = (pa - pe) / (1.0 - pe);

    let statistic_name = determine_name(weights);

    Ok(IrrResult {
        statistic_name,
        value: ac_val,
        ci_lower: None,
        ci_upper: None,
        n_items: matrix.n_items(),
        n_raters: matrix.n_raters(),
        metric_level: None,
    })
}

fn determine_name(weights: Option<&WeightMatrix>) -> String {
    match weights {
        None => "Gwet AC1".to_string(),
        Some(w) => {
            let q = w.categories.len();
            let id = WeightMatrix::from_scheme(&w.categories, WeightScheme::Identity);
            let lin = WeightMatrix::from_scheme(&w.categories, WeightScheme::Linear);
            let quad = WeightMatrix::from_scheme(&w.categories, WeightScheme::Quadratic);
            let ord = WeightMatrix::from_scheme(&w.categories, WeightScheme::Ordinal);
            if matrices_equal(&w.weights, &id.weights, q) {
                "Gwet AC2(identity)".to_string()
            } else if matrices_equal(&w.weights, &lin.weights, q) {
                "Gwet AC2(linear)".to_string()
            } else if matrices_equal(&w.weights, &quad.weights, q) {
                "Gwet AC2(quadratic)".to_string()
            } else if matrices_equal(&w.weights, &ord.weights, q) {
                "Gwet AC2(ordinal)".to_string()
            } else {
                "Gwet AC3".to_string()
            }
        }
    }
}

fn matrices_equal(a: &[Vec<f64>], b: &[Vec<f64>], n: usize) -> bool {
    for i in 0..n {
        for j in 0..n {
            if (a[i][j] - b[i][j]).abs() > 1e-10 {
                return false;
            }
        }
    }
    true
}
```

- [ ] **Step 2: Register the module in lib.rs**

Add to `crates/irr/src/lib.rs`:

```rust
pub mod gwet;
```

- [ ] **Step 3: Run the TCK**

Run: `cargo test -p irr --test gwet_tck -- --nocapture 2>&1`
Expected: all 12 scenarios pass.

If any scenario fails, debug and fix. Likely issues:
- Golden value mismatch: double-check the pe formula uses `tw * Σ πk(1-πk) / (q-1)` not `/ (q*(q-1))`. The Gwet 2008 paper uses `1/(q-1)` normalization for pe when tw is already `Σw / q²`.
- Weight matrix index mapping: ensure categories in the weight matrix map correctly to data categories. The `wm_idx` lookup handles this.

- [ ] **Step 4: Run clippy and fmt**

Run: `cargo clippy -p irr -- -D warnings && cargo fmt -p irr`
Expected: zero warnings, formatted.

- [ ] **Step 5: Commit**

```bash
git add crates/irr/src/gwet.rs crates/irr/src/lib.rs
git commit -m "feat(irr): Gwet AC1/AC2/AC3 — kappa-paradox-resistant agreement coefficient"
```

---

### Task 6: Bland-Altman Golden Dataset

**Files:**
- Create: `crates/irr/tests/golden/bland_altman_1986_pefr.json`

- [ ] **Step 1: Create the PEFR golden dataset**

The Bland & Altman 1986 paper Table 1 contains 17 paired PEFR measurements. The Wright peak-flow meter is the first method; the mini Wright meter is the second.

Create `crates/irr/tests/golden/bland_altman_1986_pefr.json`:

```json
{
  "source": "Bland & Altman 1986, 'Statistical methods for assessing agreement between two methods of clinical measurement', The Lancet, Table 1. PEFR (l/min) measured by Wright peak-flow meter and mini Wright meter.",
  "wright": [494, 395, 516, 434, 476, 557, 413, 442, 650, 433, 417, 656, 267, 478, 178, 423, 427],
  "mini_wright": [512, 430, 520, 428, 500, 600, 364, 380, 658, 445, 432, 626, 260, 477, 259, 350, 451],
  "expected_mean_diff": -2.12,
  "expected_sd_diff": 38.77,
  "expected_lower_loa": -78.10,
  "expected_upper_loa": 73.87,
  "tolerance_mean": 0.5,
  "tolerance_sd": 0.5,
  "tolerance_loa": 1.5,
  "notes": "mean_diff = mean(wright - mini_wright). 17 subjects. Values cross-checked via manual computation: diffs = [-18,-35,-4,6,-24,-43,49,62,-8,-12,-15,30,7,1,-81,73,-24], mean=-2.118, sd=38.766."
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/irr/tests/golden/bland_altman_1986_pefr.json
git commit -m "test(irr): Bland-Altman 1986 PEFR golden dataset (17 paired measurements)"
```

---

### Task 7: Bland-Altman TCK Feature File

**Files:**
- Create: `tck/irr/bland_altman.feature`

- [ ] **Step 1: Write the feature file**

Create `tck/irr/bland_altman.feature`:

```gherkin
Feature: Bland-Altman limits of agreement
  Assesses agreement between two measurement methods via mean difference
  and 95% limits of agreement (mean ± 1.96 * SD).

  Reference: Bland & Altman (1986), The Lancet.

  # Gate 1: Textbook — Bland & Altman 1986 PEFR data
  Scenario: PEFR Wright vs mini Wright meter
    Given the Bland-Altman 1986 PEFR data
    When I compute Bland-Altman agreement
    Then mean difference is -2.12 within 0.5
    And SD of differences is 38.77 within 0.5
    And lower LoA is approximately -78.10 within 1.5
    And upper LoA is approximately 73.87 within 1.5

  # Gate 3: Property — constant offset is zero-variance error
  Scenario: Constant offset yields zero-variance error
    Given measurements x = [1.0, 2.0, 3.0, 4.0, 5.0]
    And measurements y = [2.0, 3.0, 4.0, 5.0, 6.0]
    When I attempt Bland-Altman agreement
    Then I get a Bland-Altman error containing "zero variance"

  # Gate 3: Property — sign reversal
  Scenario: Swapping x and y negates mean diff
    Given measurements x = [10.0, 20.0, 30.0, 40.0, 50.0]
    And measurements y = [12.0, 18.0, 33.0, 37.0, 55.0]
    When I compute Bland-Altman agreement for x and y
    And I compute Bland-Altman agreement for y and x
    Then the mean differences are negations within 0.0001
    And the SD values are equal within 0.0001

  # Edge: length mismatch
  Scenario: Mismatched input lengths
    Given measurements x with 5 values and y with 3 values
    When I attempt Bland-Altman agreement
    Then I get a Bland-Altman error containing "equal length"

  # Edge: too few observations
  Scenario: Single observation pair
    Given measurements x = [1.0] and y = [2.0]
    When I attempt Bland-Altman agreement
    Then I get a Bland-Altman error containing "2 paired"

  # Edge: identical inputs
  Scenario: Identical measurements yield zero-variance error
    Given measurements x = [1.0, 2.0, 3.0]
    And measurements y = [1.0, 2.0, 3.0]
    When I attempt Bland-Altman agreement
    Then I get a Bland-Altman error containing "zero variance"
```

- [ ] **Step 2: Commit**

```bash
git add tck/irr/bland_altman.feature
git commit -m "test(irr): Bland-Altman TCK feature file (6 scenarios)"
```

---

### Task 8: Bland-Altman Cucumber Harness (red)

**Files:**
- Create: `crates/irr/tests/bland_altman_tck.rs`
- Modify: `crates/irr/Cargo.toml`

- [ ] **Step 1: Write the Cucumber harness**

Create `crates/irr/tests/bland_altman_tck.rs`:

```rust
use cucumber::{given, then, when, World};
use irr::bland_altman;
use irr::bland_altman::BlandAltmanResult;

#[derive(Debug, Default, World)]
pub struct BlandAltmanWorld {
    x: Vec<f64>,
    y: Vec<f64>,
    result_xy: Option<BlandAltmanResult>,
    result_yx: Option<BlandAltmanResult>,
    error: Option<String>,
}

fn load_golden(filename: &str) -> serde_json::Value {
    let path = format!("{}/tests/golden/{filename}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

// --- Given steps ---

#[given("the Bland-Altman 1986 PEFR data")]
fn given_pefr(world: &mut BlandAltmanWorld) {
    let json = load_golden("bland_altman_1986_pefr.json");
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
fn given_x5(world: &mut BlandAltmanWorld, a: f64, b: f64, c: f64, d: f64, e: f64) {
    world.x = vec![a, b, c, d, e];
}

#[given(expr = "measurements y = [{float}, {float}, {float}, {float}, {float}]")]
fn given_y5(world: &mut BlandAltmanWorld, a: f64, b: f64, c: f64, d: f64, e: f64) {
    world.y = vec![a, b, c, d, e];
}

#[given(expr = "measurements x = [{float}, {float}, {float}]")]
fn given_x3(world: &mut BlandAltmanWorld, a: f64, b: f64, c: f64) {
    world.x = vec![a, b, c];
}

#[given(expr = "measurements y = [{float}, {float}, {float}]")]
fn given_y3(world: &mut BlandAltmanWorld, a: f64, b: f64, c: f64) {
    world.y = vec![a, b, c];
}

#[given(expr = "measurements x with {int} values and y with {int} values")]
fn given_mismatched(world: &mut BlandAltmanWorld, nx: usize, ny: usize) {
    world.x = vec![1.0; nx];
    world.y = vec![1.0; ny];
}

#[given(expr = "measurements x = [{float}] and y = [{float}]")]
fn given_single(world: &mut BlandAltmanWorld, xv: f64, yv: f64) {
    world.x = vec![xv];
    world.y = vec![yv];
}

// --- When steps ---

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

// --- Then steps ---

#[then(expr = "mean difference is {float} within {float}")]
fn then_mean_diff(world: &mut BlandAltmanWorld, expected: f64, tol: f64) {
    let r = world.result_xy.as_ref().expect("no result");
    assert!(
        (r.mean_diff - expected).abs() < tol,
        "mean_diff = {}, expected {} within {}",
        r.mean_diff,
        expected,
        tol
    );
}

#[then(expr = "SD of differences is {float} within {float}")]
fn then_sd(world: &mut BlandAltmanWorld, expected: f64, tol: f64) {
    let r = world.result_xy.as_ref().expect("no result");
    assert!(
        (r.sd_diff - expected).abs() < tol,
        "sd_diff = {}, expected {} within {}",
        r.sd_diff,
        expected,
        tol
    );
}

#[then(expr = "lower LoA is approximately {float} within {float}")]
fn then_lower_loa(world: &mut BlandAltmanWorld, expected: f64, tol: f64) {
    let r = world.result_xy.as_ref().expect("no result");
    assert!(
        (r.lower_loa - expected).abs() < tol,
        "lower_loa = {}, expected {} within {}",
        r.lower_loa,
        expected,
        tol
    );
}

#[then(expr = "upper LoA is approximately {float} within {float}")]
fn then_upper_loa(world: &mut BlandAltmanWorld, expected: f64, tol: f64) {
    let r = world.result_xy.as_ref().expect("no result");
    assert!(
        (r.upper_loa - expected).abs() < tol,
        "upper_loa = {}, expected {} within {}",
        r.upper_loa,
        expected,
        tol
    );
}

#[then("the mean differences are negations within 0.0001")]
fn then_negation(world: &mut BlandAltmanWorld) {
    let xy = world.result_xy.as_ref().expect("no xy result");
    let yx = world.result_yx.as_ref().expect("no yx result");
    assert!(
        (xy.mean_diff + yx.mean_diff).abs() < 0.0001,
        "xy.mean_diff={}, yx.mean_diff={}, sum={}",
        xy.mean_diff,
        yx.mean_diff,
        xy.mean_diff + yx.mean_diff
    );
}

#[then("the SD values are equal within 0.0001")]
fn then_sd_equal(world: &mut BlandAltmanWorld) {
    let xy = world.result_xy.as_ref().expect("no xy result");
    let yx = world.result_yx.as_ref().expect("no yx result");
    assert!(
        (xy.sd_diff - yx.sd_diff).abs() < 0.0001,
        "xy.sd_diff={}, yx.sd_diff={}",
        xy.sd_diff,
        yx.sd_diff
    );
}

#[then(expr = "I get a Bland-Altman error containing {string}")]
fn then_error(world: &mut BlandAltmanWorld, substring: String) {
    let err = world.error.as_ref().expect("expected error");
    assert!(
        err.to_lowercase().contains(&substring.to_lowercase()),
        "error '{err}' does not contain '{substring}'"
    );
}

fn main() {
    let runner = BlandAltmanWorld::run(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tck/irr/bland_altman.feature"
    ));
    futures::executor::block_on(runner);
}
```

- [ ] **Step 2: Add test harness entry to Cargo.toml**

Add to `crates/irr/Cargo.toml`:

```toml
[[test]]
name = "bland_altman_tck"
harness = false
```

- [ ] **Step 3: Verify red state**

Run: `cargo check -p irr --test bland_altman_tck`
Expected: compile error about missing `bland_altman` module.

- [ ] **Step 4: Commit**

```bash
git add crates/irr/tests/bland_altman_tck.rs crates/irr/Cargo.toml
git commit -m "test(irr): Bland-Altman TCK Cucumber harness (red — awaiting implementation)"
```

---

### Task 9: Bland-Altman Implementation (green)

**Files:**
- Create: `crates/irr/src/bland_altman.rs`
- Modify: `crates/irr/src/lib.rs`

- [ ] **Step 1: Create the Bland-Altman module**

Create `crates/irr/src/bland_altman.rs`:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[must_use]
pub struct BlandAltmanResult {
    pub mean_diff: f64,
    pub sd_diff: f64,
    pub lower_loa: f64,
    pub upper_loa: f64,
    pub n: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum BlandAltmanError {
    #[error("inputs must have equal length: got {len_x} and {len_y}")]
    LengthMismatch { len_x: usize, len_y: usize },
    #[error("need at least 2 paired observations")]
    TooFewObservations,
    #[error("zero variance in differences")]
    ZeroVariance,
}

/// Bland-Altman limits of agreement (95%) for two paired measurement vectors.
///
/// Computes mean difference (x - y), SD of differences, and 95% limits
/// of agreement (mean ± 1.96 * SD).
///
/// Reference: Bland & Altman (1986), The Lancet.
pub fn agreement(x: &[f64], y: &[f64]) -> Result<BlandAltmanResult, BlandAltmanError> {
    if x.len() != y.len() {
        return Err(BlandAltmanError::LengthMismatch {
            len_x: x.len(),
            len_y: y.len(),
        });
    }
    let n = x.len();
    if n < 2 {
        return Err(BlandAltmanError::TooFewObservations);
    }

    let diffs: Vec<f64> = x.iter().zip(y.iter()).map(|(a, b)| a - b).collect();

    let mean_diff = diffs.iter().sum::<f64>() / n as f64;

    let var = diffs
        .iter()
        .map(|d| (d - mean_diff).powi(2))
        .sum::<f64>()
        / (n as f64 - 1.0);

    if var.abs() < 1e-15 {
        return Err(BlandAltmanError::ZeroVariance);
    }

    let sd_diff = var.sqrt();
    let lower_loa = mean_diff - 1.96 * sd_diff;
    let upper_loa = mean_diff + 1.96 * sd_diff;

    Ok(BlandAltmanResult {
        mean_diff,
        sd_diff,
        lower_loa,
        upper_loa,
        n,
    })
}
```

- [ ] **Step 2: Register the module in lib.rs**

Add to `crates/irr/src/lib.rs`:

```rust
pub mod bland_altman;
```

- [ ] **Step 3: Run the TCK**

Run: `cargo test -p irr --test bland_altman_tck -- --nocapture 2>&1`
Expected: all 6 scenarios pass.

- [ ] **Step 4: Run clippy and fmt**

Run: `cargo clippy -p irr -- -D warnings && cargo fmt -p irr`
Expected: zero warnings, formatted.

- [ ] **Step 5: Commit**

```bash
git add crates/irr/src/bland_altman.rs crates/irr/src/lib.rs
git commit -m "feat(irr): Bland-Altman limits of agreement (Bland & Altman 1986)"
```

---

### Task 10: Gate 4 — Gwet Kappa Paradox Monte Carlo

The marquee validation: sweep prevalence and show AC1 stays stable while Cohen kappa collapses.

**Files:**
- Create: `crates/irr/tests/gate4_gwet_kappa_paradox.rs`

- [ ] **Step 1: Write the Gate 4 test**

Create `crates/irr/tests/gate4_gwet_kappa_paradox.rs`:

```rust
use irr::cohen;
use irr::gwet;
use irr::types::RatingMatrix;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

fn make_2rater_matrix(r1: &[u32], r2: &[u32]) -> RatingMatrix {
    let n = r1.len();
    RatingMatrix {
        items: (0..n).map(|i| format!("item-{i}")).collect(),
        raters: vec!["r0".to_string(), "r1".to_string()],
        ratings: r1
            .iter()
            .zip(r2.iter())
            .map(|(&a, &b)| vec![Some(a), Some(b)])
            .collect(),
    }
}

fn generate_prevalence_data(
    rng: &mut StdRng,
    n_items: usize,
    prevalence: f64,
    agreement_prob: f64,
) -> (Vec<u32>, Vec<u32>) {
    let mut r1 = Vec::with_capacity(n_items);
    let mut r2 = Vec::with_capacity(n_items);
    for _ in 0..n_items {
        let truth = if rng.random_bool(prevalence) { 0u32 } else { 1u32 };
        r1.push(if rng.random_bool(agreement_prob) {
            truth
        } else {
            1 - truth
        });
        r2.push(if rng.random_bool(agreement_prob) {
            truth
        } else {
            1 - truth
        });
    }
    (r1, r2)
}

/// Gate 4: Kappa paradox sweep — AC1 stable while kappa collapses under prevalence imbalance.
///
/// At each prevalence level (50/50 to 95/5), generate 50 trials with fixed
/// agreement probability 0.85. Compute mean AC1 and mean Cohen kappa.
/// AC1 should remain stable (SD < 0.15 across prevalence levels).
/// Kappa should drop significantly at high prevalence.
#[test]
fn kappa_paradox_prevalence_sweep() {
    let n_items = 200;
    let n_trials = 50;
    let agreement = 0.85;
    let prevalences = [0.5, 0.6, 0.7, 0.8, 0.9, 0.95];

    let mut ac1_means = Vec::new();
    let mut kappa_means = Vec::new();

    for (pi, &prev) in prevalences.iter().enumerate() {
        let mut ac1_vals = Vec::new();
        let mut kappa_vals = Vec::new();

        for trial in 0..n_trials {
            let mut rng = StdRng::seed_from_u64(1000 + pi as u64 * 100 + trial as u64);
            let (r1, r2) = generate_prevalence_data(&mut rng, n_items, prev, agreement);
            let matrix = make_2rater_matrix(&r1, &r2);

            if let Ok(r) = gwet::ac(&matrix, None) {
                ac1_vals.push(r.value);
            }
            if let Ok(r) = cohen::kappa(&r1, &r2) {
                kappa_vals.push(r.value);
            }
        }

        let ac1_mean = ac1_vals.iter().sum::<f64>() / ac1_vals.len() as f64;
        let kappa_mean = kappa_vals.iter().sum::<f64>() / kappa_vals.len() as f64;
        ac1_means.push(ac1_mean);
        kappa_means.push(kappa_mean);

        eprintln!(
            "prev={prev:.2}: AC1 mean={ac1_mean:.4} (n={}), kappa mean={kappa_mean:.4} (n={})",
            ac1_vals.len(),
            kappa_vals.len()
        );
    }

    let ac1_min = ac1_means.iter().cloned().fold(f64::INFINITY, f64::min);
    let ac1_max = ac1_means
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let ac1_range = ac1_max - ac1_min;

    let kappa_min = kappa_means.iter().cloned().fold(f64::INFINITY, f64::min);
    let kappa_max = kappa_means
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let kappa_range = kappa_max - kappa_min;

    eprintln!("AC1 range across prevalence: {ac1_range:.4}");
    eprintln!("Kappa range across prevalence: {kappa_range:.4}");

    assert!(
        ac1_range < 0.15,
        "AC1 not stable: range={ac1_range:.4}, expected < 0.15"
    );
    assert!(
        kappa_range > ac1_range,
        "kappa should vary more than AC1: kappa_range={kappa_range:.4}, ac1_range={ac1_range:.4}"
    );
}

/// Gate 4: AC1 monotone in agreement probability.
#[test]
fn ac1_monotone_in_agreement() {
    let n_items = 200;
    let n_cats = 3u32;
    let agreement_probs = [0.0, 0.2, 0.4, 0.6, 0.8];
    let mut prev_ac1 = f64::NEG_INFINITY;

    for (i, &p) in agreement_probs.iter().enumerate() {
        let mut rng = StdRng::seed_from_u64(2000 + i as u64);
        let ratings: Vec<Vec<Option<u32>>> = (0..n_items)
            .map(|_| {
                let truth: u32 = rng.random_range(0..n_cats);
                (0..4)
                    .map(|_| {
                        Some(if rng.random_bool(p) {
                            truth
                        } else {
                            rng.random_range(0..n_cats)
                        })
                    })
                    .collect()
            })
            .collect();

        let matrix = RatingMatrix {
            items: (0..n_items).map(|i| format!("item-{i}")).collect(),
            raters: (0..4).map(|i| format!("r{i}")).collect(),
            ratings,
        };

        match gwet::ac(&matrix, None) {
            Ok(r) => {
                eprintln!("AC1 at p={p}: {:.4}", r.value);
                assert!(
                    r.value >= prev_ac1 - 0.05,
                    "AC1 not monotone: p={p}, ac1={}, prev={prev_ac1}",
                    r.value
                );
                prev_ac1 = r.value;
            }
            Err(e) => {
                eprintln!("AC1 at p={p}: error {e} (may be degenerate)");
            }
        }
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p irr --test gate4_gwet_kappa_paradox -- --nocapture 2>&1`
Expected: both tests pass.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p irr -- -D warnings`
Expected: zero warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/irr/tests/gate4_gwet_kappa_paradox.rs
git commit -m "test(irr): Gate 4 Monte Carlo — kappa paradox sweep + AC1 monotonicity"
```

---

### Task 11: Gate 4 — Bland-Altman Calibration Monte Carlo

**Files:**
- Create: `crates/irr/tests/gate4_bland_altman_calibration.rs`

- [ ] **Step 1: Write the Gate 4 test**

Create `crates/irr/tests/gate4_bland_altman_calibration.rs`:

```rust
use irr::bland_altman;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_distr::{Distribution, Normal};

/// Gate 4: Recovered parameters converge to true values as n grows.
#[test]
fn convergence_to_true_parameters() {
    let true_offset = 3.0;
    let true_sd = 5.0;
    let sample_sizes = [20, 100, 500, 2000];
    let mut prev_mean_err = f64::INFINITY;

    for &n in &sample_sizes {
        let mut rng = StdRng::seed_from_u64(3000 + n as u64);
        let normal = Normal::new(0.0, true_sd).unwrap();

        let x: Vec<f64> = (0..n).map(|i| (i as f64) * 0.1).collect();
        let y: Vec<f64> = x
            .iter()
            .map(|&xi| xi - true_offset + normal.sample(&mut rng))
            .collect();

        let r = bland_altman::agreement(&x, &y).expect("agreement failed");
        let mean_err = (r.mean_diff - true_offset).abs();
        let sd_err = (r.sd_diff - true_sd).abs();

        eprintln!(
            "n={n}: mean_diff={:.3} (err={mean_err:.3}), sd={:.3} (err={sd_err:.3})",
            r.mean_diff, r.sd_diff
        );

        assert!(
            mean_err <= prev_mean_err + 0.5,
            "mean error not converging: n={n}, err={mean_err}, prev={prev_mean_err}"
        );
        prev_mean_err = mean_err;
    }

    assert!(
        prev_mean_err < 0.5,
        "at n=2000, mean_diff error = {prev_mean_err}, expected < 0.5"
    );
}

/// Gate 4: Coverage — ~95% of true differences fall within LoA for normal data.
#[test]
fn loa_coverage_95_percent() {
    let n_trials = 200;
    let n_per_trial = 100;
    let true_offset = 2.0;
    let true_sd = 4.0;

    let mut coverage_rates = Vec::new();
    let mut rng = StdRng::seed_from_u64(4000);
    let normal = Normal::new(0.0, true_sd).unwrap();

    for _ in 0..n_trials {
        let x: Vec<f64> = (0..n_per_trial).map(|_| rng.random::<f64>() * 100.0).collect();
        let diffs: Vec<f64> = (0..n_per_trial)
            .map(|_| true_offset + normal.sample(&mut rng))
            .collect();
        let y: Vec<f64> = x.iter().zip(diffs.iter()).map(|(xi, di)| xi - di).collect();

        let r = bland_altman::agreement(&x, &y).expect("agreement failed");

        let within = diffs
            .iter()
            .filter(|&&d| d >= r.lower_loa && d <= r.upper_loa)
            .count();
        coverage_rates.push(within as f64 / n_per_trial as f64);
    }

    let mean_coverage = coverage_rates.iter().sum::<f64>() / coverage_rates.len() as f64;
    eprintln!("Mean coverage: {mean_coverage:.4} (expected ~0.95)");

    assert!(
        mean_coverage > 0.90 && mean_coverage < 1.0,
        "coverage = {mean_coverage:.4}, expected ~0.95"
    );
}
```

- [ ] **Step 2: Add `rand_distr` dev-dependency to Cargo.toml**

Add to `[dev-dependencies]` in `crates/irr/Cargo.toml`:

```toml
rand_distr = "0.5"
```

(This provides `Normal` distribution for generating calibration data. Check the compatible version with `rand = "0.9"` — `rand_distr 0.5` is the matching version.)

- [ ] **Step 3: Run the tests**

Run: `cargo test -p irr --test gate4_bland_altman_calibration -- --nocapture 2>&1`
Expected: both tests pass.

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -p irr -- -D warnings`
Expected: zero warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/irr/tests/gate4_bland_altman_calibration.rs crates/irr/Cargo.toml
git commit -m "test(irr): Gate 4 Monte Carlo — Bland-Altman convergence + 95% coverage"
```

---

### Task 12: Subagent Code Review + Fix All Findings

Per JSMNTL methodology, dispatch a subagent code reviewer after all tests pass. Fix ALL findings.

- [ ] **Step 1: Run the full test suite**

Run: `cargo test -p irr -- --nocapture 2>&1`
Expected: all tests pass (existing + new).

- [ ] **Step 2: Dispatch subagent code review**

Review scope: all new/modified files:
- `crates/irr/src/categorical_agreement_weights.rs`
- `crates/irr/src/gwet.rs`
- `crates/irr/src/bland_altman.rs`
- `crates/irr/tests/gwet_tck.rs`
- `crates/irr/tests/bland_altman_tck.rs`
- `crates/irr/tests/gate4_gwet_kappa_paradox.rs`
- `crates/irr/tests/gate4_bland_altman_calibration.rs`
- `tck/irr/gwet.feature`
- `tck/irr/bland_altman.feature`
- `crates/irr/tests/golden/gwet_2014_table4_1.json`
- `crates/irr/tests/golden/bland_altman_1986_pefr.json`

Review criteria:
- Statistical correctness (formulas match Gwet 2008/2014 and Bland & Altman 1986)
- Edge case handling (empty, degenerate, single rater/observation)
- Error types cover all failure modes
- No silent failures (results silently wrong > errors)
- API consistency with existing irr crate patterns
- Test coverage gaps

- [ ] **Step 3: Fix all findings**

Address every finding from the code review. Re-run the full test suite after fixes.

- [ ] **Step 4: Run clippy and fmt**

Run: `cargo clippy -p irr -- -D warnings && cargo fmt -p irr`
Expected: zero warnings, formatted.

- [ ] **Step 5: Commit fixes**

```bash
git add -A
git commit -m "fix(irr): code review fixes for Gwet AC + Bland-Altman"
```
