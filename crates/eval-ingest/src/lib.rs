#![forbid(unsafe_code)]

pub mod id;
pub mod inspect;
pub mod inspect_types;
pub mod jsonl;
pub mod types;
pub mod validate;

pub use jsonl::{FieldMapping, JsonlAdapter, OutcomeMapping};
pub use types::{
    IngestAdapter, IngestError, IngestResult, IngestSource, IngestWarning, SourceMeta, WarningKind,
};
