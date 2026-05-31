Feature: QMU Confidence Ratio and JCGM 106 conformity assessment
  Quantification of Margins and Uncertainties (Pilch et al. 2006 SAND2006-5001)
  composed with JCGM 106:2012 guard band decision rules.

  CR = margin / expanded_uncertainty where margin = estimate - threshold.
  Guard bands per JCGM 106 Section 8.3: acceptance limit offset inward
  from tolerance limit to control consumer risk.

  # Gate 1: Textbook reproduction (Pilch 2006 / Sharp & Wood-Schultz 2003)

  Scenario: CR computation for clear acceptance
    Given a measurement with estimate=0.82, expanded_uncertainty=0.04, threshold=0.70
    When I compute the QMU assessment
    Then margin is 0.12
    And confidence_ratio is 3.0
    And the decision is Accept

  Scenario: CR computation for clear rejection
    Given a measurement with estimate=0.65, expanded_uncertainty=0.04, threshold=0.70
    When I compute the QMU assessment
    Then margin is -0.05
    And confidence_ratio is -1.25
    And the decision is Reject

  Scenario: CR computation for borderline case without guard band
    Given a measurement with estimate=0.73, expanded_uncertainty=0.04, threshold=0.70
    When I compute the QMU assessment
    Then margin is 0.03
    And confidence_ratio is 0.75
    And the decision is Investigate

  # JCGM 106 Section 8.3: Guard band decision rules

  Scenario: guarded acceptance narrows acceptance region
    Given a measurement with estimate=0.82, expanded_uncertainty=0.04, threshold=0.70
    And a guard band width of 0.04
    When I compute the conformity decision
    Then the acceptance limit is 0.74
    And the decision is Accept

  Scenario: guarded acceptance triggers investigate for marginal case
    Given a measurement with estimate=0.76, expanded_uncertainty=0.04, threshold=0.70
    And a guard band width of 0.04
    When I compute the conformity decision
    Then the acceptance limit is 0.74
    And the decision is Investigate

  # JCGM 106 Section 8.3.2: guard band from consumer risk

  Scenario: JCGM 106 guard band width for 2.3% consumer risk (ISO 14253-1 default)
    Given expanded_uncertainty=0.10 and coverage_factor=2
    When I compute the guard band for consumer_risk=0.023
    Then the guard band width is approximately 0.10

  Scenario: JCGM 106 guard band width for 5% consumer risk
    Given expanded_uncertainty=0.10 and coverage_factor=2
    When I compute the guard band for consumer_risk=0.05
    Then the guard band width is approximately 0.0823

  # Gate 3: Property tests

  Scenario: CR increases monotonically with margin at fixed uncertainty
    Given fixed expanded_uncertainty=0.04 and threshold=0.70
    When I compute CR for estimates [0.65, 0.70, 0.75, 0.80, 0.85]
    Then CR values are strictly increasing

  Scenario: CR decreases monotonically with uncertainty at fixed margin
    Given fixed estimate=0.80 and threshold=0.70
    When I compute CR for expanded_uncertainties [0.02, 0.04, 0.06, 0.08]
    Then CR values are strictly decreasing

  Scenario: zero uncertainty produces infinite CR for positive margin
    Given a measurement with estimate=0.80, expanded_uncertainty=0.0, threshold=0.70
    When I compute the QMU assessment
    Then confidence_ratio is positive infinity
    And the decision is Accept

  Scenario: guard band of zero is equivalent to simple acceptance
    Given a measurement with estimate=0.75, expanded_uncertainty=0.04, threshold=0.70
    When I compute the assessment with guard_band=0.0
    And I compute the assessment with no guard band
    Then both decisions are identical
