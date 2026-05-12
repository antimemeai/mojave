---
id: BEAD-0009
title: Audit chain / integrity model design
status: open
priority: nice-to-have
created: 2026-05-11
---

## Description

Tamper-evident provenance for every eval run. Deferred from immediate scope — "rather have too much than too little but easy to get lost in audit fidelity." Design the interface early so it's not bolted on, but don't let it consume oxygen.

## What exists (in quarantine)

- audit-sign crate: Ed25519 signing
- verify crate: chain-walking verification
- Sealed envelope shape (SHA-256 hash-chain, COSE_Sign1)
- blob crate: content-addressed artifact store

## Design principles (from previous work)

- Every operation emits a sealed envelope
- Chain integrity provable via CLI verification
- SUT-observable vs substrate-observable partition
- Per-cell chain isolation

## When to revisit

- After math core is solid and orchestration architecture is sketched
- Before first defense customer demo (they will ask about integrity)
- Interface should be designed during orchestration work even if implementation is deferred
