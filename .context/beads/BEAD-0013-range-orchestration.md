---
id: BEAD-0013
title: Range orchestration (repeatable agent evaluation environments)
status: open
priority: nice-to-have
created: 2026-05-11
---

## Description

Ability to spin up and evaluate agents on ranges repeatably. The environment in which agents execute tasks must be controlled, reproducible, and isolated.

## What exists (in quarantine)

- Firecracker microVM orchestration via locomoco
- Cell-runner for isolated per-cell execution
- Range/tomography rig design docs
- Sensor-sim design with GUM uncertainty budgets

## Key requirements

- Repeatable environment setup (same task, same starting state)
- Isolation between eval runs
- Support for both hosted (AWS/cloud) and on-prem deployment
- Observation without interference (epistemic isolation)

## When to revisit

- Part of the orchestration layer design
- After math core is solid
- Defense customers will need this for realistic agent evaluation
