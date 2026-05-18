use crate::error::CliError;

pub fn run_sensitivity(args: &[String]) -> Result<(), CliError> {
    if args.is_empty() {
        print_sensitivity_help();
        return Ok(());
    }

    match args[0].as_str() {
        "sample" | "analyze" | "run" => {
            eprintln!(
                "mojave sensitivity {}: salib integration pending — \
                 use `salib {}` directly until integration is complete",
                args[0],
                args.join(" ")
            );
            std::process::exit(2);
        }
        "--help" | "-h" | "help" => {
            print_sensitivity_help();
            Ok(())
        }
        other => {
            eprintln!("mojave sensitivity: unknown subcommand '{other}'");
            print_sensitivity_help();
            std::process::exit(2);
        }
    }
}

fn print_sensitivity_help() {
    eprintln!(
        "mojave sensitivity — global sensitivity analysis (salib)\n\
         \n\
         Subcommands:\n\
           sample   Emit a sample matrix from a problem definition\n\
           analyze  Compute sensitivity indices from (X, y) pairs\n\
           run      Drive an end-to-end sensitivity campaign\n\
         \n\
         All subcommands delegate to the published salib crate (v0.1.1).\n\
         For direct usage: cargo install salib-cli"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_flag_does_not_error() {
        let result = run_sensitivity(&["--help".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn empty_args_prints_help() {
        let result = run_sensitivity(&[]);
        assert!(result.is_ok());
    }
}
