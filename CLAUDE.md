# Project: mojave — measurement science for AI agents

## Repo structure

```
crates/            # Rust crate workspace
tck/               # Behavioral specs (Gherkin .feature files)
docs/
  adr/             # Architectural Decision Records (formal, load-bearing)
  specs/           # Design specs from brainstorming
  reference/       # Reference material (validation strategy, etc.)
scripts/           # Operational tooling (hooks, CI helpers, measurement pipeline)
templates/         # Parametric LaTeX report templates (run cards)
```

Local-only (gitignored):
```
.context/          # LLM working memory (beads, decisions, lit-reviews)
quarantine/        # Old artifacts, shopped from as needed
data/              # Eval logs, analysis results, run card outputs
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
- Beads for ALL issue tracking (local .context/beads/, gitignored)
- Git commits: frequent, atomic, descriptive

## Language split
- Rust: core math primitives, audit infrastructure, engine
- Python: scripting/orchestration layer, torch_measure integration
- Clean API boundaries between layers — no coupling nightmares

## Eval infrastructure (RunPod + vLLM)

Eval runs use throwaway RunPod GPU pods serving Qwen2.5-7B-Instruct via vLLM.
Full runbook: `scripts/destructive/RUNPOD_RUNBOOK.md`

**Critical**: Use `vllm/vllm-openai` Docker image with `docker_args` for model config.
Do NOT use a base PyTorch image with SSH-based setup — it fails at scale (timeouts,
PTY issues, no recovery). Consumer GPUs (3090/4090) require `--enforce-eager` and
`env={"VLLM_USE_V1": "0"}` or vLLM crashes silently.

Scripts: `scripts/destructive/create_pods.py` (create), `teardown_pods.py` (terminate),
`run_destructive.py` (run evals), `gen_destructive_manifest.py` (generate variant manifests).

**ALWAYS terminate pods when done.** 15 pods × ~$0.30/hr adds up fast.

## Key decisions made
- Product: framework surfacing measurement questions + running ablations on customer infra
- Market: defense establishment first, regulated industries follow
- Name: mojave (antimeme.ai)
- saltelli-* crates → repack as standalone Rust GSA library (salib-rs or similar)
- Prior project crates → quarantine, shop from as needed
- IRR, sequential testing, adaptive testing, factor models → confirmed gaps to build
- Stanford AIMS / torch_measure → integrate via Python scripting layer
