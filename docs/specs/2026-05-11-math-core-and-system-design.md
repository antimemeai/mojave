# Design: Math Core & System Architecture

## Product

Longitudinal SPC for agent evaluation. Ingests results from any eval runner
(Inspect, HAL, lm-eval-harness, customer-built) and answers measurement-validity
questions the customer doesn't know to ask. Optionally runs statistically designed
experiments on customer infrastructure for customers who don't have a runner.

**Product frame:** "We tell you what your eval results mean" — not "we run your evals."

## Core Questions (the product surface)

1. "How reliable is your scoring?" → mixed-effects variance decomposition (fixed judge effects, random seed/sampling)
2. "Do your judges agree — or do they share biases?" → IRR + Dawid-Skene latent-class + preference-leakage diagnostic
3. "Which tasks are doing work?" → IRT item diagnostics (2PL default, dimensionality gate)
4. "What's actually driving your scores?" → Shapley effects (categorical inputs) / Sobol indices (continuous)
5. "Can you stop early?" → anytime-valid inference (confidence sequences, e-processes) + classical sequential
6. "Are some tasks redundant?" → factor models (Python/torch_measure)
7. "Did anything change?" → SPC control charts + e-detector change-point detection

## Architecture

### Boundary Diagram

```
                    ┌───────────────────────────────┐
                    │   customer's existing runner   │
                    │  (Inspect / HAL / lm-eval-     │
                    │   harness / OpenAI Evals /     │
                    │   homegrown)                   │
                    └─────────────┬─────────────────┘
                                  │ run logs
                                  ▼
   ┌──────────────────────────────────────────────────────────┐
   │  eval-ingest (Rust crate)                                │
   │   - inspect adapter                                      │
   │   - hal adapter                                          │
   │   - generic JSONL adapter (BYO schema mapping)           │
   └──────────────────────────────────────────────────────────┘
                                  │ TrialRecord stream
                                  ▼
   ┌──────────────────────────────────────────────────────────┐
   │  math core (Rust crates)                                 │
   │   irr, seq-test, reliability, salib-rs, prereg           │
   └──────────────────────────────────────────────────────────┘
                                  │
                                  ▼
                       eval integrity report

   (Optional, for customers without a runner:)
   ┌──────────────────────────────────────────────────────────┐
   │  orchestration layer (Rust crates)                       │
   │   experiment-designer, scheduler, range-manager,         │
   │   results-collector → emits TrialRecord stream           │
   └──────────────────────────────────────────────────────────┘
```

### Language & Boundaries

All logic in Rust, compiled binaries. Python is a thin user-facing shell.

- Internal: bincode serialization between components
- External (user-facing): JSON
- No FFI/PyO3 — clean process boundaries
- Rust crates independently deployable

### Foundational Crate: eval-core

The canonical schema everything consumes.

```rust
struct TrialRecord {
    trial_id: TrialId,
    run_id: RunId,
    task_id: TaskId,
    task_version: Option<String>,
    agent_id: AgentId,
    agent_version: Option<String>,
    judge_config: Option<JudgeConfig>,
    seed: Option<u64>,
    timestamp: i64,
    outcome: Outcome,
    metadata: BTreeMap<String, Value>,
}

struct JudgeConfig {
    model: String,
    family: String,          // required — preference-leakage diagnostic needs this
    prompt_template_hash: String,
    temperature: f32,
    seed: Option<u64>,
}

enum Outcome {
    Binary(bool),
    Score(f64),
    Graded(u8),
    MultiCriterion(BTreeMap<String, f64>),
}
```

Design constraints:
- `judge_config.family` is non-optional — the preference-leakage diagnostic
  (Li et al. 2025) requires stratification by judge family at ingestion time
- `Outcome` is an enum, not a float — Binary/Score/Graded/MultiCriterion route
  to different statistical models. Collapsing at ingestion loses information.
- bincode internally, JSON at user edge via serde

### Ingestion Layer: eval-ingest

Runner-agnostic adapters mapping external formats → TrialRecord.

#### Inspect adapter (ships first)

