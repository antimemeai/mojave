use std::io::Read;
use std::path::PathBuf;

use eval_core::TrialRecord;

/// Where an ingest adapter reads its bytes from.
pub enum IngestSource {
    /// A single file path.
    File(PathBuf),
    /// A directory; the adapter decides how to walk it.
    Dir(PathBuf),
    /// An in-memory reader (e.g. for tests or streaming).
    Reader(Box<dyn Read + Send>),
}

/// Metadata about the source log that was ingested.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceMeta {
    /// Human-readable name of the runner that produced the log (e.g. `"inspect_ai"`).
    pub runner_name: String,
    /// Version string of that runner, if available.
    pub runner_version: Option<String>,
    /// Version of the log format schema, if available.
    pub log_format_version: Option<String>,
    /// Original file path, if the source was a file or directory.
    pub original_path: Option<PathBuf>,
    /// Lowercase hex SHA-256 of the raw bytes read.
    pub content_hash: String,
}

/// The output produced by a successful ingestion call.
#[derive(Debug)]
pub struct IngestResult {
    /// All `TrialRecord`s extracted from the source.
    pub records: Vec<TrialRecord>,
    /// Non-fatal issues encountered during ingestion.
    pub warnings: Vec<IngestWarning>,
    /// Provenance metadata for the source.
    pub source_meta: SourceMeta,
}

/// A non-fatal issue encountered while ingesting a record.
#[derive(Debug, Clone)]
pub struct IngestWarning {
    /// Zero-based index into the source (e.g. sample index), if applicable.
    pub source_index: Option<usize>,
    /// The source identifier (e.g. sample UUID or run ID), if applicable.
    pub source_id: Option<String>,
    /// What went wrong.
    pub kind: WarningKind,
}

/// Classification of the non-fatal issue.
#[derive(Debug, Clone, PartialEq)]
pub enum WarningKind {
    /// `task_id` field is empty.
    EmptyTaskId,
    /// `agent_id` field is empty.
    EmptyAgentId,
    /// `timestamp` is before 2020-01-01 UTC.
    TimestampTooOld(i64),
    /// `timestamp` is more than 86 400 s in the future.
    TimestampInFuture(i64),
    /// An `Outcome::Score` value is not finite.
    NonFiniteScore(f64),
    /// One of the `Outcome::MultiCriterion` values is not finite.
    NonFiniteCriterion { key: String, value: f64 },
    /// The scorer name was expected but could not be determined.
    UnknownScorer,
    /// A field that should be parseable was not.
    ParseError(String),
}

/// Errors that prevent ingestion from producing any output.
#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("bytes are not valid JSON: {0}")]
    NotJson(#[from] serde_json::Error),

    #[error("JSON is valid but matches no recognised log format")]
    UnrecognizedFormat,

    #[error("no trial records could be produced from the source")]
    NoRecordsProduced,
}

/// Trait implemented by each runner-specific adapter.
pub trait IngestAdapter {
    /// Attempt to ingest `source` and return structured records.
    ///
    /// Implementations MUST:
    /// - Compute a SHA-256 `content_hash` over the raw bytes.
    /// - Validate every produced record via [`crate::validate::validate_record`].
    /// - Collect validation failures as warnings rather than hard errors.
    /// - Return [`IngestError::NoRecordsProduced`] if the final records vec is empty.
    fn ingest(&self, source: IngestSource) -> Result<IngestResult, IngestError>;
}
