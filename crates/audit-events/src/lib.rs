#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod event_kind;
pub mod types;

pub use event_kind::EventKind;
pub use types::{
    validate_tags, AuditEvent, Authorization, BlobLocation, BlobRef, Detail, Outcome, Principal,
    ResourceRef, Tags, TraceId, ValidationError,
};