Reads Inspect `.eval` log files. Maps Inspect `Sample` → TrialRecord:
- `Sample.id` → `task_id`
- `Sample.scores` (dict of scorer → Score) → one TrialRecord per scorer
- Model-graded scorers: extract grader model/prompt into `judge_config`
- Programmatic scorers: `judge_config = None`
- Multiple scorers per sample → multiple TrialRecords sharing parent grouping
  (inter-scorer agreement is exactly what the irr crate consumes)

v1: thin Python sidecar using Inspect's own log reader API (subprocess, clean
process boundary). Pin supported Inspect version range; version mismatches
fail loudly.

#### HAL adapter (ships second)

Reads Princeton hal-harness JSON results. Same pattern as Inspect.

#### Generic JSONL adapter

Customer provides a field-mapping config; we read JSONL and map to TrialRecord.
Escape hatch for arbitrary runners.

### Math Core (Rust crates)

```
crates/
  eval-core/       # TrialRecord, Outcome, JudgeConfig — foundational types
  eval-ingest/     # Runner adapters (Inspect, HAL, generic JSONL)
  salib-rs/        # Global sensitivity analysis / influence attribution
  irr/             # Inter-rater reliability + latent-class agreement
  seq-test/        # Sequential testing (classical + anytime-valid)
  reliability/     # Classical reliability + IRT
  prereg/          # Pre-registration contracts + governance
```

#### salib-rs (repack — not a rewrite)

Existing saltelli-* crates restructured for publication.

- Core: RNG (ChaCha20 multi-stream), distributions, problem spec, tree-fold reduce
- Samplers: LHS, Sobol QMC, Morris trajectories, FAST/eFAST/RBD-FAST designs
- Estimators: Sobol (Saltelli2010, Jansen, Janon, Owen), Morris, FAST, RBD-FAST,
  Borgonovo δ, PAWN, DGSM, regression (SRC/SRRC/PCC/PRCC), given-data Sobol
  (Plischke 2013), G-theory, ANOVA, PCE, Shapley, active subspaces
- Bootstrap: percentile, BCa
- Validation: Ishigami, Sobol G reference functions; frozen SALib CSV differentials

Shapley effects (Owen 2014) are the recommended default for categorical-input
designs (model, judge, prompt variant). For independent inputs, first-order Sobol
≤ Shapley ≤ total-order Sobol; equality only when zero interactions. Exact
computation feasible for d ≤ ~15 factors; permutation-based Monte Carlo for larger d.
Sobol remains the default for continuous hyperparameter sweeps (temperature, top-p, k).

Status: math complete, needs repack/rename/publish-readiness assessment.

#### irr (new build)

Inter-rater reliability + bias-aware latent-class agreement.

Methods (classical — all get bootstrap CIs):
- Krippendorff α (nominal, ordinal, interval, ratio — explicit level= required)
- Fleiss κ (multi-rater nominal)
- Cohen κ / weighted κ (2-rater)
- Gwet AC1/AC2
- Bland-Altman limits of agreement

Methods (modern — additive):
- Hierarchical Dawid-Skene (Paun et al. 2018):
  - EM over per-annotator K×K confusion matrices with hierarchical priors
  - Input: sparse (item_id, annotator_id, label) triples — handles missing data natively
  - Output: per-item posterior class probabilities, per-annotator confusion matrices,
    item-level entropy, hyperparameter estimates
  - Initialization: majority vote, then EM. Label switching fix post-inference.
- Preference Leakage Score (Li et al. 2025):
  - PLS(i,j) = [(WR(i,i) - AVG(i,j))/AVG(i,j) + (WR(j,j) - AVG(j,i))/AVG(j,i)] / 2
  - Stratify by relatedness regime: same-model > inheritance > same-family > cross-family
  - Flag when PLS exceeds cross-family baseline (~3%)
- Judge-family stratified α:
  - within-family α minus between-family α = bias-burden indicator
  - Requires JudgeConfig.family populated at ingestion
- Self-bias regression (PlayFavorites 2025):
  - OLS: S̃ = α + δ_j + β_j·S + γ_j·1_self + λ·1_family + η_d + ε
  - White/HC robust standard errors. Per-judge γ_j = self-bias coefficient.
- Intra-rater reliability (Haldar 2025):
  - Per-judge α across k≥3 repeated runs
  - Detects judges with high between-judge agreement but low self-consistency

