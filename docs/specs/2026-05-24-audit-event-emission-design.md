# Audit Event Emission System — Design Spec

## Goal

Replace the post-hoc `mojave audit seal` approach with a CloudTrail-style
event emission system where every mojave operation emits audit events as it
happens. The audit system is foundational infrastructure — mojave cannot
operate without it (hard gate), with an explicit circuit breaker for
emergencies.

## References

Required reading before implementation:

- **NIST SP 800-92** — Guide to Computer Security Log Management
- **NIST SP 800-53 rev5, AU family** — Audit and Accountability controls
- **Schneier & Kelsey 1999** — Secure Audit Logs to Support Computer Forensics
- **Crosby & Wallach 2009** — Efficient Data Structures for Tamper-Evident Logging
- **RFC 9052** — COSE Structures and Process (§4.2 for detached payload)
- **RFC 9597** — CWT Claims in COSE Headers (label 15)
- **RFC 8785** — JSON Canonicalization Scheme (for interop comparison)
- **RFC 9380 §3.1** — Domain separation tag conventions
- **RFC 9864** — COSE EdDSA algorithm update (Ed25519 = -50, future path)

## Architecture

Four new crates, one modified crate:

- **`audit-events`** — `EventKind` enum, `AuditEvent` struct, `Tags`, `Detail`, `BlobRef`
- **`audit-emit`** — emitter, gateway type, blob store, circuit breaker
- **`audit-macros`** — `#[must_audit]` proc macro, `assert_all_events_covered!()`
- **`audit-recover`** — crash recovery, chain replay, garbage collection
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

    // Envelope version — enables forward-compatible parsing
    pub envelope_version: u32,    // starts at 1, bump on schema change

    // Timing
    pub at: DateTime<Utc>,        // wall clock at emission
    pub monotonic_ns: Option<u64>, // monotonic clock for local ordering

    // Identity
    pub actor: Principal,
    pub trace_id: Option<TraceId>, // correlation across related events

    // What happened
    pub event: EventKind,
    pub resource: ResourceRef,
    pub authorization: Authorization, // was the operation allowed?
    pub outcome: Outcome,             // did the operation succeed?

    // Payload tiers
    pub tags: Tags,
    pub detail: Detail,
    pub blob_ref: Option<BlobRef>,
}
```

### `TraceId` — correlation across related events

A single user action (e.g., "run an eval") emits multiple events
(`EvalStarted` -> `DatasetLoaded` -> `ModelLoaded` -> `ScoringCompleted`
-> `EvalCompleted`). All events from the same logical operation share a
`trace_id` so consumers can reconstruct the full story.

```rust
pub struct TraceId(pub [u8; 16]); // 128-bit random, generated at operation start
```

### Tiered payload model

Three tiers, each with hard limits enforced at emission:

| Tier | Type | Limit | Purpose |
|------|------|-------|---------|
| `Tags` | `BTreeMap<String, String>` | 32 pairs, 256 bytes/value | Flat k/v for filtering and indexing |
| `Detail` | `serde_json::Value` | 4 KB serialized (see rationale below) | Small inline JSON for key facts |
| `BlobRef` | `{ hash, location, size_bytes, content_type }` | Unbounded (pointer only) | Large payloads stored externally |

**Auto-promotion**: if `detail` exceeds 4 KB at emission time, the emitter
automatically writes it to blob storage and replaces it with a `BlobRef`.
Emission never fails because of payload size.

**4 KB threshold rationale**: sized to hold typical eval metadata (run ID,
model name, dataset info, scoring summary) inline without blob indirection.
CloudTrail allows up to 256 KB per event; we chose a lower threshold because
every byte in `detail` gets hashed into the chain and stored in the JSONL —
smaller inline payloads keep the chain file compact and chain verification
fast. The threshold is configurable via `EmitterConfig` for deployments
with different tradeoffs.

### `BlobRef`

```rust
pub struct BlobRef {
    pub hash: [u8; 32],        // SHA-256 of blob content
    pub location: BlobLocation,
    pub size_bytes: u64,
    pub content_type: String,  // MIME type
}

