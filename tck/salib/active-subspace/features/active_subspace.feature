# Active subspaces (Constantine 2014) — gradient-based dimension
# reduction for global sensitivity analysis.
#
# C̃ = (1/M) Σ ∇f_j ∇f_jᵀ; eigendecompose; top-k eigenvectors form
# the active subspace via largest-eigenvalue-gap heuristic.
#
# Two scenarios pin the load-bearing claims:
#   1. Ridge function f(x) = aᵀx → C̃ rank-1, eigenvector = a/||a||.
#   2. Ishigami canonical → spectrum reflects per-factor mean-
#      squared gradients (X_2 dominates due to a=7 coefficient).
#
# Per `decisions/2026-04-29-saltelli-active-subspace.md`. Mechanized:
# `crates/saltelli-surrogate/tests/active_subspace_tck.rs`.

Feature: compute_active_subspace — Constantine 2014

  Scenario: ridge function yields rank-1 C with leading eigenvector aligned to a
    Given the ridge model f(x) = 3·x_0 + 4·x_2 on Uniform[-1, 1]³
    When I compute finite-difference gradients at N=32 LHS samples
    And I compute the active subspace
    Then the leading eigenvalue approximates 25 within 1e-6
    And the second and third eigenvalues are at most 1e-6
    And the leading eigenvector is aligned with a/||a|| up to sign
    And k_active equals 1

  Scenario: Ishigami spectrum identifies X_2 as the leading active direction
    Given the Ishigami model on Uniform[-π, π]³
    When I compute finite-difference gradients at N=256 LHS samples
    And I compute the active subspace
    Then all three eigenvalues are strictly positive
    And the leading eigenvalue approximates 24.5 within 3.0
    And the leading eigenvector is X_2-aligned with magnitude at least 0.95

  Scenario: active-subspace computation is deterministic
    Given the model f(x) = x_0² + 2·x_1 + sin(x_2) on Uniform[-1, 1]³
    When I compute the active subspace twice on the same gradient samples
    Then the two eigendecompositions are bit-identical
