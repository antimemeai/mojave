---
id: BEAD-0005
title: IRT integration via torch_measure (Python layer)
status: open
priority: nice-to-have
created: 2026-05-11
---

## Description

torch_measure (Stanford AIMS) provides GPU-accelerated IRT (Rasch, 2PL, 3PL, Beta IRT), CAT with Fisher information selection, and factor models. These should be integrated via the Python scripting layer rather than reimplemented in Rust.

## What torch_measure provides

- IRT variants: Rasch, 2PL, 3PL, AmortizedIRT, MultiFacetRasch, TestletRasch
- CAT: MaxInfoStrategy, RandomStrategy, SpanningStrategy
- Factor models: LogisticFM, Bifactor, rotation methods
- Fitting: MLE, EM, JML, Bayesian SVI
- Metrics: Cronbach's alpha, Mokken scalability, infit/outfit, DIF, network centrality

## Integration points

- Python scripting layer calls torch_measure for item diagnostics
- Results feed back into orchestration layer for adaptive task selection
- Factor structure informs the change×task matrix interpretation

## Blocked by

- Python scripting layer architecture design
- Orchestration layer API definition
