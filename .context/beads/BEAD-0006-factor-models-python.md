---
id: BEAD-0006
title: Factor models for latent task structure (Python layer)
status: closed
closed: 2026-05-19
priority: nice-to-have
created: 2026-05-11
---

## Description

Factor analysis reveals that tasks in a suite may cluster into latent capability dimensions. When change Y breaks task 4, factor structure tells you tasks 12 and 31 should also be checked (same latent factor).

## Methods

- Exploratory factor analysis (LogisticFM via torch_measure)
- Bifactor models
- Rotation methods (varimax, promax)
- Structured capabilities model (Kearns 2026)

## Use in product

- "Are some of your tasks redundant?" question
- Cross-task regression prediction (change affects a factor, not individual tasks)
- Adaptive testing informed by factor structure

## References

- Truong et al., "Beyond Mean Scores: Factor Models for Reliable and Efficient AI Evaluation"
- Kearns 2026, "Quantifying Construct Validity in LLM Evaluations" (arXiv:2602.15532)
