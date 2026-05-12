# G-theory D-study projections over item/rater counts.
#
# Per `decisions/2026-05-04-g-theory-d-study-amendment.md`.
# Mechanized in `crates/saltelli-estimators/tests/g_theory_d_study_tck.rs`.

Feature: G-theory D-study projections

  Scenario: increasing projected item and rater counts improves reliability
    Given a crossed p x i x r G-theory estimate
    When I project a D-study at 2 items and 2 raters
    And I project a D-study at 4 items and 2 raters
    And I project a D-study at 2 items and 4 raters
    And I project a D-study at 4 items and 4 raters
    Then projected G increases
    And projected Phi increases
    And projected G exceeds projected Phi at both points
    And the V1 D-study surface includes exactly four projected points
