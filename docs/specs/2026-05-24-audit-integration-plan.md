# Audit Chain Integration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden the audit-chain and audit-sign crates against protocol standards (RFC 9162, 9052, 8785), then wire them into the run card pipeline so every run card is hash-chained and attestable.

**Architecture:** Two Rust crates (`audit-chain`, `audit-sign`) provide hash chain construction and COSE_Sign1 attestation. A new `mojave audit seal` CLI subcommand exposes these as a subprocess the Python pipeline calls. The Python pipeline (`generate_run_cards.py`) sends run metadata to the Rust engine and receives back full hashes and attestation envelopes for embedding into LaTeX run cards.

**Tech Stack:** Rust (sha2, ed25519-dalek, coset, ciborium), Python (subprocess), LaTeX (runcard.tex engine)

**Spec:** `docs/specs/2026-05-24-audit-integration-design.md`

---

## File Map

### Phase 1 — Harden `audit-chain`

| Action | File | Responsibility |
|--------|------|----------------|
| Modify | `crates/audit-chain/src/seal.rs` | Domain separation tag, sentinel genesis hash |
| Modify | `crates/audit-chain/src/verify.rs` | Update genesis check for sentinel pattern |
| Modify | `crates/audit-chain/src/canonical.rs` | Add golden-file tests only (no logic changes) |
| Create | `crates/audit-chain/tests/golden_canonical.rs` | Integration test with pinned byte outputs |
| Create | `docs/adr/0001-canonical-json-encoding.md` | ADR documenting encoding scheme vs JCS |

### Phase 2 — Harden `audit-sign`

| Action | File | Responsibility |
|--------|------|----------------|
| Modify | `crates/audit-sign/src/attestation.rs` | Standard COSE headers, verifier hardening |
| Create | `tck/audit-sign/features/attestation.feature` | Gherkin specs for sign/verify |
| Create | `tck/audit-sign/features/cose_compliance.feature` | Gherkin specs for COSE standards |

### Phase 3 — `mojave audit` CLI

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `crates/mojave-cli/src/commands/audit.rs` | `seal` and `verify` subcommands |
| Modify | `crates/mojave-cli/src/commands/mod.rs` | Register audit module |
| Modify | `crates/mojave-cli/src/main.rs` | Add `Audit` variant to `Commands` enum |
| Modify | `crates/mojave-cli/src/error.rs` | Add `Audit` variant to `CliError` |
| Modify | `crates/mojave-cli/Cargo.toml` | Add audit-chain, audit-sign, base64 deps |

### Phase 4 — Pipeline Integration

| Action | File | Responsibility |
|--------|------|----------------|
| Modify | `scripts/arc-workup/generate_run_cards.py` | Call `mojave audit seal`, use full hashes |
| Modify | `templates/run-card/single-run-card/runcard.tex` | Audit trail section |

---

### Task 1: Add domain separation to `compute_entry_hash`

Spec §1.1.1. Change hash construction from `SHA-256(canonical || parent)` to
`SHA-256(b"mojave-audit-v1\x00" || canonical || parent)` with `[0u8; 32]` sentinel
for genesis entries.

**Files:**
- Modify: `crates/audit-chain/src/seal.rs:78-89`
- Test: `crates/audit-chain/src/seal.rs` (inline tests)

- [ ] **Step 1: Write test for domain-separated hash**

Add this test at the end of the `mod tests` block in `crates/audit-chain/src/seal.rs`:

```rust
#[test]
fn domain_tag_is_included_in_hash() {
    let entry = sample_entry();
    let hash = compute_entry_hash(&entry, None).unwrap();
    // Manually compute expected: SHA-256(tag || canonical || sentinel)
    use sha2::{Digest, Sha256};
    let canonical = crate::canonical::encode(&entry).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(b"mojave-audit-v1\x00");
    hasher.update(&canonical);
    hasher.update([0u8; 32]);
    let expected: [u8; 32] = hasher.finalize().into();
    assert_eq!(hash, expected);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p audit-chain domain_tag_is_included_in_hash`
Expected: FAIL — the old `compute_entry_hash` doesn't prepend the tag or use sentinel.

- [ ] **Step 3: Update `compute_entry_hash`**

Replace lines 78-89 in `crates/audit-chain/src/seal.rs`:

```rust
const DOMAIN_TAG: &[u8] = b"mojave-audit-v1\x00";
const GENESIS_SENTINEL: [u8; 32] = [0u8; 32];

pub(crate) fn compute_entry_hash(
    base: &AuditEntry,
    parent_hash: Option<[u8; 32]>,
) -> Result<[u8; 32], CanonicalEncodingError> {
    let canonical = canonical::encode(base)?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_TAG);
    hasher.update(&canonical);
    hasher.update(parent_hash.unwrap_or(GENESIS_SENTINEL));
    Ok(hasher.finalize().into())
}
```

- [ ] **Step 4: Run new test to verify it passes**

Run: `cargo test -p audit-chain domain_tag_is_included_in_hash`
Expected: PASS

- [ ] **Step 5: Update existing tests for new hash semantics**

The test `different_parent_hash_changes_entry_hash` in `seal.rs:155-159` compares
`compute_entry_hash(&entry, None)` vs `compute_entry_hash(&entry, Some([0u8; 32]))`.
Under the new scheme, `None` maps to sentinel `[0u8; 32]`, so these will be EQUAL.
Update it to use a non-zero parent:

```rust
#[test]
fn different_parent_hash_changes_entry_hash() {
    let entry = sample_entry();
    let h_genesis = compute_entry_hash(&entry, None).unwrap();
    let h_chained = compute_entry_hash(&entry, Some([1u8; 32])).unwrap();
    assert_ne!(h_genesis, h_chained);
}
```

- [ ] **Step 6: Run all audit-chain tests**

Run: `cargo test -p audit-chain`
Expected: All pass. The `genesis_entry_has_no_parent` test still passes because it
checks `sealed.parent_hash.is_none()` (the struct field), not the hash computation.

- [ ] **Step 7: Commit**

```bash
git add crates/audit-chain/src/seal.rs
git commit -m "feat(audit-chain): add domain separation tag and genesis sentinel to hash construction

SHA-256(b\"mojave-audit-v1\\x00\" || canonical || parent_hash) with [0u8; 32]
sentinel for genesis entries. Per RFC 9162 §2.1 domain separation pattern."
```

---

### Task 2: Update `ChainVerifier` for sentinel genesis pattern

