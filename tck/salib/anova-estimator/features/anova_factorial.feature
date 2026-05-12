# ANOVA decomposition on balanced crossed factorial designs.
#
# Per `decisions/2026-05-01-anova-three-component-decomposition.md`
# and `decisions/2026-05-04-anova-bootstrap-ci-amendment.md`
# and `decisions/2026-05-04-anova-inferential-amendment.md`.
# The load-bearing claim is not just that we can compute a generic
# ANOVA table, but that Thunderdome can separate the three variance
# families that matter for generative evals: data, brittleness, and
# inference.
#
# Mechanized in `crates/saltelli-estimators/tests/anova_tck.rs`.

Feature: ANOVA decomposition on balanced factorial grids

  Scenario: two-way ANOVA decomposes row, column, and interaction variance
    Given a balanced 2 x 2 factorial grid with crossed interaction structure
    When I estimate two-way ANOVA components
    Then the two-way variance fractions sum to 1 within 1e-9
    And the interaction component exceeds the row component
    And the row component exceeds the column component

  Scenario: two-way ANOVA inferential statistics land only for main effects
    Given a balanced 2 x 2 factorial grid with crossed interaction structure
    When I estimate two-way ANOVA components
    Then inferential statistics exist for the two-way main effects
    And no inferential statistic is emitted for the two-way interaction term

  Scenario: three-way ANOVA decomposes data, brittleness, inference, and interactions
    Given a balanced 2 x 2 x 2 factorial grid with named crossed effects
    When I estimate three-way ANOVA components
    Then the three-way variance fractions sum to 1 within 1e-9
    And the data component exceeds the brittleness component
    And the brittleness component exceeds the inference component
    And the data-brittleness interaction exceeds the data-inference interaction

  Scenario: bootstrap confidence intervals land for ANOVA variance fractions
    Given a balanced 2 x 2 x 2 factorial grid with named crossed effects
    When I estimate three-way ANOVA components with bootstrap confidence intervals
    Then bootstrap confidence intervals exist for every three-way variance fraction
    And the bootstrap metadata records 128 resamples at alpha 0.05

  Scenario: inferential statistics land for inferentially testable ANOVA components
    Given a balanced 2 x 2 x 2 factorial grid with named crossed effects
    When I estimate three-way ANOVA components
    Then inferential statistics exist for the main effects and two-way interactions
    And no inferential statistic is emitted for the three-way interaction term
