# Orchestration Theory for mojave

## What orchestration IS NOT

mojave does not run evals. The customer has a runner (Inspect, lm-eval-harness,
promptfoo, homegrown). mojave takes their output and subjects it to measurement
discipline they can't get anywhere else.

mojave is not a platform. It's not vivaria (agent task orchestration), not
braintrust (eval SaaS + dashboards), not evidently (monitoring observability).
Those are fine products solving different problems.

mojave is not a dashboard. Dashboards display numbers. mojave tells you whether
the numbers mean anything.

## What orchestration IS

Orchestration in mojave is **the connective tissue between measurement
instruments and the systems they measure.** It has four jobs:

1. **Ingest** — consume eval runner output into the TrialRecord data spine
2. **Route** — send TrialRecords to the right analysis instruments
3. **Decide** — make stop/continue/alert decisions based on instrument readings
4. **Design** — plan next experiments using sensitivity analysis feedback

These four jobs form a loop, not a pipeline.

## The Loop

```
    ┌─────────────────────────────────────────────────────┐
    │                                                     │
    ▼                                                     │
 INGEST                                                   │
 (eval runner output → TrialRecord stream)                │
    │                                                     │
    ▼                                                     │
 ANALYZE                                                  │
 ├── irr: do judges agree?                                │
 ├── salib-estimators: what drives variance?              │
 ├── seq-anytime-valid: can we stop?                      │
 └── spc-charts: did anything change?                     │
    │                                                     │
    ▼                                                     │
 DECIDE                                                   │
 ├── stop early (sequential test crossed threshold)       │
 ├── flag regression (control chart signal)               │
 ├── report measurement quality (IRR below floor)         │
 └── request more data (insufficient power)               │
    │                                                     │
    ▼                                                     │
 DESIGN (optional — perturbation engine)                  │
 ├── salib-samplers: generate factor-space sample points  │
 ├── perturbation atoms: apply to eval inputs             │
 └── emit new eval runs → runner executes → back to top   │
    │                                                     │
    └─────────────────────────────────────────────────────┘
```

The first three steps (ingest → analyze → decide) work for every customer.
You give us your eval logs, we give you answers.

The fourth step (design) is the perturbation engine. It closes the loop:
analysis reveals which factors drive variance → design targets those factors
with new experiments → runner executes → results come back for analysis.
This is how you map an agent's disposition profile systematically instead
of running random ablations.

## The Data Spine

TrialRecord (already built in eval-core) is the universal interchange format.
Every math crate consumes TrialRecords. Every adapter produces them.

```
TrialRecord {
    trial_id    — unique observation identifier
    run_id      — groups observations from one eval run
    task_id     — what was tested
    agent_id    — what was being evaluated
    judge_config — who scored it (model, prompt hash, temperature)
    seed        — reproducibility anchor
    timestamp   — when
    outcome     — Binary | Score | Graded | MultiCriterion
    metadata    — extensible key-value (perturbation factors, git SHA, etc.)
}
```

This maps directly to the (model × item × judge × seed) corpus shape
from NOTE_TO_SKY_CLAUDE. The metadata map handles additional facets
(prompt template version, decoding parameters, perturbation atoms)
without schema inflation.

Key question from prior thinking: "is (model × item × judge × seed)
sufficient?" Answer: yes, because metadata handles the long tail. The
core facets (task, agent, judge, seed) are first-class because every
analysis crate needs them. Everything else is metadata.

## Customer Experience Model

### For humans

**Entry point:** "Here are my eval logs."
**Exit point:** A report answering the 7 questions, with actionable decisions.

Not a dashboard they have to interpret. Not a p-value they have to
understand. Concrete statements:

- "Your judges agree on 73% of items. The disagreement is concentrated
  in tasks 4, 12, 31 — those items are measuring different things."
- "You can stop this eval run now. The current result (0.82 ± 0.03)
  will not change with more samples."
- "Something changed between runs 14 and 15. Tasks 7-9 regressed.
  Here's the commit that corresponds."
- "Your scores are 40% determined by prompt formatting, not agent
  capability. Here are the format factors that matter most."

### For models/agents (the API customer)

