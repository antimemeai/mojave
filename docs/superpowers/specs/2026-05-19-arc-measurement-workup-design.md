# Design Spec: ARC Challenge Measurement Workup

**Date:** 2026-05-19
**Origin:** Pre-launch showcase for mojave OSS release
**Drivers:** Demonstrate mojave's measurement science capabilities on a real, well-known benchmark using honest data. No positioning, no comparisons — a quantitative workup.

---

## 1. Experiment Overview

Run the AI2 Reasoning Challenge (ARC-Challenge) through Qwen 2.5 7B-Instruct with 217 perturbation variants, then analyze the results with mojave's full instrument battery. Publish findings as a blog post on antimeme.ai.

### Why ARC Challenge

- 1,172 grade-school science MCQs, 4-choice
- Qwen 2.5 7B-Instruct scores ~50-55% — the IRT sweet spot (maximum response variance)
- Ungated (Allen AI, CC-BY-SA)
- Well-known baseline everyone has intuition about
- Science subdomains enable domain decomposition
- 4-choice format supports exhaustive option-order permutation (4! = 24)

### Why Qwen 2.5 7B-Instruct

- Apache 2.0, fully ungated (no HuggingFace gating delays)
- Fits on A10G (24GB) in BF16 with room for KV cache
- Competitive with Llama 3.1 8B in the 7B-class
- Expected ~50-55% on ARC-Challenge — good IRT signal

---

## 2. Infrastructure

### Compute

| Parameter | Value |
|-----------|-------|
| Instance type | AWS g5.xlarge (NVIDIA A10G, 24GB VRAM) |
| Instance count | 4 (parallel) |
| Model serving | vLLM (version pinned, see §7) |
| vLLM config | `max_model_len=4096`, `gpu_memory_utilization=0.9`, `tensor_parallel_size=1` |
| Model | `Qwen/Qwen2.5-7B-Instruct` (specific revision hash pinned) |
| Eval runner | Inspect AI (version pinned, see §7) |

### Compute Estimate

- 217 variants × 1,172 items = 254,324 inference calls
- ARC questions: ~200-400 input tokens, ~10-50 output tokens
- vLLM continuous batching on A10G for 7B: ~30-50 requests/sec
- Per GPU: ~54 variants × ~40 seconds each = ~36 minutes
- **Wall-clock estimate: 2-3 hours across 4 GPUs**
- **Cost estimate: ~$12-16**

---

## 3. Variant Design (217 total)

### Design Rationale

The statistician review identified three critical problems with N=20 variants:
1. Grossly insufficient for 2PL IRT (need 200+; Edelen & Reeve 2007, De Ayala 2009)
2. Confounded perturbation types prevent attributing instability to specific dimensions
3. Temperature 0.0 is deterministic, collapsing effective sample size

The revised design uses a **partially crossed structure** that addresses all three.

### Dimensions

| Dimension | Levels | Rationale |
|-----------|--------|-----------|
| **A: Option order** | 24 (all permutations of ABCD) | Exhaustive coverage. Detects positional bias. |
| **B: Temperature** | 5 (0.3, 0.5, 0.7, 1.0, 1.5) | Spans conservative-to-aggressive sampling. Temp 0.0 excluded from variant pool (deterministic). |
| **C: Prompt template** | 4 (default Inspect, minimal, explicit-reasoning, answer-only) | Tests prompt sensitivity without changing question content. |

### Crossed Blocks

| Block | Cross | Count | Fixed dimension |
|-------|-------|-------|-----------------|
| 1 | Order × Temperature | 24 × 5 = 120 | Prompt = default Inspect |
| 2 | Order × Prompt | 24 × 4 = 96 | Temperature = 0.7 |
| 3 | Baseline | 1 | Temp 0.0, default order, default prompt |
| **Total** | | **217** | |

Block 1 isolates order and temperature effects. Block 2 isolates order and prompt effects. Temperature 0.7 appears in both blocks as the shared level, enabling cross-block comparison.

### Prompt Templates

