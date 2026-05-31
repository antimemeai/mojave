# Wave 2 Deep Dive: Audit Chain Architecture and Trust Model

**Date:** 2026-05-30
**Agent:** claude-opus-4-6
**Built on:** Adversary finding 4 (unsigned binary theater), Codebase finding 2 (Python-Rust parity contract), Web findings 6 (Kao2025 crypto evidence) and 7 (Sigstore model-transparency)

---

## 1. Architecture Map

The audit chain spans five crates plus one Python module, layered as follows:

```
audit-events          Type-safe event vocabulary (21 event kinds)
    |
audit-chain           Hash chain primitives: canonical encoding, SealedAuditEntry,
    |                 ChainHead (genesis + chaining), ChainVerifier, ModelIdentity
    |
audit-sign            Ed25519 signing: AuditSigner trait, COSE_Sign1 detached
    |                 attestations, ChainHeadSnapshot
    |
audit-emit            Emitter: file locking, blob store, auto-attestation,
    |                 AuditGate (must-resolve guard type)
    |
audit-recover         Replay (crash recovery), GC (orphan blob cleanup)
    |
scripts/audit.py      Python audit writer (thin reimplementation of chain logic)
mojave-cli audit       CLI: seal, emit, verify, gc subcommands
```

### Core chain construction

Every chain entry is computed as:

```
entry_hash = SHA-256(DOMAIN_TAG || canonical_json(base) || parent_hash)
```

where `DOMAIN_TAG = b"mojave-audit-v1\0"`, `canonical_json` is a custom encoder (sorted keys, no whitespace, float rejection, integer-only numbers), and `parent_hash` is either the model identity hash (for genesis) or the previous entry's hash (for chained entries).

The chain is an append-only JSONL file. The genesis entry binds the chain to a `ModelIdentity` (name, provider, quantization, weight-file or structured-descriptor hash). Subsequent entries chain via `parent_hash` linkage with monotonically increasing sequence numbers.

### Signing layer

Optional Ed25519 signing via the `AuditSigner` trait. The attestation format is COSE_Sign1 (RFC 9052) with detached payloads:
- Protected header carries: algorithm (EdDSA), key ID, content type (`application/vnd.mojave.audit.chain-head+json`), CWT issued-at timestamp.
- Payload is the canonical encoding of a `ChainHeadSnapshot` (tip hash + sequence number + optional model hash).
- One CBOR attestation file per sequence number, stored in `attestations/`.

Key management via `KeyRef` enum: in-memory PKCS#8, file path (PEM or DER auto-detect), or environment variable.

---

## 2. Trust Model Assessment

The audit chain makes six implicit security claims. I evaluate each against the actual implementation.

### Claim 1: Tamper evidence (retroactive modification detection)

**Status: SOUND, with caveats.**

The SHA-256 hash chain with domain-tagged construction is correctly implemented. The `ChainVerifier` checks: entry hash recomputation, parent hash linkage, sequence continuity, genesis presence, duplicate genesis detection, and canonical encoding failures. Property tests confirm honestly-constructed chains verify clean for lengths 1-50. Golden canonical tests pin the encoding output.

**Caveat:** The verifier operates on a `Vec<SealedAuditEntry>` loaded entirely into memory. For chains exceeding ~10M entries, this will OOM. The current maximum observed chain length is ~12,700 entries (WMDP Phase 1), so this is not an immediate problem, but the architecture assumes bounded chain length.

### Claim 2: Origin attestation (entry provenance binding)

**Status: SOUND for what it covers; INCOMPLETE in scope.**

COSE_Sign1 attestation with Ed25519 is correctly implemented using `ed25519-dalek` 2.x and `coset` 0.3. The `verify_detached_attestation` function properly checks: algorithm, key ID presence, content type, detached payload, unprotected header emptiness, critical headers, and signature verification against a keyring.

**Incompleteness:** The attestation covers the `ChainHeadSnapshot` (tip hash + seq), not individual entries. This means the attestation proves "at time T, the chain tip was hash H at sequence S" -- it does not prove that any individual entry's content is authentic, only that the chain state at a given point was attested. An attacker who replaces the entire chain with a consistent but fabricated chain can produce valid attestations for the fabricated chain, provided they hold the signing key.

