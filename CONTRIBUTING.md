# Contributing

## How to contribute

1. Fork the repo
2. Create a feature branch off `master`
3. Make your changes (see workflow below)
4. Run the checks locally
5. Open a PR

## Development workflow

Every change follows the same cycle, no exceptions:

1. **Write the behavioral spec first.** Add or update a Gherkin `.feature` file in `tck/<crate>/features/`. The spec defines expected behavior — inputs, outputs, edge cases — before any implementation exists.
2. **Get the spec compiling and running (red).** Wire up step definitions so the test harness can execute your scenarios. They should fail because the implementation doesn't exist yet.
3. **Write the implementation.** Minimal code to make the specs pass. No speculative features.
4. **Run tests — fix until green.** `cargo test --workspace --all-targets` for Rust, `uv run pytest -v` for Python.
5. **Pass all pre-commit gates.** The hook enforces everything listed below. If it fails, the commit fails.
6. **Commit.** Atomic, conventional, signed.

If you're adding a new math primitive, it also needs to pass the [4-gate validation](docs/reference/validation-4-gate.md):
textbook reproductions, reference implementation cross-checks, property-based tests, and Monte Carlo calibration.

## Code standards

### Rust

- Zero warnings: `cargo clippy --workspace --all-targets -- -D warnings`
- Format: `cargo fmt --all`
- Tests: `cargo test --workspace --all-targets`

### Python

- Lint: `ruff check python/ && ruff format --check python/`
- Types: `mypy` on all staged `.py` files
- Tests: `cd python && uv run pytest -v`

### Pre-commit hook

Install with `./scripts/install-hooks.sh`. The hook runs rustfmt, clippy, ruff, and mypy on every commit. If the hook fails, the commit is rejected.

## Behavioral specs (TCK)

The `tck/` directory contains Gherkin feature files organized by crate. These are the source of truth for expected behavior — not the tests, not the docs, not the code.

```
tck/
  irr/features/           # kappa, ICC, Dawid-Skene, ...
  seq-anytime-valid/       # SPRT, mSPRT, e-values, confidence sequences
  spc-charts/              # Shewhart, CUSUM, EWMA, e-detector
  salib/                   # one feature file per estimator/sampler
  eval-ingest/             # ingestion format specs
  ...
```

Every new feature or bug fix starts with a scenario that captures the expected behavior. PRs without corresponding spec coverage will be asked to add it.

## Commits

- Sign your commits (`git commit -s`)
- Atomic commits — one logical change per commit
- Conventional commit messages: `feat(crate): what`, `fix(crate): what`, `test: what`, `tck: what`, `docs: what`
- Commit the TCK spec and the implementation separately when practical — the spec commit should come first

## Architecture

Read the ADRs in `docs/adr/` before proposing structural changes. If your change is load-bearing, write an ADR.

**Language boundary:** Rust owns correctness, real-time decisions, and all math primitives. Python owns offline model fitting (IRT, factor analysis, CFA/SEM) via `mojave-calibrate`. They communicate via subprocess + JSON. No PyO3, no FFI, no coupling.

## Validation

Math primitives must pass the [4-gate validation](docs/reference/validation-4-gate.md):

1. **Textbook reproductions** — golden datasets from the original papers
2. **Reference impl cross-checks** — R packages at pinned versions
3. **Property-based tests** — invariants, identities, boundary conditions
4. **Monte Carlo calibration** — coverage, Type I error, power

Include the literature citation for any statistical method you implement or modify.

## What we don't want

- Dependencies where a focused implementation will do. Prior art is reference material, not a dependency.
- Speculative features. YAGNI. Build what the spec says, nothing more.
- Skipping the spec. "I'll add the feature file later" means it won't get added.
- Silent numerical edge cases. Empty data, single observations, all-identical inputs, NaN/Inf — all must have explicit, documented behavior. Never silent NaN.

## Issues

File bugs and feature requests in GitHub Issues. Include reproduction steps for bugs. If reporting a numerical discrepancy, include the reference value, source citation, and observed output.

## License

By contributing, you agree that your contributions will be licensed under MIT OR Apache-2.0.
