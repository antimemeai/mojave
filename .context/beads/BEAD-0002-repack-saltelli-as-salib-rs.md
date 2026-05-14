---
id: BEAD-0002
title: Repack saltelli-* crates as salib-rs
status: closed
priority: high
created: 2026-05-11
closed: 2026-05-14
---

## Description

The existing saltelli-* crates (core, samplers, estimators, cli, validation, surrogate, shapley) need to be repacked as a proper standalone Rust GSA library — likely under the name `salib-rs` or similar. No Rust SALib equivalent exists in the wild.

## What exists

- saltelli-core: RNG, distributions, problem spec, reduce primitives
- saltelli-samplers: LHS, Sobol, Morris, FAST/eFAST/RBD-FAST
- saltelli-estimators: Sobol (Saltelli2010, Jansen, Janon, Owen), Morris, FAST, RBD-FAST, Borgonovo, PAWN, DGSM, regression, given-data Sobol, G-theory, ANOVA, bootstrap
- saltelli-shapley: Shapley effects
- saltelli-surrogate: surrogate modeling (PCE?)
- saltelli-validation: reference functions (Ishigami, Sobol G), frozen SALib CSV data
- saltelli-cli: command-line interface

## Work needed

- Evaluate crate boundaries (may want to restructure)
- Rename/rebrand away from "saltelli" to something publishable
- Literature review on what SALib (Python) covers vs what we have
- Ensure 4-gate validation passes for all estimators
- Publish-readiness assessment (docs, API surface, examples)

## Blocked by

- ~~Overall architecture design (need to know how this fits into the larger system)~~
- ~~Literature review for any gaps vs Python SALib~~ → BEAD-0016

## Completion notes (2026-05-14)

- Rename: saltelli-* → salib-* done (commit 06238b6)
- Gap analysis: BEAD-0016 closed all 5 parity gaps (S2, FF, discrepancy, grouped, HDMR)
- salib-rs is now a strict superset of Python SALib method coverage
- Crate boundaries stable: core, samplers, estimators, validation, surrogate, shapley, cli
