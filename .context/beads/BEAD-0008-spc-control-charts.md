---
id: BEAD-0008
title: SPC control charts for longitudinal agent tracking
status: closed
priority: nice-to-have
created: 2026-05-11
closed: 2026-05-14
---

## Description

The temporal/longitudinal layer that stitches eval runs into a development history. Tracks baselines per task, detects regressions against the noise floor, builds the change-impact map over time.

## Components

- Per-task baseline establishment (multiple no-change runs → noise floor)
- Control limits (derived from G-theory variance decomposition)
- CUSUM / EWMA for drift detection
- Change attribution: when score moves, which change caused it?
- Change×task matrix accumulation over development cycles

## Integration

- Uses G-theory (salib-rs) for variance decomposition / noise floor
- Uses sequential testing for regression detection
- Uses factor models for cross-task prediction
- Orchestration layer manages the temporal state

## References

- Shewhart 1931, "Economic Control of Quality of Manufactured Product"
- Montgomery, "Introduction to Statistical Quality Control"
- NIST/SEMATECH e-Handbook of Statistical Methods (SPC chapter)
- Page 1954, "Continuous Inspection Schemes" (CUSUM)
- Lucas & Crosier 1982, "Fast Initial Response for CUSUM" (FIR CUSUM)
- Roberts 1959, "Control Chart Tests Based on Geometric Moving Averages" (EWMA)
- Lucas 1982, "Combined Shewhart-CUSUM Quality Control Schemes"
- Shin, Ramdas & Rinaldo 2023, "E-detectors: a nonparametric framework for sequential change detection"
- Brook & Evans 1972, "An approach to the probability distribution of CUSUM run length" (ARL Markov chain)
- Lucas & Saccucci 1990, "Exponentially Weighted Moving Average Control Schemes" (EWMA ARL)

## Completion notes (2026-05-14)

New crate `spc-charts` with 6 stateful chart monitors + ARL computation + G-theory adapter.

| Component | Module | Tests |
|-----------|--------|-------|
| Shewhart (WE1-WE4 rules) | shewhart.rs | 6 TCK (incl. MC ARL₀) |
| CUSUM (Page 1954) | cusum.rs | 5 TCK (incl. known trace) |
| FIR CUSUM (Lucas & Crosier 1982) | cusum_fir.rs | 3 TCK |
| EWMA (Roberts 1959) | ewma.rs | 6 TCK |
| Combined Shewhart-CUSUM (Lucas 1982) | combined.rs | 4 TCK |
| E-detector (Shin et al. 2023) | e_detector.rs | 6 TCK (incl. MC false alarm) |
| ARL (Markov chain) | arl.rs | 7 TCK (Montgomery table validation) |
| G-theory adapter (feature-gated) | g_theory.rs | 2 TCK |

Total: 39 TCK tests. Zero workspace regressions. Clippy zero warnings.
