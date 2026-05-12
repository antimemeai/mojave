use crate::krippendorff;
use crate::types::{MetricLevel, RatingMatrix};
use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[must_use]
pub struct StratifiedAlphaResult {
    pub overall_alpha: f64,
    pub within_family: BTreeMap<String, f64>,
    pub between_family_alpha: f64,
    pub bias_burden: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum StratificationError {
    #[error("need at least 2 families to stratify")]
    TooFewFamilies,
    #[error("empty rating matrix")]
    EmptyData,
    #[error("rater {0} not found in family map")]
    UnmappedRater(String),
    #[error("no family has 2+ raters; within-family alpha is undefined")]
    NoMultiRaterFamilies,
    #[error("between-family alpha computation failed: {0}")]
    BetweenFamilyFailed(String),
    #[error("krippendorff computation failed: {0}")]
    Krippendorff(#[from] krippendorff::KrippendorffError),
}

/// Decompose Krippendorff alpha into within-family and between-family components.
///
/// `rater_families` maps rater name → family name. Every rater in the matrix
/// must appear in this map.
///
/// Between-family alpha: collects all cross-family rater pairs, computes
/// pairwise agreement for each pair, and averages. This avoids the bias of
/// picking a single arbitrary representative per family.
///
/// Families with fewer than 2 raters are excluded from within-family computation.
pub fn stratified_alpha(
    matrix: &RatingMatrix,
    rater_families: &BTreeMap<String, String>,
    level: MetricLevel,
) -> Result<StratifiedAlphaResult, StratificationError> {
    if matrix.n_items() == 0 {
        return Err(StratificationError::EmptyData);
    }

    let overall_alpha = krippendorff::alpha(matrix, Some(level))?.value;

    let mut family_rater_indices: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (idx, rater) in matrix.raters.iter().enumerate() {
        let family = rater_families
            .get(rater)
            .ok_or_else(|| StratificationError::UnmappedRater(rater.clone()))?;
        family_rater_indices
            .entry(family.clone())
            .or_default()
            .push(idx);
    }

    if family_rater_indices.len() < 2 {
        return Err(StratificationError::TooFewFamilies);
    }

    let mut within_family = BTreeMap::new();
    for (family, indices) in &family_rater_indices {
        if indices.len() < 2 {
            continue;
        }
        let sub_matrix = extract_rater_subset(matrix, indices);
        if let Ok(r) = krippendorff::alpha(&sub_matrix, Some(level)) {
            within_family.insert(family.clone(), r.value);
        }
    }

    let between_family_alpha = compute_between_family_alpha(matrix, &family_rater_indices, level)?;

    let mean_within = if within_family.is_empty() {
        return Err(StratificationError::NoMultiRaterFamilies);
    } else {
        within_family.values().sum::<f64>() / within_family.len() as f64
    };
    let bias_burden = mean_within - between_family_alpha;

    Ok(StratifiedAlphaResult {
        overall_alpha,
        within_family,
        between_family_alpha,
        bias_burden,
    })
}

/// Between-family alpha via all cross-family rater pairs.
///
/// For each pair of raters from different families, compute Krippendorff alpha
/// on the 2-rater sub-matrix, then average all successful pairwise alphas.
fn compute_between_family_alpha(
    matrix: &RatingMatrix,
    family_rater_indices: &BTreeMap<String, Vec<usize>>,
    level: MetricLevel,
) -> Result<f64, StratificationError> {
    let families: Vec<(&String, &Vec<usize>)> = family_rater_indices.iter().collect();
    let mut pairwise_alphas = Vec::new();

    for i in 0..families.len() {
        for j in (i + 1)..families.len() {
            for &ri in families[i].1 {
                for &rj in families[j].1 {
                    let sub = extract_rater_subset(matrix, &[ri, rj]);
                    if let Ok(r) = krippendorff::alpha(&sub, Some(level)) {
                        if r.value.is_finite() {
                            pairwise_alphas.push(r.value);
                        }
                    }
                }
            }
        }
    }

    if pairwise_alphas.is_empty() {
        return Err(StratificationError::BetweenFamilyFailed(
            "all cross-family pairwise alpha computations failed".to_string(),
        ));
    }

    Ok(pairwise_alphas.iter().sum::<f64>() / pairwise_alphas.len() as f64)
}

fn extract_rater_subset(matrix: &RatingMatrix, indices: &[usize]) -> RatingMatrix {
    let raters: Vec<String> = indices.iter().map(|&i| matrix.raters[i].clone()).collect();
    let ratings: Vec<Vec<Option<u32>>> = matrix
        .ratings
        .iter()
        .map(|row| indices.iter().map(|&i| row[i]).collect())
        .collect();
    RatingMatrix {
        items: matrix.items.clone(),
        raters,
        ratings,
    }
}