Spec §1.1.1. The verifier currently checks `entry.parent_hash.is_some()` at index 0
as a genesis violation. The `SealedAuditEntry.parent_hash` field is still `Option` at
the struct level (genesis = `None`), so the verifier logic is unchanged — this task
is about making sure the verifier's `compute_entry_hash` call works correctly with
the new construction.

**Files:**
- Modify: `crates/audit-chain/src/verify.rs:62-114`
- Test: `crates/audit-chain/src/verify.rs` (inline tests)

- [ ] **Step 1: Run existing verifier tests**

Run: `cargo test -p audit-chain verify`
Expected: All pass. The verifier already passes `entry.parent_hash` to `compute_entry_hash`,
which now handles `None` → sentinel internally.

- [ ] **Step 2: Add test that genesis sentinel hash verifies correctly**

Add to `mod tests` in `crates/audit-chain/src/verify.rs`:

```rust
#[test]
fn genesis_with_sentinel_verifies_clean() {
    let chain = build_chain(1);
    assert!(chain[0].parent_hash.is_none());
    let result = ChainVerifier::verify(&chain);
    assert!(result.is_clean());
    // Verify the hash was computed with sentinel
    let recomputed = ChainVerifier::recompute_entry_hash(&chain[0]).unwrap();
    assert_eq!(recomputed, chain[0].entry_hash);
}
```

- [ ] **Step 3: Run test**

Run: `cargo test -p audit-chain genesis_with_sentinel_verifies_clean`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/audit-chain/src/verify.rs
git commit -m "test(audit-chain): verify genesis sentinel hash construction through ChainVerifier"
```

---

### Task 3: Add golden-file tests for canonical encoding

Spec §1.1.2. Pin exact byte output for known inputs to guard against future serde
behavior changes.

**Files:**
- Create: `crates/audit-chain/tests/golden_canonical.rs`

- [ ] **Step 1: Create the golden-file integration test**

Create `crates/audit-chain/tests/golden_canonical.rs`:

```rust
use audit_chain::canonical::encode;
use serde_json::json;

#[test]
fn golden_empty_object() {
    assert_eq!(encode(&json!({})).unwrap(), b"{}");
}

