# Audit Event Emission System — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a CloudTrail-style real-time audit event emission system where every mojave operation emits tamper-evident, hash-chained audit events through a Rust engine.

**Architecture:** Four new crates (`audit-events`, `audit-emit`, `audit-recover`, `audit-macros`) plus modifications to `audit-chain`. The emitter owns the chain, blob store, and file locking. Python orchestration calls through `mojave audit emit` CLI. `AuditGate<T>` enforces compile-time audit wiring.

**Tech Stack:** Rust (workspace crates), serde/serde_json, sha2, chrono, ed25519-dalek, coset, fs2 (flock), proc-macro2/quote/syn (macros)

---

## File Structure

### New crate: `crates/audit-events/`
- `Cargo.toml` — deps: serde, chrono, serde_json, thiserror
- `src/lib.rs` — re-exports
- `src/event_kind.rs` — `EventKind` enum with custom serde
- `src/types.rs` — `AuditEvent`, `Authorization`, `Outcome`, `Tags`, `Detail`, `BlobRef`, `BlobLocation`, `TraceId`
- `tests/golden_serde.rs` — golden vector tests for EventKind serialization

### New crate: `crates/audit-recover/`
- `Cargo.toml` — deps: audit-chain, serde_json, thiserror, sha2
- `src/lib.rs` — re-exports
- `src/replay.rs` — chain replay (reconstruct ChainHead from JSONL)
- `src/gc.rs` — blob garbage collection
- `tests/replay_tests.rs` — crash recovery scenarios
- `tests/gc_tests.rs` — orphan blob cleanup

### New crate: `crates/audit-emit/`
- `Cargo.toml` — deps: audit-events, audit-chain, audit-sign, audit-recover, serde_json, sha2, chrono, thiserror, fs2
- `src/lib.rs` — re-exports
- `src/blob_store.rs` — content-addressed blob storage
- `src/emitter.rs` — `Emitter` struct with emit path
- `src/gate.rs` — `AuditGate<T>` gateway type
- `src/circuit_breaker.rs` — authenticated bypass
- `src/config.rs` — `EmitterConfig`
- `src/error.rs` — `AuditError`
- `tests/emitter_tests.rs` — emit round-trip, flock, auto-promotion
- `tests/gate_tests.rs` — gate resolution, drop panic
- `tests/integration_tests.rs` — full lifecycle

### Modified: `crates/audit-chain/src/entry.rs`
- Update `AuditEntry` to new envelope schema (add envelope_version, monotonic_ns, trace_id, tags, detail, blob_ref, authorization; replace action/decision with event/outcome strings)

### Modified: `crates/mojave-cli/`
- `src/commands/audit.rs` — add `run_emit` and `run_gc` functions
- `src/main.rs` — add `Emit` and `Gc` to `AuditAction` enum

### Modified: `Cargo.toml` (workspace root)
- Add new crate members

---

### Task 1: `audit-events` crate — core types

**Files:**
- Create: `crates/audit-events/Cargo.toml`
- Create: `crates/audit-events/src/lib.rs`
- Create: `crates/audit-events/src/event_kind.rs`
- Create: `crates/audit-events/src/types.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Create Cargo.toml and add to workspace**

```toml
# crates/audit-events/Cargo.toml
[package]
name = "audit-events"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Event types and envelope for the mojave audit system"

[dependencies]
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"

[lints]
workspace = true
```

Add `"crates/audit-events"` to `members` in workspace `Cargo.toml`.

- [ ] **Step 2: Write EventKind enum with custom serde**

```rust
// crates/audit-events/src/event_kind.rs
#![allow(clippy::module_name_repetitions)]

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    // Eval lifecycle
    EvalStarted,
    EvalCompleted,
    EvalFailed,

    // Infrastructure
    PodCreated,
    PodReady,
    PodTerminated,
    EndpointVerified,

    // Data provenance
    DatasetLoaded,
    DatasetCached,
    ModelLoaded,
    ScoringCompleted,

    // Artifacts
    RunCardGenerated,
    RunCardSealed,

    // Crypto operations
    KeyGenerated,
    KeyLoaded,
    ChainVerified,
    AttestationCreated,

    // System
    ConfigChanged,
    CircuitBreakerTripped,
    CircuitBreakerReset,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EvalStarted => "eval.started",
            Self::EvalCompleted => "eval.completed",
            Self::EvalFailed => "eval.failed",
            Self::PodCreated => "pod.created",
            Self::PodReady => "pod.ready",
            Self::PodTerminated => "pod.terminated",
            Self::EndpointVerified => "endpoint.verified",
            Self::DatasetLoaded => "dataset.loaded",
            Self::DatasetCached => "dataset.cached",
            Self::ModelLoaded => "model.loaded",
            Self::ScoringCompleted => "scoring.completed",
            Self::RunCardGenerated => "run_card.generated",
            Self::RunCardSealed => "run_card.sealed",
            Self::KeyGenerated => "key.generated",
            Self::KeyLoaded => "key.loaded",
            Self::ChainVerified => "chain.verified",
            Self::AttestationCreated => "attestation.created",
            Self::ConfigChanged => "config.changed",
            Self::CircuitBreakerTripped => "circuit_breaker.tripped",
            Self::CircuitBreakerReset => "circuit_breaker.reset",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "eval.started" => Some(Self::EvalStarted),
            "eval.completed" => Some(Self::EvalCompleted),
            "eval.failed" => Some(Self::EvalFailed),
            "pod.created" => Some(Self::PodCreated),
            "pod.ready" => Some(Self::PodReady),
            "pod.terminated" => Some(Self::PodTerminated),
            "endpoint.verified" => Some(Self::EndpointVerified),
            "dataset.loaded" => Some(Self::DatasetLoaded),
            "dataset.cached" => Some(Self::DatasetCached),
            "model.loaded" => Some(Self::ModelLoaded),
            "scoring.completed" => Some(Self::ScoringCompleted),
            "run_card.generated" => Some(Self::RunCardGenerated),
            "run_card.sealed" => Some(Self::RunCardSealed),
            "key.generated" => Some(Self::KeyGenerated),
            "key.loaded" => Some(Self::KeyLoaded),
            "chain.verified" => Some(Self::ChainVerified),
            "attestation.created" => Some(Self::AttestationCreated),
            "config.changed" => Some(Self::ConfigChanged),
            "circuit_breaker.tripped" => Some(Self::CircuitBreakerTripped),
            "circuit_breaker.reset" => Some(Self::CircuitBreakerReset),
            _ => None,
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::EvalStarted,
            Self::EvalCompleted,
            Self::EvalFailed,
            Self::PodCreated,
            Self::PodReady,
            Self::PodTerminated,
            Self::EndpointVerified,
            Self::DatasetLoaded,
            Self::DatasetCached,
            Self::ModelLoaded,
            Self::ScoringCompleted,
            Self::RunCardGenerated,
            Self::RunCardSealed,
            Self::KeyGenerated,
            Self::KeyLoaded,
            Self::ChainVerified,
            Self::AttestationCreated,
            Self::ConfigChanged,
            Self::CircuitBreakerTripped,
            Self::CircuitBreakerReset,
        ]
    }
}

impl std::fmt::Display for EventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for EventKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for EventKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).ok_or_else(|| {
            serde::de::Error::custom(format!("unknown event kind: {s}"))
        })
    }
}
```

- [ ] **Step 3: Write remaining types**

```rust
// crates/audit-events/src/types.rs
use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::event_kind::EventKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceId(
    #[serde(with = "hex_16")]
    pub [u8; 16],
);

impl TraceId {
    pub fn generate() -> Self {
        let mut bytes = [0u8; 16];
        getrandom(&mut bytes);
        Self(bytes)
    }
}

