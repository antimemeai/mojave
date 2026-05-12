# TCK — saltelli PAWN estimator

Layer-1 outer Gherkin gate for `saltelli_estimators::estimate_pawn`
— Pianosi-Wagener 2015/2018 moment-independent index via
Kolmogorov-Smirnov statistics on conditional vs unconditional CDFs.
PR 12 of `plans/0002-saltelli-roadmap.md`.

## What this directory covers

- **`pawn_ishigami.feature`** — four scenarios:
  - Indices in `[0, 1]` (KS is bounded by definition).
  - Aggregate ordering `min ≤ median ≤ max`.
  - Factor ranking matches SALib on Ishigami (`median_2 > median_1 > median_3`).
  - Median within `0.05` of SALib's frozen reference at `N=4096`.

## What this directory does NOT cover

- **The reviewer-affordance contract's convergence-rate test +
  cargo-mutants kill rate.** These live inline in
  `crates/saltelli-estimators/tests/pawn_e2e.rs`.
- **Bootstrap CIs.** Bead-eligible.
- **Sensitivity-of-sensitivity (Pianosi 2020) parameter sweep**
  testing PAWN's robustness to slice count + aggregator choice.
  Bead-eligible.

## Step-definition home

- `crates/saltelli-estimators/tests/pawn_tck.rs` wires
  `pawn_ishigami.feature`.

## See also

- `decisions/2026-04-29-saltelli-pawn.md` — ADR.
- Pianosi, F., Wagener, T. (2015). "A simple and efficient method
  for global sensitivity analysis based on cumulative distribution
  functions." *Environmental Modelling & Software* 67.
- Pianosi, F., Wagener, T. (2018). "Distribution-based sensitivity
  analysis from a generic input-output sample." *Environmental
  Modelling & Software* 108.
- Pianosi, F. (2020). "A sensitivity analysis of the PAWN sensitivity
  index." *Environmental Modelling & Software* 127.
