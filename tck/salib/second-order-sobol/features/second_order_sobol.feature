Feature: Second-order Sobol indices — Ishigami at canonical (a=7, b=0.1, N=8192)

  Scenario: S2_02 recovers the X1-X3 interaction
    Given the Ishigami canonical model with a=7 and b=0.1
    And a Sobol base sampler at dim 6 with skip_first false
    When I build a Saltelli matrix at N=8192 with second_order=true and run Saltelli2010
    Then S2_02 is within 0.05 of analytic 0.244

  Scenario: S2_01 and S2_12 are near zero (no interactions)
    Given the Ishigami canonical model with a=7 and b=0.1
    And a Sobol base sampler at dim 6 with skip_first false
    When I build a Saltelli matrix at N=8192 with second_order=true and run Saltelli2010
    Then S2_01 is within 0.05 of zero
    And S2_12 is within 0.05 of zero

  Scenario: Sum of first and second order indices is at most 1
    Given the Ishigami canonical model with a=7 and b=0.1
    And a Sobol base sampler at dim 6 with skip_first false
    When I build a Saltelli matrix at N=8192 with second_order=true and run Saltelli2010
    Then the sum of S_i plus the sum of S2_ij is at most 1.05
