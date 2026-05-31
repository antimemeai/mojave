#![allow(clippy::cast_precision_loss)]

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use salib_core::RngState;
use salib_samplers::{build_saltelli_matrix, SobolSampler};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxesConfig {
    pub axes: Vec<AxisDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisDef {
    pub name: String,
    pub levels: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub task: String,
    pub model: String,
    pub total_cells: usize,
    pub design: DesignMetadata,
    pub cells: Vec<Cell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignMetadata {
    pub name: String,
    #[serde(rename = "N_base")]
    pub n_base: usize,
    pub k: usize,
    pub calc_second_order: bool,
    pub seed_hex: String,
    pub axes: Vec<AxisMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisMetadata {
    pub name: String,
    pub n_levels: usize,
    pub levels: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    pub cell_id: String,
    pub saltelli_index: usize,
    #[serde(flatten)]
    pub axis_values: serde_json::Map<String, serde_json::Value>,
}

fn discretize(value: f64, n_levels: usize) -> usize {
    let idx = (value * n_levels as f64).floor() as usize;
    idx.min(n_levels - 1)
}

/// Convenience wrapper that generates a manifest with `calc_second_order=false`.
/// Used by existing tests and the backward-compatible interface.
#[allow(dead_code)]
pub fn generate_manifest(
    axes_config_path: &Path,
    task: &str,
    model: &str,
    n_base: usize,
    seed: [u8; 32],
    output_path: &Path,
) -> Result<()> {
    generate_manifest_with_options(
        axes_config_path,
        task,
        model,
        n_base,
        seed,
        false,
        output_path,
    )
}

pub fn generate_manifest_with_options(
    axes_config_path: &Path,
    task: &str,
    model: &str,
    n_base: usize,
    seed: [u8; 32],
    calc_second_order: bool,
    output_path: &Path,
) -> Result<()> {
    let config_text = fs::read_to_string(axes_config_path)
        .with_context(|| format!("reading axes config: {}", axes_config_path.display()))?;
    let config: AxesConfig =
        serde_json::from_str(&config_text).with_context(|| "parsing axes config JSON")?;

    let k = config.axes.len();
    anyhow::ensure!(k >= 1, "axes config must define at least one axis");

    let sampler = SobolSampler::minimal(2 * k);
    let mut rng = RngState::from_seed(seed);
    let matrix = build_saltelli_matrix(&sampler, n_base, calc_second_order, &mut rng)
        .with_context(|| "building Saltelli matrix")?;

    let n = matrix.n;
    let total_cells = matrix.total_evaluations();

    let mut cells = Vec::with_capacity(total_cells);
    let mut cell_idx: usize = 0;

    let make_cell = |row: &[f64], idx: usize| -> Cell {
        let mut axis_values = serde_json::Map::new();
        for (j, axis) in config.axes.iter().enumerate() {
            let level_idx = discretize(row[j], axis.levels.len());
            axis_values.insert(axis.name.clone(), axis.levels[level_idx].clone());
        }
        Cell {
            cell_id: format!("c{idx:05}"),
            saltelli_index: idx,
            axis_values,
        }
    };

    // A matrix rows
    for i in 0..n {
        let row = matrix.a.row(i);
        let row_slice = row
            .as_slice()
            .with_context(|| "matrix A row not contiguous")?;
        cells.push(make_cell(row_slice, cell_idx));
        cell_idx += 1;
    }

    // B matrix rows
    for i in 0..n {
        let row = matrix.b.row(i);
        let row_slice = row
            .as_slice()
            .with_context(|| "matrix B row not contiguous")?;
        cells.push(make_cell(row_slice, cell_idx));
        cell_idx += 1;
    }

    // A_B[j] matrices (for S1, ST)
    for j in 0..k {
        for i in 0..n {
            let row = matrix.a_b[j].row(i);
            let row_slice = row
                .as_slice()
                .with_context(|| format!("A_B[{j}] row not contiguous"))?;
            cells.push(make_cell(row_slice, cell_idx));
            cell_idx += 1;
        }
    }

    // B_A[j] matrices (for S2, only when calc_second_order=true)
    if let Some(ref b_a) = matrix.b_a {
        for (j, b_a_j) in b_a.iter().enumerate() {
            for i in 0..n {
                let row = b_a_j.row(i);
                let row_slice = row
                    .as_slice()
                    .with_context(|| format!("B_A[{j}] row not contiguous"))?;
                cells.push(make_cell(row_slice, cell_idx));
                cell_idx += 1;
            }
        }
    }

    assert_eq!(cells.len(), total_cells);

    let manifest = Manifest {
        task: task.to_string(),
        model: model.to_string(),
        total_cells,
        design: DesignMetadata {
            name: "saltelli_radial".to_string(),
            n_base,
            k,
            calc_second_order,
            seed_hex: hex::encode(seed),
            axes: config
                .axes
                .iter()
                .map(|a| AxisMetadata {
                    name: a.name.clone(),
                    n_levels: a.levels.len(),
                    levels: a.levels.clone(),
                })
                .collect(),
        },
        cells,
    };

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating output directory: {}", parent.display()))?;
    }
    let json =
        serde_json::to_string_pretty(&manifest).with_context(|| "serializing manifest to JSON")?;
    fs::write(output_path, format!("{json}\n"))
        .with_context(|| format!("writing manifest: {}", output_path.display()))?;

    eprintln!("Generated {total_cells} cells for {task}");
    eprintln!("  Design: Saltelli radial, N={n_base}, k={k}");
    eprintln!("  -> {}", output_path.display());

    Ok(())
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

    fn write_temp_axes_config() -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(default_axes_config().as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    fn default_seed() -> [u8; 32] {
        let mut bytes = [0u8; 32];
        let src = b"mojave-gsa-default-seed-v1";
        bytes[..src.len()].copy_from_slice(src);
        bytes
    }

    #[test]
    fn test_discretize_zero() {
        assert_eq!(discretize(0.0, 5), 0);
    }

    #[test]
    fn test_discretize_one() {
        assert_eq!(discretize(1.0, 5), 4);
    }

    #[test]
    fn test_discretize_midpoint() {
        assert_eq!(discretize(0.5, 4), 2);
    }

    #[test]
    fn test_discretize_boundary() {
        assert_eq!(discretize(0.999, 2), 1);
    }

    #[test]
    fn test_cell_count_n4_k6() {
        let axes_file = write_temp_axes_config();
        let output = NamedTempFile::new().unwrap();
        generate_manifest(
            axes_file.path(),
            "test_task",
            "test_model",
            4,
            default_seed(),
            output.path(),
        )
        .unwrap();

        let text = fs::read_to_string(output.path()).unwrap();
        let manifest: Manifest = serde_json::from_str(&text).unwrap();
        assert_eq!(manifest.total_cells, 4 * (6 + 2));
        assert_eq!(manifest.cells.len(), 32);
    }

    #[test]
    fn test_cell_count_n1024_k6() {
        let axes_file = write_temp_axes_config();
        let output = NamedTempFile::new().unwrap();
        generate_manifest(
            axes_file.path(),
            "t",
            "m",
            1024,
            default_seed(),
            output.path(),
        )
        .unwrap();

        let text = fs::read_to_string(output.path()).unwrap();
        let manifest: Manifest = serde_json::from_str(&text).unwrap();
        assert_eq!(manifest.total_cells, 8192);
        assert_eq!(manifest.cells.len(), 8192);
    }

    #[test]
    fn test_sequential_saltelli_index() {
        let axes_file = write_temp_axes_config();
        let output = NamedTempFile::new().unwrap();
        generate_manifest(axes_file.path(), "t", "m", 4, default_seed(), output.path()).unwrap();

        let text = fs::read_to_string(output.path()).unwrap();
        let manifest: Manifest = serde_json::from_str(&text).unwrap();
        let indices: Vec<usize> = manifest.cells.iter().map(|c| c.saltelli_index).collect();
        let expected: Vec<usize> = (0..32).collect();
        assert_eq!(indices, expected);
    }

    #[test]
    fn test_deterministic() {
        let axes_file = write_temp_axes_config();
        let out1 = NamedTempFile::new().unwrap();
        let out2 = NamedTempFile::new().unwrap();
        generate_manifest(axes_file.path(), "t", "m", 4, default_seed(), out1.path()).unwrap();
        generate_manifest(axes_file.path(), "t", "m", 4, default_seed(), out2.path()).unwrap();

        let t1 = fs::read_to_string(out1.path()).unwrap();
        let t2 = fs::read_to_string(out2.path()).unwrap();
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_cell_has_all_axis_values() {
        let axes_file = write_temp_axes_config();
        let output = NamedTempFile::new().unwrap();
        generate_manifest(axes_file.path(), "t", "m", 4, default_seed(), output.path()).unwrap();

        let text = fs::read_to_string(output.path()).unwrap();
        let manifest: Manifest = serde_json::from_str(&text).unwrap();
        for cell in &manifest.cells {
            assert!(
                cell.axis_values.contains_key("prompt_template"),
                "cell {} missing prompt_template",
                cell.cell_id
            );
            assert!(
                cell.axis_values.contains_key("system_prompt"),
                "cell {} missing system_prompt",
                cell.cell_id
            );
            assert!(
                cell.axis_values.contains_key("n_shot_frac"),
                "cell {} missing n_shot_frac",
                cell.cell_id
            );
            assert!(
                cell.axis_values.contains_key("choice_order"),
                "cell {} missing choice_order",
                cell.cell_id
            );
            assert!(
                cell.axis_values.contains_key("decoding"),
                "cell {} missing decoding",
                cell.cell_id
            );
            assert!(
                cell.axis_values.contains_key("quantization"),
                "cell {} missing quantization",
                cell.cell_id
            );
        }
    }

    #[test]
    fn test_cell_values_are_valid_levels() {
        let axes_file = write_temp_axes_config();
        let output = NamedTempFile::new().unwrap();
        generate_manifest(axes_file.path(), "t", "m", 4, default_seed(), output.path()).unwrap();

        let text = fs::read_to_string(output.path()).unwrap();
        let manifest: Manifest = serde_json::from_str(&text).unwrap();

        let valid_templates = [
            "lm-eval-default",
            "bare",
            "cot",
            "letter-only",
            "verbose-rationale",
        ];
        let valid_sys = ["none", "helpful", "domain-expert", "safety-aware"];
        let valid_nshot = [0.0_f64, 0.01, 0.025, 0.05];
        let valid_choice = ["original", "shuffled"];
        let valid_decoding = ["greedy", "T=0.7", "T=1.0"];
        let valid_quant = ["bf16", "fp8"];

        for cell in &manifest.cells {
            let pt = cell.axis_values["prompt_template"]
                .as_str()
                .unwrap_or_else(|| panic!("prompt_template not string in cell {}", cell.cell_id));
            assert!(
                valid_templates.contains(&pt),
                "invalid prompt_template: {pt}"
            );

            let sp = cell.axis_values["system_prompt"]
                .as_str()
                .unwrap_or_else(|| panic!("system_prompt not string in cell {}", cell.cell_id));
            assert!(valid_sys.contains(&sp), "invalid system_prompt: {sp}");

            let nsf = cell.axis_values["n_shot_frac"]
                .as_f64()
                .unwrap_or_else(|| panic!("n_shot_frac not float in cell {}", cell.cell_id));
            assert!(valid_nshot.contains(&nsf), "invalid n_shot_frac: {nsf}");

            let co = cell.axis_values["choice_order"]
                .as_str()
                .unwrap_or_else(|| panic!("choice_order not string in cell {}", cell.cell_id));
            assert!(valid_choice.contains(&co), "invalid choice_order: {co}");

            let dec = cell.axis_values["decoding"]
                .as_str()
                .unwrap_or_else(|| panic!("decoding not string in cell {}", cell.cell_id));
            assert!(valid_decoding.contains(&dec), "invalid decoding: {dec}");

            let q = cell.axis_values["quantization"]
                .as_str()
                .unwrap_or_else(|| panic!("quantization not string in cell {}", cell.cell_id));
            assert!(valid_quant.contains(&q), "invalid quantization: {q}");
        }
    }

    #[test]
    fn test_manifest_metadata() {
        let axes_file = write_temp_axes_config();
        let output = NamedTempFile::new().unwrap();
        generate_manifest(
            axes_file.path(),
            "inspect_evals/wmdp_chem",
            "Qwen/Qwen2.5-7B-Instruct",
            8,
            default_seed(),
            output.path(),
        )
        .unwrap();

        let text = fs::read_to_string(output.path()).unwrap();
        let manifest: Manifest = serde_json::from_str(&text).unwrap();
        assert_eq!(manifest.task, "inspect_evals/wmdp_chem");
        assert_eq!(manifest.model, "Qwen/Qwen2.5-7B-Instruct");
        assert_eq!(manifest.design.name, "saltelli_radial");
        assert_eq!(manifest.design.n_base, 8);
        assert_eq!(manifest.design.k, 6);
    }
}
