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

## Architecture: Crate Decomposition

```
eval-core           TrialRecord data spine (EXISTS)
eval-ingest         Runner adapters → TrialRecord stream (BEAD-0015)
eval-orchestrator   Route/Decide loop (BEAD-0013, new)
eval-perturb        Perturbation atoms + design engine (BEAD-0012, new)

salib-*             Sensitivity analysis (EXISTS)
irr                 Inter-rater reliability (EXISTS)
seq-anytime-valid   Sequential testing (EXISTS)
spc-charts          Control charts (EXISTS)
```

The dependency graph:

```
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

eval-orchestrator and eval-perturb are peers, not parent-child.
The orchestrator consumes perturbation designs but doesn't own them.
The perturbation engine consumes analysis results but doesn't own them.
They communicate through TrialRecord metadata and Decision signals.

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
  We sit on top of runners.
- **A platform/SaaS.** No web dashboards, no user accounts, no hosted
  infrastructure. Rust binaries + clean API.
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

## Implementation Priority

Phase 1: **eval-ingest** (BEAD-0015)
- Inspect adapter (the table-stakes runner in safety eval)
- Generic JSONL adapter (BYO schema mapping)
- TrialRecord validation and normalization

Phase 2: **eval-orchestrator** (BEAD-0013)
- Analysis routing (which instruments to run on which records)
- Decision engine (stop/continue/alert based on instrument readings)
- Batch mode first, streaming later

Phase 3: **eval-perturb** (BEAD-0012)
- Atom taxonomy (format, paraphrase, multi-turn)
- Design engine bridge to salib-samplers
- Perturbation application (emit modified inputs)

Phase 4: **Close the loop**
- Orchestrator integrates perturbation designs
- Sequential testing gates perturbation refinement
- SPC monitors perturbation experiments longitudinally

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
