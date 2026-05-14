# Design Spec: SALib Parity Gaps (BEAD-0016)

**BEAD:** BEAD-0016
**Date:** 2026-05-14
**Status:** Approved
**Depends on:** BEAD-0002 (salib-rs repack)

## 1. Purpose

Close 5 method gaps between salib-rs and Python SALib (Herman et al.) so
salib-rs becomes a strict superset. After this work, there is no reason
to reach for the Python version.

## 2. Methods

### 2.1 Second-Order Sobol Indices (S2)

**Source:** Saltelli 2010 Eq (d).

**Formula:**
```
S2_{ij} = (1/N) Σ_k [f_{BA^j}(k) · f_{AB^i}(k) − f_A(k) · f_B(k)] / D − S_i − S_j
```

Sampling infrastructure exists: `SaltelliMatrix.b_a` is `Option<Vec<Array2<f64>>>`
populated when `second_order=true`. Gap is estimator-side only.

**New field on `SobolIndices`:**
```rust
pub second_order: Option<Vec<Vec<f64>>>,  // S2[i][j] for i < j, None otherwise
```

`Option` because most calls skip it (model evals double). Only populated when
`SaltelliMatrix.b_a` is `Some(...)`.

Wire into: `estimate_saltelli2010`, `estimate_jansen`, `estimate_janon`,
`estimate_owen`. Add `second_order` to `SobolIndicesAnalytic` in salib-validation
with Ishigami analytic S2 values (Saltelli 2008 Eq 5.16-5.18: S2_{12}=0,
S2_{13}≈0.244, S2_{23}=0).

**Files:**
- Modify: `salib-estimators/src/sobol_indices.rs` — add field + constructor update
- Modify: `salib-estimators/src/saltelli2010.rs` — compute S2 when b_a present
- Modify: `salib-estimators/src/jansen.rs`, `janon.rs`, `owen.rs` — same
- Modify: `salib-validation/src/analytic.rs` — add field
- Modify: `salib-validation/src/ishigami.rs` — Ishigami S2 analytic

### 2.2 Fractional Factorial Screening

**Source:** Plackett & Burman 1946; Saltelli et al. 2008 (Primer).

Plackett-Burman design for cheap main-effect screening (Resolution III).

**Sampler** — `salib-samplers/src/plackett_burman.rs`:
```rust
pub struct PlackettBurmanDesign {
    pub matrix: Array2<f64>,  // N × d, values in {-1, +1}
    pub n_runs: usize,
    pub dim: usize,
}

pub fn build_plackett_burman(dim: usize) -> Result<PlackettBurmanDesign, PbError>
```

Construction: Hadamard matrix method. N = next multiple of 4 ≥ d+1. For
standard sizes (N = 4, 8, 12, 16, 20, 24) use known generating vectors
(cyclic shift construction). Factors beyond generator length truncated
from extra Hadamard columns.

**Analyzer** — `salib-estimators/src/fractional_factorial.rs`:
```rust
pub struct FractionalFactorialEffects {
    pub dim: usize,
    pub n_runs: usize,
    pub main_effects: Vec<f64>,
    pub main_effects_abs: Vec<f64>,
}

pub fn estimate_fractional_factorial<F>(
    design: &PlackettBurmanDesign,
    problem: &Problem,
    model: F,
) -> FractionalFactorialEffects
where F: Fn(&[f64]) -> f64
```

Main effect for factor i = mean(Y | x_i = +1) − mean(Y | x_i = −1).

**Files:**
- Create: `salib-samplers/src/plackett_burman.rs`
- Modify: `salib-samplers/src/lib.rs`
- Create: `salib-estimators/src/fractional_factorial.rs`
- Modify: `salib-estimators/src/lib.rs`

### 2.3 Grouped-Factor Support

**Source:** Saltelli et al. 2008 (Primer); Morris 1991 extension.

SALib allows grouping parameters and treating groups as atomic units in
both Morris trajectory generation and Sobol sampling. Useful when factors
are conceptually linked (e.g. shape parameters of a distribution).

**Core type** — `salib-core/src/problem.rs`:
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub factor_indices: Vec<usize>,
}

