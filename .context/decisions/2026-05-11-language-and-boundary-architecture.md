---
date: 2026-05-11
title: Language split and inter-component boundaries
status: accepted
---

## Decision

- Rust: all math primitives, all orchestration logic, all heavy computation — compiled binaries
- Python: thin user-facing scripting shell only (CLI UX, torch_measure integration for IRT/CAT)
- Components communicate via binary APIs (compiled Rust binaries with structured I/O)
- Internal serialization: bincode (or similar efficient binary format)
- JSON only at user-facing boundaries
- No PyO3/FFI — clean process boundaries between Rust and Python
- Rust crates are independently deployable without Python or a Rust toolchain on the target
- Orchestration (experiment design, scheduling, state management, range management) is Rust, not Python

## Rationale

- Clean interface boundaries prevent coupling nightmares
- Rust binaries are fast when properly deployed — leverage that
- Any language can consume the binary API (not locked to Python)
- Simpler deployment (ship a binary)
- Serialization overhead is a function of payload choices, not architecture — bincode keeps it negligible
- Python layer exists for user-friendly scripting interface and torch_measure integration only
- All real logic in Rust — Python never does heavy lifting
