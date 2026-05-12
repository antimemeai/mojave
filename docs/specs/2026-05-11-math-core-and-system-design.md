# Design: Math Core & System Architecture

## Product

Longitudinal SPC for agent evaluation. Surfaces measurement-quality questions developers don't know to ask, then runs statistically rigorous experiments on customer infrastructure to answer them.

## Core Questions (the product surface)

1. "How reliable is your scoring?" → noise floor via G-theory variance decomposition
2. "Do your judges agree?" → IRR statistics (BYO supported)
3. "Which tasks are doing work?" → IRT item diagnostics (Python/torch_measure)
4. "What's actually driving your scores?" → sensitivity analysis (Sobol indices)
5. "Can you stop early?" → sequential testing (SPRT, group-sequential)
6. "Are some tasks redundant?" → factor models (Python/torch_measure)

## Architecture

### Language & Boundaries

All logic in Rust, compiled binaries. Python is a thin user-facing shell.

- Internal: bincode serialization between components
- External (user-facing): JSON
- No FFI/PyO3 — clean process boundaries
- Rust crates independently deployable

### Math Core (Rust crates)

```
crates/
  salib-rs/        # Global sensitivity analysis (repack of existing work)
  irr/             # Inter-rater reliability
  seq-test/        # Sequential testing
  reliability/     # Classical reliability + IRT
  prereg/          # Pre-registration contracts
```

#### salib-rs (repack — not a rewrite)

Existing saltelli-* crates restructured for publication. Contains:

- Core: RNG (ChaCha20 multi-stream), distributions, problem spec, tree-fold reduce
- Samplers: LHS, Sobol QMC, Morris trajectories, FAST/eFAST/RBD-FAST designs
- Estimators: Sobol (Saltelli2010, Jansen, Janon, Owen), Morris, FAST, RBD-FAST, Borgonovo δ, PAWN, DGSM, regression (SRC/SRRC/PCC/PRCC), given-data Sobol (Plischke 2013), G-theory, ANOVA, PCE, Shapley, active subspaces
- Bootstrap: percentile, BCa
- Validation: Ishigami, Sobol G reference functions; frozen SALib CSV differentials

Status: math complete, needs repack/rename/publish-readiness assessment.

#### irr (new build)

Inter-rater reliability statistics.

Methods:
- Krippendorff α (nominal, ordinal, interval, ratio — explicit level= required, no default)
- Fleiss κ (multi-rater nominal)
- Cohen κ / weighted κ (2-rater)
- Gwet AC1/AC2
- Bland-Altman limits of agreement
- Bootstrap CIs for all

Key design constraints:
- Must handle missing data (not all raters rate all items)
- Must expose paradox behavior clearly (high agreement + low κ)
- No silent defaults on metric level

Reference impls: R irr, irrCAC (Gwet), kripp.alpha

#### seq-test (new build)

Sequential testing for early stopping with controlled error.

Methods:
- Wald SPRT (binary + continuous outcomes)
- Pocock boundaries
- O'Brien-Fleming boundaries
- Lan-DeMets α-spending (flexible timing)
- Bias-adjusted estimators at stopping time (Siegmund 1985)

Key design constraints:
- Must report bias-adjusted estimates, not raw MLE at stopping
- Degenerate cases (H0=H1) must error explicitly
- Information-time scaling must be correct

Reference impls: R gsDesign (FDA-blessed), rpact

#### reliability (new build)

Classical test theory and item response theory.

Methods:
- Cronbach α (with caveat: lower bound, tau-equivalence assumption)
- McDonald's ω (preferred over α)
- IRT: 1PL (Rasch), 2PL, 3PL
- Convergence diagnostics for IRT (mandatory, not optional)
- N-floor warnings for 3PL (non-identified at N<300 with weak priors)

Key design constraints:
- ω leads the API surface, α is available but documented as limited
- IRT must report convergence diagnostics alongside parameters
- 3PL must warn/error at insufficient N rather than silently emit garbage

Reference impls: R psych (α, ω), mirt (IRT), py-irt

#### prereg (new build)

Pre-registration contracts with deviation detection.

