# TCK — saltelli QOSA estimator

Layer-1 outer Gherkin gate for `saltelli_estimators::estimate_qosa`
— Maume-Deschamps & Niang 2018 partition-based form of the
quantile-oriented sensitivity index (their Prop 3.1). PR 19a of
`plans/0003-saltelli-phase-d.md`.

## What this directory covers

- **`qosa.feature`** — four scenarios:
  - **Ishigami at α=0.5** factor ordering matches first-order Sobol'.
  - **Independent factor sanity**: `Y = X_0` model gives `S^α_1 ≈
    S^α_2 ≈ 0`, `S^α_0` substantial (Maume-Deschamps Remark).
  - **Tail-vs-median switch**: a gated tail model where the
    median-driver and tail-driver are different factors. QOSA
    correctly switches its top-ranked factor as α moves from 0.5
    to 0.95 — the headline distinguishing claim over variance-
    based Sobol'.
  - **Determinism**: same input → bit-identical indices.

## What this directory does NOT cover

- **Kernel-conditional-quantile two-sample estimator** (Maume-
  Deschamps Eq 4.3 form). Bead-eligible if a workload demonstrates
  the partition variant has insufficient resolution.
- **Grouped-factor QOSA + variance reduction** — bead `prior-project-0yt`.
- **R-reference (paper authors' code) byte-exact differential** —
  bead-eligible.

## Step-definition home

`crates/saltelli-estimators/tests/qosa_tck.rs`.
