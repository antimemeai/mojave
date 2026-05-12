# Design Spec: Gwet AC1/AC2/AC3 and Bland-Altman

**Date:** 2026-05-12
**Origin:** BEAD-0003 confirmed gaps
**Drivers:** Kappa paradox (Feinstein & Cicchetti 1990) — Cohen kappa collapses under prevalence imbalance; Gwet AC1 remains stable. Bland-Altman provides limits-of-agreement analysis for comparing two measurement methods.

---

## 1. Module Structure

Three new source files in `crates/irr/src/`:

| File | Purpose |
|------|---------|
| `categorical_agreement_weights.rs` | Weight matrices for categorical agreement coefficients (identity, linear, quadratic, ordinal, custom). Consumed by `gwet.rs`; may later serve other weighted agreement functions. |
| `gwet.rs` | Gwet AC1/AC2/AC3 computation. |
| `bland_altman.rs` | Bland-Altman limits of agreement for two paired measurement vectors. |

No `FloatRatingMatrix`. Bland-Altman takes `&[f64], &[f64]` — feeding it a matrix would be tryhard and violate YAGNI.

No generalized "slop matrix." The weight matrix in `categorical_agreement_weights.rs` is purpose-built for categorical agreement coefficients with u32 category labels.

---

## 2. Gwet AC API

### Literature basis

- Gwet, K. L. (2008). "Computing inter-rater reliability and its variance in the presence of high agreement." *British Journal of Mathematical and Statistical Methods*, 61, 29-48.
- Gwet, K. L. (2014). *Handbook of Inter-Rater Reliability*, 4th ed. Advanced Analytics, LLC.
- Feinstein, A. R. & Cicchetti, D. V. (1990). "High agreement but low kappa: I. The problems of two paradoxes." *Journal of Clinical Epidemiology*, 43(6), 543-549.

### Formula

For q categories with weights w_{k,l} and marginal proportions pi_k:

```
pa = observed weighted agreement (from coincidence matrix)
pe = sum(all weight entries) * sum_k[ pi_k * (1 - pi_k) ] / (q * (q - 1))    [Gwet 2008 eq. 3]
AC = (pa - pe) / (1 - pe)
```

When pe = 1.0, AC is undefined (degenerate case — all mass on one category with maximal chance agreement).

### Weight variants

- **AC1**: Identity weights (w_{k,l} = 1 if k==l, 0 otherwise). No `WeightMatrix` argument needed.
- **AC2**: Standard weight schemes (linear, quadratic, ordinal). Caller selects scheme via enum.
- **AC3**: Arbitrary user-provided weight matrix. Must be square, symmetric, non-negative, with 1s on diagonal. **Do not use without a motivated reason** — document this prominently.

### Public API

```rust
// categorical_agreement_weights.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl WeightMatrix {
    pub fn from_scheme(categories: &[u32], scheme: WeightScheme) -> Self { ... }
    pub fn custom(categories: &[u32], weights: Vec<Vec<f64>>) -> Result<Self, WeightError> { ... }
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
```

```rust
// gwet.rs

use crate::categorical_agreement_weights::WeightMatrix;
use crate::types::{IrrResult, RatingMatrix};

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
pub fn ac(
    matrix: &RatingMatrix,
    weights: Option<&WeightMatrix>,
) -> Result<IrrResult, GwetError> { ... }
```

The function auto-discovers categories from the data. If `weights` is `None`, identity weights are used internally. If `weights` is provided, categories must match between the weight matrix and the data (validated at runtime).

Returns `IrrResult` with `statistic_name` set to `"Gwet AC1"`, `"Gwet AC2(linear)"`, `"Gwet AC2(quadratic)"`, `"Gwet AC2(ordinal)"`, or `"Gwet AC3"` as appropriate.

n >= 2 raters. The formula works identically for n = 2 and n > 2 — no special-casing needed. The coincidence matrix formulation handles arbitrary rater counts.

---

## 3. Bland-Altman API

### Literature basis

- Bland, J. M. & Altman, D. G. (1986). "Statistical methods for assessing agreement between two methods of clinical measurement." *The Lancet*, 327(8476), 307-310.

### Formula

Given paired measurements x_i and y_i (i = 1..n):

```
diff_i = x_i - y_i
mean_diff = mean(diff)
sd_diff = sd(diff)
lower_loa = mean_diff - 1.96 * sd_diff
upper_loa = mean_diff + 1.96 * sd_diff
```