// Added to Problem:
pub struct Problem {
    pub factors: Vec<Factor>,
    pub groups: Option<Vec<Group>>,  // NEW
}
```

`ProblemBuilder` gets `.group(name, &[factor_indices])` method. Validation:
indices in range, no factor in multiple groups, at least one factor per group.

**Morris grouped trajectories** — `salib-samplers/src/morris.rs`:

When groups are present, the trajectory steps all factors in a group
simultaneously. OAT becomes one-group-at-a-time. `MorrisTrajectories` gains:
```rust
pub group_order: Option<Array2<usize>>,  // (R, n_groups) — group stepped at each position
```

Trajectory shape changes: d+1 points → n_groups+1 points per trajectory.

**Morris grouped estimator** — `salib-estimators/src/morris.rs`:

Elementary effects aggregated per group (mean of member-factor EEs).
`MorrisEffects` gains:
```rust
pub grouped_mu: Option<Vec<f64>>,
pub grouped_mu_star: Option<Vec<f64>>,
pub grouped_sigma: Option<Vec<f64>>,
pub group_names: Option<Vec<String>>,
```

**Sobol grouped** — `salib-samplers/src/saltelli_matrix.rs`:

When groups present, `SaltelliMatrix` replaces columns by group (all
factor indices in group swapped together). `a_b` length = n_groups
(not dim). Estimator output: `SobolIndices` with `dim = n_groups`.

**Files:**
- Modify: `salib-core/src/problem.rs` — `Group`, add to `Problem` + builder
- Modify: `salib-samplers/src/morris.rs` — grouped trajectories
- Modify: `salib-samplers/src/saltelli_matrix.rs` — grouped column swap
- Modify: `salib-estimators/src/morris.rs` — grouped EE aggregation
- Modify: `salib-estimators/src/saltelli2010.rs` (and jansen/janon/owen) — dim = n_groups when grouped

### 2.4 Discrepancy Indices

**Source:** Fang et al. 2006; Hickernell 1998.

Space-filling quality metrics for design matrices. Standalone analyzer
(no sampler needed). Four metrics:

1. **Centered Discrepancy (CD):** Hickernell 1998, measures departure
   from uniform via centered kernel.
2. **Wrap-around Discrepancy (WD):** Hickernell 1998, toroidal
   distance kernel — invariant to shifts mod 1.
3. **Modified Discrepancy (MD):** Fang et al. 2006, modified L2-star.
4. **L2-star Discrepancy:** Classical Niederreiter measure.

All computed from an N × d sample matrix in [0,1]^d. Complexity O(N²·d).

```rust
pub struct DiscrepancyResult {
    pub centered: f64,
    pub wrap_around: f64,
    pub modified: f64,
    pub l2_star: f64,
}

pub fn compute_discrepancy(sample: &Array2<f64>) -> Result<DiscrepancyResult, DiscrepancyError>
```

**Files:**
- Create: `salib-estimators/src/discrepancy.rs`
- Modify: `salib-estimators/src/lib.rs`

### 2.5 HDMR (High-Dimensional Model Representation)

**Source:** Li, Rosenthal, Rabitz 2001; Li et al. 2010.

RS-HDMR decomposes f(x) = f_0 + Σ f_i(x_i) + Σ f_{ij}(x_i,x_j) + ...
into component functions of increasing order and estimates variance
contributions. Uses orthogonal polynomial expansion (leveraging
salib-surrogate PCE infrastructure).

**Approach:** Fit a PCE to the random sample, then decompose the PCE
coefficients by interaction order. First-order component functions
correspond to single-factor multi-indices; second-order to pairs; etc.
This is exactly what `sobol_indices_from_pce` already does for first
and total order — extend to report per-order variance contributions.

```rust
pub struct HdmrResult {
    pub dim: usize,
    pub total_variance: f64,
    pub component_variance: Vec<Vec<f64>>,  // [order][component_index]
    pub component_factors: Vec<Vec<Vec<usize>>>,  // [order][component_index] → factor indices
    pub first_order: Vec<f64>,   // S_i (same as PCE Sobol)
    pub second_order: Vec<Vec<f64>>,  // S2_{ij}
    pub total_order: Vec<f64>,   // S_Ti
    pub pce: PolynomialChaos,    // fitted surrogate for inspection
}

