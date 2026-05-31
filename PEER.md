# Peer C: QMU + Defense Framework

**You are Peer C.** You own Stream C of the advisory execution plan.

## Your mission

Build the QMU decision framework — the thin composition layer that transforms mojave's statistical outputs into accept/reject/investigate decisions defense customers understand.

## BLOCKED

**You are blocked by Peer A (Stream A: Statistical Correctness).** The QMU framework composes confidence intervals from the CS pipeline. Those CIs are currently invalid (46% coverage instead of 95%). Do NOT begin Task C1 until you see `STREAM_A_COMPLETE.md` in the repo root.

**While blocked:** Do lit review. Read these papers from `../evals_papers/` intake:
- Pilch et al. 2006 SAND2006-5001 (QMU white paper)
- Sharp & Wood-Schultz 2003 (CR definition, free from LANL)
- National Academies 2009 QMU review (free from NAP)
- JCGM 106:2012 (conformity assessment, should be in library)
- Keller et al. 2026 NIST AI 800-3

Write notes in `.context/` (gitignored) if needed.

## Your scope (files you own)

- `crates/eval-orchestrator/src/qmu.rs` — new file
- `crates/eval-orchestrator/src/validity.rs` — new file (construct validity / CVI)
- `crates/eval-orchestrator/tests/qmu_tests.rs` — new file
- `tck/eval-orchestrator/features/qmu.feature` — new file
- `templates/run-card/single-run-card/` — NIST 800-3 alignment section
- `templates/assurance-case/` — GSN template (new directory)

## Your tasks (in order)

1. **C1: QmuAssessment struct with JCGM 106 guard bands** — CR = margin/uncertainty, three-tier decision
2. **C2: Wire QMU to pipeline outputs** — compose from SequentialResult CI
3. **C3: NIST AI 800-3 run card section** — LaTeX template addition
4. **C4: GSN assurance case template** — defense-native reporting (Tier 3)
5. **C5: Construct validity framework** — CVI computation, framed as "sensitivity profile" not validity coefficient (per dissent)

## Key context from the dissent

The dissent (deliverables/dissent.md) argues QMU is "strategic theater" because CR thresholds have no empirical calibration for AI. Response: implement the math, leave thresholds explicitly uncalibrated with documentation. Use JCGM 106 guard bands as the primary decision mechanism — those are framework-agnostic and genuinely applicable.

Frame CVI as "measurement noise budget" or "sensitivity profile," NOT as a validity coefficient. The dissent is right that it's a variance proportion, not formally connected to Borsboom's causal definition.

## Methodology

JSMNTL: lit review → TCK red → compile/run red → implement → green → code review. Commit after every green step.

## Full plan

Read `docs/plans/2026-05-30-advisory-execution.md` — Stream C section — for complete task details with code.

## Branch

You are on `stream-c/qmu-defense`. Commit frequently. Do not touch files outside your scope.
