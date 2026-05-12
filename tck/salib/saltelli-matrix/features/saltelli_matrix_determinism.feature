# Determinism — same `(sampler, RngState)` produces bit-identical
# `SaltelliMatrix` output. Pure under the inputs; replay is exact.
#
# Mechanized: `crates/saltelli-samplers/tests/saltelli_matrix_tck.rs`.

Feature: SaltelliMatrix — determinism

  Scenario: LHS base same RngState produces bit-identical matrix
    Given a classic LHS sampler with dim 6
    When I build a Saltelli matrix with n 64 second_order false using stream 0
    And I build a second Saltelli matrix with n 64 second_order false using stream 0
    Then both Saltelli matrices are bit-identical

  Scenario: Sobol base same RngState produces bit-identical matrix
    Given a Sobol sampler with dim 8 dim_set Standard skip_first true
    When I build a Saltelli matrix with n 32 second_order false using stream 0
    And I build a second Saltelli matrix with n 32 second_order false using stream 0
    Then both Saltelli matrices are bit-identical

  Scenario: distinct streams under LHS produce different matrices
    Given a classic LHS sampler with dim 4
    When I build a Saltelli matrix with n 32 second_order false using stream 1
    And I build a second Saltelli matrix with n 32 second_order false using stream 2
    Then the two Saltelli matrices differ