**Entry point:** TrialRecord stream (push or pull).
**Exit point:** Decision stream — typed signals, not prose.

```
Decision::StopEarly { evidence: EValue, threshold: f64 }
Decision::ContinueRunning { power_at_current_n: f64, target_n: f64 }
Decision::Regression { task_ids: Vec<TaskId>, chart_signal: ChartSignal }
Decision::MeasurementWarning { issue: MeasurementIssue }
Decision::ExperimentDesign { next_cells: Vec<PerturbationCell> }
```

An agent calling mojave's API should be able to make decisions without
understanding statistics. The decisions are pre-digested.

## The Perturbation Engine (BEAD-0012, integral to orchestration)

### Why it's not a later add-on

Every eval runner in the survey applies perturbations independently:
one factor at a time, no factorial design, no interaction estimation,
no sensitivity decomposition. This is the universal gap.

mojave closes it because we already have salib-samplers (Sobol sequence
generation over arbitrary factor spaces) and salib-estimators (Sobol
indices, Shapley effects). The perturbation engine is the bridge:

1. **Define factor space** — closed-enum atoms (from conex prior art):
   format atoms (separator, casing, punctuation, padding),
   paraphrase atoms (model × strength),
   multi-turn atoms (truncate, reorder, inject, replace)

2. **Generate sample points** — salib-samplers produces a Sobol or
   Saltelli design over the atom product space. Each sample point
   is a PerturbationCell: a specific combination of factor levels.

3. **Apply perturbations** — each cell specifies what transformation
   to apply to each eval input. Deterministic via ChaCha20Rng seeded
   from (cell_id, input_hash).

4. **Run through eval runner** — the customer's runner executes the
   perturbed inputs. Results come back as TrialRecords with perturbation
   factors recorded in metadata.

5. **Estimate sensitivity** — salib-estimators computes Sobol S1/ST/S2
   over the perturbation factors. "40% of score variance is format
   sensitivity, 15% is paraphrase sensitivity, 5% is their interaction."

6. **Adaptive refinement** — if a factor shows high total-order effect,
   design a focused follow-up experiment on that subspace. Sequential
   testing (seq-anytime-valid) gates the decision to stop or continue.

### The atom model

Atoms are the smallest unit of perturbation. They are:
- **Closed enums** — exhaustive, no open-ended generation
- **Deterministic** — same seed + same input = same output
- **Factor-indexed** — each atom variant has a factor_str() suitable
  for Sobol indexing
- **Composable** — atoms from different categories cross freely
  (format × paraphrase × multi-turn is a valid design)

From conex prior art, the pattern is proven. The new work is:
- Defining the canonical atom taxonomy for mojave
- Building the design-engine bridge to salib-samplers
- Building the application layer (how atoms transform eval inputs)

### CheckList's MFT/INV/DIR taxonomy

Worth stealing as a behavioral test classification:
- **MFT** (Minimum Functionality Test): "does the model do X?"
- **INV** (Invariance): "is the model stable under perturbation Y?"
- **DIR** (Directional Expectation): "does perturbation Z move scores in the expected direction?"

Every perturbation experiment in mojave is implicitly an INV or DIR test.
The taxonomy gives humans a vocabulary for what the perturbation engine
is actually measuring.

## User-Facing Surface

The math crates are the engine. Users never touch them directly.
They interact through three surfaces:

### CLI (`mojave`)

The human entry point. Subcommand structure:

```
mojave ingest <logs>           Import eval runner output
mojave analyze <dataset>       Run measurement battery on a dataset
mojave monitor <stream>        Streaming SPC on incoming results
mojave ablate <factor-spec>    Design + execute ablation study
mojave report <run-id>         Generate measurement report
mojave status                  Campaign/experiment status
```

`mojave ablate` is the control plane's CLI face. It takes a factor
specification (which models, tasks, perturbation atoms, seeds to
cross), generates the experimental design, orchestrates execution,
and reports decomposition. The user says "here are my factors,
tell me what matters" — the control plane handles everything else.

### Python SDK (`mojave`)

The eval ecosystem is Python. The SDK wraps the Rust engine via
PyO3 (not subprocess). Primary audience: eval engineers integrating
mojave into existing pipelines.

