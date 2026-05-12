# Headline reviewer-affordance close: Saltelli's 2010 first-order +
# Jansen 1999 total-order estimator over the canonical Ishigami test
# function `(a=7, b=0.1)` at N=8192. The estimate must agree with
# closed-form analytic indices within MC-noise tolerance.
#
# Ishigami has two canary properties this scenario pins:
#   - X_3 first-order is exactly 0 by closed form (X_3 enters Y only
#     through the X_1 X_3 interaction).
#   - X_2 has no interactions with X_1 or X_3, so S_T2 = S_2 to
#     within MC noise.
#
# Plus model-free identity: Σ S_i ≤ 1.
#
# The other reviewer-affordance contract artifacts (convergence-rate,
# SALib differential, identity tests) live in
# `crates/saltelli-estimators/tests/ishigami_e2e.rs` because their
# shape (parameter sweeps, file reads) doesn't fit Gherkin cleanly.
#
# Provenance: Saltelli Primer 2008 §5.4 (Ishigami closed forms);
# Saltelli et al. 2010 (Eq c first-order, Eq f total-order via
# Jansen 1999). See
# `decisions/2026-04-29-saltelli-saltelli2010-estimator.md`.
#
# Mechanized: `crates/saltelli-estimators/tests/saltelli2010_tck.rs`.

Feature: estimate_saltelli2010 — Ishigami end-to-end at canonical (a=7, b=0.1, N=8192)

  Scenario: Saltelli2010 recovers Ishigami's published values within MC tolerance
    Given the Ishigami canonical model with a=7 and b=0.1
    And a Sobol base sampler at dim 6 with skip_first false
    When I build a Saltelli matrix at N=8192 and run Saltelli2010
    Then S_1 is within 0.05 of analytic 0.3139
    And S_2 is within 0.05 of analytic 0.4424
    And S_3 is within 0.05 of analytic 0.0
    And S_T1 is within 0.05 of analytic 0.5576
    And S_T2 is within 0.05 of analytic 0.4424
    And S_T3 is within 0.05 of analytic 0.2436

  Scenario: X_3 first-order is the canary near zero
    Given the Ishigami canonical model with a=7 and b=0.1
    And a Sobol base sampler at dim 6 with skip_first false
    When I build a Saltelli matrix at N=8192 and run Saltelli2010
    Then S_3 is within 0.05 of zero

  Scenario: X_2 total-order equals first-order (no interactions)
    Given the Ishigami canonical model with a=7 and b=0.1
    And a Sobol base sampler at dim 6 with skip_first false
    When I build a Saltelli matrix at N=8192 and run Saltelli2010
    Then S_2 and S_T2 agree within 0.05

  Scenario: sum of first-order indices is at most 1
    Given the Ishigami canonical model with a=7 and b=0.1
    And a Sobol base sampler at dim 6 with skip_first false
    When I build a Saltelli matrix at N=8192 and run Saltelli2010
    Then the sum of first-order indices is at most 1.05
