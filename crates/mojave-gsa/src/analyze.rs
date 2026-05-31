#![allow(
    clippy::cast_precision_loss,
    clippy::similar_names,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use ndarray::Array2;
use rand::RngCore;
use salib_core::{tree_sum, RngState};
use salib_estimators::estimate_borgonovo_delta;
use salib_samplers::{build_saltelli_matrix, SobolSampler};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct ResultsInput {
    eval: String,
    model: String,
    cells: Vec<ResultCell>,
}

#[derive(Debug, Deserialize)]
struct ResultCell {
    saltelli_index: usize,
    accuracy: Option<f64>,
    n_samples: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ManifestInput {
    #[allow(dead_code)]
    task: String,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    total_cells: usize,
    design: DesignInput,
}

#[derive(Debug, Deserialize)]
struct DesignInput {
    name: String,
    #[serde(rename = "N_base")]
    n_base: usize,
    k: usize,
    axes: Vec<AxisInput>,
}

#[derive(Debug, Deserialize)]
struct AxisInput {
    name: String,
}

#[derive(Debug, Serialize)]
pub struct AnalysisOutput {
    pub eval: String,
    pub model: String,
    pub design: DesignOutputMeta,
    pub n_cells: usize,
    pub aggregate: AggregateStats,
    pub sobol_indices: Vec<SobolIndexEntry>,
    pub borgonovo_indices: Vec<BorgonovoIndexEntry>,
    pub sobol_diagnostics: SobolDiagnostics,
    pub convergence_diagnostics: Vec<crate::diagnostics::SobolDiagnosticEntry>,
}

#[derive(Debug, Serialize)]
pub struct DesignOutputMeta {
    pub name: String,
    #[serde(rename = "N_base")]
    pub n_base: usize,
    pub k: usize,
}

#[derive(Debug, Serialize)]
pub struct AggregateStats {
    pub mean_accuracy: f64,
    pub min_accuracy: f64,
    pub max_accuracy: f64,
    pub spread: f64,
    pub sd: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct SobolIndexEntry {
    pub axis: String,
    #[serde(rename = "S1")]
    pub s1: f64,
    #[serde(rename = "S1_ci_low")]
    pub s1_ci_low: f64,
    #[serde(rename = "S1_ci_high")]
    pub s1_ci_high: f64,
    #[serde(rename = "ST")]
    pub st: f64,
    #[serde(rename = "ST_ci_low")]
    pub st_ci_low: f64,
    #[serde(rename = "ST_ci_high")]
    pub st_ci_high: f64,
}

#[derive(Debug, Serialize)]
pub struct BorgonovoIndexEntry {
    pub axis: String,
    pub delta: f64,
}

#[derive(Debug, Serialize)]
pub struct SobolDiagnostics {
    pub sum_s1: f64,
    pub sum_st: f64,
}

/// Saltelli 2010 Eq c (first-order) + Jansen 1999 Eq f (total-order)
/// computed directly from cached fa/fb/fab arrays.
fn compute_sobol_from_cached(
    fa: &[f64],
    fb: &[f64],
    fab: &[Vec<f64>],
    n: usize,
    d: usize,
) -> (Vec<f64>, Vec<f64>, f64) {
    let n_f = n as f64;

    let f0 = tree_sum(fa) / n_f;
    let fa_sq: Vec<f64> = fa.iter().map(|x| x * x).collect();
    let d_var = tree_sum(&fa_sq) / n_f - f0 * f0;

    let mut first_order = Vec::with_capacity(d);
    let mut total_order = Vec::with_capacity(d);

    for i in 0..d {
        let cross: Vec<f64> = (0..n).map(|j| fb[j] * (fab[i][j] - fa[j])).collect();
        let s_i = if d_var.abs() < 1e-30 {
            0.0
        } else {
            tree_sum(&cross) / (n_f * d_var)
        };
        first_order.push(s_i);

        let sq_diff: Vec<f64> = (0..n)
            .map(|j| {
                let diff = fa[j] - fab[i][j];
                diff * diff
            })
            .collect();
        let st_i = if d_var.abs() < 1e-30 {
            0.0
        } else {
            tree_sum(&sq_diff) / (2.0 * n_f * d_var)
        };
        total_order.push(st_i);
    }

    (first_order, total_order, d_var)
}

fn bootstrap_sobol_cis(
    fa: &[f64],
    fb: &[f64],
    fab: &[Vec<f64>],
    n: usize,
    d: usize,
    resamples: usize,
    confidence_level: f64,
    rng: &mut RngState,
) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
    let alpha = 1.0 - confidence_level;
    let mut chacha = rng.clone().into_chacha();

