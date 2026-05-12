---
id: BEAD-0015
title: Eval runner ingest layer (runner-agnostic + Inspect adapter)
status: open
priority: high
created: 2026-05-11
---

## Description

Results-collector must define a runner-agnostic ingest interface. Any eval runner that produces results conforming to the interface can feed into the measurement engine.

- Clean trait/interface: "here's what eval results look like"
- Inspect (UK AISI) ships as a pre-built adapter — works out of the box
- Customer-built runners implement the same interface
- No coupling to any specific runner's internals

## Context

Inspect is a Python eval orchestration framework (Task = Dataset + Solver + Scorer). It runs evals and produces raw scores. It has no statistical measurement layer — that's our entire value prop. We sit on top of eval runners, we are not one.

Inspect is a common runner in the safety eval community (UK AISI, METR, Apollo Research). Shipping a working adapter for it is table stakes. But many customers will have their own runners and the architecture must not privilege Inspect.

## Design notes

- Lives in or adjacent to the results-collector crate
- Adapter pattern: one trait, multiple implementations
- Inspect adapter handles Inspect's log format (JSON-based eval logs)
- Interface must capture: task ID, agent ID, outcome (pass/fail/score), judge metadata, timestamps, optional provenance (git SHA, config hash)
- Must be extensible without modifying core

## Acceptance

- Runner-agnostic trait defined and documented
- Inspect adapter passes integration tests against real Inspect log files
- At least one example of a "custom runner" adapter (even if toy)
