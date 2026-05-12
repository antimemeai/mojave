# TCK — Phase D PR 15 alternative first-order Sobol' estimators

Layer-1 outer Gherkin gate for `saltelli_estimators::{estimate_janon,
estimate_jansen, estimate_owen}` — Janon 2014, Jansen 1999 (alt
first-order), Owen 2013 Correlation 2. PR 15 of
`plans/0003-saltelli-phase-d.md` (first Phase D PR after Phase C
SALib parity).

## What this directory covers

- **`efficiency_ishigami.feature`** — five scenarios:
  - Each estimator (Janon, Jansen, Owen) recovers Ishigami `S =
    [0.314, 0.442, 0.000]` within MC tolerance.
  - **Janon efficiency claim**: Janon's max-error against analytic
    is `≤` Saltelli2010's at `N=4096` (the asymptotic-efficiency
    property in finite-sample form).
  - **Owen small-S regime**: Owen's `S_3` estimate is tightly
    bounded (`|S_3| < 0.05`) on Ishigami's `S_3 = 0` factor.

## What this directory does NOT cover

- **Convergence-rate test** (`O(1/√N)` decay). Inner test in
  `crates/saltelli-estimators/tests/phase_d_efficiency_e2e.rs`.
- **cargo-mutants kill rate.** Bead `prior-project-63g`.
- **Cross-implementation differential against `SALib`.** Bead
  `prior-project-82n` — none of these estimators appear in `SALib`,
  so a true SALib-byte-exact differential is unavailable; we
  cross-reference against our own `saltelli2010` instead.

## Step-definition home

- `crates/saltelli-estimators/tests/phase_d_efficiency_tck.rs` wires
  `efficiency_ishigami.feature`.

## See also

- `decisions/2026-04-29-saltelli-phase-d-pr15.md` — ADR.
- `plans/0003-saltelli-phase-d.md` — Phase D plan-of-record.
- Janon, A., Klein, T., Lagnoux, A., Nodet, M., Prieur, C. (2014).
  "Asymptotic normality and efficiency of two Sobol' index
  estimators." *ESAIM: Probability and Statistics* 18.
- Jansen, M. J. W. (1999). "Analysis of variance designs for model
  output." *Computer Physics Communications* 117.
- Owen, A. B. (2013). "Better estimation of small Sobol' sensitivity
  indices." *ACM Transactions on Modeling and Computer Simulation*
  23(2).
