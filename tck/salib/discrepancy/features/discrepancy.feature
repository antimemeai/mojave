# Discrepancy indices — space-filling quality metrics.
#
# Hickernell 1998: centered, wrap-around, modified.
# Niederreiter: L2-star.
#
# Mechanized: `crates/salib-estimators/tests/discrepancy_tck.rs`.

Feature: Discrepancy indices — space-filling quality metrics

  Scenario: Regular grid has known centered discrepancy
    Given a 2D regular grid of 4 points in [0,1]^2
    When I compute discrepancy
    Then centered_discrepancy is within 0.01 of the analytic value

  Scenario: Sobol sequence has lower discrepancy than random
    Given a Sobol sample of N=256 in d=3
    And a random sample of N=256 in d=3
    When I compute discrepancy for both
    Then the Sobol centered_discrepancy is less than the random centered_discrepancy

  Scenario: Discrepancy decreases with N for Sobol
    Given Sobol samples at N=64 and N=256 in d=3
    When I compute discrepancy for both
    Then centered_discrepancy at N=256 is less than at N=64

  Scenario: All discrepancy values are non-negative
    Given any sample matrix in [0,1]^d
    When I compute discrepancy
    Then centered, wrap_around, modified, and l2_star are all non-negative
