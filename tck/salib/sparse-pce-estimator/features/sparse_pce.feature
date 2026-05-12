# Sparse Polynomial Chaos Expansion via OMP forward selection +
# leave-one-out cross-validation (Blatman-Sudret 2011, Blatman 2009
# Ch 3-4). Same `PolynomialChaos` output type as full-OLS PCE
# (PR 16b) so `sobol_indices_from_pce` works unchanged.
#
# The load-bearing PCE claim: at high `d` or high `p`, where the
# full basis P = (d+p)!/(d!·p!) blows up, sparse selection
# recovers the same Sobol' indices with O(10×) fewer non-zero
# coefficients — the engineering pay-off PCE was designed for.
#
# Per `decisions/2026-04-29-saltelli-sparse-pce.md`. Mechanized:
# `crates/saltelli-surrogate/tests/sparse_pce_tck.rs`.

Feature: fit_sparse_pce — Ishigami at d=3 + sparse-additive d=10

  Scenario: sparse PCE recovers Ishigami first-order indices
    Given the Ishigami model on Uniform[-π, π]³ mapped to Legendre canonical [-1, 1]³
    And Sobol' QMC samples at N=4096
    When I fit a sparse Legendre PCE of total degree 10 with hyperbolic q=0.75
    And I compute Sobol' indices from the PCE coefficients
    Then S_1 approximates 0.3139 within 0.02
    And S_2 approximates 0.4424 within 0.02
    And S_3 approximates 0.0 within 0.02

  Scenario: sparse PCE has dramatically fewer non-zero coefficients than full PCE
    Given the Ishigami model on Uniform[-π, π]³ mapped to Legendre canonical [-1, 1]³
    And Sobol' QMC samples at N=4096
    When I fit a sparse Legendre PCE of total degree 10 with hyperbolic q=0.75
    Then the sparse PCE keeps at most 80 non-zero coefficients out of 286 candidates

  Scenario: sparse PCE picks out the active factors of an additive model
    Given the additive model Y = ξ_0 + 0.5·ξ_2 + 2·ξ_4 on d=10
    And LHS samples at N=512
    When I fit a sparse Legendre PCE of total degree 4
    Then only factors 0, 2, 4 carry non-trivial first-order Sobol' indices
    And factor 4 dominates over factor 0 over factor 2

  Scenario: hyperbolic truncation reduces basis size below total-degree
    When I enumerate the hyperbolic-truncated basis at d=10, p=4, q=0.5
    Then the basis size is at most 200
    And the basis size for total-degree truncation at d=10, p=4 is 1001

  Scenario: sparse PCE is deterministic
    Given the Ishigami model on Uniform[-π, π]³ mapped to Legendre canonical [-1, 1]³
    And Sobol' QMC samples at N=4096
    When I fit a sparse Legendre PCE of total degree 10 with hyperbolic q=0.75 twice
    Then the two PCEs have identical coefficients