pub fn estimate_hdmr(
    x: &Array2<f64>,
    y: &[f64],
    problem: &Problem,
    max_order: usize,
    max_degree: usize,
) -> Result<HdmrResult, HdmrError>
```

Internally: maps physical inputs to canonical domain via `Distribution::quantile`,
fits PCE (full or sparse depending on basis size vs sample size), then
decomposes multi-indices by `active_factors()` cardinality.

**Files:**
- Create: `salib-estimators/src/hdmr.rs`
- Modify: `salib-estimators/src/lib.rs`
- Modify: `salib-estimators/Cargo.toml` — add `salib-surrogate` dependency

## 3. Error Types

Each new method gets its own error enum following the existing pattern:

```rust
// salib-samplers
pub enum PbError {
    ZeroDim,
    DimTooLarge(usize),  // > 23 for standard PB
}

pub enum MorrisGroupError {
    EmptyGroup(String),
    FactorOutOfRange { group: String, index: usize, dim: usize },
    FactorInMultipleGroups { index: usize },
}

// salib-estimators  
pub enum DiscrepancyError {
    EmptyMatrix,
    NotUnitInterval,  // values outside [0,1]
}

pub enum HdmrError {
    InsufficientSamples { n: usize, basis_size: usize },
    ZeroVariance,
    PceFitFailed(PceError),
}
```

## 4. Validation (4-Gate per method)

### S2
- Gate 1: Ishigami analytic S2_{13} ≈ 0.244 (Saltelli 2008)
- Gate 2: SALib cross-check (Python SALib `sobol.analyze` with `calc_second_order=True`)
- Gate 3: S2 symmetry (S2_{ij} = S2_{ji}), S2 ≥ 0, Σ S_i + Σ S2_{ij} + ... ≤ 1
- Gate 4: MC convergence of S2 estimates with N

### Fractional Factorial
- Gate 1: Known 2^3 full-factorial hand-computed effects
- Gate 2: SALib `ff.analyze` cross-check on Ishigami
- Gate 3: PB matrix orthogonality (X'X = NI), main effects sum to zero for balanced Y
- Gate 4: Screening recovery rate (identify top-k factors in Morris test function)

### Grouped Factors
- Gate 1: 2-group Morris on known function
- Gate 2: SALib grouped Morris cross-check
- Gate 3: Ungrouped = grouped with singleton groups (identity property)
- Gate 4: Group Sobol S_G sums ≤ 1

### Discrepancy
- Gate 1: Known discrepancy of regular grid (analytic for 2D)
- Gate 2: SALib `discrepancy` cross-check
- Gate 3: Random sample > grid discrepancy, Sobol < random (ordering property)
- Gate 4: Discrepancy decreases with N (convergence)

### HDMR
- Gate 1: Ishigami first/second-order HDMR components match analytic
- Gate 2: Cross-check with PCE Sobol indices (should agree exactly)
- Gate 3: Component variances sum to total variance, order-0 = E[Y]²
- Gate 4: HDMR on Sobol G-function convergence with sample size

## 5. TCK Feature Files

```
tck/salib/features/
  second_order_sobol.feature
  fractional_factorial.feature
  grouped_factors.feature
  discrepancy.feature
  hdmr.feature
```

## 6. Priority Order

1. S2 (smallest delta, biggest user-visible gap)
2. Fractional Factorial (screening entry point, standalone)
3. Discrepancy (standalone, no dependencies)
4. Grouped factors (API change across core/samplers/estimators)
5. HDMR (largest scope, depends on surrogate)

## 7. Literature Required

| Paper | Method | Status |
|-------|--------|--------|
| Saltelli et al. 2008 (Primer) | S2, FF, Groups | Need to acquire |
| Saltelli 2010 | S2 formula | Have (via existing impl) |
| Plackett & Burman 1946 | PB construction | Need to acquire |
| Fang et al. 2006 | Discrepancy | Need to acquire |
| Hickernell 1998 | WD, CD, L2-star | Need to acquire |
| Li et al. 2010 | RS-HDMR | Need to acquire |
| Li, Rosenthal, Rabitz 2001 | HDMR theory | Need to acquire |
| Morris 1991 | Grouped extension | Have (via existing impl) |