    let mut s1_samples: Vec<Vec<f64>> = vec![Vec::with_capacity(resamples); d];
    let mut st_samples: Vec<Vec<f64>> = vec![Vec::with_capacity(resamples); d];

    let mut idx = vec![0usize; n];

    for _ in 0..resamples {
        for slot in &mut idx {
            *slot = (chacha.next_u32() as usize) % n;
        }

        let fa_re: Vec<f64> = idx.iter().map(|&j| fa[j]).collect();
        let fb_re: Vec<f64> = idx.iter().map(|&j| fb[j]).collect();
        let fab_re: Vec<Vec<f64>> = (0..d)
            .map(|i| idx.iter().map(|&j| fab[i][j]).collect())
            .collect();

        let (s1, st, _) = compute_sobol_from_cached(&fa_re, &fb_re, &fab_re, n, d);
        for i in 0..d {
            s1_samples[i].push(s1[i]);
            st_samples[i].push(st[i]);
        }
    }

    let lo_pct = alpha / 2.0;
    let hi_pct = 1.0 - alpha / 2.0;

    let percentile = |samples: &mut Vec<f64>, p: f64| -> f64 {
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = ((p * samples.len() as f64).floor() as usize).min(samples.len() - 1);
        samples[idx]
    };

    let mut s1_ci = Vec::with_capacity(d);
    let mut st_ci = Vec::with_capacity(d);
    for i in 0..d {
        let lo = percentile(&mut s1_samples[i], lo_pct);
        let hi = percentile(&mut s1_samples[i], hi_pct);
        s1_ci.push((lo, hi));
        let lo = percentile(&mut st_samples[i], lo_pct);
        let hi = percentile(&mut st_samples[i], hi_pct);
        st_ci.push((lo, hi));
    }

