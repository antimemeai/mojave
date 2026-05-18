---
id: BEAD-0010
title: Game-theoretic evaluation design (anti-gaming)
status: closed
priority: nice-to-have
created: 2026-05-11
closed: 2026-05-18
---

## Description

Deterministic benchmarks are inherently gameable. Stochastic evaluation mechanisms (randomized item selection, randomized perturbation schedules) reward genuine capability over gaming.

## What was built

`crates/eval-design` — game-theoretic evaluation scheduling primitives:

### Item Pool (`item_pool` module)
- `ItemPool`: validated collection of `ItemMetadata` (difficulty, discrimination, content domain, exposure count)
- Domain-aware queries, duplicate detection, exposure tracking

### Selection Strategies (`selection` module)
- `UniformRandom`: each item equally likely — simplest anti-gaming baseline
- `StratifiedRandom`: coverage guarantee across content domains + random fill
- `InformationWeighted`: discrimination-proportional selection — balances measurement efficiency against unpredictability
- `ExposureControl`: None, MaxExposures (hard cap), ConditionalProbability (Sympson-Hetter style)
- All strategies deterministic via ChaCha20Rng seeding

### Perturbation Schedule (`perturbation_schedule` module)
- `generate_schedule`: per-run random assignment of items to perturbation families + seeds
- `generate_schedule_series`: N-run series for full coverage anti-gaming
- `coverage_report`: empirical verification that coverage converges to configured rate
- Control group management (unperturbed items as experimental controls)

### Cross-cutting
- All types `#[non_exhaustive]`, `Serialize`/`Deserialize`
- 28 unit tests covering determinism, variation, domain coverage, exposure control, convergence

## Acceptance criteria met

- [x] Randomized item selection with multiple strategies
- [x] Exposure control to prevent item predictability
- [x] Stratified sampling for domain coverage guarantee
- [x] Information-weighted selection for measurement efficiency
- [x] Randomized perturbation scheduling with control groups
- [x] Multi-run series with convergence verification
- [x] All seeded via ChaCha20Rng for determinism
- [x] All tests pass, clippy clean
