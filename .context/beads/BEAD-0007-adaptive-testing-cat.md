---
id: BEAD-0007
title: Adaptive testing / CAT for smart eval budgeting
status: closed
priority: nice-to-have
created: 2026-05-11
closed: 2026-05-18
---

## Description

After a change, don't run all 47 tasks. Use IRT ability estimates from the first few tasks to select which tasks are most informative next. Stop when estimate converges.

## What was built

`crates/eval-design` — adaptive CAT engine (ability.rs + cat.rs):

### Ability Estimation (`ability` module)
- MLE via Newton-Raphson for 2PL IRT (bounded, convergence-checked)
- EAP via quadrature with normal prior (61-point grid)
- Automatic MLE→EAP fallback for degenerate response patterns
- SE computation from observed Fisher information

### CAT Session (`cat` module)
- `CatSession`: state machine — SelectNext → administer → update θ̂ → check stopping
- Item selection: MaxInfo (θ-conditional with oversample_k) or Minimax (greedy maximin)
- Stopping rules: SE threshold, min/max items
- `run_cat()`: full adaptive session with response callback
- `simulate_cat()`: offline simulation with pre-determined responses
- Deterministic via ChaCha20Rng seeding
- 21 new tests (65 total in crate)

## Methods

- Computerized Adaptive Testing (Fisher information item selection)
- D-optimal design principles
- Integration with sequential testing (SPRT) for per-task regression detection

## Key results from literature

- AIMS: recovers benchmark rankings within 2% error using 1-18% of items (Truong et al. 2025)
- ATLAS: 41 items on HellaSwag vs 5,600 with minimal error (Li et al. 2025)
- Amortized evaluation: 50-80% cost reduction (Truong et al. ICML 2025)

## Acceptance criteria met

- [x] MLE ability estimation (Newton-Raphson, 2PL)
- [x] EAP ability estimation (normal prior quadrature)
- [x] Adaptive item selection (MaxInfo + Minimax)
- [x] Stopping rules (SE threshold + min/max items)
- [x] Session state machine with deterministic seeding
- [x] Exposure control integration
- [x] Full session runner + simulation mode
- [x] All tests pass, clippy clean

## Integration

- torch_measure provides MaxInfoStrategy, SpanningStrategy
- Orchestration layer uses this to schedule which tasks to run after each change
- Combined with sequential testing: adaptively select tasks AND stop early per-task

## References

- Truong et al. 2025, "Reliable and Efficient Amortized Model-based Evaluation" (arXiv:2503.13335)
- ATLAS 2025 (arXiv:2511.04689)
- torch_measure CAT module