Components:
- Canonical serialization of analysis plans (deterministic hashing)
- Hash-anchored plan documents (SHA-256, any single-byte change → different hash)
- Deviation detector (compares executed analysis to registered plan)
- ICH E9 R1 estimand structure adapted to eval context
- Version chaining (each revision references predecessor's hash)

Key design constraints:
- Comment/whitespace insensitive hashing (pre-reg must not be brittle)
- No silent type coercion in plan parsing
- Round-trip: parse(emit(parse(x))) == parse(x)
- Must work without the other math crates (contract enforcer, not numerics)

### Orchestration Layer (Rust)

Post-math-pivot. All Rust, compiled.

```
crates/
  experiment-designer/   # Ablation schedule generation
  scheduler/             # Dispatch runs to infra, manage concurrency
  range-manager/         # Spin up/tear down eval environments
  results-collector/     # Gather outcomes, build change×task matrix
  state-manager/         # Longitudinal tracking, baselines, control limits
```

#### experiment-designer

Takes: task suite definition, factor list (judges, seeds, prompt variants, etc.), budget constraints
Produces: statistically efficient ablation schedule

- Full factorial when affordable
- Fractional factorial (Plackett-Burman, Latin square) when not
- Proper blocking and randomization
- Integration with sequential testing (stop early when evidence sufficient)
- Integration with adaptive testing (select most informative tasks first — via torch_measure)

#### scheduler

- Dispatches eval runs to customer infrastructure
- Manages concurrency (configurable parallelism)
- Handles retries, timeouts, partial failures
- Records provenance (what ran, when, where, what happened)
- Optional git integration: tags runs with commit SHA when available

#### range-manager

- Spins up repeatable execution environments for agents
- Tears down after evaluation
- Supports: local (dev), AWS (hosted), customer-provided (on-prem)
- Ensures identical starting state across runs
- Epistemic isolation (agent observes only what it would in production)

#### results-collector

- Gathers outcomes from completed runs
- Maps results to the change×task matrix
- Feeds data to math binaries (sensitivity analysis, IRR, sequential boundaries)
- Maintains temporal history for SPC

#### state-manager

- Longitudinal state: per-task baselines, noise floors, control limits
- Change log with optional git/VCS integration
- SPC: detects when scores breach control limits
- Serves the "eval integrity report" data

### Python Layer

Thin shell only:
- CLI UX for humans (configure, trigger, view results)
- torch_measure integration (IRT, factor models, CAT for adaptive task selection)
- Report rendering (JSON → human-readable formats)
- Configuration management

### Deployment Modes

1. **Hosted (AWS)**: multi-tenant, managed ranges, cost attribution per tenant
2. **On-prem**: single-tenant, customer provides infrastructure, we provide the binary + config

Same Rust core in both cases. Multi-tenancy is an additional concern for hosted only (auth, isolation, billing).

## Development Order

### Phase 1: Math Core

1. salib-rs repack (restructure existing, publish-ready)
2. irr crate (new, full JSMNTL)
3. seq-test crate (new, full JSMNTL)
4. reliability crate (new, full JSMNTL)
5. prereg crate (new, full JSMNTL)

Each follows: lit review → TCK specs → 4-gate validation → implementation → code review

### Phase 2: Orchestration (pivot after math core solid)

Design and build orchestration crates. Integrate math core as it completes. First end-to-end: "run a judge-agreement ablation and report results."

### Nice-to-haves (beads, built as they come up)

- BEAD-0005: IRT via torch_measure (Python)
- BEAD-0006: Factor models (Python)
- BEAD-0007: Adaptive testing / CAT
- BEAD-0008: SPC control charts
- BEAD-0009: Audit chain design
- BEAD-0010: Game-theoretic eval design
- BEAD-0011: Construct validity dossier
- BEAD-0012: Perturbation engine
- BEAD-0013: Range orchestration
- BEAD-0014: Git/VCS change integration

## Validation Strategy

See: docs/reference/validation-4-gate.md

Every math primitive passes four gates:
1. Textbook reproductions (golden datasets from canonical papers)
2. Reference impl cross-checks (R packages, pinned versions)
3. Property-based tests (invariants, identities, boundaries)
4. Monte-Carlo calibration cards (coverage, Type-I, power)

## Key Constraints

- No coupling between crates unless earned
- bincode internally, JSON at user edge
- All Rust logic compiles to standalone binaries
- Python never does heavy lifting
- Pre-commit: clippy zero warnings, rustfmt
- TCK (Gherkin) specs before implementation code
- Papers obtained and preserved for every method built (../evals_papers/)
