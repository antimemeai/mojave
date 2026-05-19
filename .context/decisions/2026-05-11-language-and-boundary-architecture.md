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

## 2026-05-18 reaffirmation: subprocess, not PyO3

Session notes from 2026-05-14 proposed PyO3 for a Python SDK. After evaluating
PyO3's offline-build story (maturin vendor issue #457 still open), release
coupling (tied to PyO3's CPython support cycle), and the actual traffic pattern
(infrequent calibration calls returning small parameter sets — not tight-loop
large-array exchange), the original decision stands.

Python calls `mojave` CLI as a subprocess, exchanges JSON. The data flow that
crosses the boundary is: fit IRT/factor model in Python (py-irt, deepirtools,
semopy) → emit item parameters as JSON → Rust engine consumes them. That's
orchestration-weight traffic, not compute-weight — subprocess + JSON is the
right tool.
