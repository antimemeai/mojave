use crate::bootstrap::{bootstrap_ci, BootstrapError};
use crate::types::{IrrResult, MetricLevel, RatingMatrix};

#[derive(Debug, thiserror::Error)]
pub enum CohenError {
    #[error("empty data")]
    EmptyData,
    #[error("ratings must have equal length ({0} vs {1})")]
    UnequalLength(usize, usize),
    #[error("degenerate data: all ratings in a single category")]
    DegenerateData,
    #[error("Cohen kappa requires exactly 2 raters, got {0}")]
    NotTwoRaters(usize),
    #[error("invalid weight function: weight(c, c) must equal 0 for all observed categories")]
    InvalidWeightFunction,
}

struct Validated {
    n: f64,
    categories: Vec<u32>,
}

fn validate(rater1: &[u32], rater2: &[u32]) -> Result<Validated, CohenError> {
    if rater1.is_empty() {
        return Err(CohenError::EmptyData);
    }
    if rater1.len() != rater2.len() {
        return Err(CohenError::UnequalLength(rater1.len(), rater2.len()));
    }
    let mut categories: Vec<u32> = rater1.iter().chain(rater2.iter()).copied().collect();
    categories.sort();
    categories.dedup();
    Ok(Validated {
        n: rater1.len() as f64,
        categories,
    })
}

/// Compute Cohen's kappa for two-rater nominal agreement.
///
/// Reference: Cohen (1960) "A coefficient of agreement for nominal scales."
pub fn kappa(rater1: &[u32], rater2: &[u32]) -> Result<IrrResult, CohenError> {
    let v = validate(rater1, rater2)?;

    let p_o = rater1
        .iter()
        .zip(rater2.iter())
        .filter(|(a, b)| a == b)
        .count() as f64
        / v.n;

    let p_e: f64 = v
        .categories
        .iter()
        .map(|&c| {
            let p1 = rater1.iter().filter(|&&r| r == c).count() as f64 / v.n;
            let p2 = rater2.iter().filter(|&&r| r == c).count() as f64 / v.n;
            p1 * p2
        })
        .sum();

    // Perfect observed agreement: semantically kappa = 1.0 regardless of p_e.
    // This handles the 0/0 case when p_e is also 1.0.
    if (1.0 - p_o).abs() < 1e-15 {
        return Ok(IrrResult {
            statistic_name: "cohen_kappa".to_string(),
            value: 1.0,
            ci_lower: None,
            ci_upper: None,
            n_items: rater1.len(),
            n_raters: 2,
            metric_level: Some(MetricLevel::Nominal),
        });
    }

    if (1.0 - p_e).abs() < 1e-15 {
        return Err(CohenError::DegenerateData);
    }

    Ok(IrrResult {
        statistic_name: "cohen_kappa".to_string(),
        value: (p_o - p_e) / (1.0 - p_e),
        ci_lower: None,
        ci_upper: None,
        n_items: rater1.len(),
        n_raters: 2,
        metric_level: Some(MetricLevel::Nominal),
    })
}

/// Compute Cohen's weighted kappa for two-rater ordinal agreement.
///
/// The weight function must be a *disagreement* metric: `weight(a, a) == 0`
/// for all `a`, with larger values indicating greater disagreement.
///
/// Reference: Cohen (1968) "Weighted kappa: nominal scale agreement
/// with provision for scaled disagreement or partial credit."
pub fn weighted_kappa(
    rater1: &[u32],
    rater2: &[u32],
    weight_fn: impl Fn(u32, u32) -> f64,
    level: MetricLevel,
) -> Result<IrrResult, CohenError> {
    let v = validate(rater1, rater2)?;

    // Validate weight_fn(c, c) == 0 for all observed categories.
    for &c in &v.categories {
        let w = weight_fn(c, c);
        if w.abs() >= 1e-15 {
            return Err(CohenError::InvalidWeightFunction);
        }
    }

    let w_o: f64 = rater1
        .iter()
        .zip(rater2.iter())
        .map(|(&a, &b)| weight_fn(a, b))
        .sum::<f64>()
        / v.n;

    // Perfect agreement: no weighted disagreement means kappa = 1.0.
    if w_o.abs() < 1e-15 {
        return Ok(IrrResult {
            statistic_name: "weighted_cohen_kappa".to_string(),
            value: 1.0,
            ci_lower: None,
            ci_upper: None,
            n_items: rater1.len(),
            n_raters: 2,
            metric_level: Some(level),
        });
    }

    let w_e: f64 = v
        .categories
        .iter()
        .flat_map(|&ci| {
            let wf = &weight_fn;
            v.categories.iter().map(move |&cj| {
                let p1 = rater1.iter().filter(|&&r| r == ci).count() as f64 / v.n;
                let p2 = rater2.iter().filter(|&&r| r == cj).count() as f64 / v.n;
                p1 * p2 * wf(ci, cj)
            })
        })
        .sum();

    if w_e.abs() < 1e-15 {
        return Err(CohenError::DegenerateData);
    }

    Ok(IrrResult {
        statistic_name: "weighted_cohen_kappa".to_string(),
        value: 1.0 - w_o / w_e,
        ci_lower: None,
        ci_upper: None,
        n_items: rater1.len(),
        n_raters: 2,
        metric_level: Some(level),
    })
}

