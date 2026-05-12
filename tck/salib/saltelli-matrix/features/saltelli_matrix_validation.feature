# Validation errors from `build_saltelli_matrix`.
#
# Mechanized: `crates/saltelli-samplers/tests/saltelli_matrix_tck.rs`.

Feature: SaltelliMatrix — validation errors

  Scenario: zero rows returns ZeroN error
    Given a classic LHS sampler with dim 4
    When I attempt to build a Saltelli matrix with n 0 second_order false
    Then the result is a ZeroN error

  Scenario: odd sampler dim returns OddBaseDim error
    Given a classic LHS sampler with dim 5
    When I attempt to build a Saltelli matrix with n 32 second_order false
    Then the result is an OddBaseDim error with dim 5
