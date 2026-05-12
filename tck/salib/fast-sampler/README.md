# TCK — saltelli FAST search-curve sampler

The Layer-1 outer Gherkin gate for `saltelli_samplers::build_fast_design`
— the Saltelli-Tarantola-Chan 1999 design (also known as eFAST in
the literature, named `fast` here for SALib-API-affinity per
`decisions/2026-04-29-saltelli-fast-sampler.md`).

## What this directory covers

- **`fast_design.feature`** — structural / determinism claims:
  - every sample value in `[0, 1]`,
  - factor-of-interest `i` holds the maximum frequency `ω_i` in
    row `i`,
  - complementary frequencies stay below `ω_max / (2·M)` (no
    spectral overlap up to the `M`th harmonic),
  - complementary frequencies are pairwise distinct within a block,
  - bit-identity under repeated builds from the same seed.

## What this directory does NOT cover

- **The estimator's spectral analysis claim.** PR 9b adds
  `estimate_fast` returning `(Sᵢ, Sᵀᵢ)` and pins the Ishigami
  closed-form recovery in `tck/saltelli/fast-estimator/`.
- **Uniform-marginal property.** Inner unit test in
  `crates/saltelli-samplers/src/fast.rs::tests` checks the
  empirical CDF of each column against the uniform.

## Step-definition home

- `crates/saltelli-samplers/tests/fast_tck.rs` wires
  `fast_design.feature`.

## See also

- `decisions/2026-04-29-saltelli-fast-sampler.md` — the ADR this
  directory operationalizes.
- `decisions/2026-04-28-saltelli-tck-posture.md` — four-layer
  validation strategy.
- Saltelli, A., Tarantola, S., Chan, K. P-S. (1999). "A quantitative
  model-independent method for global sensitivity analysis of model
  output." *Technometrics* 41(1).
- Cukier, R. I., Fortuin, C. M., Shuler, K. E., Petschek, A. G.,
  Schaibly, J. H. (1973). "Study of the sensitivity of coupled
  reaction systems to uncertainties in rate coefficients." *J. Chem.
  Phys.* 59. (Original FAST.)