1. **Default Inspect:** Whatever `inspect_evals/arc_challenge` ships with (the `multiple_choice()` solver's built-in template).
2. **Minimal:** Question and options only, no system message or framing.
3. **Explicit-reasoning:** "Think step-by-step before selecting your answer."
4. **Answer-only:** "Respond with only the letter of the correct answer."

### Perturbation Application Mechanism

Each perturbation type must be applied at the correct layer in Inspect's solver pipeline:

- **Option order:** Inspect's `shuffle_choices` parameter with explicit seed. Seed determines permutation index (0-23) mapped to a specific permutation of [A,B,C,D]. Verify: the shuffled order must appear in the prompt sent to the model AND the scorer must map the shuffled answer back to the canonical correct answer.
- **Temperature:** `GenerateConfig(temperature=X)` passed to the Inspect task. vLLM respects this via the OpenAI-compatible API.
- **Prompt template:** Custom solver wrappers that replace Inspect's default `multiple_choice()` prompt assembly. These wrap AFTER `shuffle_choices` but BEFORE `generate()`, so option ordering is preserved.

**Critical verification:** After implementing each perturbation type, inspect a sample of 10 prompts to confirm the perturbation appears in the final prompt sent to vLLM and does not get clobbered by Inspect's formatting.

---

## 4. Analyses

### 4.1 Aggregate + Domain Decomposition

**Instrument:** Direct computation from TrialRecords.

- Overall accuracy with Wilson 95% CI across all 217 variants.
- Per-domain accuracy (ARC Challenge has science subdomains — verify these propagate through Inspect's eval log, see §6 risk 3).
- Report interval widths explicitly. For N≈300 per domain at p=0.50, Wilson 95% CI width ≈ 0.11. Minimum detectable difference between two domains at 80% power ≈ 0.11 (two-proportion z-test).
- No hypothesis tests — the intervals speak.

### 4.2 IRT Item Calibration

**Instrument:** `mojave-calibrate irt` (Python, py-irt Bayesian).

**Primary model: 1PL/Rasch.** Estimates difficulty (b) only. Defensible at N=217 (Linacre 1994: Rasch stable with as few as 30 examinees, N=217 is comfortable). Rank all 1,172 items by calibrated difficulty.

**Exploratory model: 2PL.** Report with explicit caveats:
- Discrimination (a) estimates are informative but not calibrated to textbook standards (Edelen & Reeve 2007 recommends N≥200 for 2PL; we meet this threshold but variants are not independent draws).
- Report Yen's Q3 residual correlations for all item pairs. Flag systematic local dependence (Q3 > 0.2). Variants share model weights — local independence is structurally violated (Yen 1984, Bradlow et al. 1999). Acknowledge this prominently.
- Standard errors from the 2PL are anti-conservative due to local dependence. Do not present them as calibrated.

**Claims NOT supported:**
- "Qwen 2.5 7B has ability θ = X" — no reference population.
- "Item X is harder than item Y" via IRT alone — raw p-values (fraction correct across 217 variants) are the primary difficulty metric. IRT calibration is supplementary.
- Unidimensionality — not tested. Science subdomains make it implausible (Reckase 2009).

**Outputs:**
- Item pool JSON with difficulty (Rasch), difficulty + discrimination (2PL exploratory)
- Q3 correlation matrix summary (distribution of Q3 values, count above 0.2)
- Items flagged: near-zero discrimination (2PL), extreme difficulty (floor/ceiling)

### 4.3 Perturbation Stability

**Instrument:** Direct computation from TrialRecords, cross-referenced with IRT.

For each of 1,172 items:
- **Overall stability:** fraction of 217 variants answering correctly.
- **Per-dimension stability:** fraction correct within each perturbation dimension (order, temperature, prompt), enabled by the crossed design.
- Items correct in 217/217 or 0/217: maximally stable (robust knowledge or robust ignorance).
- Items in the 50-150/217 range: perturbation-sensitive.

**Cross-references:**
- Spearman correlation between overall stability and IRT difficulty (Rasch b). Report with bootstrap 95% CI (per reviewer: measurement error attenuates the correlation).
- Spearman correlation between overall stability and 2PL discrimination (exploratory). Same bootstrap CI.
- Per-dimension decomposition: for perturbation-sensitive items (stability < 0.7), which dimension drives the instability? Report as a table: N items primarily order-sensitive, N primarily temperature-sensitive, N primarily prompt-sensitive, N mixed.

**All descriptive.** No per-item hypothesis tests, so no multiplicity correction needed (per statistician review: if we did test, Benjamini-Hochberg FDR at q=0.05 would be appropriate; but we don't).

### 4.4 Sequential Early Stopping (Retrospective Counterfactual)

**Instrument:** `seq-anytime-valid` crate — **requires Bernoulli mSPRT implementation (codebase gap, see §5).**

**Framing:** This is explicitly a retrospective counterfactual: "If items had been randomly ordered and we had applied mSPRT prospectively, the expected stopping time would have been approximately N." This is NOT a frequentist coverage guarantee (Johari et al. 2022 requires prospective application).

**Method:**
1. Take the baseline run (variant 0: temp 0.0, default order, default prompt) — 1,172 binary results.
2. Randomly permute item order 1,000 times.
3. For each permutation, apply Bernoulli mSPRT:
   - H0: p = 0.25 (4-choice chance level)
   - Alternative: Beta mixing distribution (shape parameters TBD during implementation based on Johari et al. 2022 §3.1)
   - Significance level: α = 0.05
4. Record the stopping sample size for each permutation (or "did not stop" if the boundary was never crossed).
5. Report: median stopping time, IQR, fraction of permutations that stopped before N=500 / N=1000 / N=1172.

**Second method: Betting confidence sequences** (Waudby-Smith & Ramdas 2024). These handle bounded random variables correctly and provide always-valid confidence intervals. Run the same 1,000-permutation analysis and report the sample size at which the CS excludes 0.25.

**Claims NOT supported:**
- "mSPRT would have stopped at item N with 95% confidence" as a prospective guarantee.

---

## 5. Codebase Gaps

Three items must be implemented before the experiment can run.

### 5.1 Bernoulli mSPRT

**Location:** `crates/seq-anytime-valid/src/evidence/`

The current `gaussian_msprt_log_lr` assumes Gaussian observations with known variance σ²=1. GPQA/ARC responses are binary (0/1). The Gaussian approximation is poor for n < 30 and for proportions near 0.25.

**Implementation:** Bernoulli log-likelihood ratio with Beta mixing distribution over the alternative.

```
log_lr(n, s) = log ∫ [p^s (1-p)^(n-s)] / [p0^s (1-p0)^(n-s)] dBeta(p; a, b)
```

where s = number of successes in n trials, p0 = null hypothesis proportion.

Reference: Johari et al. (2022), "Always Valid Inference: Continuous Monitoring of A/B Tests," Operations Research 70:1806-1821, §3.1.

**TCK:** Feature file specifying behavior under H0 (Type I control) and H1 (power), degenerate cases (s=0, s=n), and agreement with the Gaussian approximation for large n.

**Validation:** Monte Carlo calibration — 10,000 reps under H0 (p = p0), verify rejection rate ≤ α to within MC error.

### 5.2 Permutation Wrapper for Retrospective Sequential Analysis

**Location:** `crates/seq-anytime-valid/src/` or `crates/eval-orchestrator/src/`

Takes a vector of binary outcomes + a sequential test function, permutes the order K times, runs the test on each permutation, returns a distribution of stopping times.

Simple utility — no major design decisions. Must use a seeded RNG for reproducibility.

### 5.3 Inspect Perturbation Harness

**Location:** New Python script or small package, possibly in `python/` or a new `scripts/inspect-harness/` directory.

Orchestrates 217 Inspect runs with the correct perturbation parameters:
- Generates the 217 variant specifications (order seed, temperature, prompt template)
- Writes the manifest file (run_id → variant spec)
- Launches Inspect runs (parallelized across available GPUs)
- Collects `.eval` log files
- Runs post-ingest validation (all 1,172 items present per run, manifest cross-reference)

This is glue code, not a permanent part of mojave's architecture.

---

## 6. Risks and Mitigations

| # | Risk | Impact | Mitigation |
|---|------|--------|------------|
| 1 | ARC subdomains don't propagate through Inspect eval logs | Domain decomposition (§4.1) impossible | Verify before execution. If missing, build a lookup table mapping ARC item IDs to subdomains from the HuggingFace dataset directly. |
| 2 | Qwen 2.5 7B scores below 30% on ARC-Challenge | Poor IRT signal (floor effect) | Unlikely (reported ~50-55%), but if it happens: report the finding honestly, note which analyses become uninformative, proceed with the rest. |
| 3 | Prompt-template perturbations get clobbered by Inspect's `multiple_choice()` solver | 96 variants (Block 2) are actually identical to Block 1 | Verify with 10-prompt inspection before running all 217 variants. If clobbered, implement custom solver wrappers that bypass `multiple_choice()` formatting. |
| 4 | vLLM OOM on A10G | Runs fail | Pin `max_model_len=4096` to prevent KV cache overallocation. ARC questions are short. |
| 5 | IRT calibration dominated by priors | Discrimination parameters are artifacts | Expected and acknowledged — 1PL/Rasch is primary, 2PL is exploratory with caveats. |
| 6 | Wall-clock time exceeds tonight window | Can't publish tonight | Scale down: run Block 1 (120 variants) first, which is sufficient for IRT. Add Block 2 later. |

---

## 7. Version Pinning and Reproducibility

All of the following must be pinned and recorded in a lockfile committed to the repo:

- Inspect AI version (exact pip version)
- inspect_evals version (exact pip version or git commit hash)
- vLLM version (exact pip version)
- Qwen 2.5 7B-Instruct model revision (HuggingFace commit hash)
- Python version
- CUDA driver version on g5.xlarge AMI
- vLLM server configuration (full `--args` string)

The manifest file (§5.3) records per-run variant specifications. The `.eval` log files are the raw data artifacts.

---

## 8. Data Integrity

### Manifest

A JSON file mapping each run to its variant specification:

```json
{
  "runs": [
    {
      "run_id": "arc-v000",
      "variant": {
        "order_permutation_index": 0,
        "temperature": 0.0,
        "prompt_template": "default",
        "seed": 42
      }
    }
  ]
}
```

### Post-Ingest Validation

After `mojave ingest` processes all `.eval` logs:
1. Verify 1,172 TrialRecords per run (no dropped items)
2. Verify 217 runs total
3. Cross-reference run_id against manifest
4. Flag any runs with errors, timeouts, or unexpected scorer outputs

---

## 9. Mojave Instruments Exercised

This workup exercises the following mojave components:

| Component | Usage |
|-----------|-------|
| `eval-ingest` (Inspect adapter) | Ingest 217 `.eval` logs → TrialRecords |
| `eval-core` (TrialRecord) | Normalize all results to common schema |
| `mojave-calibrate irt` | 1PL + 2PL item calibration |
| `seq-anytime-valid` (Bernoulli mSPRT) | Retrospective stopping analysis |
| `seq-anytime-valid` (confidence sequences) | Betting CS for bounded RVs |
| `eval-orchestrator` | Batch analysis routing |
| `mojave-cli ingest` | CLI entry point for ingestion |
| `mojave-cli analyze` | CLI entry point for full battery |
| `perturbation-engine` | Variant specification (format perturbations) |

Not exercised: `spc-charts` (longitudinal monitoring — single experiment, not time-series), `audit-chain`/`audit-sign` (tamper-evidence — not relevant for blog post), `change-attribution` (git bisect — no code changes to attribute), `salib-*` (sensitivity analysis — no model parameters to perturb).

---

## 10. Blog Post Scope (deferred — design separately after data is earned)

The blog post structure, production values, and editorial voice will be designed after the data is collected and analyzed. Patrick will opine on rigor and measurement science in his own voice. No comparisons to other tools. The analysis is the product.
