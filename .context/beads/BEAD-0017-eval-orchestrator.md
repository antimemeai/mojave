---
id: BEAD-0017
title: eval-orchestrator — analysis routing + decision engine
status: closed
priority: high
created: 2026-05-15
closed: 2026-05-18
---

## Description

eval-orchestrator crate wiring eval-ingest output to the math crates (irr, seq-anytime-valid, spc-charts). Instrument registry pattern, batch analyze() + streaming Monitor, auto-detect routing, typed Decision vocabulary.

## Acceptance

- [x] Core types: SeriesKey, Decision, MeasurementIssue, summaries
- [x] Configuration with sane defaults
- [x] Instrument trait + 3 adapters (IRR, Sequential, SPC)
- [x] Auto-detection router with force-enable/disable
- [x] Batch analyze() producing AnalysisReport
- [x] Streaming Monitor with serde roundtrip
- [x] TCK batch + monitor integration tests
- [x] Gate 2 cross-checks against direct math crate calls
- [x] Clippy zero warnings, rustfmt clean
- [x] Full workspace test suite passes

## Deferred items (from CR)

- IRR streaming not yet in Monitor (I4) — Monitor only does sequential + SPC
- irr_results in AnalysisReport is Option, should be Vec for multi-series (I1)
- ContinueRunning power estimates are placeholders (I2)
- Judge identification by model name is coarse (I5)
- Monitor only uses EWMA chart type regardless of config (M1)
