use crate::types::{IrrResult, MetricLevel};

#[derive(Debug, thiserror::Error)]
pub enum CohenError {
    #[error("empty data")]
    EmptyData,
    #[error("ratings must have equal length ({0} vs {1})")]
    UnequalLength(usize, usize),
    #[error("degenerate data: all ratings in a single category")]
    DegenerateData,
}

pub fn kappa(rater1: &[u32], rater2: &[u32]) -> Result<IrrResult, CohenError> {
    if rater1.is_empty() {
        return Err(CohenError::EmptyData);
    }
    if rater1.len() != rater2.len() {
        return Err(CohenError::UnequalLength(rater1.len(), rater2.len()));
    }

    let n = rater1.len() as f64;

    let p_o = rater1
        .iter()
        .zip(rater2.iter())
        .filter(|(a, b)| a == b)
        .count() as f64
        / n;

    let mut categories: Vec<u32> = rater1.iter().chain(rater2.iter()).copied().collect();
    categories.sort();
    categories.dedup();

    let p_e: f64 = categories
        .iter()
        .map(|&c| {
            let p1 = rater1.iter().filter(|&&r| r == c).count() as f64 / n;
            let p2 = rater2.iter().filter(|&&r| r == c).count() as f64 / n;
            p1 * p2
        })
        .sum();

    if (1.0 - p_e).abs() < 1e-15 {
        return Err(CohenError::DegenerateData);
    }

    let kappa_val = (p_o - p_e) / (1.0 - p_e);

    Ok(IrrResult {
        statistic_name: "cohen_kappa".to_string(),
        value: kappa_val,
        ci_lower: None,
        ci_upper: None,
        n_items: rater1.len(),
        n_raters: 2,
        metric_level: Some(MetricLevel::Nominal),
    })
}

pub fn weighted_kappa(
    rater1: &[u32],
    rater2: &[u32],
    weight_fn: impl Fn(u32, u32) -> f64,
) -> Result<IrrResult, CohenError> {
    if rater1.is_empty() {
        return Err(CohenError::EmptyData);
    }
    if rater1.len() != rater2.len() {
        return Err(CohenError::UnequalLength(rater1.len(), rater2.len()));
    }

    let n = rater1.len() as f64;

    let mut categories: Vec<u32> = rater1.iter().chain(rater2.iter()).copied().collect();
    categories.sort();
    categories.dedup();

    let w_o: f64 = rater1
        .iter()
        .zip(rater2.iter())
        .map(|(&a, &b)| weight_fn(a, b))
        .sum::<f64>()
        / n;

    let w_e: f64 = categories
        .iter()
        .flat_map(|&ci| {
            let wf = &weight_fn;
            categories.iter().map(move |&cj| {
                let p1 = rater1.iter().filter(|&&r| r == ci).count() as f64 / n;
                let p2 = rater2.iter().filter(|&&r| r == cj).count() as f64 / n;
                p1 * p2 * wf(ci, cj)
            })
        })
        .sum();

    if w_e.abs() < 1e-15 {
        return Err(CohenError::DegenerateData);
    }

    let kappa_val = 1.0 - w_o / w_e;

    Ok(IrrResult {
        statistic_name: "weighted_cohen_kappa".to_string(),
        value: kappa_val,
        ci_lower: None,
        ci_upper: None,
        n_items: rater1.len(),
        n_raters: 2,
        metric_level: None,
    })
}

pub fn linear_weight(a: u32, b: u32) -> f64 {
    (a as f64 - b as f64).abs()
}

pub fn quadratic_weight(a: u32, b: u32) -> f64 {
    let diff = a as f64 - b as f64;
    diff * diff
}
