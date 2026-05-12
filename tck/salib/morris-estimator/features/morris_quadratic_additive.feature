# Substantive reviewer-affordance close for Morris elementary effects
# over the quadratic-additive test function
# `Y = Σ bᵢxᵢ + cᵢxᵢ²` with `bᵢ = cᵢ = i+1` for d=8.
#
# Unlike the additive-linear case (PR 8), this function has real MC
# noise on EE — `EE_i(x) = bᵢ + cᵢ·(2·xᵢ + Δ)` varies with the base
# point. Closed-form effects (Morris p=4, lower-half base, Δ=2/3):
#   μᵢ = bᵢ + cᵢ = 2(i+1)              → [2, 4, 6, ..., 16]
#   σᵢ = |cᵢ| / 3 = (i+1) / 3            → [0.333, 0.667, ..., 2.667]
#
# Provenance: Morris 1991 (μ, σ); Campolongo 2007 (μ*).
# Mechanized: `crates/saltelli-estimators/tests/morris_quadratic_tck.rs`.
# ADR: `decisions/2026-04-29-saltelli-morris-quadratic-contract.md`.

Feature: estimate_morris_effects — quadratic-additive at d=8, p=4

  Scenario: Morris recovers analytic μ and σ within MC tolerance at R=1000
    Given the Morris quadratic-additive model with d=8
    And Morris trajectories with R=1000 and levels=4
    When I estimate Morris elementary effects
    Then μ approximates 2 4 6 8 10 12 14 16 within 0.1
    And σ approximates 0.333 0.667 1.0 1.333 1.667 2.0 2.333 2.667 within 0.15

  Scenario: μ* always at least absolute μ on the quadratic-additive
    Given the Morris quadratic-additive model with d=8
    And Morris trajectories with R=100 and levels=4
    When I estimate Morris elementary effects
    Then μ* is at least absolute μ for every factor

  Scenario: largest-σ factor error decays below 0.15 at R=1000
    Given the Morris quadratic-additive model with d=8
    And Morris trajectories with R=1000 and levels=4
    When I estimate Morris elementary effects
    Then μ error for factor 7 is below 0.15
    And σ error for factor 7 is below 0.15
