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
    /// Audit chain management — seal entries and verify chains
    Audit {
        #[command(subcommand)]
        action: AuditAction,
    },
}

#[derive(Subcommand)]
enum AuditAction {
    /// Seal a new audit entry from pipeline data (reads JSON from stdin)
    Seal {
        #[arg(long)]
        key_file: Option<std::path::PathBuf>,
    },
    /// Verify an existing audit chain
    Verify {
        #[arg(long)]
        chain: Option<std::path::PathBuf>,
    },
    /// Emit an audit event (reads JSON from stdin)
    Emit {
        #[arg(long)]
        blob_file: Option<std::path::PathBuf>,
        #[arg(long)]
        audit_dir: Option<std::path::PathBuf>,
    },
    /// Garbage-collect orphan blobs
    Gc {
        #[arg(long)]
        audit_dir: Option<std::path::PathBuf>,
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
        Commands::Monitor {
            watch,
            config,
            irr_threshold,
            irr_metric,
            spc_chart,
            spc_phase1_windows,
            sequential_alpha,
        } => {
            let overrides = ConfigOverrides {
                irr_threshold,
                irr_metric,
                spc_chart,
                spc_phase1_windows,
                sequential_alpha,
                force_enable: None,
                force_disable: None,
            };
            let result = match watch {
                Some(path) => mojave_cli::commands::monitor::run_monitor_watch(
                    &path,
                    config.as_deref(),
                    &overrides,
                ),
                None => {
                    mojave_cli::commands::monitor::run_monitor_stdin(config.as_deref(), &overrides)
                }
            };
            match result {
                Ok(()) => Ok(()),
                Err(e) => {
                    write_error(&e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Sensitivity { args } => {
            match mojave_cli::commands::sensitivity::run_sensitivity(&args) {
                Ok(()) => Ok(()),
                Err(e) => {
                    write_error(&e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "mojave", &mut std::io::stdout());
            Ok(())
        }
        Commands::Audit { action } => match action {
            AuditAction::Seal { key_file } => {
                match mojave_cli::commands::audit::run_seal(key_file.as_deref()) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        write_error(&e);
                        std::process::exit(1);
                    }
                }
            }
            AuditAction::Verify { chain } => {
                match mojave_cli::commands::audit::run_verify(chain.as_deref()) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        write_error(&e);
                        std::process::exit(1);
                    }
                }
            }
            AuditAction::Emit {
                blob_file,
                audit_dir,
            } => {
                match mojave_cli::commands::audit::run_emit(
                    blob_file.as_deref(),
                    audit_dir.as_deref(),
                ) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        write_error(&e);
                        std::process::exit(1);
                    }
                }
            }
            AuditAction::Gc { audit_dir } => {
                match mojave_cli::commands::audit::run_gc(audit_dir.as_deref()) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        write_error(&e);
                        std::process::exit(1);
                    }
                }
            }
        },
    };

    if let Err(e) = result {
        write_error(&e);
        let code = if matches!(e, mojave_cli::error::CliError::Usage(_)) {
            2
        } else {
            1
        };
        std::process::exit(code);
    }
}
