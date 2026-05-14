# SALib Parity Gaps Implementation Plan (BEAD-0016)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close 5 method gaps between salib-rs and Python SALib so salib-rs becomes a strict superset.

**Architecture:** Each gap adds to existing crate boundaries (salib-core, salib-samplers, salib-estimators, salib-validation, salib-surrogate). No new crates. TCK-first per JSMNTL. 4-gate validation per method.

**Tech Stack:** Rust, ndarray 0.16, salib-core (tree_sum/tree_dot determinism), metric-tck-harness (Gherkin), proptest (Gate 3)

**Spec:** `docs/specs/2026-05-14-salib-parity-gaps-design.md`

---

## File Structure

**New files:**
- `tck/salib/second-order-sobol/features/second_order_sobol.feature`
- `crates/salib-estimators/tests/second_order_tck.rs`
- `tck/salib/fractional-factorial/features/fractional_factorial.feature`
- `crates/salib-samplers/src/plackett_burman.rs`
- `crates/salib-estimators/src/fractional_factorial.rs`
- `crates/salib-estimators/tests/fractional_factorial_tck.rs`
- `tck/salib/discrepancy/features/discrepancy.feature`
- `crates/salib-estimators/src/discrepancy.rs`
- `crates/salib-estimators/tests/discrepancy_tck.rs`
- `tck/salib/grouped-factors/features/grouped_factors.feature`
- `crates/salib-estimators/tests/grouped_factors_tck.rs`
- `crates/salib-estimators/src/hdmr.rs`
- `tck/salib/hdmr/features/hdmr.feature`
- `crates/salib-estimators/tests/hdmr_tck.rs`

**Modified files:**
- `crates/salib-estimators/src/sobol_indices.rs` — add `second_order` field
- `crates/salib-estimators/src/saltelli2010.rs` — compute S2
- `crates/salib-estimators/src/jansen.rs` — compute S2
- `crates/salib-estimators/src/janon.rs` — compute S2
- `crates/salib-estimators/src/owen.rs` — compute S2
- `crates/salib-estimators/src/morris.rs` — grouped EE aggregation
- `crates/salib-estimators/src/lib.rs` — re-exports
- `crates/salib-estimators/Cargo.toml` — add salib-surrogate dep
- `crates/salib-samplers/src/lib.rs` — re-exports
- `crates/salib-samplers/src/morris.rs` — grouped trajectories
- `crates/salib-samplers/src/saltelli_matrix.rs` — grouped column swap
- `crates/salib-core/src/problem.rs` — Group type + groups field on Problem
- `crates/salib-core/src/lib.rs` — re-export Group
- `crates/salib-validation/src/analytic.rs` — add second_order to SobolIndicesAnalytic
- `crates/salib-validation/src/ishigami.rs` — Ishigami S2 analytic values

---

### Task 1: Second-Order Sobol Indices — Types + Analytic + TCK

Add the `second_order` field to both `SobolIndices` (estimator output) and `SobolIndicesAnalytic` (validation ground truth). Compute Ishigami's closed-form S2 values. Write TCK feature file and test harness.

**Files:**
- Modify: `crates/salib-estimators/src/sobol_indices.rs`
- Modify: `crates/salib-validation/src/analytic.rs`
- Modify: `crates/salib-validation/src/ishigami.rs`
- Create: `tck/salib/second-order-sobol/features/second_order_sobol.feature`
- Create: `crates/salib-estimators/tests/second_order_tck.rs`

- [ ] **Step 1: Add `second_order` field to `SobolIndices`**

In `crates/salib-estimators/src/sobol_indices.rs`, add after the `total_order` field:

```rust
/// `S2_{ij}` second-order indices. `second_order[i][j]` for `i < j`.
/// Indexed such that `second_order[i]` has length `dim - i - 1` and
/// `second_order[i][k]` = `S2_{i, i+k+1}`.
/// `None` when second-order was not requested (i.e. `SaltelliMatrix.b_a` is `None`).
pub second_order: Option<Vec<Vec<f64>>>,
```

Update `SobolIndices::new` (or wherever the constructor is) to accept and store this field. Existing callers should pass `None`.

- [ ] **Step 2: Add `second_order` field to `SobolIndicesAnalytic`**

In `crates/salib-validation/src/analytic.rs`, add to `SobolIndicesAnalytic`:

```rust
/// `S2_{ij}` for `i < j`. Same indexing as `SobolIndices::second_order`.
/// `None` if no second-order analytic values are available.
pub second_order: Option<Vec<Vec<f64>>>,
```

Update `SobolIndicesAnalytic::new` to accept an optional second-order parameter. Existing callers pass `None`.

- [ ] **Step 3: Add Ishigami S2 analytic values**

In `crates/salib-validation/src/ishigami.rs`, in `analytic_indices(a, b)`:

The Ishigami second-order interaction variances (Saltelli Primer 2008):
- `V_{12} = 0` (X1 and X2 are additively separable)
- `V_{13} = 8·b²·π⁸/225` (the X1-X3 interaction)
- `V_{23} = 0` (X2 and X3 don't interact)

So `S2_{12} = 0`, `S2_{13} = V_{13}/D`, `S2_{23} = 0`.

```rust
// Second-order: S2_{ij} = V_{ij} / D
// V_{12} = 0, V_{13} = 8·b²·π⁸/225, V_{23} = 0
let s2_12 = 0.0;
let s2_13 = v13 / total_variance;  // v13 already computed above
let s2_23 = 0.0;

// Indexing: second_order[0] = [S2_{01}, S2_{02}] = [S2_{12}, S2_{13}]
//           second_order[1] = [S2_{12}] = [S2_{23}]
let second_order = Some(vec![
    vec![s2_12, s2_13],
    vec![s2_23],
]);
```

Pass `second_order` to `SobolIndicesAnalytic::new(...)`.

- [ ] **Step 4: Write TCK feature file**

Create `tck/salib/second-order-sobol/features/second_order_sobol.feature`:

```gherkin
Feature: Second-order Sobol indices — Ishigami at canonical (a=7, b=0.1, N=8192)

  Scenario: S2_13 recovers the X1-X3 interaction
    Given the Ishigami canonical model with a=7 and b=0.1
    And a Sobol base sampler at dim 6 with skip_first false
    When I build a Saltelli matrix at N=8192 with second_order=true and run Saltelli2010
    Then S2_13 is within 0.05 of analytic 0.244

  Scenario: S2_12 and S2_23 are near zero (no interactions)
    Given the Ishigami canonical model with a=7 and b=0.1
    And a Sobol base sampler at dim 6 with skip_first false
    When I build a Saltelli matrix at N=8192 with second_order=true and run Saltelli2010
    Then S2_12 is within 0.05 of zero
    And S2_23 is within 0.05 of zero

  Scenario: Second-order indices are symmetric
    Given the Ishigami canonical model with a=7 and b=0.1
    And a Sobol base sampler at dim 6 with skip_first false
    When I build a Saltelli matrix at N=8192 with second_order=true and run Saltelli2010
    Then S2_ij equals S2_ji for all i,j

  Scenario: Sum of all first and second order indices is at most 1
    Given the Ishigami canonical model with a=7 and b=0.1
    And a Sobol base sampler at dim 6 with skip_first false
    When I build a Saltelli matrix at N=8192 with second_order=true and run Saltelli2010
    Then the sum of S_i plus the sum of S2_ij is at most 1.05
```

- [ ] **Step 5: Write TCK test harness (red)**

Create `crates/salib-estimators/tests/second_order_tck.rs`. Follow the pattern from `ishigami_e2e.rs`: use `SobolSampler::standard(6).with_skip_first(false)`, `build_saltelli_matrix(&sampler, 8192, true, &mut rng)` (note `second_order=true`), map through Ishigami's Uniform(-π,π), call `estimate_saltelli2010`. Assert `second_order` is `Some(...)` and check values against analytic.

- [ ] **Step 6: Run tests — verify they fail (red)**

Run: `cargo test -p salib-estimators --test second_order_tck --release 2>&1`
Expected: compilation error or test failure because `estimate_saltelli2010` doesn't compute S2 yet.

- [ ] **Step 7: Commit types + analytic + TCK (red)**

```bash
git add crates/salib-estimators/src/sobol_indices.rs \
       crates/salib-validation/src/analytic.rs \
       crates/salib-validation/src/ishigami.rs \
       tck/salib/second-order-sobol/ \
       crates/salib-estimators/tests/second_order_tck.rs
git commit -m "feat(salib): S2 types + Ishigami analytic + TCK (red)"
```

---

### Task 2: Second-Order Sobol — Saltelli2010 Estimator (Green)

Implement S2 computation in `estimate_saltelli2010`. This is the core formula from Saltelli 2010 Eq (d).

**Files:**
- Modify: `crates/salib-estimators/src/saltelli2010.rs`

- [ ] **Step 1: Implement S2 computation**

In `estimate_saltelli2010`, after computing `first_order` and `total_order`, add S2 computation when `matrix.b_a` is `Some(...)`:

```rust
let second_order = matrix.b_a.as_ref().map(|b_a_matrices| {
    // Evaluate model on each B_A^j matrix
    let fba: Vec<Vec<f64>> = b_a_matrices
        .iter()
        .map(|m| evaluate_rows(m, &model))
        .collect();

    // S2_{ij} = (1/N) Σ_k [fba[j][k] · fab[i][k] - fa[k] · fb[k]] / D - S_i - S_j
    // for i < j
    let mut s2: Vec<Vec<f64>> = Vec::with_capacity(d);
    for i in 0..d {
        let mut row = Vec::with_capacity(d - i - 1);
        for j in (i + 1)..d {
            let cross: Vec<f64> = (0..n)
                .map(|k| fba[j][k] * fab[i][k] - fa[k] * fb[k])
                .collect();
            let vij = tree_sum(&cross) / n_f;
            let s2_ij = vij / d_var - first_order[i] - first_order[j];
            row.push(s2_ij);
        }
        s2.push(row);
    }
    s2
});
```

Pass `second_order` to the `SobolIndices` constructor.

- [ ] **Step 2: Run tests — verify green**

Run: `cargo test -p salib-estimators --test second_order_tck --release 2>&1`
Expected: PASS — S2_{13} ≈ 0.244, S2_{12} ≈ 0, S2_{23} ≈ 0.

- [ ] **Step 3: Run full existing test suite — verify no regressions**

Run: `cargo test -p salib-estimators --release 2>&1`
Expected: all existing tests still pass.

- [ ] **Step 4: Commit**

```bash
git add crates/salib-estimators/src/saltelli2010.rs
git commit -m "feat(salib): S2 computation in estimate_saltelli2010"
```

---

### Task 3: Second-Order Sobol — Wire S2 into Jansen/Janon/Owen

Same S2 formula applies to all four radial-design estimators. They differ only in first-order formula; total-order and second-order formulas are the same (Saltelli 2010 Eq d).

**Files:**
- Modify: `crates/salib-estimators/src/jansen.rs`
- Modify: `crates/salib-estimators/src/janon.rs`
- Modify: `crates/salib-estimators/src/owen.rs`

- [ ] **Step 1: Add S2 to Jansen estimator**

In `estimate_jansen`, add the same S2 computation block as Task 2. The Jansen estimator uses `SaltelliMatrix` and already evaluates `fa`, `fb`, `fab`. Add `fba` evaluation and the S2 loop. Return S2 in the `JansenIndices` struct (add `second_order: Option<Vec<Vec<f64>>>` field to `JansenIndices`).

- [ ] **Step 2: Add S2 to Janon estimator**

Same pattern in `estimate_janon`. Add `second_order` field to `JanonIndices`.

- [ ] **Step 3: Add S2 to Owen estimator**

Same pattern in `estimate_owen`. Note: Owen uses a different matrix design (3 base matrices A, B, C), but the S2 formula still needs `B_A^j` matrices. If `SaltelliMatrix.b_a` is `None`, return `second_order: None`. Add `second_order` field to `OwenIndices`.

- [ ] **Step 4: Run full test suite**

Run: `cargo test -p salib-estimators --release 2>&1`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/salib-estimators/src/jansen.rs \
       crates/salib-estimators/src/janon.rs \
       crates/salib-estimators/src/owen.rs
git commit -m "feat(salib): S2 in Jansen, Janon, Owen estimators"
```

---

### Task 4: Plackett-Burman Sampler

Implement the Plackett-Burman fractional factorial design sampler.

**Files:**
- Create: `crates/salib-samplers/src/plackett_burman.rs`
- Modify: `crates/salib-samplers/src/lib.rs`

- [ ] **Step 1: Define types and error enum**

Create `crates/salib-samplers/src/plackett_burman.rs`:

```rust
use ndarray::Array2;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct PlackettBurmanDesign {
    pub matrix: Array2<f64>,  // N × d, values in {-1.0, +1.0}
    pub n_runs: usize,
    pub dim: usize,
}

#[derive(Debug, Clone, Error)]
pub enum PbError {
    #[error("dimension must be at least 1, got 0")]
    ZeroDim,
    #[error("dimension {0} exceeds maximum supported 23")]
    DimTooLarge(usize),
}
```

- [ ] **Step 2: Implement PB construction**

Plackett-Burman construction via cyclic shift of known generating vectors.

For N runs (N = next multiple of 4 ≥ d+1), the generating vector has N-1 elements of {-1, +1}. Each subsequent row is a cyclic left-shift of the previous row. The last row is all -1.

Known generating vectors for standard sizes:
- N=4: `[+1, -1, +1]`
- N=8: `[+1, +1, +1, -1, +1, -1, -1]`
- N=12: `[+1, +1, -1, +1, +1, +1, -1, -1, -1, +1, -1]`
- N=16: Hadamard — `[+1, +1, +1, +1, -1, +1, -1, +1, +1, -1, -1, +1, -1, -1, -1]`
- N=20: `[+1, +1, -1, +1, +1, -1, -1, -1, -1, +1, -1, +1, -1, +1, +1, +1, +1, -1, -1]`
- N=24: `[+1, +1, +1, +1, +1, -1, +1, -1, +1, +1, -1, -1, +1, +1, -1, -1, +1, -1, +1, -1, -1, -1, -1]`

```rust
pub fn build_plackett_burman(dim: usize) -> Result<PlackettBurmanDesign, PbError> {
    if dim == 0 {
        return Err(PbError::ZeroDim);
    }
    if dim > 23 {
        return Err(PbError::DimTooLarge(dim));
    }
    let n = next_multiple_of_4(dim + 1);
    let gen = generating_vector(n);
    let mut matrix = Array2::<f64>::zeros((n, dim));
    // First row: first `dim` elements of gen
    for j in 0..dim {
        matrix[[0, j]] = gen[j];
    }
    // Rows 1..N-1: cyclic left-shift of gen
    for i in 1..(n - 1) {
        for j in 0..dim {
            matrix[[i, j]] = gen[(j + i) % (n - 1)];
        }
    }
    // Last row: all -1
    for j in 0..dim {
        matrix[[n - 1, j]] = -1.0;
    }
    Ok(PlackettBurmanDesign { matrix, n_runs: n, dim })
}

fn next_multiple_of_4(min_val: usize) -> usize {
    let rem = min_val % 4;
    if rem == 0 { min_val } else { min_val + (4 - rem) }
}
```

- [ ] **Step 3: Add unit tests**

In the same file under `#[cfg(test)]`:

```rust
#[test]
fn pb_dim3_gives_4_runs() {
    let pb = build_plackett_burman(3).unwrap();
    assert_eq!(pb.n_runs, 4);
    assert_eq!(pb.dim, 3);
    // Every entry is ±1
    for &v in pb.matrix.iter() {
        assert!(v == 1.0 || v == -1.0);
    }
}

#[test]
fn pb_orthogonality() {
    // X'X should be N·I (approximate for PB, exact for Hadamard)
    let pb = build_plackett_burman(7).unwrap();
    let xt = pb.matrix.t();
    let xtx = xt.dot(&pb.matrix);
    for i in 0..pb.dim {
        assert!((xtx[[i, i]] - pb.n_runs as f64).abs() < 1e-10);
    }
}

#[test]
fn pb_zero_dim_error() {
    assert!(matches!(build_plackett_burman(0), Err(PbError::ZeroDim)));
}
```

- [ ] **Step 4: Re-export from salib-samplers lib.rs**

Add to `crates/salib-samplers/src/lib.rs`:

```rust
mod plackett_burman;
pub use plackett_burman::{build_plackett_burman, PlackettBurmanDesign, PbError};
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p salib-samplers --release 2>&1`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/salib-samplers/src/plackett_burman.rs crates/salib-samplers/src/lib.rs
git commit -m "feat(salib): Plackett-Burman fractional factorial sampler"
```

---

### Task 5: Fractional Factorial Analyzer + TCK

Implement the fractional factorial effects analyzer and end-to-end TCK test.

**Files:**
- Create: `crates/salib-estimators/src/fractional_factorial.rs`
- Modify: `crates/salib-estimators/src/lib.rs`
- Create: `tck/salib/fractional-factorial/features/fractional_factorial.feature`
- Create: `crates/salib-estimators/tests/fractional_factorial_tck.rs`

- [ ] **Step 1: Write TCK feature file**

Create `tck/salib/fractional-factorial/features/fractional_factorial.feature`:

```gherkin
Feature: Fractional Factorial screening — Plackett-Burman

  Scenario: Linear 3-factor model recovers exact main effects
    Given a 3-factor linear model f(x) = 2*x1 + 3*x2 + 0.5*x3
    And x_i in [-1, +1]
    When I run Plackett-Burman screening
    Then main_effect[0] is within 0.01 of 4.0
    And main_effect[1] is within 0.01 of 6.0
    And main_effect[2] is within 0.01 of 1.0

  Scenario: Ishigami screening ranks X2 highest
    Given the Ishigami canonical model with a=7 and b=0.1
    When I run Plackett-Burman screening with dim=3
    Then the factor with the largest absolute main effect is X2

  Scenario: Main effects from a balanced design sum to near zero for zero-mean Y
    Given a 5-factor model f(x) = x1 - x2
    When I run Plackett-Burman screening
    Then factors 3, 4, 5 have main effects within 0.5 of zero
```

- [ ] **Step 2: Define result type and implement analyzer**

Create `crates/salib-estimators/src/fractional_factorial.rs`:

```rust
use salib_core::Problem;
use salib_samplers::{PlackettBurmanDesign, build_plackett_burman};

#[derive(Debug, Clone)]
pub struct FractionalFactorialEffects {
    pub dim: usize,
    pub n_runs: usize,
    pub main_effects: Vec<f64>,
    pub main_effects_abs: Vec<f64>,
}

/// Estimate main effects from a Plackett-Burman design.
///
/// For each factor i, main_effect_i = mean(Y where x_i = +1) - mean(Y where x_i = -1).
/// The model function receives inputs mapped from the design's {-1,+1} coding
/// to the problem's factor bounds: x_physical = lo + (x_coded + 1)/2 * (hi - lo).
pub fn estimate_fractional_factorial<F>(
    design: &PlackettBurmanDesign,
    problem: &Problem,
    model: F,
) -> FractionalFactorialEffects
where
    F: Fn(&[f64]) -> f64,
{
    let n = design.n_runs;
    let d = design.dim;

    // Evaluate model at each design point, mapping coded → physical
    let mut y = vec![0.0_f64; n];
    let mut x_phys = vec![0.0_f64; d];
    for i in 0..n {
        for j in 0..d {
            let coded = design.matrix[[i, j]];
            let (lo, hi) = problem.factors()[j].distribution.support();
            x_phys[j] = lo + (coded + 1.0) / 2.0 * (hi - lo);
        }
        y[i] = model(&x_phys);
    }

    // Main effect for factor j = mean(Y where x_j=+1) - mean(Y where x_j=-1)
    let mut main_effects = vec![0.0_f64; d];
    for j in 0..d {
        let mut sum_plus = 0.0_f64;
        let mut count_plus = 0usize;
        let mut sum_minus = 0.0_f64;
        let mut count_minus = 0usize;
        for i in 0..n {
            if design.matrix[[i, j]] > 0.0 {
                sum_plus += y[i];
                count_plus += 1;
            } else {
                sum_minus += y[i];
                count_minus += 1;
            }
        }
        main_effects[j] = sum_plus / count_plus as f64 - sum_minus / count_minus as f64;
    }

    let main_effects_abs: Vec<f64> = main_effects.iter().map(|e| e.abs()).collect();

    FractionalFactorialEffects {
        dim: d,
        n_runs: n,
        main_effects,
        main_effects_abs,
    }
}
```

- [ ] **Step 3: Re-export from lib.rs**

Add to `crates/salib-estimators/src/lib.rs`:

```rust
mod fractional_factorial;
pub use fractional_factorial::{estimate_fractional_factorial, FractionalFactorialEffects};
```

- [ ] **Step 4: Write TCK test harness**

Create `crates/salib-estimators/tests/fractional_factorial_tck.rs` — test the linear model scenario (exact main effects recovery) and Ishigami ranking.

- [ ] **Step 5: Run tests — verify green**

Run: `cargo test -p salib-estimators --test fractional_factorial_tck --release 2>&1`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/salib-estimators/src/fractional_factorial.rs \
       crates/salib-estimators/src/lib.rs \
       tck/salib/fractional-factorial/ \
       crates/salib-estimators/tests/fractional_factorial_tck.rs
git commit -m "feat(salib): fractional factorial screening (PB sampler + analyzer)"
```

---

### Task 6: Discrepancy Indices

Implement space-filling quality metrics: Centered Discrepancy (CD), Wrap-around Discrepancy (WD), Modified Discrepancy (MD), L2-star Discrepancy.

**Files:**
- Create: `crates/salib-estimators/src/discrepancy.rs`
- Modify: `crates/salib-estimators/src/lib.rs`
- Create: `tck/salib/discrepancy/features/discrepancy.feature`
- Create: `crates/salib-estimators/tests/discrepancy_tck.rs`

- [ ] **Step 1: Write TCK feature file**

Create `tck/salib/discrepancy/features/discrepancy.feature`:

```gherkin
Feature: Discrepancy indices — space-filling quality metrics

  Scenario: Regular grid has known centered discrepancy
    Given a 2D regular grid of 4 points in [0,1]^2
    When I compute discrepancy
    Then centered_discrepancy is within 0.01 of the analytic value

  Scenario: Sobol sequence has lower discrepancy than random
    Given a Sobol sample of N=256 in d=3
    And a random sample of N=256 in d=3
    When I compute discrepancy for both
    Then the Sobol centered_discrepancy is less than the random centered_discrepancy

  Scenario: Discrepancy decreases with N for Sobol
    Given Sobol samples at N=64 and N=256 in d=3
    When I compute discrepancy for both
    Then centered_discrepancy at N=256 is less than at N=64

  Scenario: All discrepancy values are non-negative
    Given any sample matrix in [0,1]^d
    When I compute discrepancy
    Then centered, wrap_around, modified, and l2_star are all non-negative
```

- [ ] **Step 2: Implement discrepancy formulas**

Create `crates/salib-estimators/src/discrepancy.rs`:

```rust
use ndarray::Array2;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct DiscrepancyResult {
    pub centered: f64,
    pub wrap_around: f64,
    pub modified: f64,
    pub l2_star: f64,
}

#[derive(Debug, Clone, Error)]
pub enum DiscrepancyError {
    #[error("sample matrix is empty")]
    EmptyMatrix,
    #[error("sample values must be in [0, 1], found {0}")]
    NotUnitInterval(f64),
}

/// Compute all four discrepancy metrics for a sample matrix in [0,1]^d.
///
/// Formulas from Fang et al. 2006 and Hickernell 1998.
/// Complexity: O(N² · d) for N samples in d dimensions.
pub fn compute_discrepancy(sample: &Array2<f64>) -> Result<DiscrepancyResult, DiscrepancyError> {
    let n = sample.nrows();
    let d = sample.ncols();
    if n == 0 {
        return Err(DiscrepancyError::EmptyMatrix);
    }
    // Validate [0,1] range
    for &v in sample.iter() {
        if v < 0.0 || v > 1.0 {
            return Err(DiscrepancyError::NotUnitInterval(v));
        }
    }
    let n_f = n as f64;

    Ok(DiscrepancyResult {
        centered: centered_discrepancy(sample, n, d, n_f),
        wrap_around: wrap_around_discrepancy(sample, n, d, n_f),
        modified: modified_discrepancy(sample, n, d, n_f),
        l2_star: l2_star_discrepancy(sample, n, d, n_f),
    })
}
```

**Centered Discrepancy (Hickernell 1998 Eq 3.8):**
```
CD² = (13/12)^d - (2/N) Σ_i Π_k [1 + |x_{ik} - 0.5|/2 - |x_{ik} - 0.5|²/2]
     + (1/N²) Σ_i Σ_j Π_k [1 + |x_{ik} - 0.5|/2 + |x_{jk} - 0.5|/2 - |x_{ik} - x_{jk}|/2]
```

**Wrap-around Discrepancy (Hickernell 1998 Eq 3.10):**
```
WD² = -(4/3)^d + (1/N²) Σ_i Σ_j Π_k [3/2 - |x_{ik} - x_{jk}| · (1 - |x_{ik} - x_{jk}|)]
```

**L2-star Discrepancy (Niederreiter):**
```
L2*² = (1/3)^d - (2^{1-d}/N) Σ_i Π_k (1 - x_{ik}²)
      + (1/N²) Σ_i Σ_j Π_k [1 - max(x_{ik}, x_{jk})]
```

**Modified Discrepancy (Fang et al. 2006):**
```
MD² = (19/12)^d - (2/N) Σ_i Π_k [(19 - 5|2x_{ik}-1| - 5|2x_{ik}-1|²) / 12]
     + (1/N²) Σ_i Σ_j Π_k [(19 - 5|2x_{ik}-1| - 5|2x_{jk}-1| + 5|x_{ik}-x_{jk}|) / 12]
```

Implement each as a private helper function. Return `sqrt(val)` for each (discrepancy is defined as the square root).

- [ ] **Step 3: Re-export from lib.rs**

Add to `crates/salib-estimators/src/lib.rs`:

```rust
mod discrepancy;
pub use discrepancy::{compute_discrepancy, DiscrepancyResult, DiscrepancyError};
```

- [ ] **Step 4: Write TCK test harness and unit tests**

Create `crates/salib-estimators/tests/discrepancy_tck.rs`.

Unit test in `discrepancy.rs`:
```rust
#[test]
fn regular_2d_grid() {
    // 4 points: (0.25, 0.25), (0.25, 0.75), (0.75, 0.25), (0.75, 0.75)
    let sample = Array2::from_shape_vec((4, 2), vec![
        0.25, 0.25, 0.25, 0.75, 0.75, 0.25, 0.75, 0.75,
    ]).unwrap();
    let result = compute_discrepancy(&sample).unwrap();
    assert!(result.centered > 0.0);
    assert!(result.wrap_around > 0.0);
    assert!(result.l2_star > 0.0);
}
```

- [ ] **Step 5: Run tests — verify green**

Run: `cargo test -p salib-estimators --test discrepancy_tck --release 2>&1`

- [ ] **Step 6: Commit**

```bash
git add crates/salib-estimators/src/discrepancy.rs \
       crates/salib-estimators/src/lib.rs \
       tck/salib/discrepancy/ \
       crates/salib-estimators/tests/discrepancy_tck.rs
git commit -m "feat(salib): discrepancy indices (CD, WD, MD, L2*)"
```

---

### Task 7: Grouped Factors — Core Types

Add `Group` type to `salib-core` and the `groups` field to `Problem`.

**Files:**
- Modify: `crates/salib-core/src/problem.rs`
- Modify: `crates/salib-core/src/lib.rs`

- [ ] **Step 1: Define Group type**

In `crates/salib-core/src/problem.rs`, add:

```rust
/// A named group of factors treated as a single unit in SA.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub factor_indices: Vec<usize>,
}
```

- [ ] **Step 2: Add `groups` field to `Problem`**

```rust
pub struct Problem {
    pub factors: Vec<Factor>,
    /// Factor groups for grouped SA. `None` = ungrouped (each factor independent).
    pub groups: Option<Vec<Group>>,
}
```

- [ ] **Step 3: Add `.group()` to `ProblemBuilder`**

```rust
impl ProblemBuilder {
    // ... existing methods ...

    /// Add a factor group. Indices must refer to factors already added.
    #[must_use]
    pub fn group(mut self, name: &str, factor_indices: &[usize]) -> Self {
        self.groups.push(Group {
            name: name.to_string(),
            factor_indices: factor_indices.to_vec(),
        });
        self
    }
}
```

Add a `groups: Vec<Group>` field to `ProblemBuilder` (initialized empty in `Default`).

In `ProblemBuilder::build()`, validate:
- Every index in every group is < `factors.len()`
- No factor appears in multiple groups
- Each group has at least one factor

Set `Problem.groups = if groups.is_empty() { None } else { Some(groups) }`.

- [ ] **Step 4: Re-export Group from lib.rs**

Add `Group` to the re-exports in `crates/salib-core/src/lib.rs`.

- [ ] **Step 5: Add unit tests**

In `problem.rs` tests:

```rust
#[test]
fn grouped_problem_builds() {
    let p = ProblemBuilder::new()
        .factor("x1", Distribution::Uniform { lo: 0.0, hi: 1.0 })
        .factor("x2", Distribution::Uniform { lo: 0.0, hi: 1.0 })
        .factor("x3", Distribution::Uniform { lo: 0.0, hi: 1.0 })
        .group("shape", &[0, 1])
        .group("scale", &[2])
        .build()
        .unwrap();
    assert_eq!(p.groups.as_ref().unwrap().len(), 2);
}

#[test]
fn group_index_out_of_range_fails() {
    let result = ProblemBuilder::new()
        .factor("x1", Distribution::Uniform { lo: 0.0, hi: 1.0 })
        .group("bad", &[5])
        .build();
    assert!(result.is_err());
}

#[test]
fn factor_in_multiple_groups_fails() {
    let result = ProblemBuilder::new()
        .factor("x1", Distribution::Uniform { lo: 0.0, hi: 1.0 })
        .factor("x2", Distribution::Uniform { lo: 0.0, hi: 1.0 })
        .group("a", &[0])
        .group("b", &[0, 1])
        .build();
    assert!(result.is_err());
}
```

- [ ] **Step 6: Run tests — verify green**

Run: `cargo test -p salib-core --release 2>&1`

- [ ] **Step 7: Commit**

```bash
git add crates/salib-core/src/problem.rs crates/salib-core/src/lib.rs
git commit -m "feat(salib-core): Group type + groups field on Problem"
```

---

### Task 8: Grouped Morris — Sampler + Estimator

Implement grouped trajectory generation (one-group-at-a-time instead of OAT) and grouped effect aggregation.

**Files:**
- Modify: `crates/salib-samplers/src/morris.rs`
- Modify: `crates/salib-estimators/src/morris.rs`
- Create: `tck/salib/grouped-factors/features/grouped_factors.feature`
- Create: `crates/salib-estimators/tests/grouped_factors_tck.rs`

- [ ] **Step 1: Write TCK feature file**

Create `tck/salib/grouped-factors/features/grouped_factors.feature`:

```gherkin
Feature: Grouped factor support — Morris + Sobol

  Scenario: Ungrouped equals singleton groups (Morris identity)
    Given a 3-factor linear model f(x) = x1 + x2 + x3
    When I run Morris with singleton groups [0], [1], [2]
    And I run Morris without groups
    Then the mu_star values are the same within 0.01

  Scenario: Grouped Morris on 2-group linear model
    Given a 4-factor linear model f(x) = x1 + x2 + 3*x3 + 3*x4
    And groups: A=[0,1], B=[2,3]
    When I run grouped Morris with R=100 trajectories
    Then group B mu_star is larger than group A mu_star

  Scenario: Grouped Morris trajectory has n_groups+1 points
    Given 4 factors grouped into 2 groups
    When I generate grouped Morris trajectories
    Then each trajectory has 3 points (n_groups + 1)
```

- [ ] **Step 2: Implement grouped trajectory generation**

In `crates/salib-samplers/src/morris.rs`, add a new function:

```rust
pub fn build_grouped_morris_trajectories(
    groups: &[Group],
    d: usize,
    r: usize,
    levels: u32,
    rng: &mut RngState,
) -> Result<MorrisTrajectories, MorrisError>
```

When groups are present:
- Number of steps per trajectory = `groups.len()` (not `d`)
- At each step, ALL factors in the selected group are perturbed by ±Δ simultaneously
- `factor_order` becomes group-order: `group_order: Array2<usize>` of shape `(R, n_groups)`
- `MorrisTrajectories` gains `pub group_order: Option<Array2<usize>>`

The trajectory shape changes: `trajectories` array is `(R, n_groups+1, d)`.

- [ ] **Step 3: Implement grouped Morris effect aggregation**

In `crates/salib-estimators/src/morris.rs`, add:

```rust
pub fn estimate_grouped_morris_effects<F>(
    trajectories: &MorrisTrajectories,
    groups: &[Group],
    model: F,
) -> Result<MorrisEffects, EmptyError>
where F: Fn(&[f64]) -> f64
```

This function computes elementary effects per group. For each trajectory step that perturbs group G, the elementary effect of group G is `(Y_after - Y_before) / Δ`. Then aggregate: `mu`, `mu_star`, `sigma` per group (not per factor).

`MorrisEffects` gets additional optional fields:

```rust
pub grouped_mu: Option<Vec<f64>>,
pub grouped_mu_star: Option<Vec<f64>>,
pub grouped_sigma: Option<Vec<f64>>,
pub group_names: Option<Vec<String>>,
```

- [ ] **Step 4: Write TCK test harness**

Create `crates/salib-estimators/tests/grouped_factors_tck.rs`.

- [ ] **Step 5: Run tests — verify green**

Run: `cargo test -p salib-estimators --test grouped_factors_tck --release 2>&1`

- [ ] **Step 6: Commit**

```bash
git add crates/salib-samplers/src/morris.rs \
       crates/salib-estimators/src/morris.rs \
       tck/salib/grouped-factors/ \
       crates/salib-estimators/tests/grouped_factors_tck.rs
git commit -m "feat(salib): grouped Morris trajectories + effects"
```

---

### Task 9: Grouped Saltelli Matrix + Grouped Sobol Estimator

Implement grouped column-swap in Saltelli matrix and grouped Sobol index estimation.

**Files:**
- Modify: `crates/salib-samplers/src/saltelli_matrix.rs`
- Modify: `crates/salib-estimators/src/saltelli2010.rs`

- [ ] **Step 1: Implement grouped Saltelli matrix**

In `crates/salib-samplers/src/saltelli_matrix.rs`, add:

```rust
pub fn build_grouped_saltelli_matrix(
    sampler: &dyn Sampler,
    groups: &[Group],
    n: usize,
    second_order: bool,
    rng: &mut RngState,
) -> Result<SaltelliMatrix, SaltelliError>
```

When groups present, `a_b` has `n_groups` matrices (not `dim`). `a_b[g]` replaces ALL columns in group `g` from `b` into `a`. Similarly for `b_a` if second_order. The output `SaltelliMatrix.dim` is set to `n_groups` (the effective dimension for the estimator).

Model evaluations: `N·(n_groups + 2)` instead of `N·(d + 2)`.

- [ ] **Step 2: Grouped Sobol indices**

In `crates/salib-estimators/src/saltelli2010.rs`, the existing `estimate_saltelli2010` already works on `SaltelliMatrix` where `dim` = `a_b.len()`. When `dim = n_groups` (from grouped matrix), the estimator naturally produces per-group indices. No changes needed to the estimator code — the grouping happens at the matrix construction level.

Add a convenience wrapper if desired:

```rust
pub fn estimate_grouped_saltelli2010<F>(
    matrix: &SaltelliMatrix,
    groups: &[Group],
    model: F,
) -> SobolIndices
where F: Fn(&[f64]) -> f64
```

This just calls `estimate_saltelli2010(matrix, model)` but the output `dim = n_groups` and indices are per-group.

- [ ] **Step 3: Add test for singleton-group identity**

Test that `build_grouped_saltelli_matrix` with singleton groups `[0], [1], [2]` produces the same indices as ungrouped `build_saltelli_matrix`.

- [ ] **Step 4: Run tests — verify green**

Run: `cargo test -p salib-estimators --release 2>&1`

- [ ] **Step 5: Commit**

```bash
git add crates/salib-samplers/src/saltelli_matrix.rs \
       crates/salib-estimators/src/saltelli2010.rs
git commit -m "feat(salib): grouped Saltelli matrix + grouped Sobol"
```

---

### Task 10: HDMR (High-Dimensional Model Representation)

RS-HDMR via PCE decomposition. Leverages salib-surrogate's polynomial expansion.

**Files:**
- Create: `crates/salib-estimators/src/hdmr.rs`
- Modify: `crates/salib-estimators/src/lib.rs`
- Modify: `crates/salib-estimators/Cargo.toml`
- Create: `tck/salib/hdmr/features/hdmr.feature`
- Create: `crates/salib-estimators/tests/hdmr_tck.rs`

- [ ] **Step 1: Write TCK feature file**

Create `tck/salib/hdmr/features/hdmr.feature`:

```gherkin
Feature: RS-HDMR — High-Dimensional Model Representation via PCE

  Scenario: HDMR on Ishigami recovers first-order indices
    Given the Ishigami canonical model with a=7 and b=0.1
    And N=4096 samples from Sobol sequence
    When I run HDMR with max_order=2 and max_degree=6
    Then first_order S_1 is within 0.05 of analytic 0.3139
    And first_order S_2 is within 0.05 of analytic 0.4424
    And first_order S_3 is within 0.05 of analytic 0.0

  Scenario: HDMR second-order matches Ishigami S2_13
    Given the Ishigami canonical model with a=7 and b=0.1
    And N=4096 samples from Sobol sequence
    When I run HDMR with max_order=2 and max_degree=6
    Then second_order S2_13 is within 0.05 of analytic 0.244

  Scenario: HDMR component variances sum to total variance
    Given any test function with N=1024 samples
    When I run HDMR with max_order=2
    Then the sum of all component variances equals total_variance within 0.01

  Scenario: HDMR agrees with PCE Sobol indices
    Given the Ishigami canonical model with a=7 and b=0.1
    And N=4096 samples
    When I run HDMR with max_order=2 and max_degree=6
    And I run PCE Sobol with the same degree
    Then HDMR first_order equals PCE first_order within 0.001
```

- [ ] **Step 2: Add salib-surrogate dependency**

In `crates/salib-estimators/Cargo.toml`, add:

```toml
salib-surrogate = { path = "../salib-surrogate" }
```

- [ ] **Step 3: Define HdmrResult type**

Create `crates/salib-estimators/src/hdmr.rs`:

```rust
use ndarray::Array2;
use salib_core::Problem;
use salib_surrogate::pce::{fit_full_pce, PolynomialChaos, PceError};
use salib_surrogate::multi_index::MultiIndex;
use salib_surrogate::polynomial::PolynomialFamily;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct HdmrResult {
    pub dim: usize,
    pub total_variance: f64,
    /// Variance contribution per interaction order.
    /// `order_variance[0]` = sum of all first-order component variances,
    /// `order_variance[1]` = sum of all second-order, etc.
    pub order_variance: Vec<f64>,
    /// First-order Sobol indices from HDMR decomposition.
    pub first_order: Vec<f64>,
    /// Second-order indices S2_{ij} for i < j. Same indexing as SobolIndices.
    pub second_order: Vec<Vec<f64>>,
    /// Total-order indices.
    pub total_order: Vec<f64>,
    /// The fitted PCE used internally (for inspection/reuse).
    pub pce: PolynomialChaos,
}

#[derive(Debug, Clone, Error)]
pub enum HdmrError {
    #[error("insufficient samples: {n} < basis size {basis_size}")]
    InsufficientSamples { n: usize, basis_size: usize },
    #[error("total variance is zero or negative")]
    ZeroVariance,
    #[error("PCE fit failed: {0}")]
    PceFitFailed(#[from] PceError),
}
```

- [ ] **Step 4: Implement estimate_hdmr**

```rust
/// RS-HDMR via PCE decomposition.
///
/// Fits a polynomial chaos expansion to (X, Y) data, then decomposes
/// the PCE coefficients by interaction order to get HDMR components.
///
/// `x` is N × d sample matrix in [0, 1]^d (unit cube).
/// `y` is model output vector of length N.
/// `problem` defines the input distributions (for choosing polynomial families).
/// `max_order` limits the interaction order (2 = up to pairwise).
/// `max_degree` is the PCE polynomial degree.
pub fn estimate_hdmr(
    x: &Array2<f64>,
    y: &[f64],
    problem: &Problem,
    max_order: usize,
    max_degree: usize,
) -> Result<HdmrResult, HdmrError> {
    let d = problem.dim();
    let n = x.nrows();

    // Choose polynomial families based on distributions
    // (Legendre for Uniform, Hermite for Normal, etc.)
    let families: Vec<PolynomialFamily> = problem.factors().iter().map(|f| {
        match f.distribution {
            salib_core::Distribution::Normal { .. } => PolynomialFamily::Hermite,
            _ => PolynomialFamily::Legendre,
        }
    }).collect();

    // Map physical inputs to canonical domain for PCE
    // For Uniform(lo, hi): canonical = 2*(x - lo)/(hi - lo) - 1 ∈ [-1, 1]
    let mut x_canonical = Array2::<f64>::zeros((n, d));
    for i in 0..n {
        for j in 0..d {
            let (lo, hi) = problem.factors()[j].distribution.support();
            x_canonical[[i, j]] = 2.0 * (x[[i, j]] - lo) / (hi - lo) - 1.0;
        }
    }

    // Fit PCE
    let pce = fit_full_pce(&x_canonical, y, &families, max_degree)?;

    // Decompose by interaction order using multi-index active factors
    // (same approach as sobol_indices_from_pce but organized by order)
    let mut contributions: Vec<f64> = Vec::with_capacity(pce.basis_size());
    for (alpha, &beta) in pce.multi_indices.iter().zip(pce.coefficients.iter()) {
        let mut norm_sq = 1.0;
        for (k, &deg) in alpha.indices.iter().enumerate() {
            norm_sq *= salib_surrogate::polynomial::norm_squared(pce.families[k], deg);
        }
        contributions.push(beta * beta * norm_sq);
    }

    let total_variance: f64 = pce.multi_indices.iter()
        .zip(contributions.iter())
        .filter(|(alpha, _)| !alpha.is_zero())
        .map(|(_, &c)| c)
        .sum();

    if total_variance < 1e-15 {
        return Err(HdmrError::ZeroVariance);
    }

    // Accumulate by order and by factor
    let mut first_order = vec![0.0_f64; d];
    let mut total_order = vec![0.0_f64; d];
    let mut order_variance = vec![0.0_f64; max_order];
    let mut s2 = vec![vec![0.0_f64; 0]; d];
    for i in 0..d {
        s2[i] = vec![0.0_f64; d - i - 1];
    }

    for (alpha, &c) in pce.multi_indices.iter().zip(contributions.iter()) {
        if alpha.is_zero() { continue; }
        let active = alpha.active_factors();
        let order = active.len();

        if order <= max_order {
            order_variance[order - 1] += c;
        }

        if order == 1 {
            first_order[active[0]] += c;
        }

        if order == 2 {
            let (i, j) = (active[0], active[1]);
            s2[i][j - i - 1] += c;
        }

        for &i in &active {
            total_order[i] += c;
        }
    }

    // Normalize
    for i in 0..d {
        first_order[i] = (first_order[i] / total_variance).clamp(0.0, 1.0);
        total_order[i] = (total_order[i] / total_variance).clamp(0.0, 1.0);
    }
    for i in 0..d {
        for j in 0..s2[i].len() {
            s2[i][j] = (s2[i][j] / total_variance).clamp(0.0, 1.0);
        }
    }
    for v in &mut order_variance {
        *v /= total_variance;
    }

    Ok(HdmrResult {
        dim: d,
        total_variance,
        order_variance,
        first_order,
        second_order: s2,
        total_order,
        pce,
    })
}
```

- [ ] **Step 5: Re-export from lib.rs**

Add to `crates/salib-estimators/src/lib.rs`:

```rust
mod hdmr;
pub use hdmr::{estimate_hdmr, HdmrResult, HdmrError};
```

- [ ] **Step 6: Write TCK test harness**

Create `crates/salib-estimators/tests/hdmr_tck.rs`.

- [ ] **Step 7: Run tests — verify green**

Run: `cargo test -p salib-estimators --test hdmr_tck --release 2>&1`

- [ ] **Step 8: Run full workspace test suite — no regressions**

Run: `cargo test --workspace --release 2>&1`

- [ ] **Step 9: Commit**

```bash
git add crates/salib-estimators/src/hdmr.rs \
       crates/salib-estimators/src/lib.rs \
       crates/salib-estimators/Cargo.toml \
       tck/salib/hdmr/ \
       crates/salib-estimators/tests/hdmr_tck.rs
git commit -m "feat(salib): RS-HDMR via PCE decomposition"
```

---

### Task 11: Re-export Audit + lib.rs Cleanup

Ensure all new public types and functions are properly re-exported from crate root modules.

**Files:**
- Modify: `crates/salib-estimators/src/lib.rs`
- Modify: `crates/salib-samplers/src/lib.rs`
- Modify: `crates/salib-core/src/lib.rs`

- [ ] **Step 1: Audit salib-estimators re-exports**

Verify these are all re-exported:
- `FractionalFactorialEffects`, `estimate_fractional_factorial`
- `DiscrepancyResult`, `DiscrepancyError`, `compute_discrepancy`
- `HdmrResult`, `HdmrError`, `estimate_hdmr`
- `estimate_grouped_morris_effects` (if public)

- [ ] **Step 2: Audit salib-samplers re-exports**

Verify:
- `PlackettBurmanDesign`, `PbError`, `build_plackett_burman`
- `build_grouped_morris_trajectories`
- `build_grouped_saltelli_matrix`

- [ ] **Step 3: Audit salib-core re-exports**

Verify:
- `Group`

- [ ] **Step 4: Run full workspace tests**

Run: `cargo test --workspace --release 2>&1`
Run: `cargo clippy --workspace --all-targets -- -D warnings 2>&1`

- [ ] **Step 5: Commit**

```bash
git add crates/salib-estimators/src/lib.rs \
       crates/salib-samplers/src/lib.rs \
       crates/salib-core/src/lib.rs
git commit -m "chore(salib): re-export audit for BEAD-0016 additions"
```

---

### Task 12: Update BEAD-0016 Status

Close the bead and update documentation.

**Files:**
- Modify: `.context/beads/BEAD-0016-salib-rs-parity-gaps.md`

- [ ] **Step 1: Update bead status**

Set `status: closed`, `closed: 2026-05-14`. Add completion notes listing all 5 methods implemented with test counts.

- [ ] **Step 2: Commit**

```bash
git add .context/beads/BEAD-0016-salib-rs-parity-gaps.md
git commit -m "chore: close BEAD-0016 — all 5 SALib parity gaps closed"
```
