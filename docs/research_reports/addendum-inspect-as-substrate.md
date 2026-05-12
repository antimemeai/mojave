# Addendum: Inspect (and other eval runners) are substrates, not competitors

> Companion to the critical review and method-upgrade bibliography. Pulled out as a separate document because it has direct engineering and product implications that should land in the codebase, not just the GTM doc.

## The correction

The original review framed UK AISI's Inspect as "the biggest blind spot in your competitive thesis" and posed a "extend vs displace" question. That framing was wrong. Inspect is an **eval runner** — a harness for executing tasks against models, with sandboxing, scoring hooks, and a library of evals. It produces structured run logs. It is a *substrate*, not a product layer.

What this product does sits *above* the runner. The runner produces trial-level outcomes; this product asks whether those outcomes are statistically defensible — noise floor, judge agreement, item discrimination, score driver attribution, early stopping with controlled error. Same applies to HAL (Princeton's hal-harness), lm-evaluation-harness (EleutherAI), OpenAI Evals, and any homegrown runner a customer has wired up.

So the correct competitive posture is: **first-class integration, zero adversarial framing**. Inspect (and its peers) are dependencies, integration targets, and distribution channels.

## Architectural implication: ingestion-first, orchestration-optional

The original design has an orchestration layer (`scheduler`, `range-manager`, `results-collector`) as a first-class concern. That stays — but it's no longer the central path. The central path is **ingest results from whatever runner the customer already has** and produce the integrity report on top.

This changes the boundary diagram:

```
                    ┌───────────────────────────────┐
                    │   customer's existing runner  │
                    │  (Inspect / HAL / lm-eval-    │
                    │   harness / OpenAI Evals /    │
                    │   homegrown)                  │
                    └─────────────┬─────────────────┘
                                  │ run logs
                                  ▼
   ┌──────────────────────────────────────────────────────────┐
   │  ingestion adapters (Rust crate: eval-ingest)            │
   │   - inspect_log_reader                                   │
   │   - hal_result_reader                                    │
   │   - lm_eval_harness_reader                               │
   │   - openai_evals_reader                                  │
   │   - generic_jsonl_reader (BYO schema mapping)            │
   └──────────────────────────────────────────────────────────┘
                                  │ canonical TrialRecord stream
                                  ▼
   ┌──────────────────────────────────────────────────────────┐
   │  math core (existing crates)                             │
   │   irr, seq-test, reliability, salib-rs, prereg           │
   └──────────────────────────────────────────────────────────┘
                                  │
                                  ▼
                       eval integrity report

   (Optional, for customers without a runner:)
   ┌──────────────────────────────────────────────────────────┐
   │  orchestration layer                                     │
   │   experiment-designer, scheduler, range-manager,         │
   │   results-collector  → emits canonical TrialRecord       │
   └──────────────────────────────────────────────────────────┘
```

The orchestration layer is now a *peer* of the ingestion adapters — same output schema, different input. Either path feeds the same math core. For most early customers (especially safety-eval contractors and frontier-lab adjacent teams), the ingestion path will be the only path they care about. For MRM / regulated-industry customers, the orchestration path matters because they often don't have a runner already.

## Concrete: define the canonical TrialRecord first

Before any adapter work, pin down the canonical schema that flows between ingestion/orchestration and the math core. This is the most important schema decision in the codebase because every downstream crate consumes it.

Proposed minimum (bincode internally, JSON at user edge):

```rust
struct TrialRecord {
    trial_id: TrialId,             // ULID or similar
    run_id: RunId,                 // groups trials from a single eval run
    task_id: TaskId,
    task_version: Option<String>,  // for change attribution
    agent_id: AgentId,
    agent_version: Option<String>, // commit SHA, model+config hash, etc.
    judge_config: Option<JudgeConfig>, // None for non-judged outcomes
    seed: Option<u64>,
    timestamp: i64,
    outcome: Outcome,              // enum: Binary, Score(f64), Graded(u8), MultiCriterion(...)
    metadata: BTreeMap<String, Value>, // extensibility escape hatch
}

struct JudgeConfig {
    model: String,                 // e.g. "claude-sonnet-4.6", "gpt-5"
    family: String,                // "anthropic" | "openai" | "google" | "open-weights"
    prompt_template_hash: String,  // for grouping
    temperature: f32,
    seed: Option<u64>,
}
```

Two notes on this shape:

1. `judge_config.family` is non-optional once you commit to the §5.3 preference-leakage diagnostic (bibliography). Stratifying α by judge family requires this field be populated at ingestion time.
2. `outcome` should be an enum, not a float. Binary success/fail, scored, ordinal-graded, and multi-criterion are statistically distinct cases that route to different reliability/IRT models. Collapsing them at ingestion loses information you can't reconstruct downstream.

## Inspect integration specifically

Inspect's log format is the `.eval` file — a structured log per run containing samples, scores, model interactions, and config. It's stable, documented, and consumable from outside Inspect's Python runtime.

For the first cut:

- Build `crates/eval-ingest/src/inspect.rs` reading `.eval` files (likely via a thin Python sidecar that uses Inspect's own log API, since the `.eval` format is versioned and Inspect ships a reader; calling that reader from Rust via subprocess is fine for v1 — clean process boundary, no FFI).
- Map Inspect's `Sample` → `TrialRecord`. Inspect samples include `id`, `target`, `output`, `scores` (dict from scorer name to Score with value/explanation/metadata), and metadata. The mapping is mechanical.
- Inspect scorers can be model-graded (LLM judge) or programmatic. When model-graded, extract the grader model/prompt config into `judge_config`. Programmatic scorers leave `judge_config = None`.
- Multiple scorers per sample → multiple `TrialRecord`s (one per scorer), all sharing `trial_id`'s parent grouping. This is important because inter-scorer agreement is exactly what the `irr` crate is supposed to consume.

There's a real chance the `.eval` format evolves; pin the supported version range and write the adapter so version mismatches fail loudly, not silently.

## HAL integration

Princeton's hal-harness (Stroebl, Kapoor, Narayanan, 2025) is the agent-specific reference. Its results format is a JSON structure under `results/{benchmark}/{agent}/{run}/`. Adapter pattern is the same as Inspect — read JSON, map to `TrialRecord`, populate judge_config where applicable.

HAL is the right second adapter because:
- It's agent-specific, which is your primary use case.
- It's the reference standard in the agent-eval academic community.
- Kapoor/Narayanan are intellectual allies for your overall thesis — making your tool work cleanly with theirs is good politics.

## What this means for the existing crate plan

The five math crates are unchanged. Specifically:

- `salib-rs` — no change.
- `irr` — no change to math, but the API should make it easy to stratify by `judge_config.family` for the preference-leakage diagnostic. Add a `stratified_alpha(records, stratify_by)` convenience on top of the base computation.
- `seq-test` — no change to math, but the recommendations from the bibliography (anytime-valid inference, confidence sequences, e-processes) still stand. This is independent of the Inspect framing.
- `reliability` — no change.
- `prereg` — minor: the canonical analysis plan should reference `TrialRecord` fields by name so the deviation detector can verify "the plan said to stratify by judge family, the analysis did stratify by judge family." Plans should be expressible in terms of the canonical schema.

The orchestration crates (`experiment-designer`, `scheduler`, `range-manager`, `results-collector`, `state-manager`) all stay, but their priority drops behind the ingestion adapters for v1. Order of build:

1. `TrialRecord` schema + serde + tests
2. `eval-ingest` crate with Inspect adapter
3. `eval-ingest` crate with HAL adapter
4. First end-to-end: ingest an Inspect run → compute IRR with family stratification → emit integrity diff
5. Then iterate on math crates and add orchestration

This is a smaller, faster, more demoable v1 than the original "build orchestration first" plan.

## Product / docs implications

Two specific copy changes wherever they appear:

- Anywhere docs say "we run your evals" or imply this product is an eval framework → change to "we tell you what your eval results mean." The product is **measurement validity for whatever you already run.**
- Anywhere docs mention specific frameworks → mention Inspect first, then HAL, then "or your custom harness." Inspect-first signals neutrality and integration competence.

The top-of-funnel motion this enables, which the original review didn't see clearly:

- Ship an open-source Rust binary (or Python CLI wrapping it) that reads an Inspect `.eval` file and prints a minimal integrity diff: noise floor, judge α stratified by family, sequential test verdict on score delta. Free, single-binary, MIT-licensed. The pitch on the README is "you ran 1,000 evals — were any of the deltas real?"
- That gets the product in front of every Inspect user (METR, Apollo, US CAISI, UK AISI, frontier-lab safety teams). Conversion to paid SaaS is the audit-trail / pre-registration / longitudinal-tracking layer on top.

This is a much cleaner GTM than "displace Inspect" and is roughly free given the architecture above.

## What this does NOT change

- The math upgrades from the bibliography (anytime-valid inference, Shapley effects, mixed-effects ANOVA reframing, Dawid-Skene latent-class judges, IRT done properly) — all still recommended, all independent of the runner framing.
- The on-prem vs hosted deployment story.
- The MRM / regulated-industry GTM lane — those customers don't have a runner problem, they have an evidence problem, and the orchestration layer matters for them.
- The defensibility wedge (statistical rigor as audit evidence). If anything the Inspect-as-substrate framing strengthens it: you're not competing with the eval-runner market, you're occupying the layer above it that nobody else occupies.

## TL;DR for Code Claude

1. Define `TrialRecord` and put it in a foundational crate (`eval-core` or similar) that everything else depends on.
2. Build `eval-ingest` with an Inspect adapter as the first concrete consumer. HAL adapter second.
3. Demote the orchestration crates to "optional path for customers without a runner." They still get built, just not first.
4. Make `irr` API support stratification by `judge_config.family` natively — the preference-leakage diagnostic is the most visible early differentiator.
5. Write a `prereg` plan schema that references `TrialRecord` field paths so deviation detection can verify execution against plan declaratively.
6. Build the open-source `inspect-integrity` CLI as a top-of-funnel artifact once the math core + ingestion adapter are working.

The math is unchanged. The architecture gains a clean ingestion boundary. The product positioning becomes "we read your eval logs and tell you what they mean" instead of "we run your evals" — which is both more accurate and a better wedge.
