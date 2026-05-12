# Shapley effects (Song-Nelson-Staum 2016 Algorithm 1) on Ishigami
# canonical (a=7, b=0.1).
#
# The defining property: Σ Sh_i = Var(Y) (Song 2016 Eq 10), exactly
# under the prevC-carry telescoping trick. Under independence, the
# Shapley sandwich V_i ≤ Sh_i ≤ V_T_i (Song 2016 Theorem 2) holds,
# giving each Sh_i a closed-form value derivable from the Ishigami
# Sobol' decomposition:
#
#   Sh_1 = V_1 + ½·V_13 ≈ 6.032
#   Sh_2 = V_2          ≈ 6.124   (X_2 enters no interaction)
#   Sh_3 = ½·V_13       ≈ 1.687
#
# Per `decisions/2026-04-29-saltelli-shapley.md`. Mechanized:
# `crates/saltelli-shapley/tests/shapley_tck.rs`.

Feature: estimate_shapley — Ishigami at d=3, independent inputs

  Scenario: Σ Sh_i equals Var(Y) up to telescoping precision
    Given the Ishigami model on Uniform[-π, π]³
    When I estimate Shapley effects with m=2000, N_O=1, N_I=3, N_V=4000
    Then the sum of Shapley indices equals Var(Y) within 1e-9

  Scenario: Shapley indices recover closed-form values within MC tolerance
    Given the Ishigami model on Uniform[-π, π]³
    When I estimate Shapley effects with m=4000, N_O=1, N_I=3, N_V=8000
    Then Sh_1 approximates 6.0327 within 1.0
    And Sh_2 approximates 6.1250 within 1.0
    And Sh_3 approximates 1.6868 within 1.0

  Scenario: Shapley sandwiches first-order and total-order Sobol' under independence
    Given the Ishigami model on Uniform[-π, π]³
    When I estimate Shapley effects with m=4000, N_O=1, N_I=3, N_V=8000
    Then every Sh_i lies between V_i and V_T_i within MC slack 0.5

  Scenario: Shapley estimation is deterministic
    Given the Ishigami model on Uniform[-π, π]³
    When I estimate Shapley effects twice with m=64, N_O=1, N_I=3, N_V=256
    Then the two index sets are bit-identical
