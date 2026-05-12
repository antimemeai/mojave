# Tree-fold reductions — bit-identical f64 output regardless of rayon
# partitioning.
#
# `f64 + f64` is not associative. If rayon partitions a length-N
# vector differently across runs (different machine, different rayon
# version, different available cores), a naive `par_iter().sum()`
# produces *different* `f64` values. For Sobol' indices computed as
# `(1/N) Σ f(B)_j (f(A_Bⁱ)_j - f(A)_j)`, that means bit-different
# indices across runs of the same seed. Unacceptable for ledger
# byte-exactness.
#
# `saltelli_core::reduce` defends with a fixed-tree pairwise fold
# over fixed-size blocks (`BLOCK = 4096`). Each block reduces
# sequentially in deterministic order; per-block sums are written to
# a `Vec<f64>` at known indices; the per-block sums reduce via the
# same fixed-tree fold. Result: bit-identical regardless of which
# rayon worker computed which block, regardless of how many workers
# rayon decides to use.
#
# This feature pins the bit-exactness claim across rayon thread
# counts {1, 2, 8, 32}. Cross-platform reproducibility under FMA /
# AVX-512 is a separate concern (bead `thunderdome-i8q`); this
# feature runs on whatever the local box compiles to.
#
# Provenance: `rust_salib_crate_research.md` § 6.1, ported. See
# `decisions/2026-04-28-saltelli-rng-determinism.md`.
#
# Mechanized: `crates/saltelli-core/tests/tree_fold_tck.rs`.

Feature: tree_sum / tree_dot / tree_var — bit-exact across rayon thread counts

  Scenario Outline: par_tree_sum is bit-identical to sequential tree_sum
    Given a length-<n> f64 vector seeded by ChaCha20 stream 0
    When I compute par_tree_sum with <threads> rayon worker threads
    Then the result is bit-identical to tree_sum on the same vector

    Examples:
      | n     | threads |
      | 4096  | 1       |
      | 4096  | 2       |
      | 4096  | 8       |
      | 4096  | 32      |
      | 65536 | 1       |
      | 65536 | 2       |
      | 65536 | 8       |
      | 65536 | 32      |

  Scenario Outline: par_tree_dot is bit-identical to sequential tree_dot
    Given length-<n> f64 vectors a seeded by ChaCha20 stream 1 and b seeded by stream 2
    When I compute par_tree_dot with <threads> rayon worker threads
    Then the result is bit-identical to tree_dot on the same vectors

    Examples:
      | n     | threads |
      | 4096  | 8       |
      | 65536 | 32      |

  Scenario: par_tree_sum is bit-identical to itself across reruns
    Given a length-65536 f64 vector seeded by ChaCha20 stream 0
    When I compute par_tree_sum with 8 rayon worker threads
    And I compute par_tree_sum again with 8 rayon worker threads
    Then both results are bit-identical

  Scenario: tree_var is bit-identical across rayon thread counts
    Given a length-65536 f64 vector seeded by ChaCha20 stream 0
    When I compute par_tree_var with 32 rayon worker threads
    Then the result is bit-identical to tree_var on the same vector

  Scenario: empty input reduces to zero deterministically
    Given an empty f64 vector
    When I compute par_tree_sum with 8 rayon worker threads
    Then the result is bit-identical to 0.0

  Scenario: single-element input bypasses parallel work
    Given a length-1 f64 vector with the value 1.5
    When I compute par_tree_sum with 8 rayon worker threads
    Then the result is bit-identical to 1.5
