# TCK — saltelli Borgonovo δ estimator

Layer-1 outer Gherkin gate for `saltelli_estimators::estimate_borgonovo_delta`
— Plischke-Borgonovo-Smith 2013 given-data estimator with KDE-based
density estimation. PR 11 of `plans/0002-saltelli-roadmap.md`.

## What this directory covers

- **`borgonovo_ishigami.feature`** — three scenarios:
  - Estimator recovers Ishigami's analytic δ within ~0.06 at N=4096.
  - Indices stay in `[-0.05, 1.05]` (KDE-integration ε slack).
  - Factor ranking by δ correct (`δ_2 > δ_1 > δ_3`).

## What this directory does NOT cover

- **The reviewer-affordance contract's SALib differential +
  convergence-rate test + cargo-mutants kill rate.** These live
  inline in `crates/saltelli-estimators/tests/borgonovo_e2e.rs`.
- **Plischke 2013 Eq 30 bias reduction.** Bead-eligible — would
  require bootstrap RNG plumbing.
- **PAWN, KL-based moment-independent indices.** Different
  estimators; Phase C follow-on PRs.

## Step-definition home

- `crates/saltelli-estimators/tests/borgonovo_tck.rs` wires
  `borgonovo_ishigami.feature`.

## See also

- `decisions/2026-04-29-saltelli-borgonovo-delta.md` — ADR.
- Borgonovo, E. (2007). "A new uncertainty importance measure."
  *Reliability Engineering & System Safety* 92.
- Plischke, E., Borgonovo, E., Smith, C. L. (2013). "Global
  sensitivity measures from given data." *European Journal of
  Operational Research* 226.
