---
id: BEAD-0003
title: Build IRR (inter-rater reliability) primitives
status: closed
closed: 2026-05-12
priority: high
created: 2026-05-11
---

## Description

Inter-rater reliability statistics are a confirmed gap — not built anywhere, not available from torch_measure, and identified as the most commercially legible piece (judge reliability is the acute pain point in the market).

## Methods needed

- Krippendorff α (nominal, ordinal, interval, ratio — explicit level= required)
- Fleiss κ (multi-rater nominal)
- Cohen κ / weighted κ (2-rater)
- Gwet AC1/AC2
- Bland-Altman (continuous agreement)
- Bootstrap CIs for all

## Key properties to validate (from 4-gate)

- Krippendorff α invariant under rater-label permutation
- α=1 ⟺ perfect agreement, α=0 ⟺ chance-level
- Cohen κ = Fleiss κ on 2-rater 2-category to 1e-10
- Gwet AC1 ≥ Cohen κ on high-prevalence data (paradox direction)
- Bland-Altman LoA = mean ± 1.96·SD(diff) exactly

## Reference implementations

- R: irr, irrCAC (Gwet), kripp.alpha
- Python: statsmodels.stats.inter_rater, krippendorff (Castro)

## Literature needed

- Cohen 1960, Fleiss 1971, Krippendorff 2004, Gwet 2014, Bland & Altman 1986
- Paradox papers: Feinstein & Cicchetti 1990, Quarfoot & Levine 2016
- RAND Judge Reliability Harness (2026) for LLM-judge specific context
