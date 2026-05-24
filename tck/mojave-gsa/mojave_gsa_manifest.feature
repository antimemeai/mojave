Feature: mojave-gsa generate-manifest

  The generate-manifest subcommand builds a Saltelli radial sample design
  by calling salib::samplers::build_saltelli_matrix, discretizes [0,1]
  Sobol' samples to discrete perturbation factor levels, and outputs a
  manifest JSON file. The manifest preserves canonical Saltelli evaluation
  order for downstream analysis.

  Scenario: Cell count matches Saltelli formula N*(k+2)
    Given an axes config with 6 factors
    And base sample size N = 4
    When I run "mojave-gsa generate-manifest" with those parameters
    Then the manifest contains exactly 32 cells
    And manifest.total_cells equals 32

  Scenario: Cell count at production scale N=1024
    Given an axes config with 6 factors
    And base sample size N = 1024
    When I run "mojave-gsa generate-manifest" with those parameters
    Then the manifest contains exactly 8192 cells

  Scenario: Each cell has all axis values
    Given an axes config with 6 factors
    And base sample size N = 4
    When I run "mojave-gsa generate-manifest" with those parameters
    Then every cell has keys: prompt_template, system_prompt, n_shot_frac, choice_order, decoding, quantization

  Scenario: Cell values are valid factor levels
    Given an axes config with 6 factors
    When I run "mojave-gsa generate-manifest" with N = 4
    Then every cell.prompt_template is one of: lm-eval-default, bare, cot, letter-only, verbose-rationale
    And every cell.system_prompt is one of: none, helpful, domain-expert, safety-aware
    And every cell.n_shot_frac is one of: 0.0, 0.01, 0.025, 0.05
    And every cell.choice_order is one of: original, shuffled
    And every cell.decoding is one of: greedy, T=0.7, T=1.0
    And every cell.quantization is one of: bf16, fp8

  Scenario: Cells have sequential saltelli_index
    Given an axes config with 6 factors
    When I run "mojave-gsa generate-manifest" with N = 4
    Then cell saltelli_index values are 0, 1, 2, ..., 31

  Scenario: Manifest is deterministic
    Given an axes config with 6 factors
    When I run "mojave-gsa generate-manifest" twice with the same seed
    Then both manifests are byte-identical

  Scenario: Manifest metadata is correct
    Given task = "inspect_evals/wmdp_chem" and model = "Qwen/Qwen2.5-7B-Instruct"
    When I run "mojave-gsa generate-manifest" with N = 8
    Then manifest.task equals "inspect_evals/wmdp_chem"
    And manifest.model equals "Qwen/Qwen2.5-7B-Instruct"
    And manifest.design.name equals "saltelli_radial"
    And manifest.design.N_base equals 8
    And manifest.design.k equals 6
