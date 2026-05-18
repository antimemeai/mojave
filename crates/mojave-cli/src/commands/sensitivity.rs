use crate::error::CliError;

pub fn run_sensitivity(args: &[String]) -> Result<(), CliError> {
    if args.is_empty() {
        return Err(CliError::Usage(sensitivity_help()));
    }

    match args[0].as_str() {
        "sample" | "analyze" | "run" => Err(CliError::Usage(format!(
            "mojave sensitivity {}: salib integration pending — \
             use `salib {}` directly until integration is complete",
            args[0],
            args.join(" ")
        ))),
        "--help" | "-h" | "help" => Err(CliError::Usage(sensitivity_help())),
        other => Err(CliError::Usage(format!(
            "unknown subcommand '{other}'\n{}",
            sensitivity_help()
        ))),
    }
}

fn sensitivity_help() -> String {
    "mojave sensitivity — global sensitivity analysis (salib)\n\
     \n\
     Subcommands:\n\
       sample   Emit a sample matrix from a problem definition\n\
       analyze  Compute sensitivity indices from (X, y) pairs\n\
       run      Drive an end-to-end sensitivity campaign\n\
     \n\
     All subcommands delegate to the published salib crate (v0.1.1).\n\
     For direct usage: cargo install salib-cli"
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_flag_returns_usage_error() {
        let result = run_sensitivity(&["--help".to_string()]);
        assert!(matches!(result, Err(CliError::Usage(_))));
    }

    #[test]
    fn empty_args_returns_usage_error() {
        let result = run_sensitivity(&[]);
        assert!(matches!(result, Err(CliError::Usage(_))));
    }

    #[test]
    fn unknown_subcommand_returns_usage_error() {
        let result = run_sensitivity(&["bogus".to_string()]);
        assert!(matches!(result, Err(CliError::Usage(_))));
    }

    #[test]
    fn known_subcommand_returns_usage_error() {
        let result = run_sensitivity(&["sample".to_string()]);
        assert!(matches!(result, Err(CliError::Usage(_))));
    }
}