    *rng = RngState::snapshot(&chacha, rng);
    (s1_ci, st_ci)
}

pub fn analyze(
    manifest_path: &Path,
    results_path: &Path,
    bootstrap_resamples: usize,
    confidence_level: f64,
    seed: [u8; 32],
    output_path: &Path,
) -> Result<()> {
    let manifest_text = fs::read_to_string(manifest_path)
        .with_context(|| format!("reading manifest: {}", manifest_path.display()))?;
    let manifest: ManifestInput =
        serde_json::from_str(&manifest_text).with_context(|| "parsing manifest JSON")?;

    let results_text = fs::read_to_string(results_path)
        .with_context(|| format!("reading results: {}", results_path.display()))?;
    let results: ResultsInput =
        serde_json::from_str(&results_text).with_context(|| "parsing results JSON")?;

    let n_base = manifest.design.n_base;
    let k = manifest.design.k;
    let expected_cells = n_base * (k + 2);
    let axis_names: Vec<String> = manifest
        .design
        .axes
        .iter()
        .map(|a| a.name.clone())
        .collect();

    let mut cells_sorted = results.cells;
    cells_sorted.sort_by_key(|c| c.saltelli_index);

    let zero_sample_indices: Vec<usize> = cells_sorted
        .iter()
        .filter(|c| c.n_samples == Some(0))
        .map(|c| c.saltelli_index)
        .collect();
    anyhow::ensure!(
        zero_sample_indices.is_empty(),
        "{count} cells have n_samples=0, corrupting variance decomposition. \
         Affected saltelli_indices: {indices:?}",
        count = zero_sample_indices.len(),
        indices = zero_sample_indices,
    );

    let mut y = Vec::with_capacity(expected_cells);
    for (i, cell) in cells_sorted.iter().enumerate() {
        anyhow::ensure!(
            cell.saltelli_index == i,
            "missing or out-of-order cell at saltelli_index {i}: expected index {i}, got {}",
            cell.saltelli_index
        );
        let acc = cell.accuracy.with_context(|| {
            format!(
                "cell at saltelli_index {i} has missing accuracy -- cannot analyze incomplete data"
            )
        })?;
        y.push(acc);
    }

    anyhow::ensure!(
        y.len() == expected_cells,
        "incomplete output vector: got {} cells, expected {} (N={n_base}, k={k})",
        y.len(),
        expected_cells
    );

    let n = n_base;
    let fa: Vec<f64> = y[0..n].to_vec();
    let fb: Vec<f64> = y[n..2 * n].to_vec();
    let mut fab: Vec<Vec<f64>> = Vec::with_capacity(k);
    for j in 0..k {
        let start = (2 + j) * n;
        let end = start + n;
        fab.push(y[start..end].to_vec());
    }

    let (s1, st, _total_variance) = compute_sobol_from_cached(&fa, &fb, &fab, n, k);

    let mut rng = RngState::from_seed(seed);
    let sampler = SobolSampler::minimal(2 * k);
    let _ = build_saltelli_matrix(&sampler, n_base, false, &mut rng)
        .with_context(|| "reconstructing Saltelli matrix for RNG advancement")?;

    let mut bootstrap_rng = rng.fork(b"bootstrap-sobol");
    let (s1_ci, st_ci) = bootstrap_sobol_cis(
        &fa,
        &fb,
        &fab,
        n,
        k,
        bootstrap_resamples,
        confidence_level,
        &mut bootstrap_rng,
    );

    let mut rng2 = RngState::from_seed(seed);
    let matrix = build_saltelli_matrix(&SobolSampler::minimal(2 * k), n_base, false, &mut rng2)
        .with_context(|| "reconstructing Saltelli matrix for Borgonovo")?;

    let mut x_data: Vec<f64> = Vec::with_capacity(expected_cells * k);
    for i in 0..n {
        let row = matrix.a.row(i);
        x_data.extend(row.iter());
    }
    for i in 0..n {
        let row = matrix.b.row(i);
        x_data.extend(row.iter());
    }
    for j in 0..k {
        for i in 0..n {
            let row = matrix.a_b[j].row(i);
            x_data.extend(row.iter());
        }
    }
    let x = Array2::from_shape_vec((expected_cells, k), x_data)
        .with_context(|| "building X matrix for Borgonovo")?;

    let borgonovo_result = estimate_borgonovo_delta(x.view(), &y)
        .map_err(|e| anyhow::anyhow!("Borgonovo estimation failed: {e}"))?;

    let mean_acc = y.iter().sum::<f64>() / y.len() as f64;
    let min_acc = y.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_acc = y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let spread = max_acc - min_acc;
    let sd = if y.len() > 1 {
        let variance = y.iter().map(|v| (v - mean_acc).powi(2)).sum::<f64>() / (y.len() - 1) as f64;
        Some(variance.sqrt())
    } else {
        None
    };

    let mut sobol_entries: Vec<SobolIndexEntry> = Vec::with_capacity(k);
    for i in 0..k {
        sobol_entries.push(SobolIndexEntry {
            axis: axis_names[i].clone(),
            s1: round6(s1[i]),
            s1_ci_low: round6(s1_ci[i].0),
            s1_ci_high: round6(s1_ci[i].1),
            st: round6(st[i]),
            st_ci_low: round6(st_ci[i].0),
            st_ci_high: round6(st_ci[i].1),
        });
    }

    sobol_entries.sort_by(|a, b| b.st.partial_cmp(&a.st).unwrap_or(std::cmp::Ordering::Equal));

    let borgonovo_entries: Vec<BorgonovoIndexEntry> = axis_names
        .iter()
        .zip(borgonovo_result.delta.iter())
        .map(|(name, &delta)| BorgonovoIndexEntry {
            axis: name.clone(),
            delta: round6(delta),
        })
        .collect();

    let sum_s1: f64 = s1.iter().sum();
    let sum_st: f64 = st.iter().sum();

    let convergence_diags = crate::diagnostics::run_diagnostics(
        &sobol_entries,
        &crate::diagnostics::DiagnosticConfig::default(),
    );

    let output = AnalysisOutput {
        eval: results.eval,
        model: results.model,
        design: DesignOutputMeta {
            name: manifest.design.name,
            n_base,
            k,
        },
        n_cells: expected_cells,
        aggregate: AggregateStats {
            mean_accuracy: round6(mean_acc),
            min_accuracy: round6(min_acc),
            max_accuracy: round6(max_acc),
            spread: round6(spread),
            sd: sd.map(round6),
        },
        sobol_indices: sobol_entries,
        borgonovo_indices: borgonovo_entries,
        sobol_diagnostics: SobolDiagnostics {
            sum_s1: round4(sum_s1),
            sum_st: round4(sum_st),
        },
        convergence_diagnostics: convergence_diags,
    };

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating output directory: {}", parent.display()))?;
    }
    let json =
        serde_json::to_string_pretty(&output).with_context(|| "serializing analysis to JSON")?;
    fs::write(output_path, format!("{json}\n"))
        .with_context(|| format!("writing analysis: {}", output_path.display()))?;

