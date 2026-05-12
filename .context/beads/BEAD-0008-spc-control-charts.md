---
id: BEAD-0008
title: SPC control charts for longitudinal agent tracking
status: open
priority: nice-to-have
created: 2026-05-11
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
