---
id: BEAD-0012
title: Perturbation engine (systematic eval robustness testing)
status: open
priority: nice-to-have
created: 2026-05-11
---

## Description

Systematic perturbation of eval conditions to measure robustness. Maps the boundary between "agent can do this" and "agent does this under these conditions." Disposition profile measurement.

## What exists (in quarantine)

- paraphrase-proxy: LLM-based prompt rewriting
- format-spread-proxy: formatting perturbations (separator, casing, punctuation atoms)
- multi-turn-perturbation: conversation reordering, truncation, injection

## Theoretical grounding

- Voudouris et al. 2026: capabilities as dispositional properties, mapped via systematic context variation
- Sensitivity analysis (salib-rs): Sobol indices on perturbation factors quantify what drives score variance

## When to revisit

- After orchestration layer can run experiments
- Connects naturally to sensitivity analysis (already built) and ablation scheduling
- The conex adapters in quarantine are reference implementations, not production code
