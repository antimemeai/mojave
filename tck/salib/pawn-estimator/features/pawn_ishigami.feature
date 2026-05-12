# PAWN estimator on Ishigami — Pianosi-Wagener 2015/2018 moment-
# independent index via Kolmogorov-Smirnov statistics on conditional
# vs unconditional CDFs.
#
# Per `decisions/2026-04-29-saltelli-pawn.md`. PAWN has no closed-
# form analytic value for Ishigami, so we validate via:
#   - identity: indices in [0, 1]; min ≤ median ≤ max
#   - ranking: median_2 > median_1 > median_3 (matches SALib)
#   - SALib differential at N=4096
#
# Mechanized: `crates/saltelli-estimators/tests/pawn_tck.rs`.

Feature: estimate_pawn — Ishigami at d=3, S=10

  Scenario: indices stay within unit interval
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096 with S=10 slices
    When I estimate PAWN
    Then every median is in 0 to 1
    And every max is in 0 to 1

  Scenario: aggregate ordering min ≤ median ≤ max
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096 with S=10 slices
    When I estimate PAWN
    Then for every factor min is at most median
    And for every factor median is at most max

  Scenario: factor ranking by median is correct for Ishigami
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096 with S=10 slices
    When I estimate PAWN
    Then median_2 strictly exceeds median_1
    And median_1 strictly exceeds median_3

  Scenario: median agrees with SALib reference within MC tolerance
    Given the Ishigami model on Uniform[-π, π]³
    And LHS samples at N=4096 with S=10 slices
    When I estimate PAWN
    Then median is within 0.05 of SALib's frozen reference
