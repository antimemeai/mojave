# Design: Genesis Sentinel for Audit Chain

**Date:** 2026-05-25
**Status:** Draft
**Crates affected:** `audit-chain`, `audit-events`, `audit-sign`, `mojave-cli`
**Scripts affected:** `scripts/v2/verify_cards.py`, `generate_run_cards.py`
**Depends on:** `2026-05-24-audit-integration-design.md` (domain tag, canonical encoding)

## Problem

The audit chain currently has a genesis entry (seq 0, `parent_hash = None`) that uses
`[0u8; 32]` as the sentinel in the hash computation. This zero sentinel carries no
semantic meaning. There is no cryptographic binding between a chain and the model it
evaluates. Model identity rides in `tags`/`detail` JSON -- metadata, not crypto.

This means:
- Entries from different model evaluations could be spliced between chains undetected.
- Model metadata could be modified post-hoc without breaking the hash chain.
- A verifier can confirm chain integrity but cannot answer "was this chain produced
  by evaluating model X?"

## Solution

Replace the `[0u8; 32]` genesis sentinel with the model's identity hash. Every entry
transitively depends on the model hash through the parent hash chain:

```
entry[0].entry_hash = SHA-256(domain_tag || canonical(entry[0]) || model_hash)
entry[1].entry_hash = SHA-256(domain_tag || canonical(entry[1]) || entry[0].entry_hash)
entry[n].entry_hash = SHA-256(domain_tag || canonical(entry[n]) || entry[n-1].entry_hash)
```

Change the model identity: root of chain = root of trust.

## Non-Goals

- Multi-model chains. Each chain serves exactly one model.
- Model weight verification at eval time (runtime weight integrity is a separate concern).
- PKI for model identity (model hashes are self-asserted, not CA-signed).

---

## Part 1: Model Identity

