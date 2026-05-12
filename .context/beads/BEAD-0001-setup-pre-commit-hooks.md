---
id: BEAD-0001
title: Set up language-specific pre-commit hooks
status: open
priority: high
created: 2026-05-11
---

## Description

Need robust pre-commit hooks per JSMNTL discipline:
- Rust: `cargo clippy` must pass with zero warnings, `cargo fmt --check`
- Python: ruff/black formatting, type checks
- General: no secrets, no large binaries

## Context

User specified this is bare minimum baseline for development. Rust commits must fix all clippy callouts. This gates any actual code development.

## Acceptance

- Hook installer script in scripts/
- Hooks enforce formatting + linting per language
- Bypass only via explicit `--no-verify` (sparingly)
