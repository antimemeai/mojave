# TCK — saltelli Sobol' sampler

The Layer-1 outer Gherkin gate for `saltelli_samplers::SobolSampler`
(unscrambled Sobol' QMC sequence on vendored Joe-Kuo direction
numbers, Antonov-Saleev gray-code recursion).

Mirrors the TCK posture in
`decisions/2026-04-28-saltelli-tck-posture.md` § "Layer 1." Inner
unit tests in `crates/saltelli-samplers/src/sobol.rs::tests` cover
the same properties + private-helper coverage at finer granularity.

## What this directory covers

- **`sobol_canonical_values.feature`** — known Sobol' first-point
  values for dim 1 and dim 2 (cross-check against Saltelli Primer
  2008 §4 and standard textbook values). Pins
  `decisions/2026-04-29-saltelli-sobol-sampler.md` § "Algorithm
  validation."

- **`sobol_structural.feature`** — output shape, `[0, 1)` bounds,
  skip-first toggle behavior, stratification at `N = 2^k` (each Sobol'
  point falls in a distinct sub-interval). Pins the same ADR §
  "What this gates."

- **`sobol_determinism.feature`** — same config produces bit-identical
  output regardless of `RngState`; unscrambled Sobol' does not consume
  RNG bits. Pins the same ADR § "Determinism."

## What this directory does NOT cover

- **Owen-hash scrambling.** Not yet implemented; lands as a follow-on
  PR with its own ADR (Burley 2020 hash-based scrambling). When it
  lands, `RngState` will be consumed for the scramble seed.
- **RDS scrambling.** Same.
- **Extended (21,201-dim) Joe-Kuo table.** Not vendored today;
  `SobolDimSet::Minimal` (100 dims) and `Standard` (1000 dims) are
  sufficient for typical SA workloads (≤ 50 factors).
- **`SALib`-byte-exact differential.** Layer 3 of the validation
  strategy; lands in PR 7 (the first Sobol estimator) under
  `crates/saltelli-validation/reference/salib_outputs/`.

## Step-definition home

- `crates/saltelli-samplers/tests/sobol_tck.rs` wires all three
  feature files.

## See also

- `decisions/2026-04-29-saltelli-sobol-sampler.md` — the ADR this
  directory operationalizes.
- `decisions/2026-04-28-saltelli-lhs-sampler.md` — sibling sampler
  ADR; Sobol' implements the same `Sampler` trait.
- `data/LICENSE.joe-kuo` — BSD-3-Clause attribution for the
  vendored direction-number tables.
- Joe, Kuo 2008 — original direction-number distribution.
- Owen 2020 — "On dropping the first Sobol' point."