### Claim 3: Model identity binding (chain-to-model linkage)

**Status: SOUND for structured descriptors; WEAK for weight files.**

Genesis sentinel correctly binds the chain to a `ModelIdentity` with a non-zero 32-byte hash. The `ModelHashMethod` enum distinguishes `WeightFile` (hash of actual model weights) from `StructuredDescriptor` (hash of a structured description). Weight-file hashing would provide strong binding, but the current WMDP runs use `StructuredDescriptor`, which hashes the (name, provider, version, quantization) tuple -- trivially spoofable by anyone who knows the model metadata.

**Risk:** A `StructuredDescriptor` hash proves nothing about the actual model weights. Two different models with the same name/provider/version/quantization would produce identical hashes. This is acknowledged in the `ModelHashMethod` design but not enforced: there is no gating that requires `WeightFile` for production chains.

### Claim 4: Binary integrity (the tool producing the chain is authentic)

**Status: NOT IMPLEMENTED. This is the "theater" finding.**

FUTURE_WORK.md line 29-44 states this explicitly: "Binary signing -- REQUIRED BEFORE PRODUCTION. Signed envelopes from an unsigned binary is theater." No binary signing exists. No code signing, no reproducible builds, no SBOM, no binary hash self-verification.

**Impact:** The entire trust model rests on this foundation. If an adversary replaces the mojave binary, they can:
1. Produce valid-looking chains with fabricated data
2. Sign those chains with whatever key the deployment provides
3. The chains will verify clean

This is not an exotic attack. For a defense deployment where the binary runs on customer infrastructure, binary integrity is table stakes. Without it, the audit chain is a compliance checkbox, not a security control.

### Claim 5: Temporal ordering (events happened in claimed sequence)

**Status: PARTIALLY SOUND.**