```python
import mojave

# Ingest
records = mojave.ingest.from_inspect("logs/")

# Analyze
report = mojave.analyze(records)
print(report.irr_summary)
print(report.can_stop_early)  # Decision, not a number

# Ablate
campaign = mojave.ablate(
    factors={
        "model": ["gpt-4o", "claude-sonnet"],
        "prompt_format": mojave.atoms.Format,  # all format atoms
        "temperature": [0.0, 0.5, 1.0],
    },
    tasks=my_task_set,
    runner=mojave.runners.InspectRunner("eval_config.yaml"),
    design="sobol",       # or "powerset", "factorial", "fractional"
    stopping="e-value",   # sequential testing gates each cell
)
campaign.run()
print(campaign.sensitivity)  # Sobol indices per factor
```

### Rust SDK (`mojave` crate)

For embedding in Rust pipelines or building custom tooling.
Re-exports the public API of all math crates + orchestration
through a single facade crate.

### Control Plane

The control plane is the intelligence layer that sits above
individual analyses. It manages **campaigns** — structured
experimental designs executed over time.

**Core capability: powerset/factorial ablation.**

Given an input set of factors:
```
models:        {A, B, C}
tasks:         {T1, T2, ..., T50}
perturbations: {format_atoms × paraphrase_atoms}
seeds:         {0..4}
```

The control plane:

1. **Generates the design.** Full powerset if the cell count is
   tractable. Sobol sequence or fractional factorial if not.
   Uses salib-samplers for structured sampling of the factor space.

2. **Schedules execution.** Emits cell specifications that the
   customer's runner executes. Manages concurrency, rate limiting,
   retry with discriminated failure handling (vivaria pattern).

3. **Monitors incrementally.** As results arrive, applies sequential
   testing (seq-anytime-valid) per cell and per factor. Cells that
   reach statistical significance stop early. Factors with negligible
   effect get deprioritized.

4. **Adapts the design.** If a factor shows high total-order effect
   in early Sobol estimates, the control plane allocates more samples
   to that region of the factor space. If a factor is clearly inert,
   it collapses that dimension and reallocates budget.

5. **Reports decomposition.** When the campaign completes (or is
   stopped by the user), emits the full sensitivity decomposition:
   which factors drive score variance, which interactions matter,
   which cells are redundant.

This is the perturbation engine operating at campaign scale. It's
what turns "run some ablations" into "run the statistically minimal
set of ablations that answers your question, stop as soon as the
answer is clear, and tell you exactly what drives your scores."

## Architecture: Crate Decomposition

```
USER-FACING
  mojave-cli        CLI binary (BEAD-new)
  mojave-py         Python SDK via PyO3 (BEAD-new)
  mojave            Rust facade crate (BEAD-new)

ORCHESTRATION
  eval-core         TrialRecord data spine (EXISTS)
  eval-ingest       Runner adapters → TrialRecord stream (BEAD-0015)
  eval-orchestrator Route/Decide/Schedule loop (BEAD-0013, new)
  eval-perturb      Perturbation atoms + design engine (BEAD-0012, new)
  eval-campaign     Control plane: designs, campaigns, adaptive (new)

MATH ENGINE
  salib-*           Sensitivity analysis (EXISTS)
  irr               Inter-rater reliability (EXISTS)
  seq-anytime-valid Sequential testing (EXISTS)
  spc-charts        Control charts (EXISTS)
```

The dependency graph:

```
mojave-cli ──→ eval-campaign (control plane)
           ──→ eval-ingest
           ──→ eval-orchestrator

mojave-py ──→ (same as CLI, via PyO3)

eval-campaign ──→ eval-orchestrator
              ──→ eval-perturb
              ──→ salib-samplers (design generation)
              ──→ seq-anytime-valid (per-cell stopping)

eval-ingest ──→ eval-core
eval-orchestrator ──→ eval-core
                 ──→ irr
                 ──→ seq-anytime-valid
                 ──→ spc-charts
                 ──→ salib-estimators (for sensitivity results)
eval-perturb ──→ eval-core
             ──→ salib-samplers (for experiment design)
             ──→ salib-core (for problem specs)
```