The 1.96 multiplier is hardcoded (95% limits of agreement). This is the standard presentation from Bland & Altman 1986. No configurable z-multiplier — YAGNI.

### Public API

```rust
// bland_altman.rs

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
/// Takes two slices of equal length representing paired measurements
/// from two methods/raters on the same items.
pub fn agreement(x: &[f64], y: &[f64]) -> Result<BlandAltmanResult, BlandAltmanError> { ... }
```

Takes `&[f64], &[f64]` — not a matrix. n = 2 is the minimum (need at least 2 points for SD). Cohen kappa has the special n = 2 rater form; Bland-Altman inherently takes exactly 2 measurement vectors.

---

## 4. Validation Design (4-Gate)

### Gwet AC1/AC2/AC3

| Gate | What | Source |
|------|------|--------|
| **Gate 1: Textbook reproductions** | Gwet 2014 Table 4.1: 3 abstractors, 3 categories. AC1 = 0.84933. Gwet 2014 Table 5.7: same data, AC2(quadratic) = 0.94024. Krippendorff 2011 multi-rater dataset: AC1 = 0.77544. | irrCAC R package (Gwet's canonical implementation) |
| **Gate 2: Reference cross-check** | Run irrCAC::gwet.ac1.raw() and gwet.ac2.raw() on 3+ datasets, compare to our output within 1e-6. Pin irrCAC version. | irrCAC (R, CRAN) |
| **Gate 3: Property-based tests** | Perfect agreement -> AC = 1.0. Random noise -> AC near 0. AC1 >= kappa on high-prevalence data (the kappa paradox — Feinstein & Cicchetti 1990). Identity weights recover AC1 from AC2. Symmetry: relabeling categories doesn't change AC1. | Derived from definitions |
| **Gate 4: Monte Carlo calibration** | Kappa-paradox sweep: vary prevalence from balanced to 95/5, plot AC1 vs Cohen kappa. AC1 must remain stable (SD < 0.1 across sweep) while kappa collapses. Coverage check on bootstrap CIs (if bootstrap is wired up). | Feinstein & Cicchetti 1990 |

### Bland-Altman

| Gate | What | Source |
|------|------|--------|
| **Gate 1: Textbook reproductions** | Bland & Altman 1986 Table 1 (PEFR data): Wright peak-flow meter vs mini Wright meter. Mean diff = -2.1, SD = 38.8, LoA = [-78.1, 73.9] (approximate — will pin exact values from paper). | Bland & Altman 1986 original paper |
| **Gate 2: Reference cross-check** | R's `BlandAltmanLeh::bland.altman.stats()` or manual R computation on 3+ datasets, compare within 1e-6. | BlandAltmanLeh (R, CRAN) |
| **Gate 3: Property-based tests** | Identical inputs -> mean_diff = 0, sd_diff = 0 (ZeroVariance error or LoA = [0, 0] — design decision: error). Constant offset -> mean_diff = offset, sd_diff = 0. Negating order flips sign of mean_diff and swaps LoA bounds. x - y symmetry. | Derived from definitions |
| **Gate 4: Monte Carlo calibration** | Generate paired data with known mean offset and SD. Verify recovered mean_diff and sd_diff converge to true values as n grows. Coverage: fraction of true differences falling within LoA should be ~95% for normal data. | Bland & Altman 1986 Section 4 |

---

## 5. TCK Scenario Outlines

### `tck/irr/gwet.feature`

```gherkin
Feature: Gwet AC1/AC2/AC3

  # Gate 1: Textbook — Gwet 2014 Table 4.1
  Scenario: AC1 on Gwet 3-abstractor data
    Given the Gwet 2014 Table 4.1 rating matrix
    When I compute Gwet AC1
    Then the result is 0.84933 within 0.0001

  # Gate 1: Textbook — Gwet 2014 Table 5.7
  Scenario: AC2 quadratic on Gwet 3-abstractor data
    Given the Gwet 2014 Table 4.1 rating matrix
    When I compute Gwet AC2 with quadratic weights
    Then the result is 0.94024 within 0.0001

  # Gate 1: Textbook — Krippendorff 2011 multi-rater
  Scenario: AC1 on Krippendorff multi-rater data
    Given the Krippendorff 2011 reliability data
    When I compute Gwet AC1
    Then the result is 0.77544 within 0.001

  # Gate 3: Property — perfect agreement
  Scenario: Perfect agreement yields AC1 = 1.0
    Given a rating matrix where all raters agree
    When I compute Gwet AC1
    Then the result is 1.0 within 0.0001

  # Gate 3: Property — kappa paradox
  Scenario: AC1 >= Cohen kappa on high-prevalence data
    Given a high-prevalence rating matrix (90% category 0)
    When I compute Gwet AC1
    And I compute Cohen kappa
    Then AC1 is greater than or equal to kappa

  # Gate 3: Property — identity weights recover AC1
  Scenario: AC2 with identity weights equals AC1
    Given a rating matrix with mixed agreement
    When I compute Gwet AC1
    And I compute Gwet AC2 with identity weights
    Then AC1 and AC2-identity match within 0.0001

  # Gate 3: Property — category relabeling invariance
  Scenario: Relabeling categories does not change AC1
    Given a rating matrix with categories 0,1,2
    And the same matrix with categories relabeled to 5,10,15
    When I compute Gwet AC1 on both
    Then the results match within 0.0001

  # Edge: empty data
  Scenario: Empty matrix is an error
    Given an empty rating matrix
    When I attempt Gwet AC1
    Then I get a Gwet error about empty data

  # Edge: single rater
  Scenario: Single rater is an error
    Given a rating matrix with 1 rater
    When I attempt Gwet AC1
    Then I get a Gwet error about too few raters

  # Edge: degenerate prevalence
  Scenario: All-same-category data
    Given a rating matrix where every cell is category 0
    When I attempt Gwet AC1
    Then I get a Gwet error about degenerate pe

  # AC3: custom weights
  Scenario: AC3 with custom weights
    Given a rating matrix with mixed agreement
    And a custom weight matrix
    When I compute Gwet AC3 with the custom weights
    Then the result is a finite number between -1 and 1
```

### `tck/irr/bland_altman.feature`

```gherkin
Feature: Bland-Altman limits of agreement

  # Gate 1: Textbook — Bland & Altman 1986 PEFR data
  Scenario: PEFR Wright vs mini Wright meter
    Given the Bland-Altman 1986 PEFR data
    When I compute Bland-Altman agreement
    Then mean difference is -2.1 within 0.5
    And SD of differences is 38.8 within 0.5
    And lower LoA is approximately -78.1 within 1.0
    And upper LoA is approximately 73.9 within 1.0

  # Gate 3: Property — constant offset is zero-variance error
  Scenario: Constant offset yields zero-variance error
    Given measurements x = [1.0, 2.0, 3.0, 4.0, 5.0]
    And measurements y = [2.0, 3.0, 4.0, 5.0, 6.0]
    When I attempt Bland-Altman agreement
    Then I get an error about zero variance

  # Gate 3: Property — sign reversal
  Scenario: Swapping x and y negates mean diff
    Given measurements x and y with known difference
    When I compute Bland-Altman agreement for (x, y)
    And I compute Bland-Altman agreement for (y, x)
    Then mean differences are negations of each other within 0.0001
    And LoA bounds are swapped and negated

  # Edge: length mismatch
  Scenario: Mismatched input lengths
    Given measurements x with 5 values
    And measurements y with 3 values
    When I attempt Bland-Altman agreement
    Then I get an error about length mismatch

  # Edge: too few observations
  Scenario: Single observation pair
    Given measurements x = [1.0]
    And measurements y = [2.0]
    When I attempt Bland-Altman agreement
    Then I get an error about too few observations

  # Edge: identical inputs
  Scenario: Identical measurements yield zero-variance error
    Given measurements x = [1.0, 2.0, 3.0]
    And measurements y = [1.0, 2.0, 3.0]
    When I compute Bland-Altman agreement
    Then I get an error about zero variance
```

---

## Design Decisions

1. **No FloatRatingMatrix.** Bland-Altman takes `&[f64], &[f64]`. The RatingMatrix stays `Option<u32>`.
2. **Weight matrix is categorical-agreement-specific.** Named `categorical_agreement_weights.rs` to reflect this scope. Not a general-purpose matrix.
3. **AC3 documented with warning.** Custom weights exist for motivated research use cases, not casual experimentation.
4. **1.96 hardcoded in Bland-Altman.** Standard 95% LoA per Bland & Altman 1986. No configurable z-multiplier.
5. **Zero-variance is an error in Bland-Altman.** Identical inputs or constant offsets produce SD = 0, making LoA degenerate. Return error rather than degenerate results.
6. **Kappa paradox is a first-class validation target.** Both Gate 3 (single scenario) and Gate 4 (MC sweep) test that AC1 >= kappa under prevalence imbalance.
