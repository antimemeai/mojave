---
id: BEAD-0017
title: eval-orchestrator — analysis routing + decision engine
status: open
priority: high
created: 2026-05-15
---

## Description

eval-orchestrator crate wiring eval-ingest output to the math crates (irr, seq-anytime-valid, spc-charts). Instrument registry pattern, batch analyze() + streaming Monitor, auto-detect routing, typed Decision vocabulary.

## Acceptance

- [ ] Core types: SeriesKey, Decision, MeasurementIssue, summaries
- [ ] Configuration with sane defaults
- [ ] Instrument trait + 3 adapters (IRR, Sequential, SPC)
- [ ] Auto-detection router with force-enable/disable
- [ ] Batch analyze() producing AnalysisReport
- [ ] Streaming Monitor with serde roundtrip
- [ ] TCK batch + monitor integration tests
- [ ] Gate 2 cross-checks against direct math crate calls
- [ ] Clippy zero warnings, rustfmt clean
- [ ] Full workspace test suite passes
