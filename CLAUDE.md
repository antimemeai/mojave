# Project: mojave — measurement science for AI agents

## Repo structure

```
.context/          # LLM working memory (beads, decisions, lit-reviews, session-notes)
crates/            # Rust crate workspace
tck/               # Behavioral specs (Gherkin .feature files)
docs/
  adr/             # Architectural Decision Records (formal, load-bearing)
  specs/           # Design specs from brainstorming
  reference/       # Reference material (validation strategy, etc.)
scripts/           # Operational tooling (hooks, CI helpers)
quarantine/        # Old artifacts — gitignored, shopped from as needed
```

`../evals_papers/` — PDF library (outside repo, never in git)

## Development methodology: JSMNTL

Extreme rigor baseline. No shortcuts.

### Planning
- Deep literature review for everything built (papers in `../evals_papers/`)
- Written sub-plan even at task level
- Plans built from: papers, reference implementations, community test suites

### Development cycle
1. Written sub-plan
2. TCK red (Gherkin .feature specs first)
3. Get tests compiling/running (red)
4. Write implementation code
5. Run tests → fix until green
6. Subagent code reviewer → fix ALL findings
7. Repeat

### Validation: 4-gate (see docs/reference/validation-4-gate.md)
1. Textbook reproductions (golden datasets from papers)
2. Reference impl cross-checks (R packages, pinned versions)
3. Property-based tests (invariants, identities, boundary conditions)
4. Monte-Carlo calibration cards (coverage, Type-I, power)

### Infrastructure
- Pre-commit hooks: Rust must pass clippy (zero warnings) + rustfmt
- ADRs for load-bearing architectural decisions
- Beads for ALL issue tracking (in .context/beads/)
- Git commits: frequent, atomic, descriptive

## Language split
- Rust: core math primitives, audit infrastructure, engine
- Python: scripting/orchestration layer, torch_measure integration
- Clean API boundaries between layers — no coupling nightmares

## Key decisions made
- Product: framework surfacing measurement questions + running ablations on customer infra
- Market: defense establishment first, regulated industries follow
- Name: mojave (antimeme.ai)
- saltelli-* crates → repack as standalone Rust GSA library (salib-rs or similar)
- Prior project crates → quarantine, shop from as needed
- IRR, sequential testing, adaptive testing, factor models → confirmed gaps to build
- Stanford AIMS / torch_measure → integrate via Python scripting layer
