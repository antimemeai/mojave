# Peer A: Statistical Correctness

**You are Peer A.** You own Stream A of the advisory execution plan.

## Your mission

Fix the confidence sequence pipeline — it currently produces 46% coverage instead of 95% for Bernoulli data. This is the critical path: Streams C and D are blocked until you finish.

## Your scope (files you own)

- `crates/seq-anytime-valid/` — AnytimeMonitor, BernoulliMonitor, types
- `crates/eval-orchestrator/src/instruments/sequential.rs` — SequentialInstrument
- `crates/mojave-gsa/src/analyze.rs` — data quality gate only
- `crates/mojave-gsa/src/diagnostics.rs` — convergence diagnostics (new file)
- `tck/seq-anytime-valid/` — new TCK scenarios

## Your tasks (in order)

1. **A1: Fix AnytimeMonitor sigma for Bernoulli** — dispatch on DataFamily, use sigma=0.5 for Bernoulli
2. **A2: Fix SequentialInstrument** — use DataFamily::Bernoulli for binary outcomes
3. **A3: Gate 4 Monte Carlo test** — 10,000 reps × 5 p-values through AnytimeMonitor::update()
4. **A4: Data quality gate** — reject n_samples=0 cells in Sobol analysis
5. **A5: Convergence diagnostics** — warn on negative S1, CI crossing bounds, sum_ST > 1.3
6. **A6: Waudby-Smith betting CS** — correct long-term solution (needs lit review of paper)

## The bug

`crates/seq-anytime-valid/src/monitor/anytime.rs` lines 68-74: AnytimeMonitor computes sigma via Welford's online variance regardless of DataFamily. For Bernoulli data, this voids the anytime-valid guarantee. The existing Gate 4 test (`tests/gate4_monte_carlo.rs`) tests `normal_mixture_cs_known_sigma` — NOT the production AnytimeMonitor path.

## Methodology

JSMNTL: TCK red → compile/run red → implement → green → code review. Commit after every green step.

## Full plan

Read `docs/plans/2026-05-30-advisory-execution.md` — Stream A section — for complete task details with code.

## When you're done

Signal completion by creating a file `STREAM_A_COMPLETE.md` at repo root with a summary of what was done and test results. This unblocks Peers C and D.

## Branch

You are on `stream-a/statistical-correctness`. Commit frequently. Do not touch files outside your scope.