eval-campaign is the top of the Rust dependency tree. It owns
campaigns, designs, and adaptive scheduling. eval-orchestrator
owns per-run analysis routing and decisions. eval-perturb owns
atom definitions and perturbation application. They compose
through eval-core's TrialRecord spine.

## Patterns Stolen From Reference Repos

### From eval runners
- **promptfoo's Cartesian product**: provider × prompt × test maps to
  our agent × task × perturbation × seed cell grid
- **inspect_ai's epochs + reducers**: repeated-measures with aggregation
  maps to our replicate dimension in TrialRecord
- **braintrust's git-native lineage**: auto-capture commit SHA in
  TrialRecord metadata for change attribution (BEAD-0014)
- **lm-harness's request-type abstraction**: decouple task logic from
  model backend — we do this via the adapter trait in eval-ingest

### From perturbation frameworks
- **conex's closed-enum atoms**: factor_str(), seed determinism, audit
  envelopes — carry forward directly
- **CheckList's MFT/INV/DIR**: behavioral test taxonomy for humans
- **TextAttack's goal/constraint/transform/search**: clean factorization,
  but too adversarial — we want measurement, not attack

### From monitoring frameworks
- **NannyML's chunk abstraction**: windowing as a first-class concept,
  maps to our SPC observation windows
- **Evidently's pluggable test registry**: analysis instruments should
  be registerable/composable
- **whylogs's mergeable sketches**: streaming-friendly data structures
  for online monitoring

### From orchestration systems
- **vivaria's Driver trait**: start/score/teardown contract for runner
  adapters
- **vivaria's SetupState FSM**: discriminated retry (transient vs.
  permanent failure)
- **thunderdome's audit envelopes**: everything-is-auditable, even
  analysis outputs re-enter the corpus
- **thunderdome's dep-discipline enforcement**: substrate-side crates
  cannot depend on range-side crates — test this mechanically

## What We Do NOT Build

- **An eval runner.** We are not Inspect, not lm-harness, not promptfoo.
  We sit on top of runners. We design experiments and analyze results,
  but the customer's runner executes them.
- **A hosted platform/SaaS.** No web dashboards, no user accounts, no
  managed infrastructure. CLI + SDKs + local binaries. Defense customers
  run on their own iron.
- **An LLM judge.** We measure judge quality, we don't provide judges.
- **A dataset.** We measure dataset quality, we don't provide datasets.
- **A model serving layer.** The customer serves their models.

## Open Design Questions

1. **Process boundary model.** The existing spec says "no FFI/PyO3 —
   clean process boundaries." Is this still right? PyO3 would give
   Python users a much nicer API than subprocess + JSON. The math
   crates are pure computation — no async, no I/O, no state beyond
   what's passed in. PyO3 is safe here.

2. **Streaming vs. batch.** The current TrialRecord model is
   batch-oriented (collect records, analyze). Sequential testing
   and SPC are inherently streaming. The orchestrator needs both:
   batch analysis for initial reports, streaming for monitoring.

3. **State management.** SPC charts are stateful (CUSUM accumulators,
   EWMA statistics). Where does this state live? Options:
   (a) in-memory (simple, not durable),
   (b) serialized to disk (durable, needs state file management),
   (c) in a database (overkill for now).
   Start with (a), design for (b).

4. **Perturbation application layer.** The conex proxies applied
   perturbations as HTTP sidecar proxies intercepting model calls.
   That's one model. Another: the perturbation engine emits modified
   inputs and the customer's runner handles them. The second is more
   general and doesn't require infrastructure. Start there.

5. **Audit envelope scope.** Thunderdome's everything-is-an-envelope
   discipline is correct for defense customers. But it's heavy for
   academic users who just want a report. Solution: audit is always
   computed (hashes, provenance), but envelope emission is
   configurable (off by default, on for defense deployments).

6. **Pre-registration (BEAD-0009/eval-prereg).** The NOTE_TO_SKY_CLAUDE
   document describes an ICH E9-style pre-registration layer that
   declares analysis plans upfront. This is correct and important
   for defense customers. It is NOT part of the orchestration layer
   itself — it's a meta-layer that constrains what the orchestrator
   can do. Build it later, but design the orchestrator to be
   constrainable by it.