    eprintln!("\n  {} Sobol' analysis:", output.eval);
    eprintln!("    Cells: {}", output.n_cells);
    eprintln!("    Spread: {}", output.aggregate.spread);
    eprintln!("    Sum S1: {}", output.sobol_diagnostics.sum_s1);
    eprintln!("    Sum ST: {}", output.sobol_diagnostics.sum_st);
    if let Some(dominant) = output.sobol_indices.first() {
        eprintln!(
            "    Dominant factor: {} (ST={:.4})",
            dominant.axis, dominant.st
        );
    }
    if !output.convergence_diagnostics.is_empty() {
        eprintln!(
            "    Convergence warnings: {}",
            output.convergence_diagnostics.len()
        );
        for d in &output.convergence_diagnostics {
            eprintln!("      - {}", d.message);
        }
    }
    eprintln!("    -> {}", output_path.display());

    Ok(())
}

fn round4(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}

fn round6(v: f64) -> f64 {
    (v * 1_000_000.0).round() / 1_000_000.0
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn default_axes_config() -> String {
        r#"{
  "axes": [
    {"name": "prompt_template", "levels": ["lm-eval-default", "bare", "cot", "letter-only", "verbose-rationale"]},
    {"name": "system_prompt", "levels": ["none", "helpful", "domain-expert", "safety-aware"]},
    {"name": "n_shot_frac", "levels": [0.0, 0.01, 0.025, 0.05]},
    {"name": "choice_order", "levels": ["original", "shuffled"]},
    {"name": "decoding", "levels": ["greedy", "T=0.7", "T=1.0"]},
    {"name": "quantization", "levels": ["bf16", "fp8"]}
  ]
}"#
        .to_string()
    }

    fn default_seed() -> [u8; 32] {
        let mut bytes = [0u8; 32];
        let src = b"mojave-gsa-default-seed-v1";
        bytes[..src.len()].copy_from_slice(src);
        bytes
    }

    fn generate_test_inputs() -> (NamedTempFile, NamedTempFile, NamedTempFile) {
        let mut axes_file = NamedTempFile::new().unwrap();
        axes_file
            .write_all(default_axes_config().as_bytes())
            .unwrap();
        axes_file.flush().unwrap();

        let manifest_file = NamedTempFile::new().unwrap();
        crate::manifest::generate_manifest(
            axes_file.path(),
            "test_eval",
            "test_model",
            4,
            default_seed(),
            manifest_file.path(),
        )
        .unwrap();

        let manifest_text = fs::read_to_string(manifest_file.path()).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&manifest_text).unwrap();
        let total_cells = manifest["total_cells"].as_u64().unwrap() as usize;

        let mut cells = Vec::new();
        for i in 0..total_cells {
            let acc = 0.3 + 0.4 * (i as f64 / total_cells as f64);
            cells.push(serde_json::json!({
                "saltelli_index": i,
                "accuracy": acc,
                "cell_id": format!("c{i:05}")
            }));
        }
        let results = serde_json::json!({
            "eval": "test_eval",
            "model": "test_model",
            "cells": cells,
            "item_matrix": {}
        });

        let mut results_file = NamedTempFile::new().unwrap();
        results_file
            .write_all(serde_json::to_string_pretty(&results).unwrap().as_bytes())
            .unwrap();
        results_file.flush().unwrap();

        (axes_file, manifest_file, results_file)
    }

    #[test]
    fn test_analyze_produces_output() {
        let (_axes, manifest, results) = generate_test_inputs();
        let output = NamedTempFile::new().unwrap();

        analyze(
            manifest.path(),
            results.path(),
            100,
            0.95,
            default_seed(),
            output.path(),
        )
        .unwrap();

        let text = fs::read_to_string(output.path()).unwrap();
        let analysis: serde_json::Value = serde_json::from_str(&text).unwrap();

        assert_eq!(analysis["eval"], "test_eval");
        assert_eq!(analysis["model"], "test_model");
        assert_eq!(analysis["n_cells"], 32);
    }

    #[test]
    fn test_sobol_indices_count() {
        let (_axes, manifest, results) = generate_test_inputs();
        let output = NamedTempFile::new().unwrap();

        analyze(
            manifest.path(),
            results.path(),
            100,
            0.95,
            default_seed(),
            output.path(),
        )
        .unwrap();

        let text = fs::read_to_string(output.path()).unwrap();
        let analysis: serde_json::Value = serde_json::from_str(&text).unwrap();

        let indices = analysis["sobol_indices"].as_array().unwrap();
        assert_eq!(indices.len(), 6);
    }

    #[test]
    fn test_sobol_index_fields() {
        let (_axes, manifest, results) = generate_test_inputs();
        let output = NamedTempFile::new().unwrap();

        analyze(
            manifest.path(),
            results.path(),
            100,
            0.95,
            default_seed(),
            output.path(),
        )
        .unwrap();

        let text = fs::read_to_string(output.path()).unwrap();
        let analysis: serde_json::Value = serde_json::from_str(&text).unwrap();

        for entry in analysis["sobol_indices"].as_array().unwrap() {
            assert!(entry.get("axis").is_some());
            assert!(entry.get("S1").is_some());
            assert!(entry.get("S1_ci_low").is_some());
            assert!(entry.get("S1_ci_high").is_some());
            assert!(entry.get("ST").is_some());
            assert!(entry.get("ST_ci_low").is_some());
            assert!(entry.get("ST_ci_high").is_some());
        }
    }

    #[test]
    fn test_borgonovo_indices_count() {
        let (_axes, manifest, results) = generate_test_inputs();
        let output = NamedTempFile::new().unwrap();

        analyze(
            manifest.path(),
            results.path(),
            100,
            0.95,
            default_seed(),
            output.path(),
        )
        .unwrap();

        let text = fs::read_to_string(output.path()).unwrap();
        let analysis: serde_json::Value = serde_json::from_str(&text).unwrap();

        let borg = analysis["borgonovo_indices"].as_array().unwrap();
        assert_eq!(borg.len(), 6);
    }

    #[test]
    fn test_diagnostics_finite() {
        let (_axes, manifest, results) = generate_test_inputs();
        let output = NamedTempFile::new().unwrap();

        analyze(
            manifest.path(),
            results.path(),
            100,
            0.95,
            default_seed(),
            output.path(),
        )
        .unwrap();

        let text = fs::read_to_string(output.path()).unwrap();
        let analysis: serde_json::Value = serde_json::from_str(&text).unwrap();

        let sum_s1 = analysis["sobol_diagnostics"]["sum_s1"].as_f64().unwrap();
        let sum_st = analysis["sobol_diagnostics"]["sum_st"].as_f64().unwrap();
        assert!(sum_s1.is_finite());
        assert!(sum_st.is_finite());
    }

    #[test]
    fn test_analyze_deterministic() {
        let (_axes, manifest, results) = generate_test_inputs();
        let out1 = NamedTempFile::new().unwrap();
        let out2 = NamedTempFile::new().unwrap();

        analyze(
            manifest.path(),
            results.path(),
            100,
            0.95,
            default_seed(),
            out1.path(),
        )
        .unwrap();
        analyze(
            manifest.path(),
            results.path(),
            100,
            0.95,
            default_seed(),
            out2.path(),
        )
        .unwrap();

        let t1 = fs::read_to_string(out1.path()).unwrap();
        let t2 = fs::read_to_string(out2.path()).unwrap();
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_missing_accuracy_fails() {
        let (_axes, manifest, _results) = generate_test_inputs();

        let manifest_text = fs::read_to_string(manifest.path()).unwrap();
        let mf: serde_json::Value = serde_json::from_str(&manifest_text).unwrap();
        let total_cells = mf["total_cells"].as_u64().unwrap() as usize;

        let mut cells = Vec::new();
        for i in 0..total_cells {
            if i == 5 {
                cells.push(serde_json::json!({
                    "saltelli_index": i,
                    "accuracy": null,
                    "cell_id": format!("c{i:05}")
                }));
            } else {
                cells.push(serde_json::json!({
                    "saltelli_index": i,
                    "accuracy": 0.5,
                    "cell_id": format!("c{i:05}")
                }));
            }
        }
        let bad_results = serde_json::json!({
            "eval": "test", "model": "test",
            "cells": cells, "item_matrix": {}
        });
        let mut bad_file = NamedTempFile::new().unwrap();
        bad_file
            .write_all(serde_json::to_string(&bad_results).unwrap().as_bytes())
            .unwrap();
        bad_file.flush().unwrap();

        let output = NamedTempFile::new().unwrap();
        let err = analyze(
            manifest.path(),
            bad_file.path(),
            100,
            0.95,
            default_seed(),
            output.path(),
        );
        assert!(err.is_err());
        let msg = format!("{}", err.err().unwrap());
        assert!(
            msg.contains("missing"),
            "error should mention missing: {msg}"
        );
    }

    #[test]
    fn test_zero_n_samples_rejected() {
        let (_axes, manifest, _results) = generate_test_inputs();
        let manifest_text = fs::read_to_string(manifest.path()).unwrap();
        let mf: serde_json::Value = serde_json::from_str(&manifest_text).unwrap();
        let total_cells = mf["total_cells"].as_u64().unwrap() as usize;

        let mut cells = Vec::new();
        for i in 0..total_cells {
            let n_samples = if i == 3 || i == 7 || i == 11 { 0 } else { 50 };
            cells.push(serde_json::json!({
                "saltelli_index": i,
                "accuracy": 0.5,
                "n_samples": n_samples,
                "cell_id": format!("c{i:05}")
            }));
        }
        let bad_results = serde_json::json!({
            "eval": "test", "model": "test",
            "cells": cells, "item_matrix": {}
        });
        let mut bad_file = NamedTempFile::new().unwrap();
        bad_file
            .write_all(serde_json::to_string(&bad_results).unwrap().as_bytes())
            .unwrap();
        bad_file.flush().unwrap();

        let output = NamedTempFile::new().unwrap();
        let err = analyze(
            manifest.path(),
            bad_file.path(),
            100,
            0.95,
            default_seed(),
            output.path(),
        );
        assert!(err.is_err());
        let msg = format!("{}", err.err().unwrap());
        assert!(
            msg.contains("n_samples=0"),
            "error should mention n_samples=0: {msg}"
        );
        assert!(
            msg.contains("3 cells"),
            "error should report count of 3: {msg}"
        );
    }

    #[test]
    fn test_n_samples_absent_passes() {
        let (_axes, manifest, results) = generate_test_inputs();
        let output = NamedTempFile::new().unwrap();
        let result = analyze(
            manifest.path(),
            results.path(),
            100,
            0.95,
            default_seed(),
            output.path(),
        );
        assert!(result.is_ok(), "absent n_samples should not trigger gate");
    }

    #[test]
    fn test_compute_sobol_from_cached_constant_output() {
        let n = 8;
        let d = 3;
        let fa = vec![0.5; n];
        let fb = vec![0.5; n];
        let fab = vec![vec![0.5; n]; d];
        let (s1, st, var) = compute_sobol_from_cached(&fa, &fb, &fab, n, d);
        assert!(
            var.abs() < 1e-15,
            "variance should be ~0 for constant output"
        );
        for i in 0..d {
            assert_eq!(s1[i], 0.0);
            assert_eq!(st[i], 0.0);
        }
    }
}