The hash chain enforces sequence ordering (each entry's hash depends on all previous entries). However:

- Timestamps (`at` field) are self-reported by the system clock. No trusted timestamping (RFC 3161), no NTP verification, no monotonic clock enforcement in the chain itself. The `monotonic_ns` field is optional and not used in hash computation.
- The chain does not prevent withholding: an attacker who controls the binary can accumulate events and emit them in any order, as long as the final chain is internally consistent.
- There is no external witnessing (no transparency log, no Merkle inclusion proof that proves the chain existed at a given time).

### Claim 6: Data integrity (the chain accurately records what happened)

**Status: CONTINGENT on Claim 4.**

The chain records whatever the emitting code tells it. The `AuditGate` pattern (must-resolve guard that panics in debug mode if dropped without resolution) is a clever enforcement mechanism -- it forces callers to emit an audit event for gated operations. But this only works if the caller code is the authentic mojave binary. If the binary is replaced, the gate is gone.

---

## 3. Python-Rust Parity Analysis

### The contract

The Python `audit.py` reimplements three things:
1. Canonical JSON encoding (`canonical_json()`)
2. Hash computation (`SHA256(DOMAIN_TAG || canonical_json(base) || parent_hash)`)
3. Chain JSONL format (sealed entries with base, parent_hash, entry_hash)

### Parity bugs found

**Bug 1: Genesis parent hash divergence.**

Rust genesis uses `compute_genesis_hash(base, model_identity.hash)` -- the "parent" position in the hash is filled by the model identity hash, not a sentinel. The genesis `SealedAuditEntry::Genesis` variant has no `parent_hash` field.

Python genesis uses `GENESIS_SENTINEL = bytes(32)` (32 zero bytes) as the parent hash. It does not incorporate a model identity hash at all. It has no genesis concept -- the first entry is just a regular entry with a null parent.

**Consequence:** A chain started by Python and verified by the Rust `ChainVerifier` will fail. The Rust verifier expects `SealedAuditEntry::Genesis` at index 0 with a `model_identity` field and a hash computed using the model hash. The Python writer produces `{"base": ..., "parent_hash": null, "entry_hash": ...}` which Rust will try to deserialize as either `Genesis` or `Chained` via the `#[serde(tag = "type")]` discriminator. The Python output has no `"type"` field, so deserialization will fail.

**Severity:** HIGH. These are not interoperable chains. The Python chain is self-consistent but the Rust verifier cannot consume it.

The `test_audit.py::TestRustVerification::test_rust_verifier_accepts_python_chain` test is supposed to catch this, but it requires a built mojave binary and uses `pytest.skip` if none is found. It is unclear whether this test has ever passed against the current codebase (post-genesis-sentinel merge).

**Bug 2: Envelope format divergence.**

Rust serializes `SealedAuditEntry` with a `"type"` tag (via `#[serde(tag = "type")]`): `{"type":"Genesis","base":{...},"model_identity":{...},"entry_hash":[...]}` or `{"type":"Chained","base":{...},"parent_hash":[...],"entry_hash":[...]}`.

Python serializes as: `{"base":{...},"parent_hash":[...]|null,"entry_hash":[...]}`. No `"type"` field. No `"model_identity"` field.

These are different formats. The Rust deserializer for `SealedAuditEntry` will reject Python-produced entries.

**Bug 3: Hash byte serialization divergence.**

Rust serializes `[u8; 32]` arrays as JSON arrays of integers by default (via serde's array serialization). The Python writer serializes hash bytes as `list(bytes)` -- also arrays of integers. This part is actually compatible.

However, the `AuditEntry` struct in Rust uses `[u8; 16]` for `trace_id` with a custom hex serializer (via `hex_16` module in audit-events). If the Python writer ever includes a trace_id, the format will diverge (Python would write an array of integers, Rust expects a 32-char hex string).

**Bug 4: Optional field handling divergence.**

Rust uses `#[serde(skip_serializing_if = "Option::is_none")]` on `monotonic_ns`, `trace_id`, `resource`, and `blob_ref`. When these are `None`, they are omitted from the JSON output.

Python also omits `resource` and `tags` when not provided. However, Rust's `AuditEntry` always includes `monotonic_ns: None` in the canonical encoding (because the canonical encoder serializes the struct, and `Option::None` serializes as `null` via serde before `skip_serializing_if` applies -- but wait, `skip_serializing_if` happens at serde level, so `None` fields are actually omitted from `serde_json::to_value()` output too). The canonical encoder receives the serde `Value` representation, which already has `None` fields omitted. So this is compatible, but fragile -- any change to the `skip_serializing_if` annotations on either side will silently break hash parity.

### Assessment

The Python-Rust parity contract is broken at the envelope format level (no `"type"` tag, no `model_identity` in Python genesis). At the canonical encoding level, the implementations appear compatible for the overlapping feature set. The cross-language verification test is likely skipped in practice due to requiring a compiled binary.

**Recommendation:** Either (a) drop the Python writer and have Python call the Rust binary via subprocess for chain operations, or (b) add a shared golden test vector file (JSONL with known hashes) that both implementations must produce/verify, and update the Python writer to produce genesis-sentinel-compatible entries with the tagged union format.

---

## 4. Sigstore / Model-Transparency Assessment

### What Sigstore provides

Sigstore's model-transparency v1.0 (Google/OpenSSF) offers:
- **Keyless signing** via OIDC + Fulcio short-lived certificates (no PKI to manage)
- **Transparency log** via Rekor (append-only tamper-evident log with Merkle inclusion proofs)
- **Model signing** (sign model artifacts, verify provenance)
- **Standard tooling** (cosign, sigstore-python, sigstore-go)

### What mojave's chain provides that Sigstore does not

1. **Per-entry granularity.** Sigstore signs artifacts (model files, containers). Mojave's chain records per-evaluation-event provenance. Signing 12,000+ eval events via Sigstore would require 12,000+ Rekor entries, which is technically possible but operationally unusual.

2. **Sequential binding.** The hash chain proves temporal ordering of events within a campaign. Sigstore's Rekor provides a global ordering across all signers, but does not enforce sequential dependencies between entries.

3. **Model identity binding at genesis.** The genesis sentinel binds the chain to a specific model hash. Sigstore can sign a model artifact, but does not provide the "this chain of eval events pertains to this model" binding.

4. **Domain-specific event semantics.** The audit event vocabulary (21 event kinds), principal/resource/authorization/outcome structure, and the AuditGate enforcement pattern are domain-specific to eval provenance. Sigstore is infrastructure-generic.

### What Sigstore provides that mojave's chain does not

1. **Binary signing.** Sigstore can sign the mojave binary itself, solving the "theater" problem directly. This is the single most impactful adoption.

2. **External witnessing.** Rekor provides third-party proof-of-existence timestamps. Without this, mojave's chain timestamps are self-asserted.

3. **Keyless operation.** mojave's current `KeyRef::Env` pattern puts a long-lived signing key in an environment variable. Sigstore's keyless model with short-lived certificates is strictly superior for key management.

4. **Ecosystem trust.** Sigstore is trusted by PyPI, Maven Central, Homebrew, NVIDIA NGC. Adopting Sigstore-compatible signatures gives mojave interoperability with the software supply chain ecosystem that defense customers already participate in.

### Recommendation: Layered adoption, not replacement

Sigstore should supplement, not replace, the custom audit chain. Specifically:

**Tier 1 (unblocks production):** Sign mojave release binaries with Sigstore. This solves the "theater" finding. Implementation: add `sigstore-go` or `cosign` to the CI/CD pipeline. Estimated effort: 1-2 days.

**Tier 2 (adds external witnessing):** Submit periodic chain-head snapshots to Rekor as signed attestations. This provides third-party proof-of-existence for the chain state. Implementation: a `rekor-witness` subcommand that takes the current chain head snapshot, signs it with Sigstore, and stores the Rekor log entry index. Estimated effort: 1-2 weeks.

**Tier 3 (optional, for model identity):** Use Sigstore model-transparency to sign model artifacts, and include the Sigstore bundle hash in the genesis sentinel's `ModelIdentity.hash` field (using `ModelHashMethod::WeightFile` with the hash being the Sigstore bundle hash). This replaces the weak `StructuredDescriptor` binding with a cryptographically strong model-to-chain binding. Estimated effort: depends on model signing workflow.

**Do not:** Replace the custom per-event hash chain with Sigstore. The chain's value is in its sequential, per-event granularity -- Sigstore is not designed for this use case.

---

## 5. Kao2025 Constant-Size Evidence Assessment

Kao et al. (Nov 2025) propose constant-size cryptographic evidence structures that compose with hash chains and Merkle trees. Each evidence item has O(1) storage and verification cost, under collision-resistant hashing and EUF-CMA signatures.

### Relevance to mojave

Mojave's current attestation is O(1) per snapshot (one COSE_Sign1 per chain tip), but verification of the chain itself is O(n) in chain length. For defense customers running multi-month campaigns with 100k+ entries, chain verification becomes expensive.

Kao's construction could provide:
1. **Constant-size proofs of inclusion** -- prove a specific entry is in the chain without transmitting the entire chain.
2. **Composable evidence** -- combine evidence from multiple chains (e.g., multiple model evaluations) into a single attestation.

### Assessment

The Kao construction is theoretically attractive but practically premature for mojave:
- Current chain lengths (< 15k entries) do not create verification bottlenecks.
- The construction requires a Merkle tree over the chain, which would change the chain format.
- The formal security definitions would be useful for mojave's threat model documentation but don't change what needs to be built immediately.

**Recommendation:** Read Kao2025 for security definitions and cite in mojave's threat model documentation. Defer Merkle tree integration until chain lengths exceed 100k or until a customer requires proof-of-inclusion for individual entries. Design the chain format to be forward-compatible with Merkle accumulators by ensuring entry hashes are computed independently of the Merkle structure.

---

## 6. Threat Model Gaps

Mapping what the audit chain currently provides against what genuine tamper-evidence requires:

| Property | Status | Gap |
|----------|--------|-----|
| Retroactive modification detection | IMPLEMENTED | None within the hash chain itself |
| Binary integrity verification | NOT IMPLEMENTED | Binary signing required (FUTURE_WORK.md acknowledges) |
| External witnessing / proof-of-existence | NOT IMPLEMENTED | No transparency log, no RFC 3161 timestamping |
| Key management for defense deployments | INSUFFICIENT | `KeyRef::Env` is unacceptable under NIST 800-171 / CMMC; need HSM or at minimum encrypted-at-rest key file with access logging |
| Cross-language chain interoperability | BROKEN | Python-Rust format divergence (see Section 3) |
| Model identity binding (strong) | PARTIAL | `StructuredDescriptor` is spoofable; `WeightFile` exists but is unused |
| Chain-to-evidence composition | NOT IMPLEMENTED | No Merkle proofs, no constant-size evidence |
| Attestation scope | LIMITED | Attests chain tip only, not individual entries |
| Replay / rollback prevention | NOT IMPLEMENTED | No monotonic counter, no sequence number pinning in external storage |
| Multi-chain correlation | NOT IMPLEMENTED | No mechanism to prove two chains pertain to the same campaign |
| Audit of the auditor | NOT IMPLEMENTED | No logging of chain operations themselves (key load, verify, gc) |

### Priority ordering for defense deployment

1. **Binary signing** -- blocks everything. Without it, the chain is unfalsifiable but also unverifiable.
2. **Key management upgrade** -- `KeyRef::Env` to `KeyRef::Hsm` or at minimum `KeyRef::EncryptedFile` with OS keychain integration.
3. **Python-Rust parity fix** -- either fix the Python writer or remove it and have Python call the Rust binary.
4. **External witnessing** -- Rekor integration for proof-of-existence timestamps.
5. **Model identity binding** -- enforce `WeightFile` hash method for production chains.

---

## 7. Strengths Worth Preserving

The audit chain has several genuinely strong design choices that should not be lost in a rush to address gaps:

1. **Domain-tagged hashing** (`b"mojave-audit-v1\0"`) prevents cross-protocol hash reuse attacks. This is a security-conscious choice that many custom chain implementations omit.

2. **Float rejection in canonical encoding** eliminates the most common source of cross-platform canonicalization divergence. IEEE 754 floating-point serialization is notoriously implementation-dependent; rejecting it entirely is the correct choice.

3. **AuditGate must-resolve pattern** is a compile-time enforcement mechanism that prevents "forgot to log" bugs. The debug-mode panic on unresolved gates catches errors during development. This is clever and rare.

4. **Genesis sentinel with model identity binding** makes the chain purpose-specific. A generic hash chain could be repurposed; the genesis sentinel says "this chain is about evaluating model X" and makes that unforgeable (given trust in the binary).

5. **COSE_Sign1 with CWT claims** follows IETF standards rather than inventing a custom envelope format. This makes the attestations interoperable with any COSE/CWT-aware verifier.

6. **File-level locking** (`fs2::FileExt::lock_exclusive`) in the Emitter prevents concurrent corruption. Simple, correct, appropriate for single-machine deployment.

7. **Crash recovery** via `audit-recover` replay handles truncated last lines gracefully. This is the right robustness choice for append-only logs.

---

## 8. Canonical Encoding Risk Assessment

The canonical JSON encoder is load-bearing: any change to encoding semantics silently breaks all existing chain verification. This deserves specific analysis.

### Current specification (implicit, not documented)

- Object keys sorted lexicographically by Rust `String::cmp` (UTF-8 byte order)
- No whitespace between tokens
- Integers as decimal strings
- Floats rejected (error on non-integer `Number`)
- Strings escaped: `\\"`, `\\\\`, `\\b`, `\\f`, `\\n`, `\\r`, `\\t`, `\\uXXXX` for control chars < 0x20, all other chars passed through (including non-BMP Unicode)
- Null as `null`, booleans as `true`/`false`
- Arrays preserve insertion order

### Risks

1. **Sort order is UTF-8, not UTF-16 (JCS).** The golden test `golden_supplementary_plane_keys_sort_by_utf8` explicitly documents that this differs from JSON Canonicalization Scheme (RFC 8785). If mojave ever needs to interoperate with JCS-compliant systems, the sort order will diverge for keys containing supplementary-plane characters. Current keys are all ASCII, so this is theoretical.

2. **Integer range.** The encoder handles `i64::MIN` and `u64::MAX` correctly (golden tests confirm). However, `serde_json` internally represents numbers as either `i64`, `u64`, or `f64`. A number like `2^53 + 1` that fits in `u64` but not `f64` will be preserved, but a `Value::Number` constructed from a string representation might round-trip differently. The float rejection gate catches the dangerous case.

3. **No explicit specification document.** The encoding is defined by the code, not by a specification. If the encoding ever needs to be reimplemented (e.g., in Go for a customer's verification tool), the implementer must reverse-engineer the behavior from the Rust source and golden tests. **Recommendation:** Write a formal encoding specification, even if it's just a single page.

4. **Serde version sensitivity.** The canonical encoder takes a `serde_json::Value`, which means the encoding depends on `serde_json`'s `to_value()` serialization of Rust structs. Changes to serde's serialization of `DateTime<Utc>`, `[u8; 32]`, or `BTreeMap` would silently change canonical output. The `chrono` datetime serialization format is particularly fragile -- `chrono::serde` has changed formats between versions. **Mitigation in place:** The golden canonical tests pin specific outputs, so serde changes would break tests. But the tests only cover simple cases; a chrono version bump that changes nanosecond formatting would break chain verification for existing chains before tests catch it, if the test fixtures don't include datetime fields.

---

## 9. Recommendations

### Immediate (before defense deployment)

1. **Sign release binaries with Sigstore cosign.** Add to CI/CD. This closes the "theater" gap. 1-2 days effort.

2. **Fix Python-Rust parity or remove Python writer.** The format divergence means the Python writer produces chains that the Rust verifier cannot consume. Either update `audit.py` to emit the tagged-union genesis format with model identity, or deprecate the Python writer and have `scripts/v2/` call `mojave audit emit` via subprocess. Decision should favor the subprocess approach (single source of truth for chain format).

3. **Write a canonical encoding specification.** One page. Pin the sort order, escaping rules, float rejection, and integer representation. Reference it from the chain verification documentation.

4. **Run the cross-language verification test in CI.** The `test_rust_verifier_accepts_python_chain` test in `test_audit.py` currently silently skips if no binary is built. It must run in CI against the release binary, every merge. If the Python writer is kept, this test is the only thing preventing silent format drift.

### Near-term (before first external customer)

5. **Upgrade key management.** Replace `KeyRef::Env` with `KeyRef::EncryptedFile` (using OS keychain or a KMS) for production deployments. `KeyRef::Env` is acceptable for development but not for NIST 800-171 compliance.

6. **Add Rekor witnessing.** Submit chain-head snapshots to Rekor periodically (e.g., at chain close, or every N entries). Store Rekor log entry index alongside the CBOR attestation. This provides external proof-of-existence.

7. **Enforce `WeightFile` hash method for production chains.** Add a configuration flag or emitter option that rejects `StructuredDescriptor` in production mode. The structured descriptor provides no model integrity guarantee.

### Strategic (design for, don't build yet)

8. **Merkle accumulator compatibility.** Ensure entry hashes can be used as leaves in a future Merkle tree without recomputation. The current `compute_chained_hash` construction is compatible because it produces a standalone 32-byte digest per entry. No changes needed now, but do not change the hash construction without considering this.

9. **Multi-chain campaigns.** When mojave evaluates multiple models, it creates separate chains per model (via the model hash prefix in the audit directory). There is no mechanism to prove that two chains were produced during the same campaign by the same mojave instance. A "campaign root" entry that is shared across chains would provide this. Design it; don't build it until a customer needs it.

---

## 10. Verdict

The audit chain is well-engineered at the Rust level. The canonical encoder, hash chain construction, COSE_Sign1 attestation, and AuditGate enforcement pattern are all competently implemented with good test coverage. The trust model has a genuine foundation.

But the trust model has a load-bearing gap: **binary signing**. Without it, the chain proves internal consistency of whatever the binary produced, but cannot prove the binary was authentic. For a defense customer, this is the difference between a security control and a compliance artifact.

The Python-Rust parity contract is broken at the format level and should be fixed or the Python writer should be retired. The key management story needs upgrading for production. External witnessing (Rekor) would strengthen the temporal claims. Sigstore should be adopted for binary signing and optionally for model identity binding, but should not replace the custom per-event chain.

The path from current state to genuine tamper-evidence is short -- maybe 2-3 weeks of focused work. The architecture is sound; the gaps are at the deployment boundary, not in the cryptographic construction.
