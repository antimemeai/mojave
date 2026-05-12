# TCK — saltelli RBD-FAST estimator

Layer-1 outer Gherkin gate for `saltelli_estimators::estimate_rbd_fast`
— Tarantola 2006 random-balance design with Plischke 2010 bias
correction. PR 10 of `plans/0002-saltelli-roadmap.md`.

## What this directory covers

- **`rbd_fast_ishigami.feature`** — the headline scenarios:
  - Estimator recovers Ishigami's analytic first-order indices
    within RBD-FAST's bias floor (~0.06 at N=4096).
  - Plischke-corrected indices stay within `[-0.05, 1.05]`.
  - Factor ranking by `S` exactly correct (`S_2 > S_1 > S_3`).

## What this directory does NOT cover

- **The reviewer-affordance contract's SALib differential +
  convergence-rate test + cargo-mutants kill rate.** These live
  inline in `crates/saltelli-estimators/tests/rbd_fast_e2e.rs`.
- **Total-order under RBD.** Not in `SALib`'s `rbd_fast`; deferred.

## Step-definition home

- `crates/saltelli-estimators/tests/rbd_fast_tck.rs` wires
  `rbd_fast_ishigami.feature`.

## See also

- `decisions/2026-04-29-saltelli-rbd-fast.md` — ADR.
- `decisions/2026-04-28-saltelli-tck-posture.md` — validation strategy.
- Tarantola, S., Gatelli, D., Mara, T. A. (2006). "Random balance
  designs for the estimation of first order global sensitivity
  indices." *Reliability Engineering & System Safety* 91.
- Plischke, E. (2010). "An effective algorithm for computing global
  sensitivity indices (EASI)." *Reliability Engineering & System
  Safety* 95.
- Tissot, J.-Y., Prieur, C. (2012). "Bias correction for the
  estimation of sensitivity indices based on random balance
  designs." *Reliability Engineering & System Safety* 107.
