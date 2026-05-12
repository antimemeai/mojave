# Sobol' first-point values cross-checked against canonical textbook
# values from Saltelli Primer 2008 §4 and standard references.
#
# Dim 1 (the trivial dim with all m_i = 1) produces the universally-
# recognized Sobol' "binary halving" sequence:
#   0.0, 0.5, 0.75, 0.25, 0.375, 0.875, 0.625, 0.125, 0.1875, ...
#
# Dim 2 uses Joe-Kuo's first non-trivial polynomial (s=1, a=0, m=[1])
# and produces:
#   0.0, 0.5, 0.25, 0.75, 0.375, 0.875, 0.125, 0.625, ...
#
# These values are bit-exact (no floating-point tolerance) — the
# Sobol' construction is integer arithmetic with f64 normalization
# at the end; the values are exact dyadic rationals representable
# in f64.
#
# Mechanized: `crates/saltelli-samplers/tests/sobol_tck.rs`.

Feature: SobolSampler — canonical first-point values

  Scenario: dim-1 first eight points match the canonical halving sequence
    Given a Sobol sampler with dim 1 dim_set Standard skip_first false
    When I draw a unit sample of size 8
    Then the dim-1 column equals 0.0 0.5 0.75 0.25 0.375 0.875 0.625 0.125 in order

  Scenario: dim-2 first eight points match the canonical Joe-Kuo sequence
    Given a Sobol sampler with dim 2 dim_set Standard skip_first false
    When I draw a unit sample of size 8
    Then the dim-2 column equals 0.0 0.5 0.25 0.75 0.375 0.875 0.125 0.625 in order

  Scenario: skip_first drops the all-zeros origin
    Given a Sobol sampler with dim 1 dim_set Standard skip_first true
    When I draw a unit sample of size 7
    Then the dim-1 column equals 0.5 0.75 0.25 0.375 0.875 0.625 0.125 in order

  Scenario: skip_first false keeps the origin
    Given a Sobol sampler with dim 3 dim_set Standard skip_first false
    When I draw a unit sample of size 4
    Then row 0 is the all-zeros origin
