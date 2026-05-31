# Peer B: Audit Chain Trust

**You are Peer B.** You own Stream B of the advisory execution plan.

## Your mission

Harden the audit chain trust model. Retire the broken Python audit writer, set up CI/CD, add Sigstore binary signing, write the canonical encoding spec.

## Your scope (files you own)

- `scripts/audit.py` — to be deleted
- `scripts/tests/test_audit.py` — rewrite for subprocess-based testing
- `scripts/v2/*.py` — any files that import audit.py
- `.github/workflows/` — new CI/CD pipeline (new directory)
- `crates/audit-sign/src/rekor.rs` — Rekor witnessing (new file)
- `crates/mojave-cli/src/commands/audit.rs` — add witness subcommand
- `docs/adr/` — canonical encoding spec

## Your tasks (in order)

1. **B1: Retire Python audit writer** — find all callers of audit.py, replace with `mojave audit emit` subprocess calls, delete audit.py, fix cross-language test
2. **B2: CI/CD pipeline** — create `.github/workflows/ci.yml` with clippy + fmt + test
3. **B3: Sigstore binary signing** — create `.github/workflows/release.yml` with cosign
4. **B4: Canonical encoding spec** — ADR documenting the exact JSON encoding rules
5. **B5: Rekor witnessing** — submit chain-head snapshots to Rekor (Tier 3)
6. **B6: Key management upgrade** — document the gap in an ADR (Tier 3, design only)

## Key context

- `scripts/audit.py` writes JSONL chains that are format-incompatible with the Rust verifier post-genesis-sentinel merge (lacks tagged-union genesis format, model identity binding)
- The cross-language test at `scripts/tests/test_audit.py:223` uses `pytest.skip` when no Rust binary is found — it has likely never passed against current code
- There is no `.github/workflows/` directory — CI does not exist yet
- Pre-commit hooks enforce clippy zero warnings + rustfmt

## Methodology

JSMNTL: TCK red → compile/run red → implement → green → code review. Commit after every green step.

## Full plan

Read `docs/plans/2026-05-30-advisory-execution.md` — Stream B section — for complete task details.

## Dependencies

None — you are fully independent. Start immediately.

## Branch

You are on `stream-b/audit-chain`. Commit frequently. Do not touch files outside your scope.