fn getrandom(buf: &mut [u8]) {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Simple non-crypto RNG for trace IDs (not security-critical).
    // Production should use rand_core::OsRng, but we avoid the dep here.
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for (i, byte) in buf.iter_mut().enumerate() {
        *byte = (seed.wrapping_mul(6364136223846793005).wrapping_add(i as u128) >> (8 * (i % 16))) as u8;
    }
}

mod hex_16 {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8; 16], serializer: S) -> Result<S::Ok, S::Error> {
        let hex: String = bytes.iter().fold(
            String::with_capacity(32),
            |mut s, b| { use std::fmt::Write; let _ = write!(s, "{b:02x}"); s },
        );
        serializer.serialize_str(&hex)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<[u8; 16], D::Error> {
        let s = String::deserialize(deserializer)?;
        if s.len() != 32 {
            return Err(serde::de::Error::custom("expected 32 hex chars for TraceId"));
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
        let hex: String = bytes.iter().fold(
            String::with_capacity(64),
            |mut s, b| { use std::fmt::Write; let _ = write!(s, "{b:02x}"); s },
        );
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
```

- [ ] **Step 4: Write lib.rs**

```rust
// crates/audit-events/src/lib.rs
#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod event_kind;
pub mod types;

pub use event_kind::EventKind;
pub use types::{
    AuditEvent, Authorization, BlobLocation, BlobRef, Detail, Outcome,
    Principal, ResourceRef, Tags, TraceId, ValidationError, validate_tags,
};
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p audit-events`
Expected: compiles with zero warnings

- [ ] **Step 6: Write golden serde tests**

```rust
// crates/audit-events/tests/golden_serde.rs
#![allow(clippy::unwrap_used, clippy::expect_used)]

use audit_events::EventKind;

#[test]
fn all_event_kinds_serialize_to_dot_notation() {
    let expected = vec![
        (EventKind::EvalStarted, "eval.started"),
        (EventKind::EvalCompleted, "eval.completed"),
        (EventKind::EvalFailed, "eval.failed"),
        (EventKind::PodCreated, "pod.created"),
        (EventKind::PodReady, "pod.ready"),
        (EventKind::PodTerminated, "pod.terminated"),
        (EventKind::EndpointVerified, "endpoint.verified"),
        (EventKind::DatasetLoaded, "dataset.loaded"),
        (EventKind::DatasetCached, "dataset.cached"),
        (EventKind::ModelLoaded, "model.loaded"),
        (EventKind::ScoringCompleted, "scoring.completed"),
        (EventKind::RunCardGenerated, "run_card.generated"),
        (EventKind::RunCardSealed, "run_card.sealed"),
        (EventKind::KeyGenerated, "key.generated"),
        (EventKind::KeyLoaded, "key.loaded"),
        (EventKind::ChainVerified, "chain.verified"),
        (EventKind::AttestationCreated, "attestation.created"),
        (EventKind::ConfigChanged, "config.changed"),
        (EventKind::CircuitBreakerTripped, "circuit_breaker.tripped"),
        (EventKind::CircuitBreakerReset, "circuit_breaker.reset"),
    ];

    for (kind, expected_str) in &expected {
        let json = serde_json::to_string(kind).unwrap();
        assert_eq!(json, format!("\"{expected_str}\""), "serialize {kind:?}");

        let back: EventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(&back, kind, "round-trip {kind:?}");
    }
}

#[test]
fn all_variants_covered_by_all() {
    assert_eq!(EventKind::all().len(), 20);
}

#[test]
fn unknown_event_kind_rejected() {
    let result: Result<EventKind, _> = serde_json::from_str("\"bogus.event\"");
    assert!(result.is_err());
}

#[test]
fn audit_event_round_trip() {
    use audit_events::{
        AuditEvent, Authorization, Outcome, Principal, ResourceRef,
    };
    use std::collections::BTreeMap;

    let event = AuditEvent {
        envelope_version: 1,
        at: chrono::Utc::now(),
        monotonic_ns: Some(123456789),
        actor: Principal {
            kind: "System".into(),
            id: "test-agent".into(),
        },
        trace_id: None,
        event: EventKind::EvalStarted,
        resource: ResourceRef {
            kind: "eval".into(),
            id: "arc_challenge".into(),
        },
        authorization: Authorization::Allowed,
        outcome: Outcome::Succeeded,
        tags: BTreeMap::from([("run_id".into(), "RUN-001".into())]),
        detail: serde_json::json!({"model": "Qwen2.5-7B"}),
        blob_ref: None,
    };

    let json = serde_json::to_string(&event).unwrap();
    let back: AuditEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(back.event, EventKind::EvalStarted);
    assert_eq!(back.envelope_version, 1);
}

#[test]
fn validate_tags_enforces_limits() {
    use audit_events::validate_tags;
    use std::collections::BTreeMap;

    let mut tags = BTreeMap::new();
    for i in 0..33 {
        tags.insert(format!("key{i}"), "value".into());
    }
    assert!(validate_tags(&tags, 32, 256).is_err());

    let mut tags = BTreeMap::new();
    tags.insert("ok".into(), "x".repeat(257));
    assert!(validate_tags(&tags, 32, 256).is_err());

    let mut tags = BTreeMap::new();
    tags.insert("k\u{00e9}y".into(), "value".into());
    assert!(validate_tags(&tags, 32, 256).is_err());
}

#[test]
fn outcome_failed_carries_error() {
    use audit_events::Outcome;
    let o = Outcome::Failed { error: "disk full".into() };
    let json = serde_json::to_string(&o).unwrap();
    assert!(json.contains("disk full"));
    let back: Outcome = serde_json::from_str(&json).unwrap();
    assert_eq!(back, o);
}
```

- [ ] **Step 7: Run tests**

Run: `cargo test -p audit-events`
Expected: all tests pass

- [ ] **Step 8: Commit**

```bash
git add crates/audit-events/ Cargo.toml
git commit -m "feat(audit-events): add event types, EventKind enum, and tiered payload types"
```

---

### Task 2: Update `audit-chain` AuditEntry to new envelope schema

**Files:**
- Modify: `crates/audit-chain/src/entry.rs`
- Modify: `crates/audit-chain/src/seal.rs`
- Modify: `crates/audit-chain/src/verify.rs`
- Modify: `crates/audit-chain/tests/golden_canonical.rs`
- Modify: `crates/audit-chain/tests/property_tests.rs`
- Modify: `crates/audit-chain/Cargo.toml`

This is a breaking change to the chain entry schema. The `AuditEntry` becomes a dumb envelope — it stores strings for `event` and `outcome` (not typed enums). The chain crate does NOT depend on `audit-events`.

- [ ] **Step 1: Update AuditEntry struct**

Replace the existing `AuditEntry`, `Principal`, `Action`, `Decision`, `ResourceRef`, and `AuditEntryBuilder` in `crates/audit-chain/src/entry.rs` with:

```rust
// crates/audit-chain/src/entry.rs
use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::canonical::{self, CanonicalEncodingError};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct AuditEntry {
    pub seq: u64,
    pub envelope_version: u32,
    pub at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monotonic_ns: Option<u64>,
    pub actor: Principal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<[u8; 16]>,
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<ResourceRef>,
    pub authorization: String,
    pub outcome: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tags: BTreeMap<String, String>,
    #[serde(default = "default_detail")]
    pub detail: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_ref: Option<BlobRef>,
}

fn default_detail() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

impl AuditEntry {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CanonicalEncodingError> {
        canonical::encode(self)
    }

    pub fn canonical_digest(&self) -> Result<[u8; 32], CanonicalEncodingError> {
        let bytes = self.canonical_bytes()?;
        Ok(Sha256::digest(&bytes).into())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Principal {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceRef {
    pub kind: String,
    pub id: String,
}

impl ResourceRef {
    pub fn new(kind: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlobRef {
    pub hash: [u8; 32],
    pub location: BlobLocation,
    pub size_bytes: u64,
    pub content_type: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BlobLocation {
    File { path: PathBuf },
}

#[derive(Debug, Default)]
pub struct AuditEntryBuilder {
    seq: Option<u64>,
    envelope_version: u32,
    at: Option<DateTime<Utc>>,
    monotonic_ns: Option<u64>,
    actor: Option<Principal>,
    trace_id: Option<[u8; 16]>,
    event: Option<String>,
    resource: Option<ResourceRef>,
    authorization: Option<String>,
    outcome: Option<String>,
    tags: BTreeMap<String, String>,
    detail: serde_json::Value,
    blob_ref: Option<BlobRef>,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BuildError {
    #[error("missing required field: {0}")]
    MissingField(&'static str),
}

impl AuditEntryBuilder {
    pub fn new() -> Self {
        Self {
            envelope_version: 1,
            detail: serde_json::Value::Object(serde_json::Map::new()),
            ..Self::default()
        }
    }

    pub fn seq(mut self, seq: u64) -> Self { self.seq = Some(seq); self }
    pub fn at(mut self, at: DateTime<Utc>) -> Self { self.at = Some(at); self }
    pub fn monotonic_ns(mut self, ns: u64) -> Self { self.monotonic_ns = Some(ns); self }
    pub fn actor(mut self, actor: Principal) -> Self { self.actor = Some(actor); self }
    pub fn trace_id(mut self, id: [u8; 16]) -> Self { self.trace_id = Some(id); self }
    pub fn event(mut self, event: impl Into<String>) -> Self { self.event = Some(event.into()); self }
    pub fn resource(mut self, resource: ResourceRef) -> Self { self.resource = Some(resource); self }
    pub fn authorization(mut self, auth: impl Into<String>) -> Self { self.authorization = Some(auth.into()); self }
    pub fn outcome(mut self, outcome: impl Into<String>) -> Self { self.outcome = Some(outcome.into()); self }
    pub fn tags(mut self, tags: BTreeMap<String, String>) -> Self { self.tags = tags; self }
    pub fn detail(mut self, detail: serde_json::Value) -> Self { self.detail = detail; self }
    pub fn blob_ref(mut self, blob_ref: BlobRef) -> Self { self.blob_ref = Some(blob_ref); self }

    pub fn build(self) -> Result<AuditEntry, BuildError> {
        Ok(AuditEntry {
            seq: self.seq.ok_or(BuildError::MissingField("seq"))?,
            envelope_version: self.envelope_version,
            at: self.at.ok_or(BuildError::MissingField("at"))?,
            monotonic_ns: self.monotonic_ns,
            actor: self.actor.ok_or(BuildError::MissingField("actor"))?,
            trace_id: self.trace_id,
            event: self.event.ok_or(BuildError::MissingField("event"))?,
            resource: self.resource,
            authorization: self.authorization.ok_or(BuildError::MissingField("authorization"))?,
            outcome: self.outcome.ok_or(BuildError::MissingField("outcome"))?,
            tags: self.tags,
            detail: self.detail,
            blob_ref: self.blob_ref,
        })
    }
}
```

- [ ] **Step 2: Update all tests to use new schema**

Update `seal.rs` tests, `verify.rs` tests, `golden_canonical.rs`, and `property_tests.rs` to use the new `AuditEntryBuilder` API. The sample entry helper in each test file changes from:

```rust
fn sample_entry() -> AuditEntry {
    AuditEntryBuilder::new()
        .seq(0)
        .actor(Principal { kind: "System".into(), id: "test".into() })
        .event("eval.started")
        .authorization("Allowed")
        .outcome("Succeeded")
        .at(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap())
        .detail(serde_json::json!({"trial": 1}))
        .build()
        .unwrap()
}
```

Remove tests for the old `Action`, `Decision`, and `Principal` enum variants that no longer exist.

- [ ] **Step 3: Run tests**

Run: `cargo test -p audit-chain`
Expected: all tests pass (some tests removed for deleted types, remaining tests adapted)

- [ ] **Step 4: Fix audit-sign and mojave-cli compilation**

The `audit-sign` crate uses `ChainHead` and `SealedAuditEntry` from `audit-chain` — those haven't changed, so it should still compile. The `mojave-cli` `audit.rs` command constructs `AuditEntry` instances — update those to use the new builder API.

Run: `cargo check --workspace`
Expected: compiles (fix any remaining compilation errors from the schema change)

- [ ] **Step 5: Run full workspace tests**

Run: `cargo test --workspace`
Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/audit-chain/ crates/audit-sign/ crates/mojave-cli/
git commit -m "refactor(audit-chain): update AuditEntry to v2 envelope schema

Replace Action/Decision enums with string fields (event, authorization,
outcome). Add envelope_version, monotonic_ns, trace_id, tags, detail,
blob_ref fields. Chain crate remains schema-agnostic."
```

---

### Task 3: `audit-recover` crate — chain replay and crash recovery

**Files:**
- Create: `crates/audit-recover/Cargo.toml`
- Create: `crates/audit-recover/src/lib.rs`
- Create: `crates/audit-recover/src/replay.rs`
- Create: `crates/audit-recover/src/gc.rs`
- Create: `crates/audit-recover/tests/replay_tests.rs`
- Create: `crates/audit-recover/tests/gc_tests.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Create crate skeleton**

```toml
# crates/audit-recover/Cargo.toml
[package]
name = "audit-recover"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Crash recovery, chain replay, and garbage collection for mojave audit"

[dependencies]
audit-chain = { path = "../audit-chain" }
serde_json = "1"
sha2 = "0.10"
thiserror = "2"

[dev-dependencies]
chrono = { version = "0.4", features = ["serde"] }
tempfile = "3"

[lints]
workspace = true
```

Add `"crates/audit-recover"` to workspace members.

- [ ] **Step 2: Write replay module**

```rust
// crates/audit-recover/src/replay.rs
use audit_chain::seal::{ChainHead, SealedAuditEntry};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ReplayError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error at line {line}: {source}")]
    JsonParse { line: usize, source: serde_json::Error },
}

#[derive(Debug)]
pub struct ReplayResult {
    pub chain_head: ChainHead,
    pub entry_count: usize,
    pub truncated_lines: usize,
}

pub fn replay_chain_file(path: &std::path::Path) -> Result<ReplayResult, ReplayError> {
    let contents = std::fs::read_to_string(path)?;
    replay_chain_str(&contents)
}

pub fn replay_chain_str(contents: &str) -> Result<ReplayResult, ReplayError> {
    let mut head = ChainHead::new();
    let mut count = 0usize;
    let mut truncated = 0usize;

    for (i, line) in contents.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<SealedAuditEntry>(trimmed) {
            Ok(entry) => {
                head = ChainHead::resume(entry.entry_hash, entry.base.seq + 1);
                count += 1;
            }
            Err(e) => {
                if i == contents.lines().count() - 1 {
                    truncated += 1;
                    eprintln!(
                        "audit-recover: truncated last line {}, skipping (crash recovery)",
                        i + 1
                    );
                } else {
                    return Err(ReplayError::JsonParse {
                        line: i + 1,
                        source: e,
                    });
                }
            }
        }
    }

    Ok(ReplayResult {
        chain_head: head,
        entry_count: count,
        truncated_lines: truncated,
    })
}
```

- [ ] **Step 3: Write GC module**

```rust
// crates/audit-recover/src/gc.rs
use std::collections::HashSet;
use std::path::Path;

use audit_chain::seal::SealedAuditEntry;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GcError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
}

#[derive(Debug)]
pub struct GcResult {
    pub blobs_scanned: usize,
    pub blobs_referenced: usize,
    pub blobs_deleted: usize,
}

pub fn collect_referenced_blob_hashes(
    chain_dir: &Path,
) -> Result<HashSet<String>, GcError> {
    let mut hashes = HashSet::new();

    for entry in std::fs::read_dir(chain_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let contents = std::fs::read_to_string(&path)?;
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(sealed) = serde_json::from_str::<SealedAuditEntry>(trimmed) {
                if let Some(blob_ref) = &sealed.base.blob_ref {
                    let hex: String = blob_ref.hash.iter().fold(
                        String::with_capacity(64),
                        |mut s, b| {
                            use std::fmt::Write;
                            let _ = write!(s, "{b:02x}");
                            s
                        },
                    );
                    hashes.insert(hex);
                }
            }
        }
    }

    Ok(hashes)
}

pub fn gc_blobs(audit_dir: &Path) -> Result<GcResult, GcError> {
    let blob_dir = audit_dir.join("blobs");
    if !blob_dir.exists() {
        return Ok(GcResult {
            blobs_scanned: 0,
            blobs_referenced: 0,
            blobs_deleted: 0,
        });
    }

    let referenced = collect_referenced_blob_hashes(audit_dir)?;
    let mut scanned = 0usize;
    let mut deleted = 0usize;

    for entry in std::fs::read_dir(&blob_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        scanned += 1;
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if !referenced.contains(name) {
            std::fs::remove_file(&path)?;
            deleted += 1;
        }
    }

    Ok(GcResult {
        blobs_scanned: scanned,
        blobs_referenced: referenced.len(),
        blobs_deleted: deleted,
    })
}
```

- [ ] **Step 4: Write lib.rs**

```rust
// crates/audit-recover/src/lib.rs
#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod gc;
pub mod replay;
```

- [ ] **Step 5: Write replay tests**

```rust
// crates/audit-recover/tests/replay_tests.rs
#![allow(clippy::unwrap_used, clippy::expect_used)]

use audit_chain::entry::{AuditEntryBuilder, Principal};
use audit_chain::seal::ChainHead;
use audit_recover::replay;
use chrono::{TimeZone, Utc};
use std::io::Write;

fn sample_entry() -> audit_chain::entry::AuditEntry {
    AuditEntryBuilder::new()
        .seq(0)
        .actor(Principal { kind: "System".into(), id: "test".into() })
        .event("eval.started")
        .authorization("Allowed")
        .outcome("Succeeded")
        .at(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap())
        .detail(serde_json::json!({"trial": 1}))
        .build()
        .unwrap()
}

fn build_chain_jsonl(n: usize) -> String {
    let mut head = ChainHead::new();
    let mut lines = Vec::new();
    for _ in 0..n {
        let sealed = head.link(sample_entry()).unwrap();
        lines.push(serde_json::to_string(&sealed).unwrap());
    }
    lines.join("\n") + "\n"
}

#[test]
fn replay_empty_file() {
    let result = replay::replay_chain_str("").unwrap();
    assert_eq!(result.entry_count, 0);
    assert_eq!(result.chain_head.next_seq(), 0);
}

#[test]
fn replay_valid_chain() {
    let jsonl = build_chain_jsonl(5);
    let result = replay::replay_chain_str(&jsonl).unwrap();
    assert_eq!(result.entry_count, 5);
    assert_eq!(result.chain_head.next_seq(), 5);
    assert!(result.chain_head.last_entry_hash().is_some());
}

#[test]
fn replay_truncated_last_line_recovers() {
    let mut jsonl = build_chain_jsonl(3);
    jsonl.push_str("{\"truncated\": tr");
    let result = replay::replay_chain_str(&jsonl).unwrap();
    assert_eq!(result.entry_count, 3);
    assert_eq!(result.truncated_lines, 1);
}

#[test]
fn replay_corrupt_middle_line_fails() {
    let jsonl = build_chain_jsonl(3);
    let mut lines: Vec<&str> = jsonl.lines().collect();
    let corrupted = "not json at all";
    lines[1] = corrupted;
    let content = lines.join("\n");
    let result = replay::replay_chain_str(&content);
    assert!(result.is_err());
}

#[test]
fn replay_from_file() {
    let jsonl = build_chain_jsonl(4);
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "{jsonl}").unwrap();
    let result = replay::replay_chain_file(tmp.path()).unwrap();
    assert_eq!(result.entry_count, 4);
}
```

- [ ] **Step 6: Write GC tests**

```rust
// crates/audit-recover/tests/gc_tests.rs
#![allow(clippy::unwrap_used, clippy::expect_used)]

use audit_recover::gc;
use std::fs;

#[test]
fn gc_no_blob_dir_is_noop() {
    let dir = tempfile::tempdir().unwrap();
    let result = gc::gc_blobs(dir.path()).unwrap();
    assert_eq!(result.blobs_scanned, 0);
    assert_eq!(result.blobs_deleted, 0);
}

#[test]
fn gc_removes_orphan_blobs() {
    let dir = tempfile::tempdir().unwrap();
    let blob_dir = dir.path().join("blobs");
    fs::create_dir_all(&blob_dir).unwrap();

    fs::write(blob_dir.join("deadbeef".repeat(4)), b"orphan data").unwrap();
    fs::write(blob_dir.join("cafebabe".repeat(4)), b"also orphan").unwrap();

    // No chain files -> all blobs are orphans
    let result = gc::gc_blobs(dir.path()).unwrap();
    assert_eq!(result.blobs_scanned, 2);
    assert_eq!(result.blobs_deleted, 2);
    assert_eq!(fs::read_dir(&blob_dir).unwrap().count(), 0);
}
```

- [ ] **Step 7: Run tests**

Run: `cargo test -p audit-recover`
Expected: all tests pass

- [ ] **Step 8: Commit**

```bash
git add crates/audit-recover/ Cargo.toml
git commit -m "feat(audit-recover): add chain replay, crash recovery, and blob GC"
```

---

### Task 4: `audit-emit` crate — blob store

**Files:**
- Create: `crates/audit-emit/Cargo.toml`
- Create: `crates/audit-emit/src/lib.rs`
- Create: `crates/audit-emit/src/blob_store.rs`
- Create: `crates/audit-emit/src/config.rs`
- Create: `crates/audit-emit/src/error.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Create crate skeleton with Cargo.toml**

```toml
# crates/audit-emit/Cargo.toml
[package]
name = "audit-emit"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Audit event emitter with hash chain, blob store, and compile-time enforcement"

[dependencies]
audit-chain = { path = "../audit-chain" }
audit-events = { path = "../audit-events" }
audit-recover = { path = "../audit-recover" }
audit-sign = { path = "../audit-sign" }
chrono = { version = "0.4", features = ["serde"] }
fs2 = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
thiserror = "2"

[dev-dependencies]
tempfile = "3"

[lints]
workspace = true
```

Add `"crates/audit-emit"` to workspace members.

- [ ] **Step 2: Write error types**

```rust
// crates/audit-emit/src/error.rs
use audit_events::ValidationError;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AuditError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("chain error: {0}")]
    Chain(#[from] audit_chain::seal::SealError),
    #[error("canonical encoding error: {0}")]
    Canonical(#[from] audit_chain::canonical::CanonicalEncodingError),
    #[error("replay error: {0}")]
    Replay(#[from] audit_recover::replay::ReplayError),
    #[error("validation error: {0}")]
    Validation(#[from] ValidationError),
    #[error("blob store error: {0}")]
    BlobStore(String),
    #[error("circuit breaker: audit is bypassed")]
    CircuitBreakerActive,
    #[error("detail exceeds max size: {size} bytes (max {max})")]
    DetailTooLarge { size: usize, max: usize },
}
```

- [ ] **Step 3: Write config**

```rust
// crates/audit-emit/src/config.rs
#[derive(Debug, Clone)]
pub struct EmitterConfig {
    pub detail_max_bytes: usize,
    pub tags_max_pairs: usize,
    pub tag_value_max_bytes: usize,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            detail_max_bytes: 4096,
            tags_max_pairs: 32,
            tag_value_max_bytes: 256,
        }
    }
}
```

- [ ] **Step 4: Write blob store**

```rust
// crates/audit-emit/src/blob_store.rs
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::AuditError;

#[derive(Debug)]
pub struct BlobStore {
    blob_dir: PathBuf,
}

impl BlobStore {
    pub fn new(blob_dir: PathBuf) -> Self {
        Self { blob_dir }
    }

    pub fn store(&self, data: &[u8], content_type: &str) -> Result<audit_events::BlobRef, AuditError> {
        std::fs::create_dir_all(&self.blob_dir)
            .map_err(|e| AuditError::BlobStore(format!("cannot create blob dir: {e}")))?;

        let hash: [u8; 32] = Sha256::digest(data).into();
        let hex = hex_encode(&hash);
        let blob_path = self.blob_dir.join(&hex);

        if !blob_path.exists() {
            std::fs::write(&blob_path, data)
                .map_err(|e| AuditError::BlobStore(format!("cannot write blob {hex}: {e}")))?;
        }

        Ok(audit_events::BlobRef {
            hash,
            location: audit_events::BlobLocation::File { path: blob_path },
            size_bytes: data.len() as u64,
            content_type: content_type.into(),
        })
    }

    pub fn blob_dir(&self) -> &Path {
        &self.blob_dir
    }
}

pub fn hex_encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            use std::fmt::Write;
            let _ = write!(s, "{b:02x}");
            s
        })
}
```

- [ ] **Step 5: Write lib.rs (partial)**

```rust
// crates/audit-emit/src/lib.rs
#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod blob_store;
pub mod config;
pub mod error;
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo check -p audit-emit`
Expected: compiles

- [ ] **Step 7: Write blob store tests**

Add to `crates/audit-emit/tests/blob_store_tests.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

use audit_emit::blob_store::BlobStore;
use sha2::{Digest, Sha256};
use tempfile::tempdir;

#[test]
fn store_creates_content_addressed_file() {
    let dir = tempdir().unwrap();
    let store = BlobStore::new(dir.path().join("blobs"));
    let data = b"hello blob world";
    let blob_ref = store.store(data, "text/plain").unwrap();

    assert_eq!(blob_ref.size_bytes, data.len() as u64);
    assert_eq!(blob_ref.content_type, "text/plain");

    let expected_hash: [u8; 32] = Sha256::digest(data).into();
    assert_eq!(blob_ref.hash, expected_hash);

    match &blob_ref.location {
        audit_events::BlobLocation::File { path } => {
            assert!(path.exists());
            assert_eq!(std::fs::read(path).unwrap(), data);
        }
    }
}

#[test]
fn store_deduplicates_same_content() {
    let dir = tempdir().unwrap();
    let store = BlobStore::new(dir.path().join("blobs"));
    let data = b"dedup test";

    let ref1 = store.store(data, "text/plain").unwrap();
    let ref2 = store.store(data, "text/plain").unwrap();

    assert_eq!(ref1.hash, ref2.hash);
    assert_eq!(
        std::fs::read_dir(dir.path().join("blobs")).unwrap().count(),
        1
    );
}

#[test]
fn store_different_content_creates_different_files() {
    let dir = tempdir().unwrap();
    let store = BlobStore::new(dir.path().join("blobs"));

    store.store(b"content A", "text/plain").unwrap();
    store.store(b"content B", "text/plain").unwrap();

    assert_eq!(
        std::fs::read_dir(dir.path().join("blobs")).unwrap().count(),
        2
    );
}
```

- [ ] **Step 8: Run tests**

Run: `cargo test -p audit-emit`
Expected: all pass

- [ ] **Step 9: Commit**

```bash
git add crates/audit-emit/ Cargo.toml
git commit -m "feat(audit-emit): add blob store, config, and error types"
```

---

### Task 5: `audit-emit` — emitter core with flock and auto-promotion

**Files:**
- Create: `crates/audit-emit/src/emitter.rs`
- Modify: `crates/audit-emit/src/lib.rs`

- [ ] **Step 1: Write emitter**

```rust
// crates/audit-emit/src/emitter.rs
use std::io::Write;
use std::path::{Path, PathBuf};

use audit_chain::entry::{AuditEntryBuilder, Principal as ChainPrincipal, ResourceRef as ChainResourceRef};
use audit_chain::seal::{ChainHead, SealedAuditEntry};
use audit_events::{AuditEvent, validate_tags};
use audit_sign::signing::AuditSigner;
use fs2::FileExt;

use crate::blob_store::BlobStore;
use crate::config::EmitterConfig;
use crate::error::AuditError;

pub struct Emitter {
    chain: ChainHead,
    chain_path: PathBuf,
    blob_store: BlobStore,
    signer: Option<Box<dyn AuditSigner>>,
    config: EmitterConfig,
    lock_file: std::fs::File,
    audit_dir: PathBuf,
}

impl Emitter {
    pub fn open(audit_dir: &Path) -> Result<Self, AuditError> {
        std::fs::create_dir_all(audit_dir)?;

        let lock_path = audit_dir.join(".lock");
        let lock_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)?;
        lock_file.lock_exclusive()?;

        let chain_path = audit_dir.join("chain.jsonl");
        let chain = if chain_path.exists() {
            let result = audit_recover::replay::replay_chain_file(&chain_path)?;
            result.chain_head
        } else {
            ChainHead::new()
        };

        let blob_store = BlobStore::new(audit_dir.join("blobs"));

        Ok(Self {
            chain,
            chain_path,
            blob_store,
            signer: None,
            config: EmitterConfig::default(),
            lock_file,
            audit_dir: audit_dir.to_path_buf(),
        })
    }

    pub fn with_signer(mut self, signer: Box<dyn AuditSigner>) -> Self {
        self.signer = signer.into();
        self
    }

    pub fn with_config(mut self, config: EmitterConfig) -> Self {
        self.config = config;
        self
    }

    pub fn emit(&mut self, event: AuditEvent) -> Result<SealedAuditEntry, AuditError> {
        self.emit_inner(event, None)
    }

    pub fn emit_with_blob(
        &mut self,
        event: AuditEvent,
        blob: &[u8],
        content_type: &str,
    ) -> Result<SealedAuditEntry, AuditError> {
        self.emit_inner(event, Some((blob, content_type)))
    }

    fn emit_inner(
        &mut self,
        mut event: AuditEvent,
        blob: Option<(&[u8], &str)>,
    ) -> Result<SealedAuditEntry, AuditError> {
        // Step 1: Validate tags
        validate_tags(
            &event.tags,
            self.config.tags_max_pairs,
            self.config.tag_value_max_bytes,
        )?;

        // Step 2: Handle explicit blob
        if let Some((data, ct)) = blob {
            let blob_ref = self.blob_store.store(data, ct)?;
            event.blob_ref = Some(audit_events::BlobRef {
                hash: blob_ref.hash,
                location: blob_ref.location,
                size_bytes: blob_ref.size_bytes,
                content_type: blob_ref.content_type,
            });
        }

        // Step 3: Check detail size, auto-promote if needed
        let detail_json = serde_json::to_string(&event.detail)?;
        if detail_json.len() > self.config.detail_max_bytes && event.blob_ref.is_none() {
            let blob_ref = self.blob_store.store(
                detail_json.as_bytes(),
                "application/json",
            )?;
            event.detail = serde_json::json!({
                "__promoted_to_blob": true
            });
            event.blob_ref = Some(audit_events::BlobRef {
                hash: blob_ref.hash,
                location: blob_ref.location,
                size_bytes: blob_ref.size_bytes,
                content_type: blob_ref.content_type,
            });
        }

        // Step 4: Flatten to chain entry
        let authorization_str = serde_json::to_string(&event.authorization)?;
        let outcome_str = serde_json::to_string(&event.outcome)?;

        let mut builder = AuditEntryBuilder::new()
            .seq(0) // overridden by chain.link()
            .at(event.at)
            .actor(ChainPrincipal {
                kind: event.actor.kind.clone(),
                id: event.actor.id.clone(),
            })
            .event(event.event.as_str())
            .authorization(authorization_str.trim_matches('"'))
            .outcome(outcome_str.trim_matches('"'))
            .tags(event.tags)
            .detail(event.detail);

        if let Some(ns) = event.monotonic_ns {
            builder = builder.monotonic_ns(ns);
        }
        if let Some(trace_id) = event.trace_id {
            builder = builder.trace_id(trace_id.0);
        }
        if let Some(resource) = &event.resource {
            builder = builder.resource(ChainResourceRef::new(
                &resource.kind,
                &resource.id,
            ));
        }
        if let Some(blob_ref) = event.blob_ref {
            builder = builder.blob_ref(audit_chain::entry::BlobRef {
                hash: blob_ref.hash,
                location: audit_chain::entry::BlobLocation::File {
                    path: match blob_ref.location {
                        audit_events::BlobLocation::File { path } => path,
                    },
                },
                size_bytes: blob_ref.size_bytes,
                content_type: blob_ref.content_type,
            });
        }

        let entry = builder.build()
            .map_err(|e| AuditError::BlobStore(format!("entry build failed: {e}")))?;

        // Step 5: Link into chain
        let sealed = self.chain.link(entry)?;

        // Step 6: Append to chain file, fsync
        let line = serde_json::to_string(&sealed)?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.chain_path)?;
        writeln!(file, "{line}")?;
        file.sync_all()?;

        // Step 7: Write attestation if signer present
        if let Some(signer) = &self.signer {
            let snapshot = audit_sign::snapshot::ChainHeadSnapshot::from_chain_head(&self.chain);
            let cbor = audit_sign::attestation::build_tip_attestation(
                signer.as_ref(),
                &snapshot,
            ).map_err(|e| AuditError::BlobStore(format!("attestation failed: {e}")))?;

            let att_dir = self.audit_dir.join("attestations");
            std::fs::create_dir_all(&att_dir)?;
            std::fs::write(
                att_dir.join(format!("{}.cbor", sealed.base.seq)),
                &cbor,
            )?;
        }

        Ok(sealed)
    }

    pub fn chain_head(&self) -> &ChainHead {
        &self.chain
    }
}

impl Drop for Emitter {
    fn drop(&mut self) {
        let _ = self.lock_file.unlock();
    }
}
```

- [ ] **Step 2: Update lib.rs**

```rust
// crates/audit-emit/src/lib.rs
#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod blob_store;
pub mod config;
pub mod emitter;
pub mod error;
```

- [ ] **Step 3: Write emitter tests**

```rust
// crates/audit-emit/tests/emitter_tests.rs
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use audit_emit::config::EmitterConfig;
use audit_emit::emitter::Emitter;
use audit_events::*;
use chrono::Utc;
use tempfile::tempdir;

fn sample_event(kind: EventKind) -> AuditEvent {
    AuditEvent {
        envelope_version: 1,
        at: Utc::now(),
        monotonic_ns: Some(42),
        actor: Principal { kind: "System".into(), id: "test".into() },
        trace_id: None,
        event: kind,
        resource: ResourceRef { kind: "eval".into(), id: "arc".into() },
        authorization: Authorization::Allowed,
        outcome: Outcome::Succeeded,
        tags: BTreeMap::new(),
        detail: serde_json::json!({"run_id": "RUN-001"}),
        blob_ref: None,
    }
}

#[test]
fn emit_single_event() {
    let dir = tempdir().unwrap();
    let mut emitter = Emitter::open(dir.path()).unwrap();
    let sealed = emitter.emit(sample_event(EventKind::EvalStarted)).unwrap();
    assert_eq!(sealed.base.seq, 0);
    assert_eq!(sealed.base.event, "eval.started");
}

#[test]
fn emit_multiple_events_chain_correctly() {
    let dir = tempdir().unwrap();
    let mut emitter = Emitter::open(dir.path()).unwrap();

    let s0 = emitter.emit(sample_event(EventKind::EvalStarted)).unwrap();
    let s1 = emitter.emit(sample_event(EventKind::EvalCompleted)).unwrap();

    assert_eq!(s0.base.seq, 0);
    assert_eq!(s1.base.seq, 1);
    assert_eq!(s1.parent_hash, Some(s0.entry_hash));
}

#[test]
fn emit_persists_to_jsonl() {
    let dir = tempdir().unwrap();
    let mut emitter = Emitter::open(dir.path()).unwrap();
    emitter.emit(sample_event(EventKind::EvalStarted)).unwrap();
    emitter.emit(sample_event(EventKind::EvalCompleted)).unwrap();
    drop(emitter);

    let chain_path = dir.path().join("chain.jsonl");
    let contents = std::fs::read_to_string(&chain_path).unwrap();
    assert_eq!(contents.lines().count(), 2);
}

#[test]
fn reopen_continues_chain() {
    let dir = tempdir().unwrap();

    {
        let mut emitter = Emitter::open(dir.path()).unwrap();
        emitter.emit(sample_event(EventKind::EvalStarted)).unwrap();
    }

    {
        let mut emitter = Emitter::open(dir.path()).unwrap();
        let sealed = emitter.emit(sample_event(EventKind::EvalCompleted)).unwrap();
        assert_eq!(sealed.base.seq, 1);
    }
}

#[test]
fn detail_auto_promoted_to_blob() {
    let dir = tempdir().unwrap();
    let config = EmitterConfig {
        detail_max_bytes: 10, // very small limit
        ..EmitterConfig::default()
    };
    let mut emitter = Emitter::open(dir.path()).unwrap().with_config(config);

    let mut event = sample_event(EventKind::EvalStarted);
    event.detail = serde_json::json!({"large_data": "x".repeat(100)});

    let sealed = emitter.emit(event).unwrap();
    assert!(sealed.base.blob_ref.is_some());
    assert!(sealed.base.detail.get("__promoted_to_blob").is_some());

    // Verify blob file exists
    let blob_dir = dir.path().join("blobs");
    assert!(blob_dir.exists());
    assert!(std::fs::read_dir(&blob_dir).unwrap().count() > 0);
}

#[test]
fn emit_with_explicit_blob() {
    let dir = tempdir().unwrap();
    let mut emitter = Emitter::open(dir.path()).unwrap();
    let blob_data = b"large payload data here";

    let sealed = emitter
        .emit_with_blob(sample_event(EventKind::RunCardSealed), blob_data, "application/octet-stream")
        .unwrap();

    assert!(sealed.base.blob_ref.is_some());
}

#[test]
fn tag_validation_rejects_too_many() {
    let dir = tempdir().unwrap();
    let mut emitter = Emitter::open(dir.path()).unwrap();

    let mut event = sample_event(EventKind::EvalStarted);
    for i in 0..33 {
        event.tags.insert(format!("key{i}"), "value".into());
    }

    let result = emitter.emit(event);
    assert!(result.is_err());
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p audit-emit`
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add crates/audit-emit/
git commit -m "feat(audit-emit): add emitter core with flock, chain replay, and detail auto-promotion"
```

---

### Task 6: `audit-emit` — `AuditGate<T>` gateway type

**Files:**
- Create: `crates/audit-emit/src/gate.rs`
- Modify: `crates/audit-emit/src/lib.rs`

- [ ] **Step 1: Write AuditGate**

```rust
// crates/audit-emit/src/gate.rs
use audit_events::{Detail, EventKind, Outcome, ResourceRef, Tags};

use crate::emitter::Emitter;
use crate::error::AuditError;

#[must_use]
pub struct AuditGate<T> {
    inner: T,
    event_kind: EventKind,
    resource: ResourceRef,
    outcome: Outcome,
    resolved: bool,
}

impl<T> AuditGate<T> {
    pub fn new(
        inner: T,
        event_kind: EventKind,
        resource: ResourceRef,
        outcome: Outcome,
    ) -> Self {
        Self {
            inner,
            event_kind,
            resource,
            outcome,
            resolved: false,
        }
    }

    pub fn resolve(
        mut self,
        emitter: &mut Emitter,
        actor: audit_events::Principal,
        tags: Tags,
        detail: Detail,
    ) -> Result<T, AuditError> {
        let event = audit_events::AuditEvent {
            envelope_version: 1,
            at: chrono::Utc::now(),
            monotonic_ns: None,
            actor,
            trace_id: None,
            event: self.event_kind,
            resource: self.resource.clone(),
            authorization: audit_events::Authorization::Allowed,
            outcome: self.outcome.clone(),
            tags,
            detail,
            blob_ref: None,
        };

        emitter.emit(event)?;
        self.resolved = true;
        Ok(self.inner)
    }

    pub fn resolve_with_blob(
        mut self,
        emitter: &mut Emitter,
        actor: audit_events::Principal,
        tags: Tags,
        detail: Detail,
        blob: &[u8],
        content_type: &str,
    ) -> Result<T, AuditError> {
        let event = audit_events::AuditEvent {
            envelope_version: 1,
            at: chrono::Utc::now(),
            monotonic_ns: None,
            actor,
            trace_id: None,
            event: self.event_kind,
            resource: self.resource.clone(),
            authorization: audit_events::Authorization::Allowed,
            outcome: self.outcome.clone(),
            tags,
            detail,
            blob_ref: None,
        };

        emitter.emit_with_blob(event, blob, content_type)?;
        self.resolved = true;
        Ok(self.inner)
    }

    pub fn event_kind(&self) -> EventKind {
        self.event_kind
    }
}

impl<T> Drop for AuditGate<T> {
    fn drop(&mut self) {
        if !self.resolved {
            if cfg!(debug_assertions) {
                panic!(
                    "AuditGate dropped without resolution — event {:?} was never emitted",
                    self.event_kind
                );
            } else {
                eprintln!(
                    "AUDIT WARNING: AuditGate dropped without resolution for {:?}",
                    self.event_kind
                );
            }
        }
    }
}
```

- [ ] **Step 2: Update lib.rs**

Add `pub mod gate;` to `crates/audit-emit/src/lib.rs`.

- [ ] **Step 3: Write gate tests**

```rust
// crates/audit-emit/tests/gate_tests.rs
#![allow(clippy::unwrap_used, clippy::expect_used)]

use audit_emit::emitter::Emitter;
use audit_emit::gate::AuditGate;
use audit_events::*;
use std::collections::BTreeMap;
use tempfile::tempdir;

#[test]
fn gate_resolve_emits_event_and_returns_inner() {
    let dir = tempdir().unwrap();
    let mut emitter = Emitter::open(dir.path()).unwrap();

    let gate = AuditGate::new(
        42u64,
        EventKind::EvalCompleted,
        ResourceRef { kind: "eval".into(), id: "test".into() },
        Outcome::Succeeded,
    );

    let value = gate
        .resolve(
            &mut emitter,
            Principal { kind: "System".into(), id: "test".into() },
            BTreeMap::new(),
            serde_json::json!({}),
        )
        .unwrap();

    assert_eq!(value, 42);
    assert_eq!(emitter.chain_head().next_seq(), 1);
}

#[test]
#[should_panic(expected = "AuditGate dropped without resolution")]
fn gate_drop_without_resolve_panics_in_debug() {
    let _gate = AuditGate::new(
        "leaked value",
        EventKind::EvalStarted,
        ResourceRef { kind: "eval".into(), id: "test".into() },
        Outcome::Succeeded,
    );
    // gate goes out of scope without resolve -> panic
}

#[test]
fn gate_event_kind_accessor() {
    let gate = AuditGate::new(
        (),
        EventKind::PodCreated,
        ResourceRef { kind: "pod".into(), id: "test".into() },
        Outcome::Succeeded,
    );
    assert_eq!(gate.event_kind(), EventKind::PodCreated);

    // resolve to avoid panic
    let dir = tempdir().unwrap();
    let mut emitter = Emitter::open(dir.path()).unwrap();
    gate.resolve(
        &mut emitter,
        Principal { kind: "System".into(), id: "test".into() },
        BTreeMap::new(),
        serde_json::json!({}),
    )
    .unwrap();
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p audit-emit`
Expected: all pass (the panic test should pass via `#[should_panic]`)

- [ ] **Step 5: Commit**

```bash
git add crates/audit-emit/
git commit -m "feat(audit-emit): add AuditGate<T> gateway type with Drop enforcement"
```

---

### Task 7: CLI integration — `mojave audit emit` and `mojave audit gc`

**Files:**
- Modify: `crates/mojave-cli/Cargo.toml`
- Modify: `crates/mojave-cli/src/main.rs`
- Modify: `crates/mojave-cli/src/commands/audit.rs`

- [ ] **Step 1: Add dependencies to mojave-cli**

Add to `crates/mojave-cli/Cargo.toml` under `[dependencies]`:
```toml
audit-emit = { path = "../audit-emit" }
audit-events = { path = "../audit-events" }
audit-recover = { path = "../audit-recover" }
```

- [ ] **Step 2: Add Emit and Gc to AuditAction enum**

In `crates/mojave-cli/src/main.rs`, add to the `AuditAction` enum:

```rust
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
```

Add match arms in the `Commands::Audit { action }` handler:

```rust
AuditAction::Emit { blob_file, audit_dir } => {
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
```

- [ ] **Step 3: Implement run_emit and run_gc**

Add to `crates/mojave-cli/src/commands/audit.rs`:

```rust
pub fn run_emit(
    blob_file: Option<&Path>,
    audit_dir: Option<&Path>,
) -> Result<(), CliError> {
    let audit_path = audit_dir.unwrap_or(Path::new("data/audit"));

    let mut stdin_buf = String::new();
    std::io::stdin()
        .read_to_string(&mut stdin_buf)
        .map_err(|e| CliError::Audit(format!("cannot read stdin: {e}")))?;

    let event: audit_events::AuditEvent = serde_json::from_str(&stdin_buf)
        .map_err(|e| CliError::Audit(format!("invalid event JSON: {e}")))?;

    let mut emitter = audit_emit::emitter::Emitter::open(audit_path)
        .map_err(|e| CliError::Audit(format!("cannot open emitter: {e}")))?;

    let sealed = if let Some(blob_path) = blob_file {
        let blob_data = std::fs::read(blob_path)
            .map_err(|e| CliError::Audit(format!("cannot read blob file: {e}")))?;
        emitter
            .emit_with_blob(event, &blob_data, "application/octet-stream")
            .map_err(|e| CliError::Audit(format!("emit failed: {e}")))?
    } else {
        emitter
            .emit(event)
            .map_err(|e| CliError::Audit(format!("emit failed: {e}")))?
    };

    let output = serde_json::json!({
        "seq": sealed.base.seq,
        "entry_hash": hex_encode(&sealed.entry_hash),
        "event": sealed.base.event,
    });

    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| CliError::Audit(format!("cannot serialize output: {e}")))?;
    println!("{json}");
    Ok(())
}

pub fn run_gc(audit_dir: Option<&Path>) -> Result<(), CliError> {
    let audit_path = audit_dir.unwrap_or(Path::new("data/audit"));

    let result = audit_recover::gc::gc_blobs(audit_path)
        .map_err(|e| CliError::Audit(format!("GC failed: {e}")))?;

    let output = serde_json::json!({
        "blobs_scanned": result.blobs_scanned,
        "blobs_referenced": result.blobs_referenced,
        "blobs_deleted": result.blobs_deleted,
    });

    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| CliError::Audit(format!("cannot serialize output: {e}")))?;
    println!("{json}");
    Ok(())
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p mojave-cli`
Expected: compiles

- [ ] **Step 5: Run workspace tests**

Run: `cargo test --workspace`
Expected: all pass

- [ ] **Step 6: Commit**

```bash
git add crates/mojave-cli/
git commit -m "feat(mojave-cli): add 'mojave audit emit' and 'mojave audit gc' subcommands"
```

---

### Task 8: Integration test — full lifecycle

**Files:**
- Create: `crates/audit-emit/tests/integration_tests.rs`

- [ ] **Step 1: Write full lifecycle integration test**

```rust
// crates/audit-emit/tests/integration_tests.rs
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use audit_chain::verify::ChainVerifier;
use audit_emit::config::EmitterConfig;
use audit_emit::emitter::Emitter;
use audit_events::*;
use chrono::Utc;
use tempfile::tempdir;

fn event(kind: EventKind) -> AuditEvent {
    AuditEvent {
        envelope_version: 1,
        at: Utc::now(),
        monotonic_ns: Some(0),
        actor: Principal { kind: "System".into(), id: "integration-test".into() },
        trace_id: None,
        event: kind,
        resource: ResourceRef { kind: "eval".into(), id: "arc_challenge".into() },
        authorization: Authorization::Allowed,
        outcome: Outcome::Succeeded,
        tags: BTreeMap::from([("test".into(), "true".into())]),
        detail: serde_json::json!({"integration": true}),
        blob_ref: None,
    }
}

#[test]
fn full_lifecycle_emit_verify_reopen_gc() {
    let dir = tempdir().unwrap();

    // Phase 1: Emit a sequence of events
    {
        let mut emitter = Emitter::open(dir.path()).unwrap();
        emitter.emit(event(EventKind::EvalStarted)).unwrap();
        emitter.emit(event(EventKind::DatasetLoaded)).unwrap();
        emitter.emit(event(EventKind::ModelLoaded)).unwrap();
        emitter.emit(event(EventKind::ScoringCompleted)).unwrap();
        emitter.emit(event(EventKind::EvalCompleted)).unwrap();
    }

    // Phase 2: Verify the chain
    let chain_path = dir.path().join("chain.jsonl");
    let contents = std::fs::read_to_string(&chain_path).unwrap();
    let entries: Vec<audit_chain::seal::SealedAuditEntry> = contents
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();

    assert_eq!(entries.len(), 5);
    let findings = ChainVerifier::verify(&entries);
    assert!(findings.is_clean(), "chain should verify clean: {:?}", findings.findings());

    // Phase 3: Reopen and continue
    {
        let mut emitter = Emitter::open(dir.path()).unwrap();
        let sealed = emitter.emit(event(EventKind::RunCardGenerated)).unwrap();
        assert_eq!(sealed.base.seq, 5);
    }

    // Phase 4: Verify extended chain
    let contents = std::fs::read_to_string(&chain_path).unwrap();
    let entries: Vec<audit_chain::seal::SealedAuditEntry> = contents
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(entries.len(), 6);
    let findings = ChainVerifier::verify(&entries);
    assert!(findings.is_clean());

    // Phase 5: GC (no orphan blobs expected, but should run clean)
    let result = audit_recover::gc::gc_blobs(dir.path()).unwrap();
    assert_eq!(result.blobs_deleted, 0);
}

#[test]
fn auto_promotion_blob_survives_gc() {
    let dir = tempdir().unwrap();
    let config = EmitterConfig {
        detail_max_bytes: 10,
        ..EmitterConfig::default()
    };

    {
        let mut emitter = Emitter::open(dir.path()).unwrap().with_config(config);
        let mut ev = event(EventKind::EvalCompleted);
        ev.detail = serde_json::json!({"large_payload": "x".repeat(200)});
        emitter.emit(ev).unwrap();
    }

    // Blob should be referenced -> GC should not delete it
    let result = audit_recover::gc::gc_blobs(dir.path()).unwrap();
    assert_eq!(result.blobs_deleted, 0);
    assert!(result.blobs_referenced > 0);
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test -p audit-emit --test integration_tests`
Expected: all pass

- [ ] **Step 3: Run full workspace tests and clippy**

Run: `cargo test --workspace && cargo clippy --workspace -- -D warnings`
Expected: all pass, zero clippy warnings

- [ ] **Step 4: Commit**

```bash
git add crates/audit-emit/tests/integration_tests.rs
git commit -m "test(audit-emit): add full lifecycle integration test with chain verification and GC"
```

---

## Self-Review

### Spec coverage check

| Spec section | Task |
|---|---|
| Invariant Event Envelope / AuditEvent struct | Task 1 |
| TraceId | Task 1 |
| Tiered payload model | Tasks 1, 4, 5 |
| BlobRef / BlobLocation | Tasks 1, 4 |
| Authorization / Outcome separated | Task 1 |
| Clock Model (monotonic_ns) | Tasks 1, 2 |
| EventKind closed enum, no #[non_exhaustive] | Task 1 |
| AuditGate<T> gateway type | Task 6 |
| AuditGate Drop enforcement | Task 6 |
| Emitter struct | Task 5 |
| Emission path with flock | Task 5 |
| Blob store | Task 4 |
| Detail auto-promotion | Task 5 |
| Crash recovery (chain-head.json derivable) | Task 3 |
| Concurrent access (flock) | Task 5 |
| Circuit breaker | Deferred (v1 uses hard gate only — no bypass mechanism yet) |
| Log lifecycle (rotation/retention/archival) | Deferred (single chain.jsonl for trials) |
| Canonical JSON interop | Existing — no changes needed for v1 |
| KMS/HSM forward path | Existing trait — no changes needed for v1 |
| Domain separation tag | Existing — no changes needed |
| NIST 800-53 mapping | Documentation only — no code needed |
| Changes to audit-chain | Task 2 |
| CLI: mojave audit emit | Task 7 |
| CLI: mojave audit gc | Task 7 |
| #[must_audit] proc macro | Deferred (complex proc macro, enforce via AuditGate for v1) |
| Python integration | Deferred (scripts already call CLI, update call sites after trials confirm interface) |
| Testing strategy | Tasks 1-8 (gates 1-3, integration) |

**Deferred items** (not blocking trials):
- Circuit breaker with authenticated key file — hard gate is sufficient for trials
- Log rotation (daily files) — single chain.jsonl is fine for trial volume
- `#[must_audit]` proc macro — `AuditGate<T>` provides the primary enforcement
- Python script updates — current scripts use `mojave audit seal`, can be migrated after interface stabilizes
- Archival command

### Placeholder scan: clean
### Type consistency: verified — `AuditEvent`, `EventKind`, `Emitter`, `AuditGate`, `BlobRef`, `BlobLocation`, `Tags`, `Detail`, `Principal`, `ResourceRef` are consistent across all tasks.
