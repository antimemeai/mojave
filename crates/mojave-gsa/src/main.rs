#![forbid(unsafe_code)]

mod analyze;
mod confseq;
mod diagnostics;
mod manifest;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "mojave-gsa",
    about = "Global sensitivity analysis for mojave eval pipeline"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a Saltelli radial sample manifest from an axes config.
    GenerateManifest {
        #[arg(long)]
        axes_config: PathBuf,

        #[arg(long)]
        task: String,

        #[arg(long)]
        model: String,

        #[arg(long, default_value = "1024")]
        n_base: usize,

        #[arg(long, default_value = "mojave-gsa-default-seed-v1")]
        seed: String,

        #[arg(long, short)]
        output: PathBuf,
    },

    /// Run retrospective confseq CI-width stopping analysis on per-cell results.
    Confseq {
        #[arg(long)]
        results: PathBuf,

        #[arg(long, default_value = "0.02")]
        half_width_threshold: f64,

        #[arg(long, default_value = "0.05")]
        alpha: f64,

        #[arg(long, default_value = "1000")]
        n_permutations: usize,

        #[arg(long, default_value = "42")]
        seed: u64,

        #[arg(long, short)]
        output: PathBuf,
    },

    /// Analyze Saltelli results: compute Sobol' + Borgonovo indices with bootstrap CIs.
    Analyze {
        #[arg(long)]
        manifest: PathBuf,

        #[arg(long)]
        results: PathBuf,

        #[arg(long, default_value = "1000")]
        bootstrap_resamples: usize,

        #[arg(long, default_value = "0.95")]
        confidence_level: f64,

        #[arg(long, default_value = "mojave-gsa-default-seed-v1")]
        seed: String,

        #[arg(long, short)]
        output: PathBuf,
    },
}

fn seed_to_bytes(seed: &str) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    let src = seed.as_bytes();
    let len = src.len().min(32);
    bytes[..len].copy_from_slice(&src[..len]);
    bytes
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::GenerateManifest {
            axes_config,
            task,
            model,
            n_base,
            seed,
            output,
        } => {
            let seed_bytes = seed_to_bytes(&seed);
            manifest::generate_manifest(&axes_config, &task, &model, n_base, seed_bytes, &output)?;
        }
        Command::Confseq {
            results,
            half_width_threshold,
            alpha,
            n_permutations,
            seed,
            output,
        } => {
            confseq::confseq_analyze(
                &results,
                half_width_threshold,
                alpha,
                n_permutations,
                seed,
                &output,
            )?;
        }
        Command::Analyze {
            manifest,
            results,
            bootstrap_resamples,
            confidence_level,
            seed,
            output,
        } => {
            let seed_bytes = seed_to_bytes(&seed);
            analyze::analyze(
                &manifest,
                &results,
                bootstrap_resamples,
                confidence_level,
                seed_bytes,
                &output,
            )?;
        }
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn seed_to_bytes_short_string() {
        let bytes = seed_to_bytes("abc");
        assert_eq!(bytes[0], b'a');
        assert_eq!(bytes[1], b'b');
        assert_eq!(bytes[2], b'c');
        assert_eq!(bytes[3], 0);
        assert_eq!(bytes[31], 0);
    }

    #[test]
    fn seed_to_bytes_exact_32() {
        let s = "01234567890123456789012345678901";
        assert_eq!(s.len(), 32);
        let bytes = seed_to_bytes(s);
        assert_eq!(&bytes[..], s.as_bytes());
    }

    #[test]
    fn seed_to_bytes_longer_than_32_truncates() {
        let s = "012345678901234567890123456789012345"; // 34 chars
        let bytes = seed_to_bytes(s);
        assert_eq!(&bytes[..], &s.as_bytes()[..32]);
    }
}