7. **SDK boundary: thin wrapper vs. reimplementation.** The Python
   SDK should be a thin PyO3 wrapper over the Rust engine, not a
   reimplementation. But "thin" is relative — ergonomic Python
   means pandas DataFrames in, reports out. The SDK needs a
   translation layer between Python idioms and Rust types. Keep
   the translation layer in Python, keep the logic in Rust.

8. **CLI as dogfood.** The CLI should be the first consumer of the
   Rust SDK. If the CLI can't do something, the SDK API is wrong.
   CLI-first development catches API design mistakes before the
   Python SDK locks them in.

9. **Campaign state persistence.** Campaigns (ablation studies) run
   for hours or days. Where does campaign state live between
   invocations? The control plane needs to be resumable: crash,
   restart, pick up where you left off. Options:
   (a) single JSON/bincode file per campaign,
   (b) SQLite database per campaign,
   (c) directory of cell-result files.
   (a) is simplest and sufficient for single-machine operation.

## Implementation Priority

Phase 1: **eval-ingest** (BEAD-0015)
- Inspect adapter (table stakes in the safety eval community)
- Generic JSONL adapter (BYO schema mapping)
- TrialRecord validation and normalization

Phase 2: **eval-orchestrator** (BEAD-0013)
- Analysis routing (which instruments to run on which records)
- Decision engine (stop/continue/alert based on instrument readings)
- Batch mode first, streaming later

Phase 3: **mojave-cli** + initial **mojave-py**
- `mojave ingest` + `mojave analyze` subcommands
- Python SDK: ingest + analyze (the "give us your logs" path)
- This is the first usable product surface

Phase 4: **eval-perturb** (BEAD-0012)
- Atom taxonomy (format, paraphrase, multi-turn)
- Design engine bridge to salib-samplers
- Perturbation application (emit modified inputs)

Phase 5: **eval-campaign** (control plane)
- Campaign data model (factors, designs, cell grid, state)
- Powerset/factorial/Sobol design generation
- Per-cell sequential testing gates
- Adaptive factor-space refinement

Phase 6: **Close the loop** (CLI + SDK integration)
- `mojave ablate` subcommand
- Python SDK: ablate() API
- Full campaign lifecycle: design → execute → monitor → report
- SPC monitors campaigns longitudinally

## Lit Review Sources

### Eval runners surveyed
- inspect_ai (UK AISI) — solver chain, epochs, checkpoint/resume
- lm-evaluation-harness (EleutherAI) — request-type abstraction, bootstrap stderr
- openai-evals (OpenAI) — typed event taxonomy, YAML registry
- braintrust-sdk (Braintrust) — git lineage, OpenTelemetry spans
- promptfoo — Cartesian product, rate-limit registry, redteam plugins

### Perturbation frameworks surveyed
- TextAttack — goal/constraint/transform/search factorization
- CheckList (Microsoft) — MFT/INV/DIR behavioral taxonomy
- NL-Augmenter — filter/generate split, community taxonomy

### Monitoring frameworks surveyed
- evidently — pluggable stat-test registry (20+ tests, no sequential validity)
- whylogs — mergeable sketch primitives (KLL/HLL), profile-centric
- NannyML — chunk abstraction, sampling-error quantification, CBPE

### Orchestration systems surveyed
- vivaria (METR) — Driver trait, SetupState FSM, discriminated retry
- anthropic-cookbook — task-agent examples, no reusable primitives

### Prior art (quarantine)
- thunderdome-* — audit envelopes, campaign/cell model, dep discipline
- conex/ — closed-enum atoms, seed determinism, factor_str(), HTTP proxy model
- NOTE_TO_SKY_CLAUDE — four-library design, EvalRecord spine, 4-gate validation

### Universal gap across all external frameworks
Every framework produces point estimates and treats them as ground truth.
None implements: sequential testing with anytime-valid stopping rules,
confidence intervals on reported metrics, power analysis, multi-rater
agreement, SPC-style longitudinal monitoring, or sensitivity decomposition
over perturbation factors. This is mojave's entire value proposition.
