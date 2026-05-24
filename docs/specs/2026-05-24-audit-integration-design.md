# Design: Audit Chain Integration into Run Card Pipeline

**Date:** 2026-05-24
**Status:** Draft
**Crates affected:** `audit-chain`, `audit-sign`, `mojave-cli` (new `audit` subcommand)
**Scripts affected:** `scripts/arc-workup/generate_run_cards.py`

## Problem

Run cards currently use a toy hash: `hashlib.sha256(data).hexdigest()[:16]`. Truncated,
unchained, unsigned. A run card without a verifiable hash is a claim without evidence.
A hash without a disciplined chain and cryptographic attestation behind it is theater.

This spec covers two things in strict dependency order:

1. **Validate and harden the audit primitives** (`audit-chain`, `audit-sign`) against
   protocol standards. Earn the right to use them.
2. **Wire the hardened primitives into the run card pipeline** so every run card is
   hash-chained and attestable.

## Non-Goals

- Writing our own cryptography. Ed25519 via `ed25519-dalek`, COSE via `coset`,
  SHA-256 via `sha2`. All community-maintained, audited crates.
- Full PKI / certificate authority. Key management is local PKCS#8 files for now.
- Transparency log / Merkle tree. Linear chain is sufficient for single-evaluator
  audit trail. Merkle tree is a future extension if multi-party verification is needed.
- Real-time streaming audit. The chain is constructed at pipeline completion time.

---

## Part 1: Audit Primitive Hardening

### 1.1 Protocol Gap Analysis

Source material: RFC 8785 (JCS), RFC 9052 (COSE), RFC 8032 (EdDSA), RFC 9162 (CT v2),
RFC 8949 (CBOR), RFC 6962 (CT v1), RFC 9338 (COSE Countersigning), RFC 9597 (CWT Claims
in COSE Headers). All downloaded to `../evals_papers/`.

#### 1.1.1 Hash Chain Construction (`audit-chain/src/seal.rs`)

**Current:** `SHA-256(canonical_bytes || parent_hash)`, with genesis entries omitting
the parent_hash entirely (conditional branch, shorter input).

**Findings:**

| Issue | Severity | Reference |
|-------|----------|-----------|
| No domain separation tag | High | RFC 9162 §2.1 — CT uses `0x00`/`0x01` byte prefixes to prevent second preimage attacks between leaf and internal node hashes |
| Genesis entry has structurally different hash input | Medium | Bitcoin uses `[0u8; 32]` sentinel; Hyperledger uses protobuf null. Variable-length input is an implicit rather than explicit domain separator |
| No version binding in hash construction | Medium | Future hash construction changes would produce ambiguous entries without versioned prefix |
| Concatenation boundary not length-prefixed | Low | PASETO PAE uses LE64 length prefixes. Mitigated here because parent_hash is always exactly 32 bytes (fixed-size suffix) |

**Fix — new `compute_entry_hash`:**

```
SHA-256(b"mojave-audit-v1\x00" || canonical_bytes(entry) || parent_hash)
```

Where:
- Domain tag is the literal 16 bytes `mojave-audit-v1\0` (null-terminated, fixed length)
- `parent_hash` is `[0u8; 32]` for genesis entries (sentinel, not `None`)
- All entries have identical hash input structure: 16 + len(canonical) + 32 bytes

This is a **breaking change** to all existing chain hashes. Acceptable because no
production chains exist yet.

#### 1.1.2 Canonical JSON Encoding (`audit-chain/src/canonical.rs`)

**Current:** Custom deterministic JSON — sorted keys, integer-only numbers, JCS-matching
string escaping, zero whitespace.

**Findings:**

| Feature | Our Scheme | RFC 8785 (JCS) | Assessment |
|---------|-----------|----------------|------------|
| Key sort order | UTF-8 byte order (Rust `String::cmp`) | UTF-16 code unit order | Divergent for non-BMP chars. All our keys are ASCII — equivalent in practice |
| Float handling | Rejected with error + path | ECMAScript `Number::toString` | Strictly stronger. Eliminates entire class of float-to-string bugs |
| Integers > 2^53 | Accepted (native u64) | Must be strings | More permissive. We own both ends |
| String escaping | Matches JCS §3.2.2.2 | Matches | Identical |
| Lone surrogates | Impossible (Rust `String`) | Must error | Equivalent by type system |
| Whitespace | Zero | Zero | Identical |
| Duplicate keys | BTreeMap dedup (last wins) | Must error | Acceptable — we construct programmatically |

**Decision:** Do NOT claim JCS compliance. Document as "mojave canonical JSON" with
explicit divergences. The integer-only policy and float rejection are strengths, not
weaknesses.