### 1.1 ModelIdentity Struct

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelIdentity {
    pub name: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantization: Option<String>,
    pub hash_method: ModelHashMethod,
    pub hash: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ModelHashMethod {
    WeightFile,
    StructuredDescriptor,
}
```

### 1.2 Hash Derivation

**Weight-file models** (`ModelHashMethod::WeightFile`):
- Enumerate weight files (safetensors, GGUF, bin) in the model directory.
- Sort filenames lexicographically (UTF-8 byte order).
- SHA-256 of the concatenation: `SHA-256(file_1_bytes || file_2_bytes || ...)`.
- The hash covers the actual weights. Renaming files changes the sort order and
  thus the hash (intentional -- file naming is part of the artifact identity).

**API models** (`ModelHashMethod::StructuredDescriptor`):
- Construct a canonical JSON object: `{"name": "...", "provider": "...", "version": "..."}`.
- Use the same canonical encoding as audit entries (sorted keys, integer-only, no whitespace).
- SHA-256 of the canonical bytes.
- This is weaker than weight hashing -- the provider could change weights behind the
  same version string. It is the best available when we have no artifact access.

### 1.3 Location

`ModelIdentity` lives in the `audit-chain` crate alongside `AuditEntry`. It is a
domain type, not an event payload -- it is structurally required to construct a chain.

---

## Part 2: SealedAuditEntry Enum

### 2.1 Type Change

`SealedAuditEntry` changes from a struct to a two-variant enum:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum SealedAuditEntry {
    Genesis {
        base: AuditEntry,
        model_identity: ModelIdentity,
        entry_hash: [u8; 32],
    },
    Chained {
        base: AuditEntry,
        parent_hash: [u8; 32],
        entry_hash: [u8; 32],
    },
}
```

### 2.2 Accessor Methods

```rust
impl SealedAuditEntry {
    pub fn base(&self) -> &AuditEntry;
    pub fn entry_hash(&self) -> [u8; 32];
    pub fn seq(&self) -> u64;
    pub fn is_genesis(&self) -> bool;
    pub fn parent_hash(&self) -> Option<[u8; 32]>;
    pub fn model_identity(&self) -> Option<&ModelIdentity>;
}
```

`parent_hash()` returns `None` for genesis, `Some(hash)` for chained.
`model_identity()` returns `Some(&mi)` for genesis, `None` for chained.

### 2.3 Serde Representation

Internally tagged enum via `#[serde(tag = "type")]`. Genesis entry JSON:

```json
{
  "type": "Genesis",
  "base": { "seq": 0, "event": "chain.genesis", ... },
  "model_identity": {
    "name": "Qwen2.5-7B-Instruct",
    "provider": "local/vllm",
    "hash_method": "WeightFile",
    "hash": "a1b2c3d4..."
  },
  "entry_hash": "d4e5f6..."
}
```

Chained entry JSON:

```json
{
  "type": "Chained",
  "base": { "seq": 1, "event": "eval.started", ... },
  "parent_hash": "d4e5f6...",
  "entry_hash": "789abc..."
}
```

### 2.4 Zero Sentinel Elimination

The constant `GENESIS_SENTINEL: [u8; 32] = [0u8; 32]` is removed. Its role is replaced
by `model_identity.hash`. An all-zeros model hash is rejected at construction time
(a model with zero identity is not a model).

---

## Part 3: Chain Construction

### 3.1 SealError Extension

```rust
pub enum SealError {
    CanonicalEncoding(CanonicalEncodingError),
    SeqExhausted,
    ZeroModelHash,  // NEW: model_identity.hash is [0u8; 32]
}
```

### 3.2 ChainHead Changes

```rust
impl ChainHead {
    pub fn new(model: ModelIdentity) -> Result<(Self, SealedAuditEntry), SealError> {
        // Validates model.hash != [0u8; 32] -> Err(SealError::ZeroModelHash)
        // Constructs genesis AuditEntry with event = "chain.genesis"
        // Computes genesis hash: SHA-256(domain_tag || canonical(base) || model.hash)
        // Returns (head_at_seq_1, genesis_entry)
    }

    pub fn link(&mut self, base: AuditEntry) -> Result<SealedAuditEntry, SealError> {
        // parent_hash is always self.last_entry_hash (guaranteed Some after genesis)
        // Returns Chained variant
    }

    pub fn resume(last_entry_hash: [u8; 32], next_seq: u64) -> Self {
        // For reloading from persisted chain-head.json
        // Caller responsible for having verified genesis
    }
}
```

### 3.3 Genesis Entry Construction

The genesis `AuditEntry` is built internally by `ChainHead::new()`:

- `seq`: 0
- `event`: `"chain.genesis"`
- `actor`: `Principal { kind: "System", id: "chain.init" }`
- `authorization`: `"Allowed"`
- `outcome`: `"Succeeded"`
- `at`: `Utc::now()`
- `detail`: canonical JSON of `ModelIdentity` (full metadata in body for human readers)

The caller does not construct the genesis entry -- `ChainHead::new()` owns it entirely.

### 3.4 Hash Functions

Two functions, same structure, different sentinel slot:

```rust
pub(crate) fn compute_genesis_hash(
    base: &AuditEntry,
    model_hash: [u8; 32],
) -> Result<[u8; 32], CanonicalEncodingError> {
    let canonical = canonical::encode(base)?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_TAG);
    hasher.update(&canonical);
    hasher.update(model_hash);
    Ok(hasher.finalize().into())
}

pub(crate) fn compute_chained_hash(
    base: &AuditEntry,
    parent_hash: [u8; 32],
) -> Result<[u8; 32], CanonicalEncodingError> {
    let canonical = canonical::encode(base)?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_TAG);
    hasher.update(&canonical);
    hasher.update(parent_hash);
    Ok(hasher.finalize().into())
}
```

These are structurally identical (both hash `domain_tag || canonical || 32_bytes`).
They are separate functions for clarity and to prevent accidentally passing model_hash
where parent_hash is expected (or vice versa). The old `compute_entry_hash` with
`Option<[u8; 32]>` is removed.

---

## Part 4: Verification

### 4.1 New ChainFinding Variants

```rust
pub enum ChainFinding {
    // Hash integrity
    EntryHashMismatch { index: usize, seq: u64 },
    ParentHashMismatch { index: usize, seq: u64 },
    SeqDiscontinuity { index: usize, expected: u64, actual: u64 },

    // Genesis enforcement
    MissingGenesis,
    ChainedAtIndexZero,
    GenesisNotAtIndexZero { index: usize },
    DuplicateGenesis { index: usize },
    GenesisHashMismatch,
}
```

Replaces `NonGenesisAtIndexZero` with five more specific findings.

### 4.2 Verification Walk

```
for (i, entry) in entries.iter().enumerate():
    if i == 0:
        if entry is Chained -> ChainedAtIndexZero
        if entry is Genesis:
            recompute = compute_genesis_hash(base, model_identity.hash)
            if recompute != entry_hash -> GenesisHashMismatch
    else:
        if entry is Genesis -> DuplicateGenesis { index: i }
        if entry is Chained:
            if parent_hash != entries[i-1].entry_hash -> ParentHashMismatch
            recompute = compute_chained_hash(base, parent_hash)
            if recompute != entry_hash -> EntryHashMismatch

    check seq continuity (unchanged)
```

If the chain is empty, emit `MissingGenesis`.

### 4.3 Model Identity Accessor

```rust
impl ChainVerifier {
    pub fn model_identity(entries: &[SealedAuditEntry]) -> Option<&ModelIdentity> {
        match entries.first()? {
            SealedAuditEntry::Genesis { model_identity, .. } => Some(model_identity),
            _ => None,
        }
    }
}
```

---

## Part 5: Persistence

### 5.1 Per-Model Chain Directory

```
data/audit/
  chains/
    <model_hash_hex_16>/         # first 16 hex chars of model hash
      chain.jsonl                # append-only, one JSON entry per line
      chain-head.json            # tip state for resume
      attestations/
        <seq>.cbor               # COSE_Sign1 per entry (if signed)
  pubkey.spki.der                # shared signing key
```

### 5.2 chain-head.json

```json
{
  "tip_hash": "<64-char hex>",
  "next_seq": 42,
  "model_hash": "<64-char hex>"
}
```

`model_hash` enables `resume()` callers to verify they are resuming the correct chain
before appending entries.

---

## Part 6: Event Kind

Add `ChainGenesis` to `EventKind`:

```rust
pub enum EventKind {
    // ... existing variants ...
    ChainGenesis,  // "chain.genesis"
}
```

Wire through `as_str()`, `parse()`, `all()`.

---

## Part 7: Impact Summary

### Breaking Changes

| Change | Reason | Acceptable? |
|--------|--------|-------------|
| `SealedAuditEntry` struct -> enum | Genesis is structurally different from chained | Yes -- no production chains |
| `ChainHead::new()` signature | Requires `ModelIdentity` | Yes -- essential |
| `ChainFinding::NonGenesisAtIndexZero` removed | Replaced by five specific findings | Yes -- more precise |
| `compute_entry_hash` removed | Split into `compute_genesis_hash` / `compute_chained_hash` | Yes -- prevents misuse |
| `GENESIS_SENTINEL` removed | Replaced by model hash | Yes -- the point |
| chain.jsonl format | Tagged enum JSON | Yes -- no production chains |

### Affected Crates

- **`audit-chain`**: `entry.rs` (ModelIdentity), `seal.rs` (ChainHead, hash functions),
  `verify.rs` (ChainVerifier, ChainFinding)
- **`audit-events`**: `event_kind.rs` (ChainGenesis variant)
- **`audit-sign`**: `snapshot.rs` (add model_hash to ChainHeadSnapshot)
- **`mojave-cli`**: `commands/audit.rs` (seal and verify subcommands)

### Affected TCK

- `tck/audit-chain/features/chain_integrity.feature` -- all scenarios updated,
  new genesis-specific scenarios added

### Affected Scripts

- `scripts/v2/verify_cards.py` -- updated for tagged enum JSON
- Pipeline scripts that call `mojave audit seal` -- pass model identity

---

## Appendix A: Security Properties

The genesis sentinel provides:

1. **Model binding**: Every entry's hash transitively depends on `model_identity.hash`.
   Substituting a different model invalidates the entire chain from entry 0 forward.

2. **Splice resistance**: Entries from chain A (model X) cannot be spliced into chain B
   (model Y) because their hashes are rooted in different model identities.

3. **Metadata integrity**: The model's human-readable metadata (name, provider, version,
   quantization) is in the genesis entry's `detail` JSON, which is covered by the
   canonical encoding and thus by the entry hash. Modifying any metadata field
   invalidates the genesis hash.

4. **Transitive trust**: A verifier who trusts the genesis entry's model hash
   transitively trusts every subsequent entry in the chain, because each entry's
   hash includes its parent's hash.

## Appendix B: Weight Hashing Considerations

Weight-file hashing (SHA-256 of concatenated sorted weight files) has practical
constraints:

- **Large files**: A 14 GB model (Qwen2.5-7B in fp16) takes ~30 seconds to hash
  on NVMe. This is a one-time cost per chain creation.
- **Quantization variants**: Different quantizations of the same model produce
  different hashes (intentional -- they are different artifacts).
- **Sharded models**: Multiple safetensors files are sorted and concatenated.
  File boundaries are not length-prefixed because SHA-256 of concatenation is
  sufficient when the file set is fixed at hash time.
- **Reproducibility**: Weight hashing is reproducible given the same files in the
  same directory. Filenames determine concatenation order (via lexicographic sort)
  but are not included in the hash input themselves. Renaming a file can change
  which position it occupies in the concatenation, changing the hash.