pub enum BlobLocation {
    File { path: PathBuf },
    // Future: S3, GCS, classified storage backends
}
```

Using `BlobLocation` enum instead of a raw URI string avoids stringly-typed
storage backend routing and makes future backend additions a compile-time
checked code change.

### `Authorization` and `Outcome` — separated concerns

The old `Decision` enum conflated "was the operation allowed" with "did the
operation succeed." These are independent axes:

```rust
pub enum Authorization {
    Allowed,
    Denied,
    NotApplicable, // system-internal events with no auth decision
}

pub enum Outcome {
    Succeeded,
    Failed { error: String },
    Observed, // informational events with no success/failure
}
```

A `Denied` + `Observed` event means "auth rejected, nothing happened."
An `Allowed` + `Failed { error }` event means "auth passed, operation
crashed." CloudTrail models this the same way (`errorCode` +
`errorMessage` as separate fields from the access decision).

## Clock Model

### Problem

Wall clock `DateTime<Utc>` is insufficient for ordering guarantees.
NTP steps, VM migration, and leap seconds cause wall clocks to disagree
or go backwards. With 15+ RunPod pods emitting events, cross-node
timestamps WILL be inconsistent.

### Design

Two timestamp fields on every event:

- **`at: DateTime<Utc>`** — wall clock, best-effort, for human
  consumption and approximate time-range queries.
- **`monotonic_ns: Option<u64>`** — monotonic clock (e.g.,
  `Instant::now()` delta from process start), for local ordering within
  a single emitter process. `None` for events from external systems
  where monotonic time is unavailable.

**Within a single chain**: `seq` is the authoritative ordering.
Monotonic timestamps provide a secondary consistency check.

**Across chains** (distributed deployment): consumers must use `seq`
within a chain and treat `at` as approximate when correlating across
chains. A future NTP synchronization requirement with documented skew
bounds will be needed for production distributed deployments (per NIST
SP 800-92 §4.3).

## Event Catalog (`EventKind`)

Closed Rust enum. Adding a new event type is a deliberate code change —
a breaking change that requires updating all match sites.

**No `#[non_exhaustive]`** — the enum is intentionally closed so that
`match` statements in downstream crates are exhaustive. Adding a new
variant is a compile error everywhere, which is the enforcement mechanism.
This is a different choice than `#[non_exhaustive]` (which allows
non-breaking additions but prevents exhaustive matching). We want the
breakage.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
`"pod.created"`). Custom `Serialize`/`Deserialize` impl enforces this.

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
    resolved: bool,        // private, for Drop check
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

impl<T> Drop for AuditGate<T> {
    fn drop(&mut self) {
        if !self.resolved {
            // In debug builds: panic to catch accidental drops
            // In release builds: log to stderr as last resort
            if cfg!(debug_assertions) {
                panic!("AuditGate dropped without resolution — \
                        event {:?} was never emitted", self.event_kind);
            } else {
                eprintln!("AUDIT WARNING: AuditGate dropped without \
                           resolution for {:?}", self.event_kind);
            }
        }
    }
}
```

**Known limitation**: `#[must_use]` is a lint, not a linear type. Callers
can bypass via `std::mem::forget()` or `ManuallyDrop`. This is
defense-in-depth — the `Drop` impl catches the accidental case (struct
dropped without resolution), and the `#[must_use]` lint catches the
"forgot to bind the return value" case. Together they make accidental
omission hard; deliberate circumvention remains possible because Rust
does not have linear types. This is documented, not hidden.

- No `Deref`, no `Clone`, no `Debug` on inner, no public fields.
- `#[must_use]` + workspace `#![deny(unused_must_use)]` = compiler error if
  you ignore the gate.
- `Drop` impl panics in debug if unresolved = catches the "stored in a
  struct that gets dropped" case that `#[must_use]` misses.
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

**Limitation**: `assert_all_events_covered!()` can only see `#[must_audit]`
sites linked into the test binary. Functions behind `#[cfg(feature = "...")]`
or in separate binary crates will be invisible. Document which binaries
must run the assertion.

## Emitter

```rust
pub struct Emitter {
    chain: ChainHead,
    chain_path: PathBuf,
    blob_dir: PathBuf,
    signer: Option<Box<dyn AuditSigner>>,
    circuit_breaker: CircuitBreaker,
    config: EmitterConfig,
    lock: std::fs::File,  // flock on chain dir
}

pub struct EmitterConfig {
    pub detail_max_bytes: usize,    // default 4096
    pub tags_max_pairs: usize,      // default 32
    pub tag_value_max_bytes: usize, // default 256
}

impl Emitter {
    pub fn open(audit_dir: &Path) -> Result<Self, AuditError>;
    pub fn with_signer(self, signer: Box<dyn AuditSigner>) -> Self;

    pub fn emit(&mut self, event: AuditEvent)
        -> Result<SealedAuditEntry, AuditError>;

    pub fn emit_with_blob(&mut self, event: AuditEvent,
                          blob: &[u8], content_type: &str)
        -> Result<SealedAuditEntry, AuditError>;
}
```

