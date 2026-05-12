# Phase D PR 15 — Janon 2014 + Jansen 1999 alt + Owen 2013 first-
# order Sobol' estimators on Ishigami.
#
# Per `decisions/2026-04-29-saltelli-phase-d-pr15.md`. Three new
# first-order estimators consuming saltelli/Owen sampling matrices.
# Ishigami at canonical (a=7, b=0.1):  S = [0.314, 0.442, 0.000].
#
# Mechanized: `crates/saltelli-estimators/tests/phase_d_efficiency_tck.rs`.

Feature: Phase D PR 15 — alternative first-order Sobol' estimators

  Scenario: Janon T_N^X recovers Ishigami within MC tolerance
    Given the Ishigami model on Uniform[-π, π]³
    And a Saltelli matrix at N=4096
    When I estimate Janon first-order
    Then S approximates 0.314 0.442 0.000 within 0.05

  Scenario: Jansen 1999 squared-difference recovers Ishigami within MC tolerance
    Given the Ishigami model on Uniform[-π, π]³
    And a Saltelli matrix at N=4096
    When I estimate Jansen first-order
    Then S approximates 0.314 0.442 0.000 within 0.05

  Scenario: Owen Correlation 2 recovers Ishigami within MC tolerance
    Given the Ishigami model on Uniform[-π, π]³
    And an Owen matrix at N=4096
    When I estimate Owen first-order
    Then S approximates 0.314 0.442 0.000 within 0.05

  Scenario: Janon at least as accurate as Saltelli2010 at N=4096
    Given the Ishigami model on Uniform[-π, π]³
    And a Saltelli matrix at N=4096
    When I estimate both Saltelli2010 and Janon
    Then Janon max-error does not exceed Saltelli2010 max-error

  Scenario: Owen tightly bounds the small-S_3 Ishigami factor
    Given the Ishigami model on Uniform[-π, π]³
    And an Owen matrix at N=4096
    When I estimate Owen first-order
    Then S for factor 2 has magnitude below 0.05
