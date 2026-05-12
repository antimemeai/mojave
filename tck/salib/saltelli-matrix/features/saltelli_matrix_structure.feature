# Structural invariants of the radial Saltelli `(A, B, A_Bⁱ)`
# matrix construction. The load-bearing claim:
#
#   `A_Bⁱ` is `A` with column `i` replaced by `B.column(i)`.
#
# Mirror claim for `second_order`:
#
#   `B_Aⁱ` is `B` with column `i` replaced by `A.column(i)`.
#
# Plus shape invariants — `A` and `B` are `n × d` where
# `d = sampler.dim() / 2`; there are `d` `A_Bⁱ` matrices.
#
# Provenance: Saltelli 2010 Eqs 1-3 (radial design). See
# `decisions/2026-04-29-saltelli-matrix-construction.md`.
#
# Mechanized: `crates/saltelli-samplers/tests/saltelli_matrix_tck.rs`.

Feature: SaltelliMatrix — radial-design structural invariants

  Scenario: LHS base of dim 6 produces 3-factor matrices
    Given a classic LHS sampler with dim 6
    When I build a Saltelli matrix with n 64 second_order false
    Then the result has n 64 dim 3
    And A and B both have shape 64 by 3
    And there are 3 A_Bⁱ matrices each of shape 64 by 3

  Scenario: Sobol base of dim 8 produces 4-factor matrices
    Given a Sobol sampler with dim 8 dim_set Standard skip_first true
    When I build a Saltelli matrix with n 32 second_order false
    Then the result has n 32 dim 4
    And A and B both have shape 32 by 4

  Scenario: A_Bⁱ replaces column i with B's column i
    Given a classic LHS sampler with dim 6
    When I build a Saltelli matrix with n 32 second_order false
    Then for every i in 0 to dim minus 1 the i-th A_Bⁱ has column i equal to B's column i
    And for every i and every j not equal to i the i-th A_Bⁱ has column j equal to A's column j

  Scenario: second_order populates B_Aⁱ with symmetric structure
    Given a classic LHS sampler with dim 6
    When I build a Saltelli matrix with n 16 second_order true
    Then there are 3 B_Aⁱ matrices
    And for every i in 0 to dim minus 1 the i-th B_Aⁱ has column i equal to A's column i
    And for every i and every j not equal to i the i-th B_Aⁱ has column j equal to B's column j

  Scenario: total evaluations equals n times d plus 2 for first plus total
    Given a classic LHS sampler with dim 6
    When I build a Saltelli matrix with n 64 second_order false
    Then total evaluations is 320

  Scenario: total evaluations equals n times 2d plus 2 with second_order
    Given a classic LHS sampler with dim 6
    When I build a Saltelli matrix with n 32 second_order true
    Then total evaluations is 256

  Scenario: A and B are the disjoint halves of the 2d-dim base sample
    Given a Sobol sampler with dim 6 dim_set Standard skip_first false
    When I build a Saltelli matrix with n 16 second_order false
    Then A's columns are the first 3 columns of the base 2d-dim sample
    And B's columns are the last 3 columns of the base 2d-dim sample
