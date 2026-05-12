use std::collections::BTreeMap;

use crate::categorical_agreement_weights::{WeightMatrix, WeightScheme};
use crate::types::{IrrResult, MetricLevel, RatingMatrix};

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

/// Compute Gwet's AC (AC1/AC2/AC3) agreement coefficient.
///
/// - `weights = None` → AC1 (identity weights, nominal).
/// - `weights = Some(wm)` → AC2 or AC3 depending on whether the weight
///   matrix matches a standard scheme (identity, linear, quadratic, ordinal).
///
/// Reference: Gwet (2008) "Computing inter-rater reliability and its variance
/// in the presence of high agreement", eq. 3.
pub fn ac(matrix: &RatingMatrix, weights: Option<&WeightMatrix>) -> Result<IrrResult, GwetError> {
    // --- Validate ---
    if matrix.n_items() == 0 {
        return Err(GwetError::EmptyData);
    }
    if matrix.n_raters() < 2 {
        return Err(GwetError::TooFewRaters);
    }

    // --- Discover categories from data ---
    let mut cat_set: Vec<u32> = matrix
        .ratings
        .iter()
        .flat_map(|row| row.iter().filter_map(|&v| v))
        .collect();
    cat_set.sort_unstable();
    cat_set.dedup();
    let q = cat_set.len();

    if q == 0 {
        return Err(GwetError::EmptyData);
    }

    // Map category value → index in our sorted category list
    let cat_index: BTreeMap<u32, usize> =
        cat_set.iter().enumerate().map(|(i, &c)| (c, i)).collect();

    // --- Build effective weight lookup ---
    // If weights provided, map data categories to weight matrix categories.
    // If a data category is absent from the weight matrix, fall back to identity.
    let w_lookup: Vec<Vec<f64>> = if let Some(wm) = weights {
        let wm_cat_index: BTreeMap<u32, usize> = wm
            .categories
            .iter()
            .enumerate()
            .map(|(i, &c)| (c, i))
            .collect();

        let mut w = vec![vec![0.0; q]; q];
        for (di, &dc) in cat_set.iter().enumerate() {
            for (dj, &dc2) in cat_set.iter().enumerate() {
                if let (Some(&wi), Some(&wj)) = (wm_cat_index.get(&dc), wm_cat_index.get(&dc2)) {
                    w[di][dj] = wm.weights[wi][wj];
                } else {
                    // Fallback: identity weight
                    w[di][dj] = if di == dj { 1.0 } else { 0.0 };
                }
            }
        }
        w
    } else {
        // Identity weights for AC1
        let mut w = vec![vec![0.0; q]; q];
        for (i, row) in w.iter_mut().enumerate() {
            row[i] = 1.0;
        }
        w
    };

    // --- Accumulate observed agreement and marginal proportions ---
    // Following irrCAC R package: pi is computed over ALL items (including
    // those with only 1 rater), normalized by n (total items with >=1 rating).
    // pa is computed only over items with >=2 raters.
    let mut pa_sum = 0.0_f64;
    let mut n_usable = 0usize; // items with >= 2 raters (for pa)
    let mut n_rated = 0usize; // items with >= 1 rating (for pi normalization)
    let mut pi = vec![0.0_f64; q]; // marginal proportions (unnormalized)

    for row in &matrix.ratings {
        // Collect present ratings for this item
        let present: Vec<u32> = row.iter().filter_map(|&v| v).collect();
        let r = present.len();
        if r == 0 {
            continue;
        }
        n_rated += 1;

        // Marginal proportions: for each present rating, add 1/r
        // This runs for ALL items with >=1 rating (irrCAC convention)
        let inv_r = 1.0 / r as f64;
        for &c in &present {
            let idx = cat_index[&c];
            pi[idx] += inv_r;
        }

        // Observed agreement only for items with >=2 raters
        if r >= 2 {
            n_usable += 1;
            let mut item_agree = 0.0_f64;
            for (i, &ci) in present.iter().enumerate() {
                for (j, &cj) in present.iter().enumerate() {
                    if i != j {
                        let idx_i = cat_index[&ci];
                        let idx_j = cat_index[&cj];
                        item_agree += w_lookup[idx_i][idx_j];
                    }
                }
            }
            item_agree /= (r * (r - 1)) as f64;
            pa_sum += item_agree;
        }
    }

    if n_usable == 0 {
        return Err(GwetError::DegenerateData);
    }

    let pa = pa_sum / n_usable as f64;

    // Normalize pi by n_rated (all items with >=1 rating), matching irrCAC
    let n_for_pi = n_rated as f64;
    for p in &mut pi {
        *p /= n_for_pi;
    }

    // --- Chance agreement (Gwet 2014; irrCAC R package) ---
    // pe = sum(weights_mat) * sum_k[pi_k(1-pi_k)] / (q*(q-1))
    if q < 2 {
        // Only 1 category → all-same → pe would be degenerate
        return Err(GwetError::DegeneratePe);
    }

    let sum_weights: f64 = w_lookup.iter().flat_map(|row| row.iter()).sum();
    let pi_diversity: f64 = pi.iter().map(|&p| p * (1.0 - p)).sum();
    let pe = sum_weights * pi_diversity / (q * (q - 1)) as f64;

    if (1.0 - pe).abs() < 1e-15 {
        return Err(GwetError::DegeneratePe);
    }

    let ac_value = (pa - pe) / (1.0 - pe);

    // --- Determine statistic name ---
    let statistic_name = determine_name(weights, &cat_set);

    let metric_level = if weights.is_none() {
        Some(MetricLevel::Nominal)
    } else {
        // For weighted variants, we don't presume a level
        None
    };

    Ok(IrrResult {
        statistic_name,
        value: ac_value,
        ci_lower: None,
        ci_upper: None,
        n_items: n_usable,
        n_raters: matrix.n_raters(),
        metric_level,
    })
}

/// Determine the statistic name based on whether/which weight scheme was used.
fn determine_name(weights: Option<&WeightMatrix>, categories: &[u32]) -> String {
    let wm = match weights {
        None => return "Gwet AC1".to_string(),
        Some(wm) => wm,
    };

    // Check if the provided weight matrix matches any standard scheme
    for (scheme, label) in [
        (WeightScheme::Identity, "Gwet AC2(identity)"),
        (WeightScheme::Linear, "Gwet AC2(linear)"),
        (WeightScheme::Quadratic, "Gwet AC2(quadratic)"),
        (WeightScheme::Ordinal, "Gwet AC2(ordinal)"),
    ] {
        let reference = WeightMatrix::from_scheme(categories, scheme);
        if weights_match(&reference, wm) {
            return label.to_string();
        }
    }

    "Gwet AC3".to_string()
}

/// Check if two weight matrices are approximately equal (same categories, same weights).
fn weights_match(a: &WeightMatrix, b: &WeightMatrix) -> bool {
    if a.categories.len() != b.categories.len() {
        return false;
    }
    // Build a map from b's categories to their indices
    let b_cat_index: BTreeMap<u32, usize> = b
        .categories
        .iter()
        .enumerate()
        .map(|(i, &c)| (c, i))
        .collect();

    for (ai, &ac) in a.categories.iter().enumerate() {
        let bi = match b_cat_index.get(&ac) {
            Some(&idx) => idx,
            None => return false,
        };
        for (aj, &ac2) in a.categories.iter().enumerate() {
            let bj = match b_cat_index.get(&ac2) {
                Some(&idx) => idx,
                None => return false,
            };
            if (a.weights[ai][aj] - b.weights[bi][bj]).abs() > 1e-10 {
                return false;
            }
        }
    }
    true
}
