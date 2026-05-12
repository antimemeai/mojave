use crate::types::{IrrResult, MetricLevel, RatingMatrix};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum KrippendorffError {
    #[error("metric level must be specified explicitly")]
    NoMetricLevel,
    #[error("empty rating matrix")]
    EmptyData,
    #[error("degenerate data: fewer than 2 pairable values")]
    DegenerateData,
}

/// Compute Krippendorff's alpha reliability coefficient.
///
/// Uses the coincidence-matrix formulation from Krippendorff (2011)
/// "Computing Krippendorff's Alpha-Reliability."
pub fn alpha(
    matrix: &RatingMatrix,
    level: Option<MetricLevel>,
) -> Result<IrrResult, KrippendorffError> {
    let level = level.ok_or(KrippendorffError::NoMetricLevel)?;

    if matrix.n_items() == 0 {
        return Err(KrippendorffError::EmptyData);
    }

    // Collect all distinct values (sorted)
    let mut all_values: Vec<u32> = Vec::new();
    for row in &matrix.ratings {
        for val in row.iter().flatten() {
            all_values.push(*val);
        }
    }
    all_values.sort();
    all_values.dedup();

    let n_vals = all_values.len();
    let val_idx: BTreeMap<u32, usize> = all_values
        .iter()
        .enumerate()
        .map(|(i, &v)| (v, i))
        .collect();

    // Build the coincidence matrix
    let mut coincidence = vec![vec![0.0f64; n_vals]; n_vals];

    for row in &matrix.ratings {
        let present: Vec<u32> = row.iter().filter_map(|v| *v).collect();
        let m = present.len();
        if m < 2 {
            continue;
        }
        let weight = 1.0 / (m as f64 - 1.0);
        for i in 0..m {
            for j in 0..m {
                if i == j {
                    continue;
                }
                let ci = val_idx[&present[i]];
                let cj = val_idx[&present[j]];
                coincidence[ci][cj] += weight;
            }
        }
    }

    // Marginals of the coincidence matrix
    let n_c: Vec<f64> = (0..n_vals)
        .map(|c| coincidence[c].iter().sum::<f64>())
        .collect();
    let n_coinc: f64 = n_c.iter().sum();

    if n_coinc < 2.0 {
        return Err(KrippendorffError::DegenerateData);
    }

    // Compute distance matrix for all value pairs
    let dist = compute_distance_matrix(&all_values, &n_c, level);

    // D_o = sum_{c<k} o_ck * delta²(c,k)
    let mut d_o = 0.0;
    for c in 0..n_vals {
        for k in (c + 1)..n_vals {
            d_o += coincidence[c][k] * dist[c][k];
        }
    }

    // D_e = (1/(n-1)) * sum_{c<k} n_c * n_k * delta²(c,k)
    let mut d_e = 0.0;
    for c in 0..n_vals {
        for k in (c + 1)..n_vals {
            d_e += n_c[c] * n_c[k] * dist[c][k];
        }
    }
    d_e /= n_coinc - 1.0;

    if d_e.abs() < 1e-15 {
        return Err(KrippendorffError::DegenerateData);
    }
    let alpha_val = 1.0 - d_o / d_e;

    Ok(IrrResult {
        statistic_name: "krippendorff_alpha".to_string(),
        value: alpha_val,
        ci_lower: None,
        ci_upper: None,
        n_items: matrix.n_items(),
        n_raters: matrix.n_raters(),
        metric_level: Some(level),
    })
}

fn compute_distance_matrix(values: &[u32], marginals: &[f64], level: MetricLevel) -> Vec<Vec<f64>> {
    let n = values.len();
    let mut dist = vec![vec![0.0f64; n]; n];

    for c in 0..n {
        for k in (c + 1)..n {
            let d = match level {
                MetricLevel::Nominal => 1.0,
                MetricLevel::Interval => {
                    let diff = values[c] as f64 - values[k] as f64;
                    diff * diff
                }
                MetricLevel::Ratio => {
                    let sum = values[c] as f64 + values[k] as f64;
                    if sum == 0.0 {
                        0.0
                    } else {
                        let diff = values[c] as f64 - values[k] as f64;
                        (diff / sum) * (diff / sum)
                    }
                }
                MetricLevel::Ordinal => {
                    // Krippendorff ordinal distance:
                    // d²(c,k) = (Σ_{g=c..k} n_g - (n_c + n_k)/2)²
                    // where the sum is over all values g with index between c and k inclusive
                    let sum_between: f64 = marginals[c..=k].iter().sum();
                    let adj = (marginals[c] + marginals[k]) / 2.0;
                    let d = sum_between - adj;
                    d * d
                }
            };
            dist[c][k] = d;
            dist[k][c] = d;
        }
    }

    dist
}
