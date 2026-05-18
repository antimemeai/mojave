---
id: BEAD-0012
title: Perturbation engine (systematic eval robustness testing)
status: closed
priority: nice-to-have
created: 2026-05-11
closed: 2026-05-18
---

## Description

Systematic perturbation of eval conditions to measure robustness. Maps the boundary between "agent can do this" and "agent does this under these conditions." Disposition profile measurement.

## What was built

`crates/perturbation-engine` — standalone crate consolidating three atom families:

### Format atoms (`format` module)
- `Separator` (ColonSpace/Newline/ArrowSpace), `Casing` (Original/Upper/Lower), `Punctuation` (Question/Period/None), `Padding` (Original/QuotesEnclose/NewlinesPrepend/NewlinesAppend/NewlinesBoth)
- `FormatAtoms::from_seed(u64)` — deterministic sampling via ChaCha20Rng
- `longest_string_region(&[u8])` — schema-agnostic longest-quoted-string finder
- `apply_atoms(body, atoms, region)` — transform pipeline: casing → separator → punctuation → padding
- `signed_confound: false` — format perturbations should NOT change scores

### Paraphrase atoms (`paraphrase` module)
- `ParaphraseModel` (Mini/Standard/Frontier), `ParaphraseStrength` (Mild/Moderate/Aggressive)
- `ParaphraseAtoms::from_seed(u64)` — deterministic sampling
- `signed_confound: true` — paraphrase perturbations may change scores

### Multi-turn atoms (`multi_turn` module)
- `MultiTurnAtom`: Original, TruncateEarly, Reorder, Inject
- `MultiTurnPlan` — validated plan with history + params per atom
- `apply(plan, seed)` — deterministic perturbation execution
- `signed_confound: true` — all multi-turn perturbations may change scores

### Cross-cutting
- All enums `#[non_exhaustive]` + `Serialize`/`Deserialize`
- `factor_str()` on every atom for sensitivity analysis bridge
- 39 unit tests covering determinism, validation, transform correctness

## Acceptance criteria met

- [x] Three atom families ported from quarantine conex adapters
- [x] Deterministic seeding via ChaCha20Rng
- [x] `signed_confound` flag on all perturbation outputs
- [x] `factor_str()` flat keys for sensitivity bridge
- [x] Validation on multi-turn plans (empty history, bounds, etc.)
- [x] Schema-agnostic body walk (no JSON parsing dependency)
- [x] All tests pass, clippy clean, workspace lints satisfied