Key design constraints:
- Must handle missing data (not all raters rate all items)
- Must expose paradox behavior clearly (high agreement + low κ)
- No silent defaults on metric level
- API supports stratification by JudgeConfig.family natively
- α alone is never sufficient for LLM judges — report alongside latent-class diagnostics

Reference impls: R irr, irrCAC (Gwet), kripp.alpha, Paun et al. 2018 (CrowdTruth)
Papers: Dawid & Skene 1979 (LIBRARY TRIP — paywalled), Paun 2018, Li 2025, Haldar 2025

#### seq-test (new build)

Sequential testing with anytime-valid inference as the primary primitive.

Core types (from Ramdas et al. 2023 SAVI framework):
- `EValue`: nonneg f64 with E_P[E] ≤ 1. Supports merge_avg (safe under arbitrary
  dependence) and merge_product (requires independence).
- `EProcess`: sequence of e-values satisfying supermartingale property at stopping
  times. Internally Vec<f64> of cumulative wealth.
- `ConfidenceSequence<T>`: parameterized by estimand. Constructed by inverting a
  family of test martingales: C_t = {θ : M_t^θ < 1/α}. Tuning parameter ρ controls
  where boundary is tightest.
- `CapitalProcess`: for bounded means — K_t(m) = ∏(1 + λ_i · (X_i - m)).
  Lambda bounds: -1/(1-m) < λ < 1/m enforced at type level. `HedgedCapital`
  wraps two one-sided capitals for two-sided testing.
- `EDetector`: sequential change-point detection for SPC drift.
  - SRDetector (Shiryaev-Roberts): M_n = L_n · (M_{n-1} + 1)
  - CUSUMDetector: M_n = L_n · max(M_{n-1}, 1)
  Both feed the state-manager's SPC layer.
- `AsympCS`: asymptotic confidence sequence (Howard et al. 2021). Requires ρ
  tuning parameter and running variance estimator.

Methods (classical — retained):
- Wald SPRT (binary + continuous outcomes)
- Pocock boundaries
- O'Brien-Fleming boundaries
- Lan-DeMets α-spending (flexible timing)
- Bias-adjusted estimators at stopping time (Siegmund 1985)

Key design constraints:
- Lambda predictability enforced at API level: strategy function takes
  &history[..t-1], returns λ_t. Using X_t in computing λ_t destroys validity.
- Never use raw MLE for plug-in numerator — always smooth/shrink to avoid
  K_t = 0 (absorbing, evidence permanently lost)
- Two-sided testing requires HedgedCapital, not a single CapitalProcess
- E-value averaging safe under dependence; product requires independence —
  using product under dependence silently inflates Type I error
- AsympCS ρ must be chosen before seeing data or via delayed-start sequence
- Classical SPRT documented as special case (simple P vs simple Q, i.i.d. only)
- Anytime-valid conversion (Koning & van Meer 2026): any fixed-sample test can
  be lifted to anytime-valid with essentially no power loss

Reference impls: R gsDesign, rpact, confseq (github.com/gostevehoward/confseq)
Papers: Howard et al. 2021, Ramdas et al. 2023, Ramdas & Wang 2025 (book ch.7),
  Waudby-Smith & Ramdas 2024, Shin et al. 2023, Koning & van Meer 2026

#### reliability (new build)

Classical test theory and item response theory.

IRT core (from Castleman et al. 2025 cBMM algorithm):
- Primary model: 2PL logistic — P(X=1|θ) = σ(a·(θ - b))
- Algorithm: constrained Block Majorization-Minimization (cBMM).
  Reformulates 2PL as matrix factorization X = θ·a^T + 1·b^T over observed
  entries. Each iteration: (1) quadratic majorization for surrogate,
  (2) NNLS solve for discrimination a (constrained a ≥ 0), (3) closed-form
  update for difficulty b, (4) closed-form update for ability θ.
  Convergence to KKT point via Kurdyka-Łojasiewicz inequality, linear rate.
  Cost O(|Ω|) per iteration. Pure linear algebra — nalgebra/faer in Rust.
  41-86x speedup over R mirt demonstrated at scale.
- Data structures: sparse binary response matrix X (models × items),
  parameter vectors a (discrimination), b (difficulty), θ (ability),
  observation mask Ω for missing data.
