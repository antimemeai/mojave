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
