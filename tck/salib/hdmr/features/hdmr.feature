Feature: RS-HDMR — High-Dimensional Model Representation via PCE

  RS-HDMR decomposes model output variance by interaction order
  using Polynomial Chaos Expansion. Fits a PCE to (X, Y) data,
  then groups the basis functions by which factors they involve
  (via MultiIndex::active_factors()), accumulating variance
  contributions into first-order, second-order, and total-order
  Sobol' indices.

  Scenario: HDMR on Ishigami recovers first-order indices
    Given the Ishigami canonical model with a=7 and b=0.1
    And N=4096 samples from Sobol sequence
    When I run HDMR with max_order=2 and max_degree=6
    Then first_order S_1 is within 0.05 of analytic 0.3139
    And first_order S_2 is within 0.05 of analytic 0.4424
    And first_order S_3 is within 0.05 of analytic 0.0

  Scenario: HDMR second-order matches Ishigami S2_13
    Given the Ishigami canonical model with a=7 and b=0.1
    And N=4096 samples from Sobol sequence
    When I run HDMR with max_order=2 and max_degree=6
    Then second_order S2_13 is within 0.05 of analytic 0.244

  Scenario: HDMR component variances sum to total variance
    Given any test function with N=1024 samples
    When I run HDMR with max_order=2
    Then the sum of all component variances equals total_variance within 0.01

  Scenario: HDMR agrees with PCE Sobol indices
    Given the Ishigami canonical model with a=7 and b=0.1
    And N=4096 samples
    When I run HDMR with max_order=2 and max_degree=6
    And I run PCE Sobol with the same degree
    Then HDMR first_order equals PCE first_order within 0.001
