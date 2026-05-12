---
id: BEAD-0007
title: Adaptive testing / CAT for smart eval budgeting
status: open
priority: nice-to-have
created: 2026-05-11
---

## Description

After a change, don't run all 47 tasks. Use IRT ability estimates from the first few tasks to select which tasks are most informative next. Stop when estimate converges.

## Methods

- Computerized Adaptive Testing (Fisher information item selection)
- D-optimal design principles
- Integration with sequential testing (SPRT) for per-task regression detection

## Key results from literature

- AIMS: recovers benchmark rankings within 2% error using 1-18% of items (Truong et al. 2025)
- ATLAS: 41 items on HellaSwag vs 5,600 with minimal error (Li et al. 2025)
- Amortized evaluation: 50-80% cost reduction (Truong et al. ICML 2025)

## Integration

- torch_measure provides MaxInfoStrategy, SpanningStrategy
- Orchestration layer uses this to schedule which tasks to run after each change
- Combined with sequential testing: adaptively select tasks AND stop early per-task

## References

- Truong et al. 2025, "Reliable and Efficient Amortized Model-based Evaluation" (arXiv:2503.13335)
- ATLAS 2025 (arXiv:2511.04689)
- torch_measure CAT module
