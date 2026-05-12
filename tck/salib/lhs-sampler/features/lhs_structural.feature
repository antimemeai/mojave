# Structural invariants of the LHS sampler.
#
# The load-bearing LHS property: in each column of the output
# matrix, the values stratify the unit interval — there is exactly
# one value in each cell `[k/n, (k+1)/n)` for k ∈ {0, …, n-1}. This
# is what distinguishes LHS from naive uniform sampling.
#
# For Centered LHS additionally: each value is *exactly* the center
# of its cell, `(k + 0.5) / n`.
#
# Provenance: McKay-Beckman-Conover 1979. See
# `decisions/2026-04-28-saltelli-lhs-sampler.md` § "What this gates."
#
# Mechanized: `crates/saltelli-samplers/tests/lhs_tck.rs`.

Feature: LhsSampler — structural invariants

  Scenario: classic LHS produces an n-by-d matrix
    Given a classic LHS sampler with dim 4
    When I draw a unit sample of size 64 with seed [0x42; 32] and stream 0
    Then the matrix shape is 64 by 4

  Scenario: classic LHS values are in the unit interval [0, 1)
    Given a classic LHS sampler with dim 3
    When I draw a unit sample of size 128 with seed [0x42; 32] and stream 0
    Then every value is in [0, 1)

  Scenario: classic LHS stratifies each column into one sample per cell
    Given a classic LHS sampler with dim 5
    When I draw a unit sample of size 32 with seed [0x42; 32] and stream 0
    Then for every column the floor of value times n is a permutation of 0 through n-1

  Scenario: centered LHS values are exactly cell centers
    Given a centered LHS sampler with dim 3
    When I draw a unit sample of size 16 with seed [0x42; 32] and stream 0
    Then for every column the sorted values equal the cell-center sequence

  Scenario: zero rows produces a 0-by-d matrix
    Given a classic LHS sampler with dim 3
    When I draw a unit sample of size 0 with seed [0x42; 32] and stream 0
    Then the matrix shape is 0 by 3

  Scenario: zero dim produces an n-by-0 matrix
    Given a classic LHS sampler with dim 0
    When I draw a unit sample of size 8 with seed [0x42; 32] and stream 0
    Then the matrix shape is 8 by 0

  Scenario: n=1 classic produces a single in-range value per column
    Given a classic LHS sampler with dim 2
    When I draw a unit sample of size 1 with seed [0x42; 32] and stream 0
    Then the matrix shape is 1 by 2
    And every value is in [0, 1)

  Scenario: n=1 centered produces 0.5 in every column
    Given a centered LHS sampler with dim 2
    When I draw a unit sample of size 1 with seed [0x42; 32] and stream 0
    Then every value equals 0.5
