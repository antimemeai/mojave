// Range loops index into t_matrix, pi, counts, and class_priors simultaneously —
// iterator adapters obscure the EM algebra.
#![allow(clippy::needless_range_loop)]

/// Dawid-Skene EM latent-class model for annotation aggregation.
///
/// Jointly estimates latent true labels and per-annotator K×K confusion
/// matrices via expectation-maximization.
///
/// Reference: Dawid & Skene (1979); Paun et al. (2018) Section 2.2.
use crate::types::{AnnotationTriple, DawidSkeneResult};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum DawidSkeneError {
    #[error("empty annotation data")]
    EmptyData,
    #[error("failed to converge after {0} iterations")]
    NotConverged(usize),
}

pub struct DawidSkeneConfig {
    pub max_iterations: usize,
    pub tolerance: f64,
}

impl Default for DawidSkeneConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-6,
        }
    }
}

pub fn fit(
    triples: &[AnnotationTriple],
    config: &DawidSkeneConfig,
) -> Result<DawidSkeneResult, DawidSkeneError> {
    if triples.is_empty() {
        return Err(DawidSkeneError::EmptyData);
    }

    let mut item_map: BTreeMap<&str, usize> = BTreeMap::new();
    let mut ann_map: BTreeMap<&str, usize> = BTreeMap::new();
    let mut class_map: BTreeMap<u32, usize> = BTreeMap::new();

    for t in triples {
        let next = item_map.len();
        item_map.entry(&t.item_id).or_insert(next);
        let next = ann_map.len();
        ann_map.entry(&t.annotator_id).or_insert(next);
        let next = class_map.len();
        class_map.entry(t.label).or_insert(next);
    }

    let n_items = item_map.len();
    let n_annotators = ann_map.len();
    let n_classes = class_map.len();

    let mut classes: Vec<u32> = class_map.keys().copied().collect();
    classes.sort();
    let class_idx: BTreeMap<u32, usize> =
        classes.iter().enumerate().map(|(i, &c)| (c, i)).collect();

    // Build annotation lookup: item -> [(annotator_idx, class_idx)]
    let mut annotations: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n_items];
    for t in triples {
        let i = item_map[t.item_id.as_str()];
        let j = ann_map[t.annotator_id.as_str()];
        let k = class_idx[&t.label];
        annotations[i].push((j, k));
    }

    // Initialize T via majority vote
    // T[i][k] = P(true class of item i = k)
    let mut t_matrix = vec![vec![0.0f64; n_classes]; n_items];
    for (i, anns) in annotations.iter().enumerate() {
        let mut counts = vec![0usize; n_classes];
        for &(_, k) in anns {
            counts[k] += 1;
        }
        let total: f64 = counts.iter().sum::<usize>() as f64;
        if total > 0.0 {
            for k in 0..n_classes {
                t_matrix[i][k] = counts[k] as f64 / total;
            }
        } else {
            for k in 0..n_classes {
                t_matrix[i][k] = 1.0 / n_classes as f64;
            }
        }
    }

    let mut class_priors = vec![1.0 / n_classes as f64; n_classes];

    // pi[j][k][l] = P(annotator j says l | true class = k)
    let mut pi = vec![vec![vec![0.0f64; n_classes]; n_classes]; n_annotators];

    let mut prev_ll = f64::NEG_INFINITY;
    let mut converged = false;
    let mut n_iter = 0;

    for iter in 0..config.max_iterations {
        n_iter = iter + 1;

        // --- M-step ---

        // Update class priors: π_k = (1/I) Σ_i T[i][k]
        for k in 0..n_classes {
            class_priors[k] = t_matrix.iter().map(|t| t[k]).sum::<f64>() / n_items as f64;
        }

        // Update confusion matrices
        for j in 0..n_annotators {
            for k in 0..n_classes {
                // Denominator: Σ_i T[i][k] for items where annotator j provided a label
                let denom: f64 = annotations
                    .iter()
                    .enumerate()
                    .filter(|(_, anns)| anns.iter().any(|&(aj, _)| aj == j))
                    .map(|(i, _)| t_matrix[i][k])
                    .sum();

                for l in 0..n_classes {
                    // Numerator: Σ_i T[i][k] * I(annotator j labeled item i as l)
                    let numer: f64 = annotations
                        .iter()
                        .enumerate()
                        .filter(|(_, anns)| anns.iter().any(|&(aj, al)| aj == j && al == l))
                        .map(|(i, _)| t_matrix[i][k])
                        .sum();

                    pi[j][k][l] = if denom > 1e-15 {
                        numer / denom
                    } else {
                        1.0 / n_classes as f64
                    };
                }
            }
        }

        // --- E-step ---

        let mut log_likelihood = 0.0;

        for (i, anns) in annotations.iter().enumerate() {
            // log P(y_i | c_i = k) + log π_k for each k
            let mut log_probs = vec![0.0f64; n_classes];
            for k in 0..n_classes {
                log_probs[k] = class_priors[k].ln();
                for &(j, l) in anns {
                    let p = pi[j][k][l].max(1e-300);
                    log_probs[k] += p.ln();
                }
            }

            // Log-sum-exp for numerical stability
            let max_lp = log_probs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let log_sum = max_lp
                + log_probs
                    .iter()
                    .map(|lp| (lp - max_lp).exp())
                    .sum::<f64>()
                    .ln();

            for k in 0..n_classes {
                t_matrix[i][k] = (log_probs[k] - log_sum).exp();
            }
            log_likelihood += log_sum;
        }

        // Convergence check
        if (log_likelihood - prev_ll).abs() < config.tolerance {
            converged = true;
            prev_ll = log_likelihood;
            break;
        }
        prev_ll = log_likelihood;
    }

    let estimated_labels: Vec<u32> = t_matrix
        .iter()
        .map(|t| {
            let max_idx = t
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap()
                .0;
            classes[max_idx]
        })
        .collect();

    Ok(DawidSkeneResult {
        estimated_labels,
        label_probabilities: t_matrix,
        confusion_matrices: pi,
        class_priors,
        n_iterations: n_iter,
        converged,
        log_likelihood: prev_ll,
    })
}
