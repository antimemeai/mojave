use crate::types::{IrrResult, MetricLevel, RatingMatrix};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum FleissError {
    #[error("empty rating matrix")]
    EmptyData,
    #[error("Fleiss kappa requires at least 2 raters, got {0}")]
    InsufficientRaters(usize),
    #[error("Fleiss kappa requires complete data (no missing values)")]
    MissingData,
    #[error("degenerate data: all ratings in a single category")]
    DegenerateData,
}

pub fn kappa(matrix: &RatingMatrix) -> Result<IrrResult, FleissError> {
    if matrix.n_items() == 0 {
        return Err(FleissError::EmptyData);
    }

    let n = matrix.n_items();
    let k = matrix.n_raters();

    if k < 2 {
        return Err(FleissError::InsufficientRaters(k));
    }

    let mut categories: Vec<u32> = Vec::new();
    for row in &matrix.ratings {
        for val in row.iter() {
            match val {
                Some(v) => {
                    if !categories.contains(v) {
                        categories.push(*v);
                    }
                }
                None => return Err(FleissError::MissingData),
            }
        }
    }
    categories.sort();
    let q = categories.len();

    let cat_idx: BTreeMap<u32, usize> = categories
        .iter()
        .enumerate()
        .map(|(i, &v)| (v, i))
        .collect();

    let mut n_matrix = vec![vec![0usize; q]; n];
    for (i, row) in matrix.ratings.iter().enumerate() {
        for v in row.iter().flatten() {
            n_matrix[i][cat_idx[v]] += 1;
        }
    }

    let kf = k as f64;
    let nf = n as f64;

    let p_i: Vec<f64> = n_matrix
        .iter()
        .map(|row| {
            let sum_sq: f64 = row.iter().map(|&x| (x as f64).powi(2)).sum();
            (sum_sq - kf) / (kf * (kf - 1.0))
        })
        .collect();

    let p_bar: f64 = p_i.iter().sum::<f64>() / nf;

    let p_j: Vec<f64> = (0..q)
        .map(|j| {
            let count: f64 = n_matrix.iter().map(|row| row[j] as f64).sum();
            count / (nf * kf)
        })
        .collect();

    let p_e: f64 = p_j.iter().map(|p| p * p).sum();

    if (1.0 - p_e).abs() < 1e-15 {
        return Err(FleissError::DegenerateData);
    }

    let kappa_val = (p_bar - p_e) / (1.0 - p_e);

    Ok(IrrResult {
        statistic_name: "fleiss_kappa".to_string(),
        value: kappa_val,
        ci_lower: None,
        ci_upper: None,
        n_items: n,
        n_raters: k,
        metric_level: Some(MetricLevel::Nominal),
    })
}