**Required changes:**
- Add ADR documenting the canonical encoding scheme and its relationship to JCS
- Add golden-file tests pinning exact byte output for known inputs (guards against
  future serde behavior changes)
- Add a test with supplementary-plane Unicode keys documenting the sort order
  difference (and proving it's irrelevant for our ASCII-key schema)

#### 1.1.3 COSE_Sign1 Attestation (`audit-sign/src/attestation.rs`)

**Current:** Custom header labels 999 (content_type) and 1000 (signed_at) in the
IANA "Specification Required" range (256-65535). Detached payload via `Vec::new()`.

**Findings:**

| Issue | Severity | Reference |
|-------|----------|-----------|
| Label 999 for content_type — standard label is 3 | High | RFC 9052 §3.1, IANA COSE Header Parameters registry |
| Label 1000 for timestamp — should use CWT Claims (label 15) with `iat` (key 6) | High | RFC 9597 — standardized mechanism for timestamps in COSE headers |
| Labels 999/1000 in IANA-managed range — could be assigned to future specs | Medium | IANA registry: 256-65535 requires "Specification Required" |
| `Vec::new()` (empty bstr `0x40`) vs CBOR null (`0xf6`) for detached payload | Medium | RFC 9052 §4.4 — nil CBOR value for detached. Need to verify `coset` serialization |
| No algorithm allowlist check in verifier | Medium | Algorithm confusion attacks — verifier should check `protected.alg` before dispatching |
| No `crit` header processing | Low | RFC 9052 §3.1 — must reject messages with `crit` labels we don't understand |
| Timestamp as RFC 3339 string, not epoch seconds | Low | CWT/CBOR convention is epoch seconds (NumericDate) |

**Fixes:**

1. **Replace label 999 → standard label 3.** Use `HeaderBuilder::content_type()` method
   in `coset`. Value remains `"application/vnd.mojave.audit.chain-head+json"`.

2. **Replace label 1000 → CWT Claims (label 15) with `iat` (key 6).** Epoch seconds
   as CBOR integer, not RFC 3339 string.

   ```rust
   // Protected header construction
   let cwt_claims = ciborium::Value::Map(vec![
       (ciborium::Value::Integer(6.into()),
        ciborium::Value::Integer(epoch_seconds.into())),
   ]);
   HeaderBuilder::new()
       .algorithm(cose_alg(signer.algorithm()))
       .key_id(signer.key_id().as_bytes().to_vec())
       .content_type(CONTENT_TYPE_VALUE.to_string())
       .value(15, cwt_claims)
       .build()
   ```

3. **Add algorithm allowlist** in `verify_detached_attestation`. Check
   `protected.header.alg` is `Some(EdDSA)` before attempting verification.

4. **Add `crit` header check.** Reject if `crit` is present (we don't process
   any critical extension labels).

5. **Verify nil vs empty bstr** in `coset` output. Add a byte-level test that
   inspects the CBOR encoding of the payload position. If `coset` emits `0x40`
   instead of `0xf6`, file upstream or wrap.

#### 1.1.4 Missing TCK: audit-sign

No Gherkin feature files exist for `audit-sign`. Required before integration:

- `tck/audit-sign/features/attestation.feature` — sign/verify round-trip, tampered
  payload rejection, unknown key rejection, detached payload semantics
- `tck/audit-sign/features/key_management.feature` — PKCS#8 DER/PEM loading,
  env var loading, garbage rejection, SPKI round-trip
- `tck/audit-sign/features/cose_compliance.feature` — standard header labels,
  CWT Claims structure, algorithm in protected header, empty unprotected headers,
  `crit` rejection

### 1.2 Validation Gates for Audit Primitives

Per `docs/reference/validation-4-gate.md`:

**Gate 1 — Textbook Reproductions:**
- RFC 9162 Appendix test vectors for domain-separated hashing (adapted to linear chain)
- RFC 8785 §B test vectors for canonical JSON (the ones applicable to integer-only mode)
- COSE WG test vectors for Sign1 from `cose-wg/Examples` repo

**Gate 2 — Reference Implementation Cross-Checks:**
- Python `canonicaljson` library for canonical encoding agreement on ASCII inputs
- `pycose` library for COSE_Sign1 construction — cross-verify that our Rust output
  can be verified by `pycose` and vice versa
- `ed25519-dalek` test vectors already in upstream crate

**Gate 3 — Property-Based Tests:**
- Canonical encoding determinism: `encode(x) == encode(x)` for all valid inputs
- Hash chain monotonicity: `seq(entry[i+1]) == seq(entry[i]) + 1`
- Chain integrity: `verify(chain) == clean` for any honestly-constructed chain
- Attestation binding: `verify(attest, payload_a) == ok` implies
  `verify(attest, payload_b) == err` for `a != b`
- Domain separation: `hash(entry, parent=sentinel) != hash(entry, parent=None)`
  where sentinel is `[0u8; 32]` (this verifies the old scheme differs from the new)

**Gate 4 — Monte-Carlo Calibration:**
- Not applicable for cryptographic primitives (SHA-256 collision resistance is not
  something we calibrate). Gate 4 is satisfied by upstream crate validation.

---

## Part 2: Pipeline Integration

### 2.1 Architecture

```
Python pipeline                          Rust engine
─────────────────                        ──────────────
extract_results.py                       
    ↓                                    
analyze_results.py                       
    ↓                                    
generate_run_cards.py ──── subprocess ──→ mojave audit seal
    ↓                        stdin: JSON      ↓
    ↓                        stdout: JSON  audit-chain (ChainHead::link)
    ↓                                      audit-sign  (build_tip_attestation)
    ↓                                         ↓
    ↓                    ←── stdout ────── { chain_tip, attestation_cbor,
    ↓                                        entry_hashes, ... }
    ↓
  writes runcard-config.tex with real hashes
    ↓
  pdflatex (2 passes)
```

The Python pipeline calls a Rust binary (`mojave audit seal`) via subprocess. No
Python crypto, no ephemeral Python-to-Python hash passing. The Rust engine is the
single source of truth for all hash chain and signing operations.

### 2.2 `mojave audit seal` Subcommand

New subcommand on `mojave-cli`. Reads pipeline data from stdin, produces audit
artifacts to stdout.

**Input** (JSON on stdin):
```json
{
  "run_id": "MOJAVE-2026-0519-ARC",
  "eval_name": "arc_challenge",
  "date_issued": "2026-05-19",
  "data_file": "data/analysis/arc_challenge_analysis.json",
  "data_sha256": "<full hex digest of data file>",
  "actor": {
    "kind": "System",
    "id": "generate_run_cards.py"
  }
}
```

**Processing:**
1. Compute SHA-256 of the referenced data file (verifying `data_sha256` matches)
2. Create an `AuditEntry` recording the run card generation event
3. Link into the audit chain via `ChainHead::link()`
4. If a signing key is available (`MOJAVE_AUDIT_KEY` env or `--key-file`),
   produce a COSE_Sign1 attestation of the chain tip
5. Persist the chain state to `data/audit/chain.jsonl`
6. Output results as JSON to stdout

**Output** (JSON on stdout):
```json
{
  "chain_tip_hash": "<64-char hex>",
  "chain_tip_seq": 42,
  "entry_hash": "<64-char hex>",
  "data_file_hash": "<64-char hex, full, NOT truncated>",
  "attestation_cbor_b64": "<base64 COSE_Sign1, omitted if no key>",
  "verifying_key_spki_b64": "<base64 SPKI DER, omitted if no key>"
}
```

**Chain persistence:**
- Chain entries stored as one-JSONL-per-line in `data/audit/chain.jsonl`
- Chain head state (tip hash, next seq) in `data/audit/chain-head.json`
- Attestation envelopes in `data/audit/attestations/<seq>.cbor`
- Public key in `data/audit/pubkey.spki.der`

### 2.3 `mojave audit verify` Subcommand

Verifies an existing chain and its attestations.

```
mojave audit verify [--chain data/audit/chain.jsonl]
```

- Replays the entire chain via `ChainVerifier::verify()`
- For each attestation in `data/audit/attestations/`, verifies the COSE_Sign1
  signature against the public key and the corresponding chain tip snapshot
- Reports: entries verified, findings (if any), attestation status

### 2.4 Run Card Integration Points

**`generate_run_cards.py` changes:**

1. Before generating any config files, compute the full SHA-256 of each data file
   (not truncated)
2. For each eval, call `mojave audit seal` via subprocess with the run metadata
3. Receive back the chain tip hash, entry hash, and attestation
4. Write the **full** hash into `runcard-config.tex`:
   ```latex
   \rcset{companion.hash}    {\texttt{sha256:<full 64-char hex>}}
   \rcset{audit.chain.tip}   {\texttt{<full 64-char hex>}}
   \rcset{audit.chain.seq}   {42}
   \rcset{audit.signed}      {Yes — Ed25519 COSE\_Sign1}
   ```
5. Cross-eval summary gets the same treatment — it references the chain tip
   that includes all individual eval entries

**New `\rcset` keys in `runcard-config.tex`:**

| Key | Value | Description |
|-----|-------|-------------|
| `companion.hash` | Full SHA-256 hex | Hash of the companion data file |
| `audit.chain.tip` | Full SHA-256 hex | Hash chain tip at time of run card generation |
| `audit.chain.seq` | Integer | Sequence number in the audit chain |
| `audit.signed` | Yes/No + algorithm | Whether a cryptographic attestation was produced |

**New section in `runcard.tex` engine** (optional — only renders if `audit.chain.tip`
is non-empty):

```
Audit Trail
  Chain tip:   \rc{audit.chain.tip}
  Chain seq:   \rc{audit.chain.seq}
  Signed:      \rc{audit.signed}
  Verify:      mojave audit verify --chain data/audit/chain.jsonl
```

### 2.5 Template Engine Changes

The `runcard.tex` engine needs a new section for audit trail metadata. This follows
the existing pattern: `\rcset{key}{value}` in config, `\rc{key}` in engine. If the
audit keys are empty, the section renders em-dashes per the existing empty-field
convention.

### 2.6 Unsigned Mode

Not every run needs a signing key. The pipeline must work in two modes:

1. **Signed mode** (`MOJAVE_AUDIT_KEY` set or `--key-file` provided): Full chain +
   COSE_Sign1 attestation. Run card shows "Yes — Ed25519 COSE_Sign1".
2. **Unsigned mode** (no key): Chain only, no attestation. Run card shows
   "No — chain only (unsigned)". The hash chain still provides tamper evidence;
   the attestation provides non-repudiation.

Both modes produce a full (not truncated) SHA-256 hash of the data file.

---

## Part 3: Implementation Sequence

Strict dependency order. Each phase gates on the previous.

### Phase 1: Harden `audit-chain`

1. Add domain separation tag to `compute_entry_hash`
2. Use `[0u8; 32]` sentinel for genesis parent_hash
3. Update all existing tests for new hash values
4. Add golden-file tests for canonical encoding
5. Add supplementary-plane Unicode key sort test
6. Write ADR for canonical encoding scheme
7. Update `ChainVerifier` for new hash construction

### Phase 2: Harden `audit-sign`

1. Replace header labels 999/1000 with standard labels (3, 15)
2. Switch timestamp to epoch seconds via CWT Claims
3. Add algorithm allowlist check in verifier
4. Add `crit` header rejection
5. Verify nil vs empty bstr for detached payload
6. Write TCK feature files for `audit-sign`
7. Cross-verify with `pycose` (Gate 2)

### Phase 3: `mojave audit` CLI

1. Add `audit seal` subcommand to `mojave-cli`
2. Add `audit verify` subcommand
3. Chain persistence layer (JSONL + head state)
4. Attestation file management
5. TCK feature file for CLI audit commands

### Phase 4: Pipeline Integration

1. Update `generate_run_cards.py` to call `mojave audit seal`
2. Add new `\rcset` keys to config template
3. Add audit trail section to `runcard.tex` engine
4. Update cross-eval summary similarly
5. End-to-end test: generate → seal → verify → build PDF

---

## Appendix A: Crate Dependencies

No new external crates required. All cryptographic operations use existing deps:

- `sha2` (SHA-256) — already in `audit-chain`
- `ed25519-dalek` (Ed25519) — already in `audit-sign`
- `coset` (COSE) — already in `audit-sign`
- `ciborium` (CBOR) — already in `audit-sign`
- `rand_core` (key generation) — already in `audit-sign`

## Appendix B: RFC Reference Index

| RFC | Title | Relevance |
|-----|-------|-----------|
| 8785 | JSON Canonicalization Scheme | Canonical encoding baseline (we diverge intentionally) |
| 9052 | COSE Structures and Process | COSE_Sign1 construction, Sig_structure, header labels |
| 8032 | Edwards-Curve Digital Signature Algorithm (EdDSA) | Ed25519 signing (via `ed25519-dalek`) |
| 9162 | Certificate Transparency Version 2.0 | Domain separation pattern (`0x00`/`0x01` prefixes) |
| 8949 | Concise Binary Object Representation (CBOR) | Deterministic encoding for COSE structures |
| 6962 | Certificate Transparency (v1) | Original domain separation motivation |
| 9338 | COSE Countersignatures | Future extension reference |
| 9597 | CWT Claims in COSE Headers | Timestamp via label 15 + `iat` (key 6) |

## Appendix C: Security Model

**Threat model:** A single evaluator producing run cards for external consumption.
The audit chain provides:

1. **Tamper evidence** (hash chain): Any modification to a run card's companion data
   is detectable by re-hashing and comparing against the chain.
2. **Ordering evidence** (sequence numbers): The chain proves the order in which
   run cards were produced.
3. **Non-repudiation** (COSE_Sign1 attestation, when signed): The evaluator
   cannot deny having produced a specific run card, because the chain tip
   attestation binds the evaluator's key to the chain state.

**Not in scope:**
- Third-party timestamping authority (evaluator's clock is trusted for now)
- Distributed consensus on chain state
- Revocation of signing keys
- Multi-evaluator chains
