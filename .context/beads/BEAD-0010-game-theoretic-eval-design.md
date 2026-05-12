---
id: BEAD-0010
title: Game-theoretic evaluation design (anti-gaming)
status: open
priority: nice-to-have
created: 2026-05-11
---

## Description

Deterministic benchmarks are inherently gameable. Stochastic evaluation mechanisms (randomized item selection, randomized perturbation schedules) reward genuine capability over gaming.

## Key reference

- Truong, Wang & Koyejo 2026, "Guardians of the Measurement: Information Design for Robust AI Evaluation"
- Models benchmarking as a Stackelberg game between evaluators and model developers

## Relevance

- If customers' agents are trained against their own eval suites, the eval becomes gameable
- Randomized perturbation schedules + adaptive item selection provide natural resistance
- Connects to the perturbation engine (already exists in quarantine conceptually)

## When to revisit

- After orchestration layer can run evaluations
- When customers raise "my agent might be overfitting to my eval" concern
