Feature: mojave-gsa analyze

  The analyze subcommand reads a Saltelli manifest and a results JSON
  (containing per-cell accuracy keyed by saltelli_index), reconstructs
  the same SaltelliMatrix via deterministic RngState, splits the output
  vector into fa/fb/fab, computes Sobol' Si/STi and Borgonovo delta
  with bootstrap CIs, and writes an analysis JSON.

  Scenario: Analysis output has required fields
    Given a manifest with N=4 and k=6
    And a results JSON with 32 cell accuracies
    When I run "mojave-gsa analyze"
    Then the output JSON has fields: eval, model, design, n_cells, sobol_indices, borgonovo_indices, sobol_diagnostics, aggregate

  Scenario: Sobol indices have one entry per factor
    Given a manifest with N=4 and k=6
    And a results JSON with 32 cell accuracies
    When I run "mojave-gsa analyze"
    Then sobol_indices has exactly 6 entries
    And each entry has fields: axis, S1, S1_ci_low, S1_ci_high, ST, ST_ci_low, ST_ci_high

  Scenario: Borgonovo indices have one entry per factor
    Given a manifest with N=4 and k=6
    And a results JSON with 32 cell accuracies
    When I run "mojave-gsa analyze"
    Then borgonovo_indices has exactly 6 entries
    And each entry has fields: axis, delta

  Scenario: Sum diagnostics are computed
    Given a manifest with N=4 and k=6
    And a results JSON with 32 cell accuracies
    When I run "mojave-gsa analyze"
    Then sobol_diagnostics.sum_S1 is a finite number
    And sobol_diagnostics.sum_ST is a finite number

  Scenario: Analysis fails on incomplete output vector
    Given a manifest with N=4 and k=6
    And a results JSON with only 30 cell accuracies (2 missing)
    When I run "mojave-gsa analyze"
    Then the process exits with a nonzero status
    And stderr contains "missing" or "incomplete"

  Scenario: Analysis is deterministic
    Given a manifest and results JSON
    When I run "mojave-gsa analyze" twice
    Then both analysis outputs are byte-identical
