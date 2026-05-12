# Generalizability theory on balanced crossed p x i x r designs.
#
# Per `decisions/2026-05-01-g-theory-pir-integration.md`
# and `decisions/2026-05-04-g-theory-bootstrap-ci-amendment.md`.
# Mechanized in `crates/saltelli-estimators/tests/g_theory_tck.rs`.

Feature: G-theory p x i x r decomposition

  Scenario: crossed p x i x r decomposition yields ordered variance components and reliability coefficients
    Given a balanced 2 x 2 x 2 p x i x r grid with crossed random effects
    When I estimate G-theory p x i x r components
    Then sigma_p exceeds sigma_i
    And sigma_i exceeds sigma_r
    And G exceeds Phi
    And G and Phi lie strictly between 0 and 1

  Scenario: bootstrap confidence intervals land for G-theory components and reliability coefficients
    Given a balanced 2 x 2 x 2 p x i x r grid with crossed random effects
    When I estimate G-theory p x i x r components with bootstrap confidence intervals
    Then bootstrap confidence intervals exist for every variance component
    And bootstrap confidence intervals exist for G and Phi
    And the bootstrap metadata records 128 resamples at alpha 0.05
    And the bootstrap metadata records skipped resamples
