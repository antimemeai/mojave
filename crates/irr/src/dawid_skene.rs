// Range loops index into t_matrix, pi, counts, and class_priors simultaneously —
// iterator adapters obscure the EM algebra.
#![allow(clippy::needless_range_loop)]

/// Dawid-Skene EM latent-class model for annotation aggregation.
///
/// Jointly estimates latent true labels and per-annotator K×K confusion
/// matrices via expectation-maximization. Supports repeated annotations
/// (the same annotator may rate the same item multiple times).
///
/// Reference: Dawid & Skene (1979); Paun et al. (2018) Section 2.2.
use crate::types::{AnnotationTriple, DawidSkeneResult};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, thiserror::Error)]
pub enum DawidSkeneError {
    #[error("empty annotation data")]
    EmptyData,
}

#[derive(Debug, Clone)]
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
    let mut class_set: BTreeSet<u32> = BTreeSet::new();

    for t in triples {
        let next = item_map.len();
        item_map.entry(&t.item_id).or_insert(next);
        let next = ann_map.len();
        ann_map.entry(&t.annotator_id).or_insert(next);
        class_set.insert(t.label);
    }

    let n_items = item_map.len();
    let n_annotators = ann_map.len();
    let classes: Vec<u32> = class_set.into_iter().collect();
    let n_classes = classes.len();
    let class_idx: BTreeMap<u32, usize> =
        classes.iter().enumerate().map(|(i, &c)| (c, i)).collect();

    // Per-item annotations for E-step likelihood computation
    let mut annotations: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n_items];
    // Per-annotator label counts for single-pass M-step (eq 2.3 from paper)
    // ann_label_counts[j] = { item_idx → counts_per_class }
    let mut ann_label_counts: Vec<BTreeMap<usize, Vec<usize>>> =
        vec![BTreeMap::new(); n_annotators];

    for t in triples {
        let i = item_map[t.item_id.as_str()];
        let j = ann_map[t.annotator_id.as_str()];
        let k = class_idx[&t.label];
        annotations[i].push((j, k));
        let counts = ann_label_counts[j]
            .entry(i)
            .or_insert_with(|| vec![0usize; n_classes]);
        counts[k] += 1;
    }

    let mut annotator_ids: Vec<(usize, String)> =
        ann_map.iter().map(|(&k, &v)| (v, k.to_string())).collect();
    annotator_ids.sort_by_key(|(idx, _)| *idx);
    let annotator_ids: Vec<String> = annotator_ids.into_iter().map(|(_, s)| s).collect();

    // Initialize T via majority vote (eq 3.1)
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

        // --- M-step (eqs 2.3, 2.4) ---

        // Update class priors: p_j = (1/I) Σ_i T[i][j]
        for k in 0..n_classes {
            class_priors[k] = t_matrix.iter().map(|t| t[k]).sum::<f64>() / n_items as f64;
        }

        // Update confusion matrices using precomputed per-annotator label counts
        for j in 0..n_annotators {
            for k in 0..n_classes {
                let mut denom = 0.0;
                let mut numer = vec![0.0f64; n_classes];
                for (&i, counts) in &ann_label_counts[j] {
                    // n_total > 1 when an annotator rated the same item multiple times
                    let n_total: usize = counts.iter().sum();
                    denom += t_matrix[i][k] * n_total as f64;
                    for l in 0..n_classes {
                        numer[l] += t_matrix[i][k] * counts[l] as f64;
                    }
                }
                for l in 0..n_classes {
                    pi[j][k][l] = if denom > 1e-15 {
                        numer[l] / denom
                    } else {
                        1.0 / n_classes as f64
                    };
                }
            }
        }

        // --- E-step (eq 2.5) ---

        let mut log_likelihood = 0.0;

        for (i, anns) in annotations.iter().enumerate() {
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
        annotator_ids,
        class_labels: classes,
    })
}
