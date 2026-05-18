---
id: BEAD-0009
title: Audit chain / integrity model — core primitives
status: closed
priority: nice-to-have
created: 2026-05-11
closed: 2026-05-18
---

## Description

Tamper-evident provenance for every eval run. Two crates ported from quarantine thunderdome-* design: `audit-chain` (canonical encoding, entry types, SHA-256 hash chain, chain verification) and `audit-sign` (Ed25519 signing via `AuditSigner` trait, COSE_Sign1 detached attestations, chain head snapshots).

## Acceptance

- [x] audit-chain crate: canonical encoding (sorted keys, integer-only, minimal escaping)
- [x] audit-chain crate: AuditEntry + builder, Principal, Action, Decision, ResourceRef
- [x] audit-chain crate: ChainHead sealing (SHA-256 hash chain, parent_hash linkage)
- [x] audit-chain crate: ChainVerifier with finding types (hash mismatch, parent mismatch, seq discontinuity, non-genesis)
- [x] audit-sign crate: AuditSigner trait, LocalEd25519Signer, KeyRef (file/env/in-memory)
- [x] audit-sign crate: COSE_Sign1 detached attestation build + verify
- [x] audit-sign crate: ChainHeadSnapshot from ChainHead
- [x] TCK Gherkin feature files (canonical_encoding, chain_integrity)
- [x] 67 unit tests passing (48 audit-chain + 19 audit-sign)
- [x] Clippy zero warnings, rustfmt clean
- [x] Full workspace compiles

## Deferred

- Blob store (content-addressed artifact storage) — needs object_store dep
- mojave-cli `verify` subcommand — wire after blob store
- Integration with eval-orchestrator (auto-emit audit entries on analysis runs)
- P256/ECDSA signer variant
