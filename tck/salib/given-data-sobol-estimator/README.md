# TCK — saltelli given-data Sobol' estimator

Layer-1 outer Gherkin gate for `saltelli_estimators::estimate_given_data_sobol`
— Plischke-Borgonovo-Smith 2013 partition-based first-order Sobol'.
PR 14b of `plans/0002-saltelli-roadmap.md` (closes Phase C).

## What this directory covers

- **`given_data_sobol_ishigami.feature`** — three scenarios:
  - Estimator recovers Ishigami's analytic `S_1 = [0.314, 0.442, 0]` within `0.03`.
  - Indices in `[0, 1]`.
  - Factor ranking correct (`S_1[1] > S_1[0] > S_1[2]`).

## What this directory does NOT cover

- **Cross-implementation differential vs `rbd_fast`** (same `(X, Y)`,
  partition vs spectral mechanism). Lives inline in
  `crates/saltelli-estimators/tests/given_data_sobol_e2e.rs`.
- **Convergence-rate test, cargo-mutants kill rate.** Bead-tracked
  (`prior-project-63g`).
- **Total-order via given-data partition.** Bead-eligible — paper
  also discusses this, but `SALib` doesn't ship it.

## Step-definition home

- `crates/saltelli-estimators/tests/given_data_sobol_tck.rs` wires
  `given_data_sobol_ishigami.feature`.

## See also

- `decisions/2026-04-29-saltelli-given-data-sobol.md` — ADR.
- `decisions/2026-04-29-saltelli-borgonovo-delta.md` — sibling
  partition-based estimator (PDF-divergence variant).
- Plischke, E., Borgonovo, E., Smith, C. L. (2013). "Global
  sensitivity measures from given data." *European Journal of
  Operational Research* 226.
