# Determinism invariants — unscrambled Sobol' is fully deterministic
# given (dim, dim_set, skip_first). The `RngState` argument is part
# of the `Sampler` trait shape (will be consumed when Owen-hash
# scrambling lands in a follow-on PR) but is not consumed in PR 5b.
#
# This feature pins the no-RNG-consumption property: word_pos is
# invariant under unit_sample, and outputs are bit-identical
# regardless of the input RngState's stream / word_pos.
#
# Mechanized: `crates/saltelli-samplers/tests/sobol_tck.rs`.

Feature: SobolSampler — determinism

  Scenario: same config produces bit-identical output across calls
    Given a Sobol sampler with dim 4 dim_set Standard skip_first true
    When I draw a unit sample of size 128 with stream 0
    And I draw a second unit sample of size 128 with stream 0
    Then both matrices are bit-identical

  Scenario: different RngState streams produce identical output for unscrambled Sobol
    Given a Sobol sampler with dim 4 dim_set Standard skip_first true
    When I draw a unit sample of size 64 with stream 1
    And I draw a second unit sample of size 64 with stream 999
    Then both matrices are bit-identical

  Scenario: unit_sample does not advance word_pos under unscrambled Sobol
    Given a Sobol sampler with dim 3 dim_set Standard skip_first true
    When I draw a unit sample of size 64 with stream 0
    Then the post-draw RngState word_pos is 0
