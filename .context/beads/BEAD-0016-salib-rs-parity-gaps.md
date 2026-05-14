---
id: BEAD-0016
title: Close SALib parity gaps in salib-rs
status: closed
priority: high
created: 2026-05-12
closed: 2026-05-14
depends-on: [BEAD-0002]
---

## Description

Gap analysis (2026-05-12) against Python SALib (Herman et al.) identified five methods SALib ships that the Rust impl does not. Closing these makes salib-rs a strict superset of Python SALib — no reason for anyone to reach for the Python version.

## Gaps to close

### 1. Second-order Sobol indices (S2)

Sampling infrastructure exists (`SaltelliMatrix.b_a`, `second_order: bool` in `build_saltelli_matrix`). Estimator side is stubbed — `SobolIndices` has a comment noting `second_order: Vec<Vec<f64>>` as future field but no estimator computes it.

- Formula: Saltelli 2010 Eq (d) for S2_{ij}
- Scope: wire through Saltelli2010, Jansen, Janon, Owen estimators
- Effort: small — plumbing exists, just need the formula + output field

### 2. HDMR (High-Dimensional Model Representation)

RS-HDMR (Random Sampling HDMR) decomposes model output into component functions of increasing order. Li et al. 2010.

- Literature: Li, Rosenthal, Rabitz 2001; Li et al. 2010
- Scope: sampler + analyzer
- Effort: medium

### 3. Fractional Factorial screening sampler + analyzer

2-level fractional factorial design for cheap factor screening before full SA. Saltelli et al. 2008.

- Literature: Saltelli et al. 2008 (Primer), Box-Hunter-Hunter
- Scope: sampler (Plackett-Burman or resolution-IV) + aliased-effects analyzer
- Effort: small

### 4. Discrepancy indices

Space-filling quality metrics: Wrap-around Discrepancy (WD), Centered Discrepancy (CD), Modified Discrepancy (MD), L2-star discrepancy. Computed from 2D projections of (Xi, Y).

- Literature: Fang et al. 2006, Hickernell 1998
- Scope: standalone analyzer, no sampler needed
- Effort: small

### 5. Grouped-factor support in Morris/Sobol samplers

SALib supports grouping parameters and treating groups as atomic units in both Morris trajectory generation and Sobol sampling. Useful when factors are conceptually linked.

- Literature: Saltelli et al. 2008 (Primer), Morris 1991 extension
- Scope: affects sampler APIs (Morris trajectories, Saltelli matrix) + estimator aggregation
- Effort: medium — API surface change

## What we already have that SALib doesn't

For context, salib-rs is already ahead in: PCE (full + sparse LARS), active subspaces, Shapley effects, QOSA, ANOVA, G-theory, given-data Sobol, Iman-Conover, Owen matrix, deterministic multi-stream RNG, bootstrap CIs as first-class.

## Validation approach

4-gate per method:
1. Textbook reproductions (published examples from each paper)
2. SALib cross-check (frozen CSV differentials at canonical params)
3. Property-based tests (identities, symmetries, bounds)
4. Monte Carlo calibration (convergence rates, coverage)

## Priority order

1. S2 (smallest delta, biggest user-visible gap)
2. Fractional Factorial (screening is a natural workflow entry point)
3. Grouped factors (API change, better done before publish)
4. Discrepancy indices (standalone, no dependencies)
5. HDMR (largest scope, least urgent)

## Completion notes (2026-05-14)

All 5 gaps closed. Summary:

| Method | Crate | Tests |
|--------|-------|-------|
| S2 Sobol indices | salib-estimators (saltelli2010, jansen, janon, owen) | 3 TCK + 4×unit |
| Fractional Factorial (Plackett-Burman) | salib-samplers + salib-estimators | 3 TCK + 6 unit |
| Discrepancy (CD, WD, MD, L2*) | salib-estimators | 6 TCK |
| Grouped factors (Morris + Sobol) | salib-core + salib-samplers + salib-estimators | 6 TCK + 5 unit |
| RS-HDMR via PCE | salib-estimators | 4 TCK |

Total: 22 TCK integration tests + 15 unit tests added. Zero workspace regressions.

salib-rs is now a strict superset of Python SALib's method coverage.
