---
id: BEAD-0014
title: Git/VCS integration for change attribution
status: closed
priority: nice-to-have
created: 2026-05-11
closed: 2026-05-18
---

## Description

Change-to-score attribution for longitudinal eval tracking. Maps commits to task score movements, predicts blast radius from file changes, and provides binary search (bisect) for regression localization.

## Acceptance

- [x] change-attribution crate with 4 modules
- [x] ChangeRecord type (SHA, author, timestamp, message, file changes)
- [x] ChangeTaskMatrix (change x score entries, regression/improvement detection)
- [x] BlastRadius prediction (file overlap + path prefix matching)
- [x] BisectState (binary search for regression between commits)
- [x] 26 tests passing
- [x] Clippy zero warnings, rustfmt clean
- [x] No git library dependency — accepts data as inputs for testability

## Deferred

- Actual git repo reading (gix or git2 integration in CLI/orchestrator)
- PR/issue tracker integration
- Wiring into SPC control chart points
