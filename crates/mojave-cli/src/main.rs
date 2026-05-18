#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mojave", about = "Measurement engine for AI agent evaluation")]
struct Cli {
    #[arg(long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest eval runner output into normalized TrialRecords
    Ingest {
        #[arg(required = true)]
        paths: Vec<std::path::PathBuf>,
        #[arg(long, default_value = "auto")]
        format: String,
        #[arg(long)]
        field_mapping: Option<std::path::PathBuf>,
    },
    /// Run measurement battery on eval data
    Analyze {
        #[arg(required = true)]
        paths: Vec<std::path::PathBuf>,
        #[arg(long)]
        config: Option<std::path::PathBuf>,
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long)]
        irr_threshold: Option<f64>,
        #[arg(long)]
        irr_metric: Option<String>,
        #[arg(long)]
        spc_chart: Option<String>,
        #[arg(long)]
        spc_phase1_windows: Option<usize>,
        #[arg(long)]
        sequential_alpha: Option<f64>,
        #[arg(long)]
        force_enable: Option<String>,
        #[arg(long)]
        force_disable: Option<String>,
    },
    /// Stream analysis — read records incrementally, emit decisions
    Monitor {
        #[arg(long)]
        watch: Option<std::path::PathBuf>,
        #[arg(long)]
        config: Option<std::path::PathBuf>,
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long)]
        irr_threshold: Option<f64>,
        #[arg(long)]
        irr_metric: Option<String>,
        #[arg(long)]
        spc_chart: Option<String>,
        #[arg(long)]
        spc_phase1_windows: Option<usize>,
        #[arg(long)]
        sequential_alpha: Option<f64>,
    },
    /// Sensitivity analysis (delegates to salib)
    Sensitivity {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();
    }

    match cli.command {
        Commands::Ingest { .. } => {
            eprintln!("mojave ingest: not yet implemented");
            std::process::exit(2)
        }
        Commands::Analyze { .. } => {
            eprintln!("mojave analyze: not yet implemented");
            std::process::exit(2)
        }
        Commands::Monitor { .. } => {
            eprintln!("mojave monitor: not yet implemented");
            std::process::exit(2)
        }
        Commands::Sensitivity { .. } => {
            eprintln!("mojave sensitivity: not yet implemented");
            std::process::exit(2)
        }
    }
}