#[test]
fn golden_sorted_keys() {
    let out = encode(&json!({"z": 1, "a": 2, "m": 3})).unwrap();
    assert_eq!(out, br#"{"a":2,"m":3,"z":1}"#);
}

#[test]
fn golden_nested_sorted() {
    let out = encode(&json!({"b": {"z": 1, "a": 2}, "a": 0})).unwrap();
    assert_eq!(out, br#"{"a":0,"b":{"a":2,"z":1}}"#);
}

#[test]
fn golden_string_escaping() {
    let out = String::from_utf8(encode(&json!({"s": "a\tb\nc"})).unwrap()).unwrap();
    assert_eq!(out, r#"{"s":"a\tb\nc"}"#);
}

#[test]
fn golden_control_char_hex() {
    let out = String::from_utf8(encode(&json!({"s": "\x01\x1f"})).unwrap()).unwrap();
    assert_eq!(out, r#"{"s":""}"#);
}

#[test]
fn golden_unicode_passthrough() {
    let out = String::from_utf8(encode(&json!({"k": "漢字🔥"})).unwrap()).unwrap();
    assert_eq!(out, r#"{"k":"漢字🔥"}"#);
}

#[test]
fn golden_array_preserves_order() {
    assert_eq!(encode(&json!([3, 1, 2])).unwrap(), b"[3,1,2]");
}

#[test]
fn golden_integers() {
    assert_eq!(
        encode(&json!({"a": -1, "b": 0, "c": 18446744073709551615u64})).unwrap(),
        br#"{"a":-1,"b":0,"c":18446744073709551615}"#
    );
}

#[test]
fn golden_mixed_types() {
    let out = encode(&json!({
        "arr": [1, "two", null, true, false],
        "n": 42,
        "s": "hello"
    }))
    .unwrap();
    assert_eq!(
        out,
        br#"{"arr":[1,"two",null,true,false],"n":42,"s":"hello"}"#
    );
}

#[test]
fn golden_supplementary_plane_keys_sort_by_utf8() {
    // U+10002 (Linear B) and U+FF61 (Halfwidth Katakana)
    // UTF-8 order: U+FF61 (ef bd a1) < U+10002 (f0 90 80 82)
    // UTF-16 order: U+10002 (D800 DC02) < U+FF61
    // We sort by UTF-8 (Rust String::cmp), not UTF-16 (JCS).
    let v = json!({"\u{10002}": 1, "\u{FF61}": 2});
    let out = String::from_utf8(encode(&v).unwrap()).unwrap();
    // U+FF61 should come first in UTF-8 order
    let pos_ff61 = out.find('\u{FF61}').unwrap();
    let pos_10002 = out.find('\u{10002}').unwrap();
    assert!(
        pos_ff61 < pos_10002,
        "UTF-8 sort: U+FF61 ({pos_ff61}) should precede U+10002 ({pos_10002})"
    );
}
```

- [ ] **Step 2: Run the golden tests**

Run: `cargo test -p audit-chain --test golden_canonical`
Expected: All PASS

- [ ] **Step 3: Commit**

```bash
git add crates/audit-chain/tests/golden_canonical.rs
git commit -m "test(audit-chain): add golden-file tests for canonical encoding

Pins exact byte output for known inputs. Includes supplementary-plane
Unicode key sort test documenting UTF-8 vs UTF-16 (JCS) divergence."
```

---

### Task 4: Write ADR for canonical encoding scheme

Spec §1.1.2. Document the encoding scheme as "mojave canonical JSON" with explicit
relationship to JCS (RFC 8785).

**Files:**
- Create: `docs/adr/0001-canonical-json-encoding.md`

- [ ] **Step 1: Create the ADR**

Create `docs/adr/0001-canonical-json-encoding.md`:

```markdown
# ADR-0001: Mojave Canonical JSON Encoding

**Status:** Accepted
**Date:** 2026-05-24
**Deciders:** Patrick Beam

## Context

The audit-chain crate needs a deterministic JSON encoding for hash chain
pre-images. RFC 8785 (JSON Canonicalization Scheme / JCS) is the obvious
standard, but full compliance requires implementing the ECMAScript
`Number::toString` algorithm for float serialization — the single most
error-prone part of JSON canonicalization.

## Decision

Use a custom "mojave canonical JSON" scheme that is stricter than JCS:

1. **Key sort order:** UTF-8 byte order (Rust `String::cmp`), not JCS's
   UTF-16 code unit order. Equivalent for ASCII keys. All audit entry
   keys are ASCII by construction.

2. **Numbers:** Integer-only (i64/u64). Floats are rejected with an error
   including the JSON path. This eliminates the entire class of
   float-to-string serialization bugs.

3. **String escaping:** Identical to JCS §3.2.2.2.

4. **Whitespace:** Zero, matching JCS.

5. **Lone surrogates:** Impossible — Rust's `String` type guarantees
   valid UTF-8.

## Consequences

- We do NOT claim JCS compliance. Any documentation or error message must
  use "mojave canonical JSON", never "JCS" or "RFC 8785".
- The encoding is internally consistent: same implementation on both ends
  (Rust `audit_chain::canonical::encode`). Cross-language verification
  must use our specification, not JCS.
- Golden-file tests in `crates/audit-chain/tests/golden_canonical.rs`
  pin exact byte output.
- The integer-only restriction means audit entry `context` fields must
  not contain floats. This is enforced at encoding time with a clear
  error message.
```

- [ ] **Step 2: Commit**

```bash
git add docs/adr/0001-canonical-json-encoding.md
git commit -m "docs(adr): ADR-0001 canonical JSON encoding scheme vs JCS (RFC 8785)"
```

---

### Task 5: Replace COSE header labels with standards

Spec §1.1.3. Replace custom labels 999/1000 with standard label 3 (content_type) and
label 15 (CWT Claims) with `iat` key 6.

**Files:**
- Modify: `crates/audit-sign/src/attestation.rs:1-73` (builder)
- Modify: `crates/audit-sign/src/attestation.rs:83-132` (verifier)
- Test: `crates/audit-sign/src/attestation.rs` (inline tests)

- [ ] **Step 1: Write test for standard content_type header**

Add to `mod tests` in `crates/audit-sign/src/attestation.rs`:

```rust
#[test]
fn attestation_uses_standard_content_type_label() {
    let signer = test_signer();
    let cbor = build_detached_attestation(&signer, b"test").unwrap();
    let envelope = CoseSign1::from_slice(&cbor).unwrap();
    let ct = &envelope.protected.header.content_type;
    assert_eq!(
        ct,
        &Some(coset::ContentType::Text(CONTENT_TYPE_VALUE.to_string()))
    );
}

#[test]
fn attestation_uses_cwt_claims_for_timestamp() {
    let signer = test_signer();
    let cbor = build_detached_attestation(&signer, b"test").unwrap();
    let envelope = CoseSign1::from_slice(&cbor).unwrap();
    let has_cwt = envelope
        .protected
        .header
        .rest
        .iter()
        .any(|(label, _)| *label == Label::Int(CWT_CLAIMS_LABEL));
    assert!(has_cwt, "protected header must contain CWT Claims (label 15)");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p audit-sign attestation_uses_standard`
Expected: FAIL — old code uses label 999 in `rest`, not `content_type` field.

- [ ] **Step 3: Update the builder**

Replace lines 1-73 of `crates/audit-sign/src/attestation.rs`:

```rust
use chrono::Utc;
use coset::{CborSerializable, CoseSign1, CoseSign1Builder, HeaderBuilder, Label};

use crate::signing::{AuditSigner, SignerError, SigningAlgorithm};
use crate::snapshot::ChainHeadSnapshot;

const CONTENT_TYPE_VALUE: &str = "application/vnd.mojave.audit.chain-head+json";
const CWT_CLAIMS_LABEL: i64 = 15;
const CWT_IAT_KEY: i64 = 6;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AttestationBuildError {
    #[error("signing failed: {0}")]
    Signing(#[from] SignerError),
    #[error("canonical encoding failed: {0}")]
    CanonicalEncoding(#[from] audit_chain::canonical::CanonicalEncodingError),
    #[error("CBOR serialization failed: {0}")]
    Cbor(String),
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AttestationVerifyError {
    #[error("signature is invalid")]
    SignatureInvalid,
    #[error("missing kid header")]
    MissingKid,
    #[error("content type mismatch")]
    ContentTypeMismatch,
    #[error("payload must be detached (external)")]
    PayloadNotDetached,
    #[error("unprotected headers must be empty")]
    NonEmptyUnprotectedHeader,
    #[error("unknown key id")]
    UnknownKeyId,
    #[error("unsupported algorithm")]
    UnsupportedAlgorithm,
    #[error("critical headers present but not understood")]
    CriticalHeadersNotUnderstood,
    #[error("CBOR deserialization failed: {0}")]
    Cbor(String),
}

fn cose_alg(algo: SigningAlgorithm) -> coset::iana::Algorithm {
    match algo {
        SigningAlgorithm::Ed25519 => coset::iana::Algorithm::EdDSA,
    }
}

pub fn build_detached_attestation(
    signer: &dyn AuditSigner,
    payload: &[u8],
) -> Result<Vec<u8>, AttestationBuildError> {
    let epoch_seconds = Utc::now().timestamp();

    let cwt_claims = ciborium::Value::Map(vec![(
        ciborium::Value::Integer(CWT_IAT_KEY.into()),
        ciborium::Value::Integer(epoch_seconds.into()),
    )]);

    let protected = HeaderBuilder::new()
        .algorithm(cose_alg(signer.algorithm()))
        .key_id(signer.key_id().as_bytes().to_vec())
        .content_type(CONTENT_TYPE_VALUE.to_string())
        .value(CWT_CLAIMS_LABEL, cwt_claims)
        .build();

    let sign1 = CoseSign1Builder::new()
        .protected(protected)
        .payload(Vec::new())
        .try_create_signature(payload, |tbs| signer.sign(tbs))
        .map_err(AttestationBuildError::Signing)?
        .build();

    sign1
        .to_vec()
        .map_err(|e| AttestationBuildError::Cbor(format!("{e:?}")))
}

pub fn build_tip_attestation(
    signer: &dyn AuditSigner,
    snapshot: &ChainHeadSnapshot,
) -> Result<Vec<u8>, AttestationBuildError> {
    let payload = snapshot.canonical_bytes()?;
    build_detached_attestation(signer, &payload)
}
```

- [ ] **Step 4: Update the verifier**

Replace the `verify_detached_attestation` function in `crates/audit-sign/src/attestation.rs`:

```rust
pub fn verify_detached_attestation(
    cbor_bytes: &[u8],
    payload: &[u8],
    keyring: &std::collections::HashMap<Vec<u8>, ed25519_dalek::VerifyingKey>,
) -> Result<(), AttestationVerifyError> {
    let envelope = CoseSign1::from_slice(cbor_bytes)
        .map_err(|e| AttestationVerifyError::Cbor(format!("{e:?}")))?;

    if envelope.payload.as_ref().is_some_and(|p| !p.is_empty()) {
        return Err(AttestationVerifyError::PayloadNotDetached);
    }

    if !envelope.unprotected.rest.is_empty()
        || !envelope.unprotected.key_id.is_empty()
        || envelope.unprotected.content_type.is_some()
    {
        return Err(AttestationVerifyError::NonEmptyUnprotectedHeader);
    }

    let protected = &envelope.protected.header;

    // Algorithm allowlist: only EdDSA
    match &protected.alg {
        Some(coset::RegisteredLabelWithPrivate::Assigned(coset::iana::Algorithm::EdDSA)) => {}
        _ => return Err(AttestationVerifyError::UnsupportedAlgorithm),
    }

    // Reject crit headers we don't understand
    if !protected.crit.is_empty() {
        return Err(AttestationVerifyError::CriticalHeadersNotUnderstood);
    }

    if protected.key_id.is_empty() {
        return Err(AttestationVerifyError::MissingKid);
    }
    let kid = &protected.key_id;

    let ct_matches = match &protected.content_type {
        Some(coset::ContentType::Text(s)) => s == CONTENT_TYPE_VALUE,
        _ => false,
    };
    if !ct_matches {
        return Err(AttestationVerifyError::ContentTypeMismatch);
    }

    let vk = keyring
        .get(kid.as_slice())
        .ok_or(AttestationVerifyError::UnknownKeyId)?;

    let tbs = envelope.tbs_data(payload);

    let sig_bytes: &[u8] = &envelope.signature;
    let sig_arr: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| AttestationVerifyError::SignatureInvalid)?;
    let sig = ed25519_dalek::Signature::from_bytes(&sig_arr);

    use ed25519_dalek::Verifier;
    vk.verify(&tbs, &sig)
        .map_err(|_| AttestationVerifyError::SignatureInvalid)
}
```

- [ ] **Step 5: Run all audit-sign tests**

Run: `cargo test -p audit-sign`
Expected: All PASS

- [ ] **Step 6: Commit**

```bash
git add crates/audit-sign/src/attestation.rs
git commit -m "feat(audit-sign): migrate to standard COSE header labels

- content_type: label 999 → standard label 3 (RFC 9052 §3.1)
- timestamp: label 1000 → CWT Claims label 15 with iat key 6 (RFC 9597)
- Add algorithm allowlist (EdDSA only) in verifier
- Add crit header rejection (RFC 9052 §3.1)"
```

---

### Task 6: Write TCK feature files for `audit-sign`

Spec §1.1.4.

**Files:**
- Create: `tck/audit-sign/features/attestation.feature`
- Create: `tck/audit-sign/features/cose_compliance.feature`

- [ ] **Step 1: Create attestation feature file**

Create `tck/audit-sign/features/attestation.feature`:

```gherkin
Feature: COSE_Sign1 detached attestation for audit chain tips
  Ed25519 signing and verification of chain head snapshots via COSE_Sign1
  with detached payload (RFC 9052 §4.4).

  Background:
    Given a generated Ed25519 signing key with key_id "test-key"

  Scenario: Sign and verify round-trip
    Given a payload "hello audit chain"
    When I build a detached attestation over the payload
    And I verify the attestation with the correct payload
    Then verification succeeds

  Scenario: Tampered payload rejected
    Given a payload "original"
    When I build a detached attestation over the payload
    And I verify the attestation with payload "tampered"
    Then verification fails with SignatureInvalid

  Scenario: Unknown key id rejected
    Given a payload "hello"
    When I build a detached attestation over the payload
    And I verify the attestation with an empty keyring
    Then verification fails with UnknownKeyId

  Scenario: Invalid CBOR rejected
    When I verify raw bytes "not cbor" as an attestation
    Then verification fails with Cbor error

  Scenario: Chain tip attestation round-trip
    Given an audit chain with 3 entries
    When I build a tip attestation from the chain head
    And I verify the tip attestation with the chain head snapshot
    Then verification succeeds
```

- [ ] **Step 2: Create COSE compliance feature file**

Create `tck/audit-sign/features/cose_compliance.feature`:

```gherkin
Feature: COSE_Sign1 header compliance with RFC 9052 and RFC 9597
  Attestation envelopes must use standard IANA COSE header labels.

  Background:
    Given a generated Ed25519 signing key with key_id "compliance-test"

  Scenario: Content type uses standard label 3
    When I build a detached attestation over payload "test"
    Then the protected header content_type is "application/vnd.mojave.audit.chain-head+json"
    And the content_type uses the standard coset content_type field (label 3)

  Scenario: Timestamp uses CWT Claims label 15 with iat key 6
    When I build a detached attestation over payload "test"
    Then the protected header contains CWT Claims (label 15)
    And the CWT Claims map contains iat (key 6) as an integer

  Scenario: Algorithm is EdDSA in protected header
    When I build a detached attestation over payload "test"
    Then the protected header algorithm is EdDSA

  Scenario: Unprotected headers are empty
    When I build a detached attestation over payload "test"
    Then the unprotected header map is empty

  Scenario: Payload is detached
    When I build a detached attestation over payload "test"
    Then the COSE_Sign1 payload field is empty or nil

  Scenario: Algorithm allowlist rejects non-EdDSA
    Given a COSE_Sign1 envelope with algorithm ES256 and a valid structure
    When I verify the envelope
    Then verification fails with UnsupportedAlgorithm

  Scenario: Critical headers rejected
    Given a COSE_Sign1 envelope with crit header listing label 99
    When I verify the envelope
    Then verification fails with CriticalHeadersNotUnderstood
```

- [ ] **Step 3: Commit**

```bash
git add tck/audit-sign/features/attestation.feature tck/audit-sign/features/cose_compliance.feature
git commit -m "tck(audit-sign): add Gherkin feature specs for attestation and COSE compliance"
```

---

### Task 7: Add `audit` module to `mojave-cli` Cargo.toml and module registry

**Files:**
- Modify: `crates/mojave-cli/Cargo.toml`
- Modify: `crates/mojave-cli/src/commands/mod.rs`
- Modify: `crates/mojave-cli/src/error.rs`

- [ ] **Step 1: Add dependencies to Cargo.toml**

Add to `[dependencies]` in `crates/mojave-cli/Cargo.toml`:

```toml
audit-chain = { path = "../audit-chain" }
audit-sign = { path = "../audit-sign" }
base64 = "0.22"
sha2 = "0.10"
```

- [ ] **Step 2: Register audit module**

In `crates/mojave-cli/src/commands/mod.rs`, add:

```rust
pub mod audit;
```

- [ ] **Step 3: Add Audit variant to CliError**

In `crates/mojave-cli/src/error.rs`, add `Audit(String)` to the `CliError` enum:

```rust
#[derive(Debug)]
pub enum CliError {
    Ingest(eval_ingest::IngestError),
    Orchestrator(eval_orchestrator::OrchestratorError),
    Config(ConfigError),
    Io(std::io::Error),
    Usage(String),
    Audit(String),
}
```

Add the Display arm:

```rust
CliError::Audit(e) => write!(f, "{e}"),
```

Add the kind arm:

```rust
CliError::Audit(_) => "audit_error",
```

- [ ] **Step 4: Verify it compiles (will fail until audit.rs exists)**

Run: `cargo check -p mojave-cli 2>&1 | head -5`
Expected: Error about missing `commands/audit.rs`. That's correct — we create it next.

- [ ] **Step 5: Commit the scaffolding**

```bash
git add crates/mojave-cli/Cargo.toml crates/mojave-cli/src/commands/mod.rs crates/mojave-cli/src/error.rs
git commit -m "chore(mojave-cli): add audit-chain, audit-sign deps and audit module scaffold"
```

---

### Task 8: Implement `mojave audit seal` subcommand

Spec §2.2.

**Files:**
- Create: `crates/mojave-cli/src/commands/audit.rs`
- Modify: `crates/mojave-cli/src/main.rs`

- [ ] **Step 1: Create `commands/audit.rs`**

Create `crates/mojave-cli/src/commands/audit.rs`:

```rust
use std::io::Read;
use std::path::{Path, PathBuf};

use audit_chain::entry::{Action, AuditEntryBuilder, Decision, Principal, ResourceRef};
use audit_chain::seal::{ChainHead, SealedAuditEntry};
use audit_sign::signing::{KeyRef, LocalEd25519Signer, SignerKeyId};
use audit_sign::snapshot::ChainHeadSnapshot;
use sha2::{Digest, Sha256};

use crate::error::CliError;

#[derive(Debug, serde::Deserialize)]
pub struct SealInput {
    pub run_id: String,
    pub eval_name: String,
    pub date_issued: String,
    pub data_file: PathBuf,
    pub data_sha256: String,
    pub actor: ActorInput,
}

#[derive(Debug, serde::Deserialize)]
pub struct ActorInput {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, serde::Serialize)]
pub struct SealOutput {
    pub chain_tip_hash: String,
    pub chain_tip_seq: u64,
    pub entry_hash: String,
    pub data_file_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestation_cbor_b64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verifying_key_spki_b64: Option<String>,
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn hash_file(path: &Path) -> Result<String, CliError> {
    let data = std::fs::read(path).map_err(|e| {
        CliError::Audit(format!("cannot read data file {}: {e}", path.display()))
    })?;
    let digest = Sha256::digest(&data);
    Ok(hex_encode(&digest))
}

fn load_chain_head(audit_dir: &Path) -> Result<ChainHead, CliError> {
    let head_path = audit_dir.join("chain-head.json");
    if !head_path.exists() {
        return Ok(ChainHead::new());
    }
    let data = std::fs::read_to_string(&head_path).map_err(|e| {
        CliError::Audit(format!("cannot read chain head: {e}"))
    })?;
    let state: ChainHeadState = serde_json::from_str(&data).map_err(|e| {
        CliError::Audit(format!("invalid chain head JSON: {e}"))
    })?;
    match state.tip_hash {
        Some(hex) => {
            let bytes = hex_decode_32(&hex)?;
            Ok(ChainHead::resume(bytes, state.next_seq))
        }
        None => Ok(ChainHead::new()),
    }
}

fn save_chain_head(audit_dir: &Path, head: &ChainHead) -> Result<(), CliError> {
    let state = ChainHeadState {
        tip_hash: head.last_entry_hash().map(|h| hex_encode(&h)),
        next_seq: head.next_seq(),
    };
    let json = serde_json::to_string_pretty(&state).map_err(|e| {
        CliError::Audit(format!("cannot serialize chain head: {e}"))
    })?;
    std::fs::write(audit_dir.join("chain-head.json"), json).map_err(|e| {
        CliError::Audit(format!("cannot write chain head: {e}"))
    })?;
    Ok(())
}

fn append_chain_entry(audit_dir: &Path, sealed: &SealedAuditEntry) -> Result<(), CliError> {
    use std::io::Write;
    let line = serde_json::to_string(sealed).map_err(|e| {
        CliError::Audit(format!("cannot serialize chain entry: {e}"))
    })?;
    let chain_path = audit_dir.join("chain.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&chain_path)
        .map_err(|e| CliError::Audit(format!("cannot open chain file: {e}")))?;
    writeln!(file, "{line}").map_err(|e| CliError::Audit(format!("cannot write chain entry: {e}")))?;
    Ok(())
}

fn hex_decode_32(hex: &str) -> Result<[u8; 32], CliError> {
    if hex.len() != 64 {
        return Err(CliError::Audit(format!(
            "expected 64-char hex string, got {} chars",
            hex.len()
        )));
    }
    let mut out = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk)
            .map_err(|_| CliError::Audit("invalid hex".into()))?;
        out[i] = u8::from_str_radix(s, 16)
            .map_err(|_| CliError::Audit(format!("invalid hex byte: {s}")))?;
    }
    Ok(out)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ChainHeadState {
    #[serde(skip_serializing_if = "Option::is_none")]
    tip_hash: Option<String>,
    next_seq: u64,
}

pub fn run_seal(key_file: Option<&Path>) -> Result<(), CliError> {
    let mut stdin_buf = String::new();
    std::io::stdin()
        .read_to_string(&mut stdin_buf)
        .map_err(|e| CliError::Audit(format!("cannot read stdin: {e}")))?;

    let input: SealInput = serde_json::from_str(&stdin_buf)
        .map_err(|e| CliError::Audit(format!("invalid seal input JSON: {e}")))?;

    let actual_hash = hash_file(&input.data_file)?;
    if actual_hash != input.data_sha256 {
        return Err(CliError::Audit(format!(
            "data file hash mismatch: expected {}, got {actual_hash}",
            input.data_sha256
        )));
    }

    let audit_dir = PathBuf::from("data/audit");
    std::fs::create_dir_all(&audit_dir)
        .map_err(|e| CliError::Audit(format!("cannot create audit dir: {e}")))?;

    let mut head = load_chain_head(&audit_dir)?;

    let actor = match input.actor.kind.as_str() {
        "System" => Principal::System {
            id: input.actor.id.clone(),
        },
        _ => Principal::Actor {
            id: input.actor.id.clone(),
            role: input.actor.kind.clone(),
        },
    };

    let entry = AuditEntryBuilder::new()
        .seq(0) // overwritten by ChainHead::link
        .actor(actor)
        .action(Action::Custom("run_card_generated".into()))
        .resource(ResourceRef::new("eval", &input.eval_name))
        .decision(Decision::Completed)
        .at(chrono::Utc::now())
        .context(serde_json::json!({
            "run_id": input.run_id,
            "eval_name": input.eval_name,
            "date_issued": input.date_issued,
            "data_file": input.data_file.to_string_lossy(),
            "data_sha256": input.data_sha256,
        }))
        .build()
        .map_err(|e| CliError::Audit(format!("cannot build audit entry: {e}")))?;

    let sealed = head
        .link(entry)
        .map_err(|e| CliError::Audit(format!("cannot seal audit entry: {e}")))?;

    append_chain_entry(&audit_dir, &sealed)?;
    save_chain_head(&audit_dir, &head)?;

    let entry_hash = hex_encode(&sealed.entry_hash);
    let chain_tip_hash = hex_encode(&head.last_entry_hash().expect("just linked"));
    let chain_tip_seq = head.next_seq() - 1;

    let (attestation_cbor_b64, verifying_key_spki_b64) = match resolve_signer(key_file)? {
        Some(signer) => {
            let snapshot = ChainHeadSnapshot::from_chain_head(&head);
            let cbor = audit_sign::attestation::build_tip_attestation(&signer, &snapshot)
                .map_err(|e| CliError::Audit(format!("attestation failed: {e}")))?;

            let att_dir = audit_dir.join("attestations");
            std::fs::create_dir_all(&att_dir)
                .map_err(|e| CliError::Audit(format!("cannot create attestations dir: {e}")))?;
            std::fs::write(att_dir.join(format!("{chain_tip_seq}.cbor")), &cbor)
                .map_err(|e| CliError::Audit(format!("cannot write attestation: {e}")))?;

            let spki = signer
                .verifying_key_spki_der()
                .map_err(|e| CliError::Audit(format!("cannot export public key: {e}")))?;
            std::fs::write(audit_dir.join("pubkey.spki.der"), &spki)
                .map_err(|e| CliError::Audit(format!("cannot write public key: {e}")))?;

            use base64::Engine;
            let b64_cbor = base64::engine::general_purpose::STANDARD.encode(&cbor);
            let b64_spki = base64::engine::general_purpose::STANDARD.encode(&spki);
            (Some(b64_cbor), Some(b64_spki))
        }
        None => (None, None),
    };

    let output = SealOutput {
        chain_tip_hash,
        chain_tip_seq,
        entry_hash,
        data_file_hash: actual_hash,
        attestation_cbor_b64,
        verifying_key_spki_b64,
    };

    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| CliError::Audit(format!("cannot serialize output: {e}")))?;
    println!("{json}");
    Ok(())
}

fn resolve_signer(key_file: Option<&Path>) -> Result<Option<LocalEd25519Signer>, CliError> {
    if let Some(path) = key_file {
        let signer = KeyRef::FilePath {
            key_id: SignerKeyId::new("mojave-audit"),
            path: path.to_path_buf(),
        }
        .load()
        .map_err(|e| CliError::Audit(format!("cannot load signing key: {e}")))?;
        return Ok(Some(signer));
    }

    if std::env::var("MOJAVE_AUDIT_KEY").is_ok() {
        let signer = KeyRef::Env {
            key_id: SignerKeyId::new("mojave-audit"),
            var: "MOJAVE_AUDIT_KEY".into(),
        }
        .load()
        .map_err(|e| CliError::Audit(format!("cannot load signing key from env: {e}")))?;
        return Ok(Some(signer));
    }

    Ok(None)
}

pub fn run_verify(chain_path: Option<&Path>) -> Result<(), CliError> {
    let chain_file = chain_path.unwrap_or(Path::new("data/audit/chain.jsonl"));
    if !chain_file.exists() {
        return Err(CliError::Audit(format!(
            "chain file not found: {}",
            chain_file.display()
        )));
    }

    let contents = std::fs::read_to_string(chain_file)
        .map_err(|e| CliError::Audit(format!("cannot read chain file: {e}")))?;

    let entries: Vec<SealedAuditEntry> = contents
        .lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str(line)
                .map_err(|e| CliError::Audit(format!("line {}: {e}", i + 1)))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let findings = audit_chain::verify::ChainVerifier::verify(&entries);

    let output = serde_json::json!({
        "entries_verified": entries.len(),
        "is_clean": findings.is_clean(),
        "findings": findings.findings().iter().map(|f| format!("{f:?}")).collect::<Vec<_>>(),
    });

    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| CliError::Audit(format!("cannot serialize output: {e}")))?;
    println!("{json}");

    if !findings.is_clean() {
        return Err(CliError::Audit("chain verification found issues".into()));
    }
    Ok(())
}
```

- [ ] **Step 2: Wire into `main.rs`**

Add the `Audit` subcommand to the `Commands` enum in `crates/mojave-cli/src/main.rs`.
Add after the `Completions` variant:

```rust
/// Audit chain management — seal entries and verify chains
Audit {
    #[command(subcommand)]
    action: AuditAction,
},
```

Add the `AuditAction` enum after the `Commands` enum:

```rust
#[derive(Subcommand)]
enum AuditAction {
    /// Seal a new audit entry from pipeline data (reads JSON from stdin)
    Seal {
        #[arg(long)]
        key_file: Option<std::path::PathBuf>,
    },
    /// Verify an existing audit chain
    Verify {
        #[arg(long)]
        chain: Option<std::path::PathBuf>,
    },
}
```

Add the match arm in `main()` after the `Completions` arm:

```rust
Commands::Audit { action } => match action {
    AuditAction::Seal { key_file } => {
        match mojave_cli::commands::audit::run_seal(key_file.as_deref()) {
            Ok(()) => Ok(()),
            Err(e) => {
                write_error(&e);
                std::process::exit(1);
            }
        }
    }
    AuditAction::Verify { chain } => {
        match mojave_cli::commands::audit::run_verify(chain.as_deref()) {
            Ok(()) => Ok(()),
            Err(e) => {
                write_error(&e);
                std::process::exit(1);
            }
        }
    }
},
```

Add import at top of main.rs (the `use` for `Subcommand` is already there via `clap`).

- [ ] **Step 3: Add chrono dependency to mojave-cli**

Add to `[dependencies]` in `crates/mojave-cli/Cargo.toml`:

```toml
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 4: Build and verify it compiles**

Run: `cargo build -p mojave-cli`
Expected: Compiles cleanly

- [ ] **Step 5: Verify help text**

Run: `cargo run -p mojave-cli -- audit --help`
Expected: Shows `seal` and `verify` subcommands

- [ ] **Step 6: Commit**

```bash
git add crates/mojave-cli/
git commit -m "feat(mojave-cli): add 'mojave audit seal' and 'mojave audit verify' subcommands

- seal: reads pipeline JSON from stdin, links audit chain entry,
  optionally produces COSE_Sign1 attestation, outputs JSON to stdout
- verify: replays chain from JSONL file, reports findings"
```

---

### Task 9: Update `generate_run_cards.py` to call `mojave audit seal`

Spec §2.4.

**Files:**
- Modify: `scripts/arc-workup/generate_run_cards.py`

- [ ] **Step 1: Add audit sealing function**

Add after the imports at the top of `scripts/arc-workup/generate_run_cards.py`:

```python
import subprocess


def compute_file_sha256(path: Path) -> str:
    """Compute full SHA-256 hex digest of a file."""
    import hashlib
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def audit_seal(run_id: str, eval_name: str, data_file: Path) -> dict | None:
    """Call mojave audit seal and return the output, or None if mojave is not available."""
    data_sha256 = compute_file_sha256(data_file)
    seal_input = {
        "run_id": run_id,
        "eval_name": eval_name,
        "date_issued": "2026-05-19",
        "data_file": str(data_file),
        "data_sha256": data_sha256,
        "actor": {"kind": "System", "id": "generate_run_cards.py"},
    }
    try:
        result = subprocess.run(
            ["mojave", "audit", "seal"],
            input=json.dumps(seal_input),
            capture_output=True,
            text=True,
            timeout=30,
        )
        if result.returncode != 0:
            print(f"  WARN: mojave audit seal failed: {result.stderr.strip()}")
            return None
        return json.loads(result.stdout)
    except FileNotFoundError:
        print("  WARN: mojave binary not found, skipping audit seal")
        return None
    except subprocess.TimeoutExpired:
        print("  WARN: mojave audit seal timed out")
        return None
```

- [ ] **Step 2: Replace toy hash with full hash and audit fields in `generate_config`**

In `generate_config`, replace the companion hash block (lines 212-214):

Old:
```python
    data_json = json.dumps(data, sort_keys=True)
    data_hash = hashlib.sha256(data_json.encode()).hexdigest()[:16]
    lines.append(rf"\rcset{{companion.hash}}    {{\texttt{{sha256:{data_hash}}}}}")
```

New:
```python
    data_file = Path(f"data/analysis/{name}_analysis.json")
    if data_file.exists():
        data_hash = compute_file_sha256(data_file)
    else:
        data_json = json.dumps(data, sort_keys=True)
        data_hash = hashlib.sha256(data_json.encode()).hexdigest()
    lines.append(rf"\rcset{{companion.hash}}    {{\texttt{{sha256:{data_hash}}}}}")
```

- [ ] **Step 3: Add audit fields to `generate_config`**

Add after the companion.contents line (after line 219):

```python
    lines.append("")
    lines.append(r"% ---------------------------------------------------------- AUDIT TRAIL ----")
    lines.append(r"\rcset{audit.chain.tip}   {}")
    lines.append(r"\rcset{audit.chain.seq}   {}")
    lines.append(r"\rcset{audit.signed}      {}")
```

- [ ] **Step 4: Wire audit sealing into `main()`**

In the `main()` function, after generating each config file (after line 434
`print(f"  {config_path}")`), add:

```python
        # Audit seal
        data_file = Path(f"data/analysis/{name}_analysis.json")
        if data_file.exists():
            seal_result = audit_seal(
                run_id=f"MOJAVE-2026-0519-{EVAL_META[name]['id_suffix']}",
                eval_name=name,
                data_file=data_file,
            )
            if seal_result:
                # Re-read the config and patch in audit fields
                config_lines = config_content.rstrip().split("\n")
                patched = []
                for line in config_lines:
                    if r"\rcset{audit.chain.tip}" in line:
                        line = rf"\rcset{{audit.chain.tip}}   {{\texttt{{{seal_result['chain_tip_hash']}}}}}"
                    elif r"\rcset{audit.chain.seq}" in line:
                        line = rf"\rcset{{audit.chain.seq}}   {{{seal_result['chain_tip_seq']}}}"
                    elif r"\rcset{audit.signed}" in line:
                        if seal_result.get("attestation_cbor_b64"):
                            line = r"\rcset{audit.signed}      {Yes --- Ed25519 COSE\_Sign1}"
                        else:
                            line = r"\rcset{audit.signed}      {No --- chain only (unsigned)}"
                    patched.append(line)
                config_content = "\n".join(patched) + "\n"
                config_path.write_text(config_content)
                print(f"    audit: seq={seal_result['chain_tip_seq']} tip={seal_result['chain_tip_hash'][:16]}...")
```

- [ ] **Step 5: Do the same for cross-eval summary hash**

In `generate_cross_eval_config`, replace the summary hash block (lines 386-388):

Old:
```python
    summary_json = json.dumps(summary, sort_keys=True)
    summary_hash = hashlib.sha256(summary_json.encode()).hexdigest()[:16]
    lines.append(rf"\rcset{{companion.hash}}    {{\texttt{{sha256:{summary_hash}}}}}")
```

New:
```python
    summary_file = SUMMARY_PATH
    if summary_file.exists():
        summary_hash = compute_file_sha256(summary_file)
    else:
        summary_json = json.dumps(summary, sort_keys=True)
        summary_hash = hashlib.sha256(summary_json.encode()).hexdigest()
    lines.append(rf"\rcset{{companion.hash}}    {{\texttt{{sha256:{summary_hash}}}}}")
```

- [ ] **Step 6: Commit**

```bash
git add scripts/arc-workup/generate_run_cards.py
git commit -m "feat(pipeline): integrate mojave audit seal into run card generation

- Full SHA-256 hashes (not truncated) for companion files
- Calls mojave audit seal via subprocess for each eval
- Graceful fallback if mojave binary is not available
- Adds audit.chain.tip, audit.chain.seq, audit.signed fields"
```

---

### Task 10: Add audit trail section to LaTeX run card engine

Spec §2.5.

**Files:**
- Modify: `templates/run-card/single-run-card/runcard.tex:264-270`

- [ ] **Step 1: Add Audit Trail section**

Insert a new section BEFORE the "Raw Data Reference" section (before line 264) in
`templates/run-card/single-run-card/runcard.tex`:

```latex
% ------------------------------------------------------- AUDIT TRAIL ----
\section{Audit Trail}
\begin{rcfacts}
\fact{Chain tip}{\texttt{\rc{audit.chain.tip}}}
\fact{Chain seq}{\rc{audit.chain.seq}}
\fact{Signed}{\rc{audit.signed}}
\fact{Verify}{\texttt{mojave audit verify -{}-chain data/audit/chain.jsonl}}
\end{rcfacts}
```

- [ ] **Step 2: Verify the template still builds with empty audit fields**

The `\rc{}` macro renders em-dashes for empty/missing keys, so this should work
with the existing demo config that has no audit fields.

Run: `make -C templates/run-card/single-run-card` (if pdflatex is available)
Or: verify visually that the LaTeX is syntactically valid.

- [ ] **Step 3: Commit**

```bash
git add templates/run-card/single-run-card/runcard.tex
git commit -m "feat(runcard): add Audit Trail section to LaTeX engine

Shows chain tip hash, sequence number, signature status, and
verification command. Empty fields render as em-dashes."
```

---

### Task 11: Property-based tests for audit primitives (Gate 3)

Spec §1.2, Gate 3.

**Files:**
- Create: `crates/audit-chain/tests/property_tests.rs`

- [ ] **Step 1: Create property-based test file**

Create `crates/audit-chain/tests/property_tests.rs`:

```rust
use audit_chain::canonical::encode;
use audit_chain::entry::{Action, AuditEntryBuilder, Decision, Principal};
use audit_chain::seal::{ChainHead, SealedAuditEntry};
use audit_chain::verify::ChainVerifier;
use chrono::{TimeZone, Utc};

fn sample_entry() -> audit_chain::entry::AuditEntry {
    AuditEntryBuilder::new()
        .seq(0)
        .actor(Principal::System {
            id: "prop-test".into(),
        })
        .action(Action::Observed)
        .decision(Decision::Observed)
        .at(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap())
        .context(serde_json::json!({"trial": 1}))
        .build()
        .unwrap()
}

#[test]
fn canonical_encoding_is_deterministic() {
    let entry = sample_entry();
    let b1 = encode(&entry).unwrap();
    let b2 = encode(&entry).unwrap();
    assert_eq!(b1, b2);
    // Also across re-construction
    let entry2 = sample_entry();
    let b3 = encode(&entry2).unwrap();
    assert_eq!(b1, b3);
}

#[test]
fn chain_monotonicity_for_100_entries() {
    let mut head = ChainHead::new();
    let mut prev_seq = None;
    for _ in 0..100 {
        let sealed = head.link(sample_entry()).unwrap();
        if let Some(prev) = prev_seq {
            assert_eq!(sealed.base.seq, prev + 1);
        }
        prev_seq = Some(sealed.base.seq);
    }
}

#[test]
fn honestly_constructed_chain_always_verifies_clean() {
    for n in [1, 2, 5, 10, 50] {
        let mut head = ChainHead::new();
        let chain: Vec<SealedAuditEntry> =
            (0..n).map(|_| head.link(sample_entry()).unwrap()).collect();
        let findings = ChainVerifier::verify(&chain);
        assert!(
            findings.is_clean(),
            "chain of {n} entries should verify clean, got {:?}",
            findings.findings()
        );
    }
}

#[test]
fn genesis_sentinel_differs_from_arbitrary_parent() {
    let entry = sample_entry();
    let h_genesis = audit_chain::seal::compute_entry_hash(&entry, None).unwrap();
    let h_with_parent = audit_chain::seal::compute_entry_hash(&entry, Some([1u8; 32])).unwrap();
    assert_ne!(h_genesis, h_with_parent);
}

#[test]
fn every_entry_in_chain_has_unique_hash() {
    let mut head = ChainHead::new();
    let chain: Vec<SealedAuditEntry> =
        (0..20).map(|_| head.link(sample_entry()).unwrap()).collect();
    let mut hashes: Vec<[u8; 32]> = chain.iter().map(|e| e.entry_hash).collect();
    let original_len = hashes.len();
    hashes.sort();
    hashes.dedup();
    assert_eq!(hashes.len(), original_len, "all entry hashes must be unique");
}

#[test]
fn parent_hash_links_are_consistent() {
    let mut head = ChainHead::new();
    let chain: Vec<SealedAuditEntry> =
        (0..10).map(|_| head.link(sample_entry()).unwrap()).collect();
    for i in 1..chain.len() {
        assert_eq!(
            chain[i].parent_hash,
            Some(chain[i - 1].entry_hash),
            "entry {i} parent_hash must equal entry {} entry_hash",
            i - 1
        );
    }
}
```

- [ ] **Step 2: Run property tests**

Run: `cargo test -p audit-chain --test property_tests`
Expected: All PASS

- [ ] **Step 3: Commit**

```bash
git add crates/audit-chain/tests/property_tests.rs
git commit -m "test(audit-chain): add property-based tests (Gate 3)

Determinism, monotonicity, chain integrity, sentinel divergence,
uniqueness, and parent-hash consistency."
```

---

## Self-Review Checklist

**Spec coverage:**
- [x] §1.1.1 Domain separation + genesis sentinel → Task 1, 2
- [x] §1.1.2 Golden files + ADR → Task 3, 4
- [x] §1.1.3 COSE headers + verifier hardening → Task 5
- [x] §1.1.4 audit-sign TCK → Task 6
- [x] §1.2 Gate 3 property tests → Task 11
- [x] §2.2 `mojave audit seal` → Task 7, 8
- [x] §2.3 `mojave audit verify` → Task 8
- [x] §2.4 Pipeline integration → Task 9
- [x] §2.5 Template engine → Task 10
- [x] §2.6 Unsigned mode → Task 8 (`resolve_signer` returns `None`)

**Placeholder scan:** No TBD, TODO, or "fill in" placeholders found.

**Type consistency:**
- `compute_entry_hash` signature unchanged (takes `Option<[u8; 32]>`)
- `SealedAuditEntry` struct unchanged (parent_hash still `Option`)
- `SealInput`/`SealOutput` structs defined in Task 8, used in Task 9
- `CONTENT_TYPE_VALUE` const name preserved across builder/verifier
- `CWT_CLAIMS_LABEL` and `CWT_IAT_KEY` used consistently
- `AttestationVerifyError` gains `UnsupportedAlgorithm` and `CriticalHeadersNotUnderstood` — both added in Task 5
- `CliError::Audit(String)` defined in Task 7, used in Task 8
