use std::fmt;

#[derive(Debug)]
pub enum CliError {
    Ingest(eval_ingest::IngestError),
    Orchestrator(eval_orchestrator::OrchestratorError),
    Config(ConfigError),
    Io(std::io::Error),
    Usage(String),
}

#[derive(Debug)]
pub enum ConfigError {
    FileReadError(std::io::Error),
    ParseError(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Ingest(e) => write!(f, "{e}"),
            CliError::Orchestrator(e) => write!(f, "{e}"),
            CliError::Config(e) => write!(f, "{e}"),
            CliError::Io(e) => write!(f, "{e}"),
            CliError::Usage(msg) => write!(f, "{msg}"),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::FileReadError(e) => write!(f, "config file read error: {e}"),
            ConfigError::ParseError(msg) => write!(f, "config parse error: {msg}"),
        }
    }
}

impl std::error::Error for CliError {}
impl std::error::Error for ConfigError {}

impl From<eval_ingest::IngestError> for CliError {
    fn from(e: eval_ingest::IngestError) -> Self {
        CliError::Ingest(e)
    }
}

impl From<eval_orchestrator::OrchestratorError> for CliError {
    fn from(e: eval_orchestrator::OrchestratorError) -> Self {
        CliError::Orchestrator(e)
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Io(e)
    }
}

impl From<ConfigError> for CliError {
    fn from(e: ConfigError) -> Self {
        CliError::Config(e)
    }
}

impl CliError {
    pub fn kind(&self) -> &'static str {
        match self {
            CliError::Ingest(_) => "ingest_error",
            CliError::Orchestrator(_) => "orchestrator_error",
            CliError::Config(_) => "config_error",
            CliError::Io(_) => "io_error",
            CliError::Usage(_) => "usage_error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_error_kind_strings() {
        let io_err = CliError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));
        assert_eq!(io_err.kind(), "io_error");

        let config_err = CliError::Config(ConfigError::ParseError("bad yaml".into()));
        assert_eq!(config_err.kind(), "config_error");
    }

    #[test]
    fn cli_error_display() {
        let config_err = CliError::Config(ConfigError::ParseError("bad yaml".into()));
        let msg = format!("{config_err}");
        assert!(msg.contains("bad yaml"));
    }
}