/// Compute Cohen's kappa from a two-rater `RatingMatrix`.
///
/// Extracts the two rater columns and delegates to [`kappa`]. Missing
/// values are not supported; returns `EmptyData` if any cell is `None`.
pub fn kappa_from_matrix(matrix: &RatingMatrix) -> Result<IrrResult, CohenError> {
    let (r1, r2) = extract_pair(matrix)?;
    kappa(&r1, &r2)
}

/// Compute Cohen's weighted kappa from a two-rater `RatingMatrix`.
pub fn weighted_kappa_from_matrix(
    matrix: &RatingMatrix,
    weight_fn: impl Fn(u32, u32) -> f64,
    level: MetricLevel,
) -> Result<IrrResult, CohenError> {
    let (r1, r2) = extract_pair(matrix)?;
    weighted_kappa(&r1, &r2, weight_fn, level)
}

fn extract_pair(matrix: &RatingMatrix) -> Result<(Vec<u32>, Vec<u32>), CohenError> {
    if matrix.n_items() == 0 {
        return Err(CohenError::EmptyData);
    }
    if matrix.n_raters() != 2 {
        return Err(CohenError::NotTwoRaters(matrix.n_raters()));
    }
    let mut r1 = Vec::with_capacity(matrix.n_items());
    let mut r2 = Vec::with_capacity(matrix.n_items());
    for row in &matrix.ratings {
        r1.push(row[0].ok_or(CohenError::EmptyData)?);
        r2.push(row[1].ok_or(CohenError::EmptyData)?);
    }
    Ok((r1, r2))
}

/// Linear disagreement weight: `|a - b|`.
///
/// Unnormalized — the constant scale factor cancels in the kappa ratio.
pub fn linear_weight(a: u32, b: u32) -> f64 {
    (a as f64 - b as f64).abs()
}

/// Quadratic disagreement weight: `(a - b)²`.
///
/// Unnormalized — the constant scale factor cancels in the kappa ratio.
pub fn quadratic_weight(a: u32, b: u32) -> f64 {
    let diff = a as f64 - b as f64;
    diff * diff
}

/// Cohen's kappa with bootstrap confidence intervals.
///
/// Calls [`kappa_from_matrix`] for the point estimate, then runs
/// [`bootstrap_ci`] with kappa as the statistic closure.
pub fn kappa_with_ci(
    matrix: &RatingMatrix,
    n_resamples: usize,
    confidence_level: f64,
    seed: u64,
) -> Result<IrrResult, CohenCiError> {
    let point = kappa_from_matrix(matrix)?;
    let ci = bootstrap_ci(
        matrix,
        |m| {
            kappa_from_matrix(m)
                .map(|r| r.value)
                .map_err(|e| e.to_string())
        },
        n_resamples,
        confidence_level,
        seed,
    )?;
    Ok(IrrResult {
        ci_lower: Some(ci.ci_lower),
        ci_upper: Some(ci.ci_upper),
        ..point
    })
}

/// Cohen's weighted kappa with bootstrap confidence intervals.
///
/// The weight function must be `Clone`-able (or use a function pointer).
pub fn weighted_kappa_with_ci(
    matrix: &RatingMatrix,
    weight_fn: fn(u32, u32) -> f64,
    level: MetricLevel,
    n_resamples: usize,
    confidence_level: f64,
    seed: u64,
) -> Result<IrrResult, CohenCiError> {
    let point = weighted_kappa_from_matrix(matrix, weight_fn, level)?;
    let ci = bootstrap_ci(
        matrix,
        |m| {
            weighted_kappa_from_matrix(m, weight_fn, level)
                .map(|r| r.value)
                .map_err(|e| e.to_string())
        },
        n_resamples,
        confidence_level,
        seed,
    )?;
    Ok(IrrResult {
        ci_lower: Some(ci.ci_lower),
        ci_upper: Some(ci.ci_upper),
        ..point
    })
}

/// Error type for Cohen kappa with CI, combining Cohen and bootstrap errors.
#[derive(Debug, thiserror::Error)]
pub enum CohenCiError {
    #[error(transparent)]
    Cohen(#[from] CohenError),
    #[error(transparent)]
    Bootstrap(#[from] BootstrapError),
}
