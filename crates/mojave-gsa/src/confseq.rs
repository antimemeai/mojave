#![allow(clippy::cast_precision_loss, clippy::similar_names)]

use std::path::Path;

use anyhow::{bail, Context, Result};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use seq_anytime_valid::{AnytimeMonitor, DataFamily, MsprtConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellStopping {
    pub cell_id: String,
    pub n_items: usize,
    pub accuracy: f64,
    pub n_stopped: usize,
    pub median_stopping_n: f64,
    pub iqr_low: f64,
    pub iqr_high: f64,
    pub frac_stopped_q4: f64,
    pub frac_stopped_half: f64,
    pub frac_stopped_full: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfseqOutput {
    pub eval: String,
    pub model: String,
    pub method: String,
    pub half_width_threshold: f64,
    pub alpha: f64,
    pub n_permutations: usize,
    pub seed: u64,
    pub n_cells: usize,
    pub aggregate: AggregateStopping,
    pub cells: Vec<CellStopping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateStopping {
    pub total_cells: usize,
    pub cells_with_early_stop: usize,
    pub median_stopping_n: f64,
    pub iqr_low: f64,
    pub iqr_high: f64,
    pub frac_stopped_q4: f64,
    pub frac_stopped_half: f64,
    pub frac_stopped_full: f64,
}

#[derive(Deserialize)]
struct ResultsFile {
    eval: String,
    model: String,
    cells: Vec<CellEntry>,
    item_matrix: serde_json::Map<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct CellEntry {
    cell_id: String,
    accuracy: Option<f64>,
}

fn run_ci_width_stopping(
    outcomes: &[f64],
    half_width_threshold: f64,
    alpha: f64,
    n_permutations: usize,
    seed: u64,
) -> Result<(usize, Vec<usize>)> {
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let n = outcomes.len();
    let mut stopping_times = Vec::with_capacity(n_permutations);
    let mut n_stopped = 0;

    let config = MsprtConfig {
        theta_0: 0.5,
        mixing_variance: 1.0,
        family: DataFamily::Bernoulli,
        max_samples: None,
    };

    for _ in 0..n_permutations {
        let mut shuffled = outcomes.to_vec();
        shuffled.shuffle(&mut rng);

        let mut monitor = AnytimeMonitor::new(config.clone(), alpha)
            .map_err(|e| anyhow::anyhow!("monitor init: {e}"))?;

        let mut stopped_at = n;
        for (i, &obs) in shuffled.iter().enumerate() {
            let snap = monitor
                .update(obs)
                .map_err(|e| anyhow::anyhow!("monitor update: {e}"))?;
            if let Some((lo, hi)) = snap.confidence_interval {
                let hw = (hi - lo) / 2.0;
                if hw < half_width_threshold {
                    stopped_at = i + 1;
                    break;
                }
            }
        }

        if stopped_at < n {
            n_stopped += 1;
        }
        stopping_times.push(stopped_at);
    }

    Ok((n_stopped, stopping_times))
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n.is_multiple_of(2) {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    }
}

fn quantile(values: &[f64], q: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    let pos = q * (n - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let frac = pos - lo as f64;
        sorted[lo] * (1.0 - frac) + sorted[hi] * frac
    }
}

fn compute_cell_stopping(
    cell_id: &str,
    outcomes: &[f64],
    accuracy: f64,
    half_width_threshold: f64,
    alpha: f64,
    n_permutations: usize,
    seed: u64,
) -> Result<CellStopping> {
    let n_items = outcomes.len();
    let (n_stopped, stopping_times) =
        run_ci_width_stopping(outcomes, half_width_threshold, alpha, n_permutations, seed)?;

    let times_f64: Vec<f64> = stopping_times.iter().map(|&t| t as f64).collect();
    let n_perms_f64 = n_permutations as f64;
    let n_q4 = n_items / 4;
    let n_half = n_items / 2;

    Ok(CellStopping {
        cell_id: cell_id.to_string(),
        n_items,
        accuracy,
        n_stopped,
        median_stopping_n: median(&times_f64),
        iqr_low: quantile(&times_f64, 0.25),
        iqr_high: quantile(&times_f64, 0.75),
        frac_stopped_q4: stopping_times.iter().filter(|&&t| t <= n_q4).count() as f64 / n_perms_f64,
        frac_stopped_half: stopping_times.iter().filter(|&&t| t <= n_half).count() as f64
            / n_perms_f64,
        frac_stopped_full: stopping_times.iter().filter(|&&t| t < n_items).count() as f64
            / n_perms_f64,
    })
}

pub fn confseq_analyze(
    results_path: &Path,
    half_width_threshold: f64,
    alpha: f64,
    n_permutations: usize,
    seed: u64,
    output_path: &Path,
) -> Result<()> {
    let results_text = std::fs::read_to_string(results_path).context("reading results file")?;
    let results: ResultsFile =
        serde_json::from_str(&results_text).context("parsing results JSON")?;

    let mut cell_results = Vec::new();

    for (i, cell) in results.cells.iter().enumerate() {
        let accuracy = match cell.accuracy {
            Some(a) => a,
            None => continue,
        };

        let mut outcomes: Vec<(String, f64)> = Vec::new();
        for (item_id, cell_map) in &results.item_matrix {
            if let Some(val) = cell_map.get(&cell.cell_id) {
                if let Some(v) = val.as_f64() {
                    outcomes.push((item_id.clone(), v));
                }
            }
        }

        if outcomes.is_empty() {
            continue;
        }

        outcomes.sort_by(|a, b| a.0.cmp(&b.0));
        let obs: Vec<f64> = outcomes.iter().map(|(_, v)| *v).collect();

        let cell_seed = seed.wrapping_add(i as u64);
        let cell_stopping = compute_cell_stopping(
            &cell.cell_id,
            &obs,
            accuracy,
            half_width_threshold,
            alpha,
            n_permutations,
            cell_seed,
        )?;

        cell_results.push(cell_stopping);

        if (i + 1) % 500 == 0 || i + 1 == results.cells.len() {
            eprintln!(
                "  confseq: {}/{} cells analyzed",
                i + 1,
                results.cells.len()
            );
        }
    }

    let all_medians: Vec<f64> = cell_results.iter().map(|c| c.median_stopping_n).collect();
    let cells_with_early = cell_results.iter().filter(|c| c.n_stopped > 0).count();

    let aggregate = AggregateStopping {
        total_cells: cell_results.len(),
        cells_with_early_stop: cells_with_early,
        median_stopping_n: median(&all_medians),
        iqr_low: quantile(&all_medians, 0.25),
        iqr_high: quantile(&all_medians, 0.75),
        frac_stopped_q4: if cell_results.is_empty() {
            0.0
        } else {
            cell_results.iter().map(|c| c.frac_stopped_q4).sum::<f64>() / cell_results.len() as f64
        },
        frac_stopped_half: if cell_results.is_empty() {
            0.0
        } else {
            cell_results
                .iter()
                .map(|c| c.frac_stopped_half)
                .sum::<f64>()
                / cell_results.len() as f64
        },
        frac_stopped_full: if cell_results.is_empty() {
            0.0
        } else {
            cell_results
                .iter()
                .map(|c| c.frac_stopped_full)
                .sum::<f64>()
                / cell_results.len() as f64
        },
    };

    if cell_results.is_empty() {
        bail!("no cells with item-level data found in results");
    }

    let output = ConfseqOutput {
        eval: results.eval,
        model: results.model,
        method: "normal_mixture_cs_ci_width".to_string(),
        half_width_threshold,
        alpha,
        n_permutations,
        seed,
        n_cells: cell_results.len(),
        aggregate,
        cells: cell_results,
    };

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&output)?;
    std::fs::write(output_path, format!("{json}\n"))?;

    eprintln!(
        "  Confseq analysis: {} cells, {}/{} with any early stop",
        output.n_cells, output.aggregate.cells_with_early_stop, output.n_cells
    );
    eprintln!(
        "  Median stopping N: {:.0}, IQR [{:.0}, {:.0}]",
        output.aggregate.median_stopping_n, output.aggregate.iqr_low, output.aggregate.iqr_high
    );

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_results_json(n_items: usize, accuracy: f64) -> String {
        let n_correct = (n_items as f64 * accuracy).round() as usize;
        let mut item_matrix = serde_json::Map::new();
        let mut cell_map = serde_json::Map::new();

        for i in 0..n_items {
            let val = if i < n_correct { 1.0 } else { 0.0 };
            cell_map.insert(format!("item_{i:04}"), serde_json::json!({"cell_001": val}));
        }

        for (item_id, cell_vals) in &cell_map {
            let mut m = serde_json::Map::new();
            if let Some(v) = cell_vals.get("cell_001") {
                m.insert("cell_001".to_string(), v.clone());
            }
            item_matrix.insert(item_id.clone(), serde_json::Value::Object(m));
        }

        serde_json::json!({
            "eval": "test_eval",
            "model": "test_model",
            "cells": [{"cell_id": "cell_001", "accuracy": accuracy}],
            "item_matrix": item_matrix,
        })
        .to_string()
    }

    #[test]
    fn confseq_produces_output() {
        let json = make_results_json(100, 0.7);
        let mut input = NamedTempFile::new().unwrap();
        write!(input, "{json}").unwrap();
        let output = NamedTempFile::new().unwrap();

        confseq_analyze(input.path(), 0.02, 0.05, 50, 42, output.path()).unwrap();

        let result: ConfseqOutput =
            serde_json::from_str(&std::fs::read_to_string(output.path()).unwrap()).unwrap();
        assert_eq!(result.n_cells, 1);
        assert_eq!(result.method, "normal_mixture_cs_ci_width");
    }

    #[test]
    fn stopping_times_are_bounded() {
        let json = make_results_json(200, 0.8);
        let mut input = NamedTempFile::new().unwrap();
        write!(input, "{json}").unwrap();
        let output = NamedTempFile::new().unwrap();

        confseq_analyze(input.path(), 0.02, 0.05, 50, 42, output.path()).unwrap();

        let result: ConfseqOutput =
            serde_json::from_str(&std::fs::read_to_string(output.path()).unwrap()).unwrap();
        let cell = &result.cells[0];
        assert!(cell.median_stopping_n >= 1.0);
        assert!(cell.median_stopping_n <= cell.n_items as f64);
    }

    #[test]
    fn deterministic_with_same_seed() {
        let json = make_results_json(100, 0.6);

        let mut input1 = NamedTempFile::new().unwrap();
        write!(input1, "{json}").unwrap();
        let output1 = NamedTempFile::new().unwrap();
        confseq_analyze(input1.path(), 0.02, 0.05, 50, 42, output1.path()).unwrap();

        let mut input2 = NamedTempFile::new().unwrap();
        write!(input2, "{json}").unwrap();
        let output2 = NamedTempFile::new().unwrap();
        confseq_analyze(input2.path(), 0.02, 0.05, 50, 42, output2.path()).unwrap();

        let r1: ConfseqOutput =
            serde_json::from_str(&std::fs::read_to_string(output1.path()).unwrap()).unwrap();
        let r2: ConfseqOutput =
            serde_json::from_str(&std::fs::read_to_string(output2.path()).unwrap()).unwrap();

        assert_eq!(r1.cells[0].median_stopping_n, r2.cells[0].median_stopping_n);
        assert_eq!(r1.cells[0].n_stopped, r2.cells[0].n_stopped);
    }

    #[test]
    fn aggregate_fields_present() {
        let json = make_results_json(100, 0.5);
        let mut input = NamedTempFile::new().unwrap();
        write!(input, "{json}").unwrap();
        let output = NamedTempFile::new().unwrap();

        confseq_analyze(input.path(), 0.02, 0.05, 50, 42, output.path()).unwrap();

        let result: ConfseqOutput =
            serde_json::from_str(&std::fs::read_to_string(output.path()).unwrap()).unwrap();
        assert_eq!(result.aggregate.total_cells, 1);
        assert!(result.aggregate.frac_stopped_full >= 0.0);
        assert!(result.aggregate.frac_stopped_full <= 1.0);
    }
}
