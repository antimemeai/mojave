# Multi-stream ChaCha20 with deterministic salt-derived forking.
#
# Pins the load-bearing claims for `saltelli_core::rng::RngState`:
#
# 1. Pure determinism. Same `(seed, stream, word_pos)` produces
#    bit-identical output. The verifier of any sample matrix can
#    reconstruct it from the recorded `RngState` alone.
#
# 2. Deterministic forking. `parent.fork(salt)` is a pure function
#    of `(parent.stream, salt)`; same parent + same salt always
#    produces the same child. This is what makes parallel sampling
#    deterministic — workers fork by block index (`format!("block-{i}")`
#    as salt), and the resulting per-block streams are stable across
#    different rayon thread counts and across reruns.
#
# 3. Salt-stream collision resistance. Distinct salts produce
#    distinct streams (modulo SHA-256 collision odds, which are
#    crypto-strength irrelevant here).
#
# 4. `word_pos` snapshot resumption. Recording `word_pos` after a
#    partial draw and re-creating a `ChaCha20Rng` from the snapshot
#    produces the same byte sequence as continuing the original.
#    This is what lets the ledger persist `RngState` mid-campaign
#    and resume cleanly.
#
# Provenance: substrate's audit-canonical pattern + the `rust-rand`
# book's multi-stream recipe. See
# `decisions/2026-04-28-saltelli-rng-determinism.md`.
#
# Mechanized: `crates/saltelli-core/tests/rng_determinism_tck.rs`.

Feature: RngState — multi-stream ChaCha20 with deterministic forking

  Scenario: same seed + stream + word_pos produces identical bytes
    Given an RngState seeded with 0x42 repeated and stream 7
    When I draw 1024 bytes from the underlying ChaCha20
    And I draw 1024 bytes from a fresh RngState with the same seed and stream
    Then both draws are bit-identical

  Scenario: forking with the same salt is deterministic
    Given an RngState seeded with 0x42 repeated and stream 0
    When I fork it with salt "saltelli-block-0"
    And I fork the same parent again with salt "saltelli-block-0"
    Then both forked RngStates are equal field-for-field

  Scenario: distinct salts produce distinct streams
    Given an RngState seeded with 0x42 repeated and stream 0
    When I fork it with salt "saltelli-block-0"
    And I fork the same parent with salt "saltelli-block-1"
    Then the two forked RngStates differ in their stream value
    And drawing 1024 bytes from each fork produces different bytes

  Scenario: forking is pure under the parent's stream
    Given an RngState seeded with 0x42 repeated and stream 12345
    When I fork it with salt "saltelli-block-0"
    Then the fork is a pure function of (parent.stream, salt) and the parent's seed
    And re-deriving with the same parent.stream and salt produces an equal fork

  Scenario: word_pos snapshot enables byte-for-byte resumption
    Given an RngState seeded with 0x42 repeated and stream 0
    When I draw 8192 bytes from the underlying ChaCha20
    And I snapshot the RngState at the post-draw word_pos
    And I create a fresh ChaCha20 from the snapshot
    Then drawing 1024 more bytes from the resumed ChaCha20 matches the next 1024 bytes of an unbroken draw
