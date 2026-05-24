use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::event_kind::EventKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceId(#[serde(with = "hex_16")] pub [u8; 16]);

impl TraceId {
    pub fn generate() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::{SystemTime, UNIX_EPOCH};

        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let count = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id() as u128;

        let mixed = nanos
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(pid)
            .wrapping_add(count as u128);

        let bytes = mixed.to_le_bytes();
        Self(bytes)
    }
}

mod hex_16 {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8; 16], serializer: S) -> Result<S::Ok, S::Error> {
        let hex: String = bytes.iter().fold(String::with_capacity(32), |mut s, b| {
            use std::fmt::Write;
            let _ = write!(s, "{b:02x}");
            s
        });
        serializer.serialize_str(&hex)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<[u8; 16], D::Error> {
        let s = String::deserialize(deserializer)?;
        if s.len() != 32 {
            return Err(serde::de::Error::custom(
                "expected 32 hex chars for TraceId",
            ));
        }
        let mut out = [0u8; 16];
        for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
            let hex = std::str::from_utf8(chunk).map_err(serde::de::Error::custom)?;
            out[i] = u8::from_str_radix(hex, 16).map_err(serde::de::Error::custom)?;
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Authorization {
    Allowed,
    Denied,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Outcome {
    Succeeded,
    Failed { error: String },
    Observed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobRef {
    #[serde(with = "hex_32")]
    pub hash: [u8; 32],
    pub location: BlobLocation,
    pub size_bytes: u64,
    pub content_type: String,
}

mod hex_32 {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error> {
        let hex: String = bytes.iter().fold(String::with_capacity(64), |mut s, b| {
            use std::fmt::Write;
            let _ = write!(s, "{b:02x}");
            s
        });
        serializer.serialize_str(&hex)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<[u8; 32], D::Error> {
        let s = String::deserialize(deserializer)?;
        if s.len() != 64 {
            return Err(serde::de::Error::custom("expected 64 hex chars"));
        }
        let mut out = [0u8; 32];
        for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
            let hex = std::str::from_utf8(chunk).map_err(serde::de::Error::custom)?;
            out[i] = u8::from_str_radix(hex, 16).map_err(serde::de::Error::custom)?;
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlobLocation {
    File { path: PathBuf },
}

pub type Tags = BTreeMap<String, String>;
pub type Detail = serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principal {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRef {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub envelope_version: u32,
    pub at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monotonic_ns: Option<u64>,
    pub actor: Principal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<TraceId>,
    pub event: EventKind,
    pub resource: ResourceRef,
    pub authorization: Authorization,
    pub outcome: Outcome,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tags: Tags,
    #[serde(default = "default_detail", skip_serializing_if = "is_null")]
    pub detail: Detail,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_ref: Option<BlobRef>,
}

fn default_detail() -> Detail {
    serde_json::Value::Null
}

fn is_null(v: &Detail) -> bool {
    v.is_null()
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ValidationError {
    #[error("too many tags: {count} (max {max})")]
    TooManyTags { count: usize, max: usize },
    #[error("tag key too long: {key} ({len} bytes, max {max})")]
    TagKeyTooLong { key: String, len: usize, max: usize },
    #[error("tag value too long for key {key}: {len} bytes (max {max})")]
    TagValueTooLong { key: String, len: usize, max: usize },
    #[error("non-ASCII tag key: {key}")]
    NonAsciiTagKey { key: String },
}

pub fn validate_tags(
    tags: &Tags,
    max_pairs: usize,
    max_value_bytes: usize,
) -> Result<(), ValidationError> {
    if tags.len() > max_pairs {
        return Err(ValidationError::TooManyTags {
            count: tags.len(),
            max: max_pairs,
        });
    }
    for (key, value) in tags {
        if !key.is_ascii() {
            return Err(ValidationError::NonAsciiTagKey { key: key.clone() });
        }
        if value.len() > max_value_bytes {
            return Err(ValidationError::TagValueTooLong {
                key: key.clone(),
                len: value.len(),
                max: max_value_bytes,
            });
        }
    }
    Ok(())
}
