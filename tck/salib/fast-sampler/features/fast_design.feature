# FAST search-curve design — load-bearing structural claims for the
# Saltelli-Tarantola-Chan 1999 (a.k.a. eFAST) sampler.
#
# Per `decisions/2026-04-29-saltelli-fast-sampler.md`. The sampler
# produces (n_per_factor · d) samples of d unit-uniform factor values
# along d independent search curves, one per factor-of-interest.
#
# Load-bearing claims:
#   - Each sample is in [0, 1].
#   - For factor-of-interest i, ωᵢ is the maximum frequency assigned
#     and every other factor's frequency is below ωᵢ / (2·M)
#     (no spectral overlap up to the Mth harmonic).
#   - Complementary frequencies are pairwise distinct in the
#     "linspace" regime (m ≥ d−1). The cycling regime (m < d−1)
#     produces collisions by construction — `SALib` parity.
#   - Output is bit-identical given the same `RngState` input.
#
# Mechanized: `crates/saltelli-samplers/tests/fast_tck.rs`.

Feature: build_fast_design — Saltelli-Tarantola-Chan 1999 search curve

  Scenario: every sample lies in the unit interval
    Given d=6 factors and n_per_factor=129 with harmonic M=4
    When I build the FAST design
    Then every sample value is in the closed interval 0 to 1

  Scenario: factor-of-interest holds the maximum frequency
    Given d=6 factors and n_per_factor=129 with harmonic M=4
    When I build the FAST design
    Then for each block i, ω_i is the maximum frequency in row i

  Scenario: complementary frequencies are below ω_max/(2M)
    Given d=6 factors and n_per_factor=129 with harmonic M=4
    When I build the FAST design
    Then complementary frequencies stay below the harmonic-bandwidth bound

  Scenario: complementary frequencies are pairwise distinct in the linspace regime
    Given d=3 factors and n_per_factor=129 with harmonic M=4
    When I build the FAST design
    Then complementary frequencies within each block are pairwise distinct

  Scenario: design output is bit-identical from the same seed
    Given d=6 factors and n_per_factor=129 with harmonic M=4
    When I build the FAST design twice from the same seed
    Then the two designs are bit-identical
