# Future Work

Items we've considered, decided to defer, but must not forget.
Design decisions should account for these even if we're not building them yet.

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

## IRT / item analysis (BEAD-0005)

Item Response Theory for identifying which eval tasks are doing work
and which are redundant. Python layer (torch_measure integration).
Depends on the eval-ingest pipeline being live.

## Factor models (BEAD-0006)

Latent factor analysis for detecting redundancy across eval tasks.
Python layer. Depends on having enough longitudinal data.

## Adaptive testing / CAT (BEAD-0007)

Computerized adaptive testing — select the next eval item based on
what you've learned so far. Reduces eval cost by skipping items
that won't change the conclusion. Depends on IRT calibration.

## Construct validity dossier (BEAD-0011)

Systematic documentation of what an eval actually measures.
Convergent/discriminant validity, content coverage analysis.
Meta-layer over the math crates.

## Game-theoretic eval design (BEAD-0010)

Mechanism design for eval systems where agents have incentives
to game the evaluation. Adversarial robustness of the measurement
itself. Research-grade, not near-term.