Note: `signer` uses `Box<dyn AuditSigner>` (trait object) instead of
concrete `LocalEd25519Signer`. This allows P256/KMS signers to be
plugged in without changing the emitter. See **KMS/HSM Forward Path**.

### Emission path

1. Acquire exclusive file lock (`flock`) on chain directory.
2. Validate tags (count, value size).
3. Serialize detail, check size.
4. If detail > limit: write to blob store, replace with `BlobRef`.
5. Flatten `AuditEvent` into `AuditEntry` (chain envelope).
6. `chain.link(entry)` — assigns seq, computes hash.
7. Append sealed entry to `chain.jsonl`, fsync.
8. If signer present: write attestation.
9. Release file lock.
10. Return sealed entry.

**`chain-head.json` is NOT updated in the emission path.** It is derived
from `chain.jsonl` on `Emitter::open()`. See **Crash Recovery**.

If any step fails and circuit breaker is off: return error (hard gate).
If circuit breaker is on: log to stderr, return tainted success marker.

### Blob store

Content-addressed local filesystem: `data/audit/blobs/<sha256-hex>`.

- Blobs written **before** the chain entry referencing them (step 4
  before step 7).
- If blob write succeeds but chain append fails: orphan blob (harmless,
  cleaned up by `mojave audit gc`).
- If blob write fails: emission fails (hard gate).
- Dedup is free: same content = same hash = same file = no write.

## Crash Recovery

### Problem

The emission path has multiple write steps. A crash between any two steps
leaves the system in an inconsistent state if `chain-head.json` is treated
as authoritative.

### Design

**`chain-head.json` is an optimization hint, not a source of truth.**

On `Emitter::open()`:
1. Replay `chain.jsonl` from the beginning.
2. Reconstruct `ChainHead` (last entry hash, next seq) from the replayed
   entries.
3. Verify the reconstructed state matches `chain-head.json` (if present).
4. If mismatch: log a warning, use the replayed state, overwrite
   `chain-head.json`.

This means:
- Crash after append to `chain.jsonl` but before anything else: the entry
  is in the chain, next open recovers it. Safe.
- Crash after blob write but before chain append: orphan blob, no chain
  entry. Safe (blob is harmless, `mojave audit gc` cleans it up).
- Crash during chain append (partial line): the last line of `chain.jsonl`
  fails JSON parse. Recovery truncates the partial line and logs a warning.
  The failed event must be re-emitted by the caller (which will see an
  error from the crashed process and retry).

**Write ordering invariant** (from quarantine `thunderdome-sensitivity`):
blob -> chain entry -> attestation. Tip (chain-head.json) is always
derivable. A partial-write crash never yields a tip over an incomplete
chain.

### Recovery test

A dedicated test creates a chain, simulates crash at each step
(truncated write, missing head file, orphan blob), and verifies that
`Emitter::open()` recovers correctly.

## Concurrent Access

### Problem

Multiple Python scripts calling `mojave audit emit` in parallel will
race on `chain.jsonl` and `chain-head.json`.

### Design: file locking + per-run chain option

**Primary mechanism: `flock`**. The emitter acquires an exclusive file
lock on `data/audit/.lock` during `open()`. The lock is held for the
duration of the emission (steps 1-9 in the emission path). This
serializes concurrent emitters — only one process can append to the chain
at a time.

`flock` is advisory on Unix, which is sufficient because all access goes
through the Rust emitter (Python never touches the chain directly).

**Alternative for high-throughput deployments: per-run chains.** Each eval
run gets its own chain directory (`data/audit/runs/<run-id>/`). Concurrent
runs never contend. A global chain can be reconstructed by merging per-run
chains in seq order via `mojave audit merge`. This pattern matches the
quarantine's per-cell-chain design. Not in v1 scope but the architecture
supports it — the emitter takes an `audit_dir` path, not a global singleton.

