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

3. **Apply perturbations** — the perturbation gateway sits between
   the eval runner and the model as a MitM. The runner points at
   the gateway endpoint. The gateway applies atoms per the campaign
   design, forwards to the real model, records both the perturbation
   and the response. The runner doesn't know perturbation is happening.

4. **Run through eval runner** — the customer's runner executes normally.
   Results come back as TrialRecords with perturbation factors recorded
   in metadata via the gateway's audit log.

5. **Estimate sensitivity** — salib-estimators computes Sobol S1/ST/S2
   over the perturbation factors. "40% of score variance is format
   sensitivity, 15% is paraphrase sensitivity, 5% is their interaction."

6. **Adaptive refinement** — if a factor shows high total-order effect,
   design a focused follow-up experiment on that subspace. Sequential
   testing (seq-anytime-valid) gates the decision to stop or continue.

### The perturbation gateway

```
eval runner ──HTTP──→ mojave perturbation gateway ──HTTP──→ model
                      (applies atoms per campaign design)
                      (records audit: perturbation + response)
                      (gRPC back to engine for design/audit)
```

The gateway is the configured "inference endpoint" — not a sidecar
bolted onto the side, but the primary endpoint the runner talks to.
This is the conex model (format-spread-proxy, paraphrase-proxy)
generalized.

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
- Building the gateway (how atoms transform requests in-flight)

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

## Serialization & Wire Format

```
bincode     — internal state (campaigns, SPC monitors, caches)
              + audit corpus (schema is a fixed contract, rarely changes)
protobuf    — gRPC wire format only
cbor        — audit corpus (self-describing, decodable without our
              types, readable by third-party auditors/compliance tools)
              via ciborium crate (pinned, vendored)
arrow IPC   — bulk data interchange
json        — REST boundary
parquet     — read-only export, NEVER primary store (footer-at-end
              design means interrupted writes lose everything)
```

Internal: bincode for hot-path state. CBOR for audit corpus (self-describing,
third-party auditors can read it without our binary). Protobuf on the gRPC
wire because that's what gRPC speaks. Arrow IPC for bulk data interchange.
JSON at REST boundaries. Parquet only as export from durably-stored data.

## API Surface

### gRPC (primary)

The real API. Typed contracts, efficient binary wire format.
The CLI and Python SDK are both gRPC clients when talking to a
running engine daemon.

### HTTP/REST (secondary)

Exists because eternal september. Same backing logic, JSON
serialization at the boundary.

### Engine modes

- **In-process:** CLI embeds the engine directly for single-machine
  use (`mojave analyze logs/`). No daemon required.
- **Daemon:** `mojave serve` runs the engine as a gRPC server.
  CLI and SDKs connect as clients. Required for streaming
  monitoring and multi-client campaigns.

## Build Discipline

ALL dependencies pinned and vendored. Builds MUST NOT require
network access. Network is only for cold-starting a repo on a
new machine (initial `cargo vendor`). After that, fully offline.

## Audit & Instrumentation

- **Always on:** Full instrumentation, content hashes, provenance,
  audit log strategies. Never disabled. The cost is trivial and
  there is no off switch.
- **Sealing (opt-in):** Cryptographic signing, tamper-evident corpus
  chains. This is the trust differentiator for defense customers.
  Heavy, but massive upside.
- **Reports reflect integrity level.** A report generated under sealed
  discipline says so. A report with instrumentation-only says that.
  The credibility claim matches what was enforced. No overclaiming.
- **Audit corpus format:** CBOR (self-describing, third-party readable).

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

1. ~~**Process boundary model.**~~ DECIDED: PyO3 for Python SDK.

2. ~~**Streaming vs. batch.**~~ DECIDED: Batch-primary API, streaming
   Monitor as incremental API. Batch implemented atop streaming internally.

3. **State management.** File mode (default, JSON/bincode per entity)
   AND database connect mode (for customers with existing infra who
   want queryable state). State trait abstracts over both. File ships
   first, DB connector when a customer needs it.

4. ~~**Perturbation application layer.**~~ DECIDED: MitM gateway.
   The perturbation engine IS the inference endpoint. The runner
   points at it, it applies atoms, forwards to the real model.
   Not a sidecar — the primary endpoint.

5. ~~**Audit envelope scope.**~~ DECIDED: Instrumentation always on
   (no off switch). Sealing opt-in. Reports reflect integrity level.
   CBOR for audit corpus.

6. **Pre-registration.** Design for it, build later. NOT on the
   critical path. Goes in FUTURE_WORK.

7. **SDK boundary.** mojave is an interface, not a workspace. Python
   SDK is a typed gRPC client with PyO3 in-process option. A few
   canonical format conveniences (DataFrame) are fine. Everything
   else is open-an-issue.

8. **CLI as dogfood.** CLI embeds engine in-process for single-machine
   use. Also works as gRPC client to a running daemon. Both modes.

9. **Campaign state persistence.** Covered by Q3 (state management).
   File-per-campaign default, DB connect mode for heavy users.

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
- Perturbation gateway (MitM inference endpoint)

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
