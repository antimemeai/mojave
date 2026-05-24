# Audit Event Emission System — Design Spec

## Goal

Replace the post-hoc `mojave audit seal` approach with a CloudTrail-style
event emission system where every mojave operation emits audit events as it
happens. The audit system is foundational infrastructure — mojave cannot
operate without it (hard gate), with an explicit circuit breaker for
emergencies.

## Architecture

Three new crates, one modified crate:

- **`audit-emit`** — emitter, gateway type, blob store, circuit breaker
- **`audit-macros`** — `#[must_audit]` proc macro, `assert_all_events_covered!()`
- **`audit-events`** — `EventKind` enum, `AuditEvent` struct, `Tags`, `Detail`, `BlobRef`
- **`audit-chain`** (modified) — receives flattened entries from the emitter

Python orchestration calls through `mojave audit emit` CLI subcommand.
Python never touches the chain directly.

## Invariant Event Envelope

Design principles (from CloudTrail operational experience):

1. **Small invariant core** — the envelope schema is sacred, never changes.
2. **Payload independence** — event emission success/failure never depends on
   the payload. Details either attach inline or point to a blob.

### `AuditEvent` struct

```rust
pub struct AuditEvent {
    // Chain-assigned (by Emitter, not caller)
    // seq: u64,

    // Caller-provided
    pub at: DateTime<Utc>,
    pub actor: Principal,
    pub event: EventKind,
    pub resource: ResourceRef,
    pub outcome: Outcome,
    pub tags: Tags,
    pub detail: Detail,
    pub blob_ref: Option<BlobRef>,
}
```

### Tiered payload model

Three tiers, each with hard limits enforced at emission:

| Tier | Type | Limit | Purpose |
|------|------|-------|---------|
| `Tags` | `BTreeMap<String, String>` | 32 pairs, 256 bytes/value | Flat k/v for filtering and indexing |
| `Detail` | `serde_json::Value` | 4 KB serialized | Small inline JSON for key facts |
| `BlobRef` | `{ hash, location, size_bytes, content_type }` | Unbounded (pointer only) | Large payloads stored externally |

**Auto-promotion**: if `detail` exceeds 4 KB at emission time, the emitter
automatically writes it to blob storage and replaces it with a `BlobRef`.
Emission never fails because of payload size.

### `BlobRef`

```rust
pub struct BlobRef {
    pub hash: [u8; 32],        // SHA-256 of blob content
    pub location: String,      // URI, e.g. "file://data/audit/blobs/<hex>"
    pub size_bytes: u64,
    pub content_type: String,  // MIME type
}
```

### `Outcome` enum

Replaces the current `Decision` enum:

```rust
pub enum Outcome {
    Succeeded,
    Failed,
    Denied,
    Observed,
}
```

## Event Catalog (`EventKind`)

Closed Rust enum. Adding a new event type is a deliberate code change —
a breaking change that requires updating all consumers.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
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
```

Serde representation: dot-separated lowercase (`"eval.started"`,
`"pod.created"`). The `#[serde(rename_all)]` or custom impl enforces this.

## Compile-Time Enforcement

Two mechanisms ensure operations cannot exist without audit wiring.

### 1. Gateway type: `AuditGate<T>`

Operations that must be audited return their result wrapped in `AuditGate<T>`.
The only way to extract the inner value is through the emitter:

```rust
#[must_use]
pub struct AuditGate<T> {
    inner: T,              // private
    event_kind: EventKind, // private
    resource: ResourceRef, // private
    outcome: Outcome,      // private
}

impl<T> AuditGate<T> {
    pub(crate) fn new(inner: T, event_kind: EventKind,
                      resource: ResourceRef, outcome: Outcome) -> Self;

    pub fn resolve(self, emitter: &mut Emitter,
                   tags: Tags, detail: Detail) -> Result<T, AuditError>;

    pub fn resolve_with_blob(self, emitter: &mut Emitter,
                             tags: Tags, detail: Detail,
                             blob: &[u8], content_type: &str)
        -> Result<T, AuditError>;
}
```

