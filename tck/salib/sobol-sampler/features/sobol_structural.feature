# Structural invariants of the Sobol' sampler — output shape,
# [0, 1) bounds, the load-bearing stratification property at
# N = 2^k.
#
# Stratification: Sobol' is a low-discrepancy QMC sequence; for
# any N = 2^k with k ≤ RES (RES = 32 in our impl), the first N
# points partition [0, 1) into N exact bins per dimension. Each
# bin contains exactly one point. This is the QMC equivalent of
# the LHS stratification property (per
# `tck/saltelli/lhs-sampler/`) but at higher resolution.
#
# Mechanized: `crates/saltelli-samplers/tests/sobol_tck.rs`.

Feature: SobolSampler — structural invariants

  Scenario: output shape matches n by dim
    Given a Sobol sampler with dim 4 dim_set Standard skip_first true
    When I draw a unit sample of size 64
    Then the matrix shape is 64 by 4

  Scenario: zero rows produces a 0-by-d matrix
    Given a Sobol sampler with dim 3 dim_set Standard skip_first true
    When I draw a unit sample of size 0
    Then the matrix shape is 0 by 3

  Scenario: skip_first true keeps all values in [0, 1)
    Given a Sobol sampler with dim 5 dim_set Standard skip_first true
    When I draw a unit sample of size 256
    Then every value is in [0, 1)

  Scenario: skip_first false includes the origin row
    Given a Sobol sampler with dim 3 dim_set Standard skip_first false
    When I draw a unit sample of size 8
    Then every value is in [0, 1)

  Scenario: stratification at N = 2^10 partitions each column into N bins
    Given a Sobol sampler with dim 3 dim_set Standard skip_first false
    When I draw a unit sample of size 1024
    Then for every column the floor of value times n is a permutation of 0 through n-1

  Scenario: dim-set Minimal supports up to 100 dims
    Given a Sobol sampler with dim 100 dim_set Minimal skip_first true
    When I draw a unit sample of size 16
    Then the matrix shape is 16 by 100

  Scenario: dim-set Standard supports up to 1000 dims
    Given a Sobol sampler with dim 500 dim_set Standard skip_first true
    When I draw a unit sample of size 8
    Then the matrix shape is 8 by 500
