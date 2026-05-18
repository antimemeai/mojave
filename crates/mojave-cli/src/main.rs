#![forbid(unsafe_code)]

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use mojave_cli::commands::{analyze, ingest};
use mojave_cli::config::ConfigOverrides;
use mojave_cli::output::{write_error, write_json};

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
    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for: bash, zsh, fish, powershell, elvish
        #[arg(value_enum)]
        shell: Shell,
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

    let result = match cli.command {
        Commands::Ingest {
            paths,
            format,
            field_mapping,
        } => {
            let output = ingest::run_ingest(&paths, &format, field_mapping.as_deref());
            match output {
                Ok(out) => write_json(&out),
                Err(e) => {
                    write_error(&e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Analyze {
            paths,
            config,
            format: _,
            irr_threshold,
            irr_metric,
            spc_chart,
            spc_phase1_windows,
            sequential_alpha,
            force_enable,
            force_disable,
        } => {
            let overrides = ConfigOverrides {
                irr_threshold,
                irr_metric,
                spc_chart,
                spc_phase1_windows,
                sequential_alpha,
                force_enable,
                force_disable,
            };
            let output = analyze::run_analyze(&paths, config.as_deref(), &overrides);
            match output {
                Ok(out) => write_json(&out),
                Err(e) => {
                    write_error(&e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Monitor { .. } => {
            eprintln!("mojave monitor: not yet implemented");
            std::process::exit(2);
        }
        Commands::Sensitivity { .. } => {
            eprintln!("mojave sensitivity: not yet implemented");
            std::process::exit(2);
        }
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "mojave", &mut std::io::stdout());
            Ok(())
        }
    };

    if let Err(e) = result {
        write_error(&e);
        std::process::exit(1);
    }
}