- No `Deref`, no `Clone`, no `Debug` on inner, no public fields.
- `#[must_use]` + workspace `#![deny(unused_must_use)]` = compiler error if
  you ignore the gate.
- Trying to return a raw value instead of `AuditGate<T>` is a type mismatch
  at the call site.

### 2. `#[must_audit]` proc macro

Attribute macro in `audit-macros` crate:

```rust
#[must_audit(EventKind::EvalStarted)]
pub fn run_eval(...) -> AuditGate<EvalResult> { ... }
```

Generates compile-time registration of the function-to-event mapping.

A test macro `assert_all_events_covered!()` runs as `#[test]` in the CLI
crate and verifies every `EventKind` variant has at least one
`#[must_audit]` call site. CI catches dead events — no variant can exist
without an emission site.

## Emitter

```rust
pub struct Emitter {
    chain: ChainHead,
    chain_path: PathBuf,
    blob_dir: PathBuf,
    signer: Option<LocalEd25519Signer>,
    circuit_breaker: CircuitBreaker,
    config: EmitterConfig,
}

pub struct EmitterConfig {
    pub detail_max_bytes: usize,  // default 4096
    pub tags_max_pairs: usize,    // default 32
    pub tag_value_max_bytes: usize, // default 256
}

impl Emitter {
    pub fn open(audit_dir: &Path) -> Result<Self, AuditError>;
    pub fn with_signer(self, signer: LocalEd25519Signer) -> Self;

    pub fn emit(&mut self, event: AuditEvent) -> Result<SealedAuditEntry, AuditError>;

    pub fn emit_with_blob(&mut self, event: AuditEvent,
                          blob: &[u8], content_type: &str)
        -> Result<SealedAuditEntry, AuditError>;
}
```

### Emission path

1. Validate tags (count, value size).
2. Serialize detail, check size.
3. If detail > limit: write to blob store, replace with `BlobRef`.
4. Flatten `AuditEvent` into `AuditEntry` (chain envelope).
5. `chain.link(entry)` — assigns seq, computes hash.
6. Append sealed entry to `chain.jsonl`, fsync.
7. Update `chain-head.json`.
8. If signer present: write attestation.
9. Return sealed entry.

If any step fails and circuit breaker is off: return error (hard gate).
If circuit breaker is on: log to stderr, return tainted success marker.

### Blob store

Content-addressed local filesystem: `data/audit/blobs/<sha256-hex>`.

- Blobs written **before** the chain entry referencing them.
- If blob write succeeds but chain append fails: orphan blob (harmless).
- If blob write fails: emission fails (hard gate).
- Dedup is free: same content = same hash = same file = no write.

## Circuit Breaker

Activated by env var `MOJAVE_AUDIT_BYPASS=1` or CLI flag `--audit-bypass`.

When tripped:
- Operations proceed.
- Every output artifact gets a taint marker:
  - Run card config: `audit.tainted` set to `true`, visible red banner in PDF.
  - CLI output JSON: `"tainted": true` field.
- `CircuitBreakerTripped` event emitted to **stderr** (not the chain — the
  chain might be broken).
- When reset: `CircuitBreakerReset` event written to the chain with context
  about the gap (start time, end time, reason).

## Failure Semantics

**Hard gate by default.** If audit emission fails, the operation fails.
No eval runs, no run cards, no pod operations without a functioning audit chain.

The circuit breaker is the only escape hatch, and it leaves visible marks
on everything it touches.

## Python Integration

Python scripts call `mojave audit emit` CLI subcommand:

```bash
echo '{"event":"eval.started","actor":{...},"resource":{...},...}' \
  | mojave audit emit
```

For blob payloads:

```bash
mojave audit emit --blob-file /path/to/large/payload.json < event.json
```

The CLI deserializes, validates against the closed `EventKind` enum, and
calls the Rust emitter. Python never touches the chain directly.

**Future state**: `mojave eval run` wraps the entire eval lifecycle and
handles all emission internally. Python scripts that bypass the CLI and
call Inspect directly operate outside the audit perimeter — the circuit
breaker taint makes this visible on any resulting artifacts.

## CLI Changes

### New subcommand: `mojave audit emit`