## Circuit Breaker

### Activation

**NOT an env var.** Setting `MOJAVE_AUDIT_BYPASS=1` is too easy —
any process that can set environment variables can silently suppress
audit (NIST 800-53 AU-9: audit protection must match system protection).

Instead, bypass requires a **key file**:
`--audit-bypass-key /path/to/bypass.key`

The bypass key is a separate Ed25519 key (not the signing key) whose
public key is embedded in the mojave binary or loaded from a config.
The bypass token is a signed timestamp proving the key holder authorized
the bypass. This means:

- Bypass requires possession of a physical key file.
- The bypass event (written to stderr + syslog) includes the key ID,
  so you know WHO bypassed audit.
- The bypass key can be stored on a hardware token / HSM for high
  security deployments.
- Revoking a bypass key is a config change, not a binary rebuild.

### Behavior when tripped

- Operations proceed.
- Every output artifact gets a taint marker:
  - Run card config: `audit.tainted` set to `true`, visible red banner
    in PDF.
  - CLI output JSON: `"tainted": true` field.
- `CircuitBreakerTripped` event emitted to **stderr AND syslog** (not
  the chain — the chain might be what's broken). Includes bypass key ID.
- When reset: `CircuitBreakerReset` event written to the chain with
  context about the gap (start time, end time, bypass key ID, reason).

## Log Lifecycle

### Rotation

One chain file per calendar day by default:
`data/audit/chain-YYYY-MM-DD.jsonl`. The emitter rolls to a new file at
midnight UTC. The `chain-head.json` always points to the current file.

For per-run chains (high-throughput mode), one file per eval run.

### Retention

Configurable retention period in `EmitterConfig`:

```rust
pub retention_days: Option<u32>, // default None (keep forever)
```

`mojave audit gc` enforces the retention policy: deletes chain files and
their referenced blobs older than `retention_days`. GC never deletes
the current active chain file.

### Archival

`mojave audit archive` creates a signed, timestamped tarball of chain
files + blobs for a date range. The archive itself is sealed into the
chain as an `ArchiveCreated` event (future `EventKind` addition). For
defense customers, the archive format is the delivery mechanism for log
retention compliance.

### Blob garbage collection

`mojave audit gc` walks all chain files, collects referenced blob hashes,
and deletes any blobs in `data/audit/blobs/` not referenced by any chain
entry. Orphan blobs (from crash recovery) are cleaned up here.

## Canonical JSON Interoperability

The existing canonical encoding sorts keys by UTF-8 byte order (Rust
`String::cmp`). JCS (RFC 8785) sorts by UTF-16 code units. For
ASCII-only keys, the two orderings are identical.

**Invariant**: all `AuditEntry` field names are ASCII. All `Tags` keys
are validated to be ASCII at emission time. The only possible non-ASCII
content is in `detail` (arbitrary JSON) and `actor`/`resource` string
fields.

**For v1**: enforce ASCII-only keys at the `Tags` level (emission rejects
non-ASCII tag keys). Document that `detail` JSON keys may contain
non-ASCII, and the canonical encoding uses UTF-8 sort order.

**Standalone encoding specification**: before defense deployment, write a
standalone "Mojave Canonical JSON" specification document that a
third-party verifier can implement without reading the Rust source.
Include test vectors. This is required for independent audit verification.

## KMS/HSM Forward Path

The quarantine `thunderdome-audit-sign` had both Ed25519 and ECDSA-P256
signers, with P256 specifically for AWS KMS compatibility (KMS does not
support Ed25519). The current `audit-sign` only has Ed25519.

**Design for algorithm agility**: the `AuditSigner` trait is already
algorithm-agnostic. The emitter takes `Box<dyn AuditSigner>`. For v1,
only `LocalEd25519Signer` ships. Before defense deployment:

1. Re-introduce `LocalEcdsaP256Signer` from quarantine (with
   S-normalization, already implemented).
2. Add `KmsSigner` that delegates to AWS KMS / Azure Key Vault / GCP
   Cloud KMS via their respective SDKs.
3. The COSE algorithm header updates accordingly (`ES256` for P256,
   `EdDSA` or `Ed25519` per RFC 9864 for Ed25519).

The attestation verifier's algorithm allowlist must be extended to
accept `ES256` alongside `EdDSA` when P256 signers are in use.

## Domain Separation Tag

The current tag is `b"mojave-audit-v1\x00"`. RFC 9380 §3.1 recommends
domain separation tags "without null termination." The null byte is not
harmful (it makes the tag suffix-free, preventing a shorter tag from
being a prefix of a longer one), but it contradicts the RFC recommendation.

**Decision**: keep the null byte. The tag is used in a custom hash chain
context, not in hash-to-curve (where RFC 9380 is normative). The
suffix-free property is desirable for future versioning (`mojave-audit-v2`
could not be confused with `mojave-audit-v1` + payload starting with `\x00`
... except it could, which is why the null byte doesn't actually help with
prefix-freeness in the general case). Document the rationale: the null
byte is a convention from the original implementation, carried forward
for chain continuity.

## NIST 800-53 AU Control Mapping

Defense customers will ask which AU controls this system satisfies.
Preliminary mapping:

| Control | Description | Coverage |
|---------|-------------|----------|
| AU-2 | Event Logging | Full — `EventKind` catalog covers all auditable events |
| AU-3 | Content of Audit Records | Full — envelope has who, what, when, where, outcome |
| AU-4 | Audit Log Storage Capacity | Partial — blob promotion prevents unbounded inline growth; rotation + retention handles lifecycle |
| AU-5 | Response to Audit Logging Process Failures | Full — hard gate stops operations on audit failure |
| AU-6 | Audit Record Review, Analysis, and Reporting | Partial — chain verification exists; query/analysis tooling TBD |
| AU-8 | Time Stamps | Partial — wall clock + monotonic; NTP sync documentation TBD |
| AU-9 | Protection of Audit Information | Full — hash chain tamper evidence, COSE_Sign1 attestations, authenticated bypass |
| AU-10 | Non-repudiation | Full — Ed25519 signatures with key ID attribution |
| AU-11 | Audit Record Retention | Full — configurable retention, archival command |
| AU-12 | Audit Record Generation | Full — `AuditGate<T>` + `#[must_audit]` compile-time enforcement |

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
    pub envelope_version: u32,
    pub at: DateTime<Utc>,
    pub monotonic_ns: Option<u64>,
    pub actor: Principal,
    pub trace_id: Option<[u8; 16]>,
    pub event: String,
    pub resource: Option<ResourceRef>,
    pub authorization: String,
    pub outcome: String,
    pub tags: BTreeMap<String, String>,
    pub detail: serde_json::Value,
    pub blob_ref: Option<BlobRef>,
}
```

The `Principal` and `ResourceRef` types stay in `audit-chain` as they
are stable envelope types.

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
Options: `--blob-file <path>`, `--audit-bypass-key <path>`.

### New subcommand: `mojave audit gc`

Enforces retention policy, cleans orphan blobs.

### Modified: `mojave audit seal` (deprecated)

Thin wrapper that constructs a `RunCardSealed` event and calls the emitter.
Marked deprecated — callers should migrate to `mojave audit emit`.

### Modified: `mojave audit verify`

Unchanged — chain verification works the same regardless of how entries
were created.

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
| 3 | Property tests | Detail auto-promotion at boundary; tag limit enforcement; chain monotonicity under rapid emission; blob dedup; crash recovery at each step; concurrent flock contention |
| 4 | N/A | Crypto primitives, not statistical estimators |
| Compile-time | `assert_all_events_covered!()` | Every `EventKind` variant has >= 1 `#[must_audit]` site |
| Integration | Full lifecycle | Emit all event types, verify chain, verify blob refs resolve, GC cleans orphans |
| Recovery | Crash simulation | Truncated chain lines, missing head file, orphan blobs — all recover cleanly |

## Crate Dependency Graph

```
audit-events  (EventKind, AuditEvent, Tags, Detail, BlobRef, Outcome,
               Authorization, TraceId)
    |
    v
audit-emit    (Emitter, AuditGate, CircuitBreaker, blob store, flock)
    |
    +---> audit-chain    (ChainHead, SealedAuditEntry, ChainVerifier)
    +---> audit-sign     (AuditSigner trait, Ed25519, future P256/KMS)
    +---> audit-macros   (#[must_audit], assert_all_events_covered!)
    +---> audit-recover  (chain replay, crash recovery, GC)

mojave-cli
    +---> audit-emit
    +---> audit-events
    +---> audit-macros
```
