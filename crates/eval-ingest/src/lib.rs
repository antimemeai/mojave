#![forbid(unsafe_code)]

pub mod id;
pub mod inspect;
pub mod inspect_types;
pub mod types;
pub mod validate;

pub use types::{
    IngestAdapter, IngestError, IngestResult, IngestSource, IngestWarning, SourceMeta, WarningKind,
};
