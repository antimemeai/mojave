---
id: BEAD-0014
title: Git/VCS integration for change attribution
status: open
priority: nice-to-have
created: 2026-05-11
---

## Description

Orchestration layer should be able to integrate with git (or other VCS) such that the actual changes under evaluation can be mapped to score movements. Recommended, not required — the system should work without it but provide richer attribution when it's available.

## What this enables

- Automatic change×task matrix entries tagged with commit SHAs
- "This commit regressed tasks 4, 12, 31" with links to the diff
- Blast radius prediction: "based on past changes to similar files, these tasks are likely affected"
- Bisect-like capability: "score regressed somewhere between commit A and commit B, let's binary search"

## Design principles

- Optional integration (system works without VCS access)
- When available, captures: commit SHA, diff summary, author, timestamp
- Maps to the longitudinal SPC layer (each point on the control chart can link to a change)
- Could also integrate with PR/issue trackers for richer context

## When to revisit

- During orchestration layer design
- This is a "recommended connector" not a hard dependency