Reads JSON event from stdin, validates, emits through Rust emitter.
Options: `--blob-file <path>`, `--audit-bypass`.

### Modified: `mojave audit seal` (deprecated)

Thin wrapper that constructs a `RunCardSealed` event and calls the emitter.
Marked deprecated — callers should migrate to `mojave audit emit`.

### Modified: `mojave audit verify`

Unchanged — chain verification works the same regardless of how entries
were created.

## Changes to `audit-chain`

The `AuditEntry` struct in `audit-chain` becomes a dumb envelope.
The `Action` and `Decision` enums are replaced by string fields that
receive the serialized `EventKind` and `Outcome` from the emitter.

The chain crate does not depend on `audit-events` — it stores what
the emitter gives it. This keeps the chain crate minimal and stable.

New fields on `AuditEntry`:

```rust
pub struct AuditEntry {
    pub seq: u64,
    pub at: DateTime<Utc>,
    pub actor: Principal,
    pub event: String,            // was: Action enum
    pub resource: Option<ResourceRef>,
    pub outcome: String,          // was: Decision enum
    pub tags: BTreeMap<String, String>,
    pub detail: serde_json::Value,
    pub blob_ref: Option<BlobRef>,
}
```

The `Principal` and `ResourceRef` types stay in `audit-chain` as they
are stable envelope types.

## Instrumentation Points

Every operation in the system emits events. Complete catalog:

| Call site | Event | Notes |
|-----------|-------|-------|
| `run_destructive.py` / future `mojave eval run` | `EvalStarted` | Before Inspect invocation |
| `run_destructive.py` / future `mojave eval run` | `EvalCompleted` | After successful Inspect run |
| `run_destructive.py` / future `mojave eval run` | `EvalFailed` | On Inspect error |
| `create_pods.py` | `PodCreated` | After each RunPod API call |
| `create_pods.py` | `PodReady` | When vLLM endpoint responds 200 |
| `teardown_pods.py` | `PodTerminated` | After each terminate call |
| `create_pods.py` / `setup_pods.py` | `EndpointVerified` | On health check pass |
| `generate_run_cards.py` | `RunCardGenerated` | After LaTeX PDF built |
| `generate_run_cards.py` | `RunCardSealed` | After chain append (replaces current seal) |
| Inspect dataset load | `DatasetLoaded` | When HF dataset fetched |
| Inspect dataset cache | `DatasetCached` | On cache hit |
| Inspect model init | `ModelLoaded` | When model endpoint confirmed |
| Inspect scoring | `ScoringCompleted` | After all items scored |
| `mojave audit verify` | `ChainVerified` | After successful chain replay |
| Key provisioning | `KeyGenerated` | When Ed25519 key created |
| Key loading | `KeyLoaded` | When signing key loaded from file/env |
| Attestation creation | `AttestationCreated` | After CBOR attestation written |
| Any config change | `ConfigChanged` | Emitter config, key rotation, etc. |

## Testing Strategy

| Gate | What | How |
|------|------|-----|
| 1 | Golden vectors | Serialize every `EventKind` variant, compare to pinned strings |
| 2 | Round-trip | Emit event -> read chain -> verify all fields survive |
| 3 | Property tests | Detail auto-promotion at boundary; tag limit enforcement; chain monotonicity under rapid emission; blob dedup |
| 4 | N/A | Crypto primitives, not statistical estimators |
| Compile-time | `assert_all_events_covered!()` | Every `EventKind` variant has >= 1 `#[must_audit]` site |
| Integration | Full lifecycle | Emit all event types, verify chain, verify blob refs resolve |

## Crate Dependency Graph

```
audit-events  (EventKind, AuditEvent, Tags, Detail, BlobRef, Outcome)
    |
    v
audit-emit    (Emitter, AuditGate, CircuitBreaker, blob store)
    |
    +---> audit-chain   (ChainHead, SealedAuditEntry, ChainVerifier)
    +---> audit-sign    (LocalEd25519Signer, attestation)
    +---> audit-macros  (#[must_audit], assert_all_events_covered!)
    
mojave-cli
    +---> audit-emit
    +---> audit-events
    +---> audit-macros
```
