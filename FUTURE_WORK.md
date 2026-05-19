# Future Work

Items we've considered, decided to defer, but must not forget.
Design decisions should account for these even if we're not building them yet.

---

## Completed (moved here for reference)

These were previously on the future work list. They're built, tested, and merged.

- **IRT / item analysis (BEAD-0005)** — `mojave-calibrate` IrtCalibrator wrapping py-irt. 2PL Bayesian IRT with GPU support. Closed 2026-05-19.
- **Factor models (BEAD-0006)** — `mojave-calibrate` FactorCalibrator (deepirtools IWAVE) + CfaCalibrator (semopy SEM/CFA). Closed 2026-05-19.
- **Adaptive testing / CAT (BEAD-0007)** — `eval-design` crate. Randomized item selection, anti-gaming scheduling, CAT engine with 2PL ability estimation. Closed.
- **Game-theoretic eval design (BEAD-0010)** — Folded into `eval-design` crate. Mechanism design for adversarial robustness of evaluations. Closed.

---

## Pre-registration (eval-prereg)

ICH E9-style analysis plan declaration. Hash the plan upfront, enforce
that what ran matches what was declared. The orchestrator must emit
enough provenance that pre-reg can verify after the fact. Not on the
critical path to a live product, but essential for defense customers
who need to demonstrate that they didn't p-hack their eval results.

Prior thinking: NOTE_TO_SKY_CLAUDE §eval-prereg, BEAD-0009.

## Binary signing — REQUIRED BEFORE PRODUCTION

Signed release binaries for mojave-cli and the engine daemon.
Not deferred in spirit — this is required for production deployment.
The integrity of reporting depends on the integrity of the tool
producing it. If the binary can't prove it hasn't been tampered with,
the audit chain is meaningless.

Hardware fuse analogy (Nintendo Switch eFuse model): the binary
should carry a tamper-evident marker that customers can verify
before trusting its output. Signed envelopes from an unsigned
binary is theater.

Deferred from the initial build phases only because the math and
orchestration need to exist before there's something to sign.
Slots in before first external deployment, not after.

## Sealed audit corpus

Cryptographic sealing of the audit corpus into tamper-evident chains.
Instrumentation is always on; sealing is opt-in. Massive trust upside
for defense deployments, significant implementation cost. Design the
audit system so sealing can be added without restructuring.

## Construct validity dossier (BEAD-0011)

Systematic documentation of what an eval actually measures.
Convergent/discriminant validity, content coverage analysis.
Meta-layer over the math crates. Deferred 2026-05-18.

## Range orchestration (BEAD-0013)

Repeatable agent evaluation environments. Firecracker microVM
orchestration, cell-runner for isolated per-cell execution,
reproducible ranges. Substantial infrastructure lift — depends
on the measurement stack being stable first.

## REEval integration

Stanford AIMS amortized calibration. Stub exists in
`mojave-calibrate` (`reeval_stub.py`). Blocked on REEval
becoming pip-installable and dropping the CUDA 12.2 +
flash-attention + Llama 8B embedding requirements.

## Runner integrations

Live integration with eval runners (Inspect, HAL, lm-eval-harness).
The ingest layer (`eval-ingest`) accepts their output formats;
what's missing is the glue that makes this zero-config for
end users. Probably a thin adapter per runner.

## Reporting surface

Human-readable diagnostic reports. Control chart visualizations,
item analysis summaries, factor structure plots, reliability
dashboards. The math produces the numbers; this makes them
legible to stakeholders who don't read covariance matrices.