- Output: IrtFit { theta, a, b, se, convergence_info }
- Builder pattern: Model2PL::builder().max_iter(500).tol(1e-6).build()

Dimensionality gate (from Luo et al. 2025 MEDIRT):
- Before fitting: EFA with tetrachoric correlations per domain
- Retain items with factor loading > 0.30, exclude negative loaders
- Decision rule: if single-factor estimation is unstable (correlated dims,
  local item dependencies via Yen's Q3), fall back to per-domain 2PL
- Validation module separate from estimation: irt::validate::efa_check(&responses)
  returns dimensionality report
- Person-fit via Zh statistic for anomaly detection

Model selection:
- 1PL (Rasch): when items are exchangeable and simplicity is paramount
- 2PL: default — handles varying discrimination
- 3PL: only when guessing is mechanically present (MCQ). Warn/error at N<300.
- Graded Response Model (Samejima 1969): for ordinal judge scores. Reserve
  2PL/3PL for binary success/fail. (LIBRARY TRIP — Samejima 1969 paywalled)

Classical reliability:
- Cronbach α (documented as lower bound under tau-equivalence assumption)
- McDonald's ω (preferred — leads the API surface)

Reference impls: R psych (α, ω), mirt (IRT), py-irt (Lalor 2022), Castleman cBMM
Papers: Castleman 2025, Lalor EACL 2024, Zhou PSN-IRT 2026, Luo MEDIRT 2025,
  Rasch 1960 (LIBRARY TRIP), Lord & Novick 1968 (LIBRARY TRIP),
  Reckase 2009 M-IRT (LIBRARY TRIP), Brennan 2001 §3.4 (LIBRARY TRIP)

#### prereg (new build)

Pre-registration contracts with deviation detection + governance layer.

Core components:
- Canonical serialization of analysis plans (deterministic hashing)
- Hash-anchored plan documents (SHA-256, whitespace-insensitive)
- Deviation detector (compares executed analysis to registered plan)
- Version chaining (each revision references predecessor's hash)
- Plans reference TrialRecord field paths so deviation detection can verify
  "the plan said to stratify by judge family; the analysis did stratify by
  judge family" declaratively

Schema fields (from Binette 2024, Mitchell 2019, Gebru 2021, NIST AI RMF):

P0 (blocks v1):
- Estimand block: scope/population, data acquisition strategy, metric choice,
  aggregation method (Binette 2024 — these four prevent rank reversals)
- Model identity: name, version, type, owner, date, license (Mitchell 2019)
- Intended use + out-of-scope use (Mitchell 2019)
- Dataset provenance: motivation, composition, collection method (Gebru 2021)
- Metric specification: primary metric(s), tie-breaking, confidence level, run count
- Cost budget: max cost per inference, total cap (Kapoor 2025)

P1 (v1 quality):
- NIST AI RMF crosswalk: map each MEASURE subcategory (2.1, 2.3, 2.5) to the
  prereg field that satisfies it
- Reproducibility fields: seed, run count, version pins
- Risk tier (NIST 100-1 MAP 1.5)

P2 (v2):
- NIST 600-1 GAI risk category tags
- Full model card (all 9 Mitchell categories)
- Full datasheet (all 57 Gebru questions)
- GOVERN/MANAGE layer integration

Governance stack (layered, not monolithic):
- NIST AI RMF 1.0 + GenAI Profile (AI 600-1) as scaffold
- Mitchell model cards / Gebru datasheets for artifact description
- Estimand framework (Binette & Reiter 2024) for analysis-plan specificity
- ICH E9(R1) as optional layer for FDA/clinical/actuarial customers
- Hash-anchored plans as cryptographic substrate

Compliance output artifact:
- Signed evaluation report = frozen prereg + TrialRecords + auto-populated
  model card + NIST crosswalk table + deviation log + cost summary
- Maps to NIST MEASURE 2.1 (document test sets/metrics/tools) and
  MEASURE 2.5 (demonstrate validity/reliability with generalizability limits)

Key design constraints:
- Whitespace-insensitive hashing (pre-reg must not be brittle)
- No silent type coercion in plan parsing
- Round-trip: parse(emit(parse(x))) == parse(x)
- Must work without the other math crates (contract enforcer, not numerics)

Reference impls: none directly — novel integration
Papers: Binette & Reiter 2024, Mitchell 2019, Gebru 2021, NIST AI 100-1,
  NIST AI 600-1, Kapoor 2025, Kahan BMJ 2024

### Orchestration Layer (Rust — optional path)

For customers who don't have an eval runner. Same output schema (TrialRecord)
as the ingestion adapters — the math core doesn't know or care which path
produced the data.

```
crates/
  experiment-designer/   # Ablation schedule generation
  scheduler/             # Dispatch runs to infra, manage concurrency
  range-manager/         # Spin up/tear down eval environments
  results-collector/     # Gather outcomes → TrialRecord stream
  state-manager/         # Longitudinal tracking, baselines, control limits
```

#### experiment-designer

Takes: task suite definition, factor list (judges, seeds, prompt variants, etc.), budget
Produces: statistically efficient ablation schedule

- Full factorial when affordable
- Fractional factorial (Plackett-Burman, Latin square) when not
- Proper blocking and randomization
- Integration with seq-test (stop early when evidence sufficient)
- Integration with adaptive testing (select most informative tasks first)

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

- Gathers outcomes from orchestrated runs
- Emits TrialRecord stream (same schema as eval-ingest output)
- Maps results to the change×task matrix
- Maintains temporal history for SPC

#### state-manager

- Longitudinal state: per-task baselines, noise floors, control limits
- Change log with optional git/VCS integration
- SPC: e-detector based change-point detection (Shin et al. 2023)
- Serves the eval integrity report data

### Python Layer

Thin shell only:
- CLI UX for humans (configure, trigger, view results)
- torch_measure integration (IRT factor models, CAT for adaptive task selection)
- Report rendering (JSON → human-readable formats)
- Configuration management
- Inspect log reader sidecar (v1 — uses Inspect's Python API via subprocess)

### Deployment Modes

1. **Hosted (AWS)**: multi-tenant, managed ranges, cost attribution per tenant
2. **On-prem**: single-tenant, customer provides infrastructure, we provide binary + config

Same Rust core in both cases. Multi-tenancy is hosted-only concern.

## Development Order

### Phase 1: Foundation + First End-to-End

1. eval-core crate (TrialRecord, Outcome, JudgeConfig — types + serde + tests)
2. eval-ingest crate with Inspect adapter
3. irr crate (classical α/κ + Dawid-Skene + family stratification)
4. **First demo: ingest Inspect run → compute IRR with family stratification → emit integrity diff**
5. eval-ingest: HAL adapter + generic JSONL adapter

### Phase 2: Math Core Buildout

6. seq-test crate (e-values + confidence sequences + classical SPRT)
7. salib-rs repack (restructure existing, publish-ready, surface Shapley)
8. reliability crate (2PL IRT via cBMM + dimensionality gate + classical α/ω)
9. prereg crate (hash-anchored plans + estimand block + deviation detector)

Each follows: lit review → TCK specs → 4-gate validation → implementation → code review

### Phase 3: Orchestration + SPC

10. state-manager (longitudinal tracking + e-detector change-point detection)
11. experiment-designer (ablation schedules)
12. scheduler + range-manager + results-collector

### Phase 4: Product Surface

13. Eval integrity report generator (consumes all math crate outputs)
14. Open-source CLI: reads Inspect .eval file, prints minimal integrity diff
15. Python CLI shell

### Beads (built as they come up)

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
- BEAD-0015: Eval runner ingest layer

## Validation Strategy

See: docs/reference/validation-4-gate.md

Every math primitive passes four gates:
1. Textbook reproductions (golden datasets from canonical papers)
2. Reference impl cross-checks (R packages, pinned versions)
3. Property-based tests (invariants, identities, boundaries)
4. Monte-Carlo calibration cards (coverage, Type-I, power)

## Key Constraints

- TrialRecord is the canonical data contract — everything consumes it
- No coupling between crates unless earned
- bincode internally, JSON at user edge
- All Rust logic compiles to standalone binaries
- Python never does heavy lifting
- Pre-commit: clippy zero warnings, rustfmt
- TCK (Gherkin) specs before implementation code
- Papers obtained and preserved for every method built (../evals_papers/)
- Ingestion path is first-class; orchestration is optional
- No privileged eval runner — Inspect ships first but the trait is open
