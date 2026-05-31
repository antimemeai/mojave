Feature: MSA gauge discrimination diagnostics

  AIAG MSA Manual 4th edition formulas for measurement system adequacy.
  ndc = number of distinct categories the gauge can distinguish.
  P/T = precision-to-tolerance ratio.

  Scenario: ndc from AIAG formula with inadequate gauge
    Given a rating study with sigma_parts 0.30 and sigma_gauge_rr 0.10
    When I compute ndc
    Then ndc equals 4
    And ndc is flagged as inadequate because AIAG requires ndc >= 5

  Scenario: ndc from AIAG formula with adequate gauge
    Given a rating study with sigma_parts 0.50 and sigma_gauge_rr 0.10
    When I compute ndc
    Then ndc equals 7
    And ndc is flagged as adequate

  Scenario: P/T ratio for inadequate gauge
    Given a rating study with sigma_gauge_rr 0.10 and tolerance 0.50
    When I compute the P-T ratio
    Then the P-T ratio equals 1.20
    And the gauge P-T is flagged as inadequate

  Scenario: P/T ratio for adequate gauge
    Given a rating study with sigma_gauge_rr 0.01 and tolerance 0.50
    When I compute the P-T ratio
    Then the P-T ratio equals 0.12
    And the gauge P-T is flagged as adequate
