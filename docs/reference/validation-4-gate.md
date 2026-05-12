# Four-Gate Validation Strategy

Every public function in a math primitive crate must pass all four gates before release.

## Gate 1: Textbook Reproductions

Reproduce known results from canonical papers using golden datasets.

- Tolerances: `rtol=1e-6, atol=1e-8` for closed-form, `rtol=1e-3` for iterative (EM/REML), `rtol=1e-2` for stochastic (bootstrap)
- Source: original papers (e.g., Cronbach 1951 Table 1, Brennan 2001 variance components, Bland-Altman 1986 PEFR)
- Each reproduction is a named test with a citation

## Gate 2: Reference Implementation Cross-Checks

Agreement with authoritative reference implementations (primarily R).

- R packages are canonical: psych, mirt, gtheory, irr, irrCAC, gsDesign, rpact, lme4
- Pinned reference versions in CI (Docker image with R + packages at specific versions)
- Cross-checks run via subprocess/FFI — not rpy2 (we're Rust, not Python)
- Quarterly version matrix run against latest reference package versions
- Drift alarm: any delta > 2x tolerance opens a Beads issue automatically

## Gate 3: Property-Based Tests

Invariant and identity tests that hold regardless of input data.

Examples:
- Permutation invariance (α under item reorder, Krippendorff under rater-label permutation)
- Degenerate cases recover known forms (3PL with c=0 = 2PL, group-sequential K=1 = fixed-sample)
- Boundary conditions (perfect agreement → α=1, chance-level → α=0)
- Algebraic identities (Spearman-Brown, G-theory under perfect-rater design = Cronbach α)

## Gate 4: Monte-Carlo Calibration Cards

Per-release statistical validation under simulation.

- Bootstrap coverage: observed 95% CI coverage ∈ [0.93, 0.97] over 1000 simulated datasets
- IRT recovery: RMSE(parameters) vs N at known ground truth
- SPRT Type-I: 100k reps under H0, observed rate = nominal α to MC error
- Group-sequential power: observed power matches reference implementation
- IRR prevalence sweep: κ, AC1, α vs base rate at known agreement level

## Edge Cases (adversarial inputs)

Every public API must handle:
- Empty data (n=0) → explicit error, never silent NaN
- Single observation → documented degenerate behavior
- All-identical responses → method-specific documented behavior
- NaN/Inf → explicit policy (reject/propagate/impute), never silent
- Wrong dtype → reject at boundary, not deep in numerics
- Massive inputs (10^7 rows) → no OOM, streaming path tested

## Release Discipline

- Per-statistic `tolerances.toml` with explicit thresholds
- Golden dataset outputs pinned to JSON; CI diffs against them
- Snapshot regression on every release
- Drift in any reference output → Beads issue with diff and captured artifact
