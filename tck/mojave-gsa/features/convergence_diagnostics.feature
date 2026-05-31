Feature: Sobol convergence diagnostics

  Automated convergence diagnostics for Sobol' sensitivity analysis
  to detect insufficient sample size and guide the "double N" decision.

  Scenario: negative S1 triggers warning
    Given Sobol results with S1_quantization = -0.070
    When I run convergence diagnostics
    Then a warning is emitted for factor "quantization" with reason "negative S1"
    And a recommendation to double N is emitted

  Scenario: CI crossing zero triggers warning
    Given Sobol results with S1_decoding CI [-0.05, 0.08]
    When I run convergence diagnostics
    Then a warning is emitted for factor "decoding" with kind CiCrossesBound

  Scenario: CI width exceeding threshold triggers doubling recommendation
    Given Sobol results with S1_prompt_template CI width = 0.44 and point estimate = 0.85
    When I run convergence diagnostics with ci_width_ratio_threshold = 0.10
    Then a recommendation is emitted to double N

  Scenario: sum_ST exceeding 1.3 triggers interaction warning
    Given Sobol results with sum of ST = 1.35
    When I run convergence diagnostics with sum_st_threshold = 1.3
    Then a warning is emitted for substantial factor interactions

  Scenario: clean results produce no diagnostics
    Given Sobol results with well-converged indices
    When I run convergence diagnostics
    Then no warnings are emitted
