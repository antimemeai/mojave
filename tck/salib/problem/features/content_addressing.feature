# Content-addressing of `Problem` via SHA-256 over canonical-JSON
# bytes. Pins the load-bearing claims for
# `Problem::content_hash() -> [u8; 32]`:
#
# 1. Stability across calls. The same `Problem` value hashed twice in
#    a single process produces bit-identical output.
#
# 2. Equivalence under value equality. Two `Problem` values built
#    from identical builder calls hash identically.
#
# 3. Distinctness under any semantic difference. Changing a factor
#    name, distribution parameter, factor order, or `FactorKind`
#    flips the hash. (No silent collisions modulo SHA-256 itself.)
#
# Provenance: per `decisions/2026-04-28-saltelli-problem-shape.md` §
# "Content-addressing." The hash lives inside saltelli's `context`
# payloads on the audit envelope (per
# `decisions/2026-04-28-saltelli-ledger-composition.md`); recording
# `Problem::content_hash()` lets a verifier assert "this result was
# produced by *this* Problem" without re-deriving the Problem from
# the audit log.
#
# Mechanized: `crates/saltelli-core/tests/problem_tck.rs`.

Feature: Problem — content-addressing via SHA-256 over canonical-JSON

  Scenario: same Problem hashes identically across repeated calls
    Given a Problem with one Uniform factor "x" on [0, 1]
    When I compute its content_hash twice
    Then both hashes are bit-identical

  Scenario: equal Problems built independently hash identically
    Given two Problems independently built with the same factor specs
    When I compute each Problem's content_hash
    Then the two hashes are bit-identical

  Scenario: differing distribution parameters produce different hashes
    Given a Problem with one Uniform factor "x" on [0, 1]
    And a Problem with one Uniform factor "x" on [0, 2]
    When I compute each Problem's content_hash
    Then the two hashes differ

  Scenario: differing factor names produce different hashes
    Given a Problem with one Uniform factor "x" on [0, 1]
    And a Problem with one Uniform factor "y" on [0, 1]
    When I compute each Problem's content_hash
    Then the two hashes differ

  Scenario: differing factor order produces different hashes
    Given a Problem with two factors named "a" then "b"
    And a Problem with the same two factors in order "b" then "a"
    When I compute each Problem's content_hash
    Then the two hashes differ

  Scenario: differing FactorKind produces different hashes
    Given a Problem with one Continuous Uniform factor "x" on [0, 1]
    And a Problem with one Discrete Uniform factor "x" on [0, 1]
    When I compute each Problem's content_hash
    Then the two hashes differ

  Scenario: hash output is exactly 32 bytes
    Given a Problem with one Uniform factor "x" on [0, 1]
    When I compute its content_hash
    Then the result is exactly 32 bytes

  Scenario: serde round-trip preserves content_hash
    Given a Problem with one Uniform factor "x" on [0, 1]
    When I serialize it to JSON and deserialize back
    Then the deserialized Problem's content_hash equals the original
