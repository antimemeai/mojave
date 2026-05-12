# Determinism invariants of the LHS sampler.
#
# Same `RngState` in → bit-identical `Array2<f64>` out. Different
# streams produce different matrices. The sampler advances the
# input `RngState`'s `word_pos` to reflect bytes consumed; centered
# LHS consumes fewer bytes than classic (no per-cell offset draws).
#
# Replay-determinism is the load-bearing claim: the audit envelope
# records pre-draw `RngState`; a verifier rebuilds the matrix from
# scratch and compares.
#
# Provenance: per `decisions/2026-04-28-saltelli-rng-determinism.md`
# (multi-stream ChaCha20 with `set_word_pos` resumption) and
# `decisions/2026-04-28-saltelli-lhs-sampler.md` § "Determinism."
#
# Mechanized: `crates/saltelli-samplers/tests/lhs_tck.rs`.

Feature: LhsSampler — determinism

  Scenario: same RngState produces bit-identical matrix
    Given a classic LHS sampler with dim 4
    When I draw a unit sample of size 64 with seed [0x42; 32] and stream 7
    And I draw another unit sample of size 64 with seed [0x42; 32] and stream 7
    Then both matrices are bit-identical

  Scenario: same RngState advances RngState identically
    Given a classic LHS sampler with dim 4
    When I draw a unit sample of size 32 with seed [0x42; 32] and stream 0 and capture the post-draw RngState
    And I draw a second unit sample of size 32 with seed [0x42; 32] and stream 0 and capture the post-draw RngState
    Then both post-draw RngStates are equal

  Scenario: distinct streams produce different matrices
    Given a classic LHS sampler with dim 3
    When I draw a unit sample of size 32 with seed [0x42; 32] and stream 1
    And I draw a unit sample of size 32 with seed [0x42; 32] and stream 2
    Then the two matrices differ

  Scenario: unit_sample advances word_pos
    Given a classic LHS sampler with dim 3
    When I draw a unit sample of size 32 with seed [0x42; 32] and stream 0
    Then the post-draw RngState word_pos is greater than 0

  Scenario: zero-row draw does not consume RNG
    Given a classic LHS sampler with dim 3
    When I draw a unit sample of size 0 with seed [0x42; 32] and stream 0
    Then the post-draw RngState word_pos is 0

  Scenario: centered consumes fewer bytes than classic
    Given a classic LHS sampler with dim 2 and a centered LHS sampler with dim 2
    When I draw size-32 samples from each with seed [0x42; 32] and stream 0
    Then the classic sampler's post-draw word_pos exceeds the centered sampler's
