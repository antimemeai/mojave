# Headline reviewer-affordance close: Morris elementary effects over
# the additive-linear test function `Y = Σᵢ i·xᵢ` for d=8.
#
# Purely linear ⇒ EE_i is constant per trajectory ⇒ μ_i = i,
# μ*_i = i, σ_i = 0 exactly (modulo FP arithmetic). The function is
# perfect for cross-checking Morris formula correctness without MC
# noise confounding.
#
# Provenance: Morris 1991 (μ, σ); Campolongo 2007 (μ*).
# Mechanized: `crates/saltelli-estimators/tests/morris_tck.rs`.

Feature: estimate_morris_effects — additive-linear at d=8, R=50, p=4

  Scenario: Morris recovers the analytic effects exactly
    Given the Morris additive-linear model with d=8
    And Morris trajectories with R=50 and levels=4
    When I estimate Morris elementary effects
    Then μ equals 1 2 3 4 5 6 7 8 within 1e-10
    And μ* equals 1 2 3 4 5 6 7 8 within 1e-10
    And σ equals 0 0 0 0 0 0 0 0 within 1e-10

  Scenario: μ* always at least absolute μ (Campolongo 2007)
    Given the Morris additive-linear model with d=8
    And Morris trajectories with R=50 and levels=4
    When I estimate Morris elementary effects
    Then μ* is at least absolute μ for every factor

  Scenario: factors rank exactly by coefficient
    Given the Morris additive-linear model with d=8
    And Morris trajectories with R=30 and levels=4
    When I estimate Morris elementary effects
    Then μ* is strictly increasing across factors 0 through 7
