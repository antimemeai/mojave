# TCK — saltelli sensitivity-analysis subsystem

Behavioral spec layer for the `saltelli-*` crates. Mirrors the prior-project-wide `tck/` discipline (`tck/README.md`): every architectural commitment lands as a Gherkin `.feature` file before code, the harness wires `.feature` files to `#[test]` invocations, the spec is intentionally implementation-language-agnostic so it survives a reimplementation.

Harness is the substrate-ported parser shipped in `crates/prior-project-test/`, per `decisions/2026-04-28-tck-harness-port-substrate.md`.

Validation discipline is the four-layer strategy in `decisions/2026-04-28-saltelli-tck-posture.md` — Gherkin TCK is **Layer 1** of four. The other three layers (inner property + identity tests, frozen-CSV SALib differential, convergence-rate + criterion + cargo-mutants) live inside the saltelli crates' own `tests/` and `benches/` directories, not here.

## Status

Pre-code (2026-04-28). PR 1 (`feat/saltelli-scaffold`) ships this README; no `.feature` files yet because no behavior has landed. Sub-domain directories accrete as PR 2+ open them.

## Sub-domain layout

One directory per saltelli architectural commitment, each with its own `README.md` describing what it covers and what's out of scope. Anticipated layout:

```
tck/saltelli/
├── README.md                              ← this file
├── rng-determinism/                       ← PR 2 of plans/0002-saltelli-roadmap.md
│   ├── README.md
│   └── features/
│       └── deterministic_across_threads.feature
├── problem/                               ← PR 3
│   ├── README.md
│   └── features/
│       ├── content_addressing.feature
│       └── inverse_cdf_round_trip.feature
├── sobol-sampler/                         ← PR 5 (gated on bead prior-project-kss)
│   ├── README.md
│   └── features/
├── saltelli-sampler/                      ← PR 6
│   ├── README.md
│   └── features/
├── sobol-estimator/                       ← PR 7 — first reviewer-affordance-contract close
│   ├── README.md
│   └── features/
├── morris-estimator/                      ← PR 8
│   ├── README.md
│   └── features/
└── ...                                    ← Phase C+ adds one sub-domain per estimator family
```

Each sub-domain `README.md` answers:
- What architectural commitment is being TCK'd here?
- Which ADR(s) does this sub-domain operationalize?
- What's *out of scope* (what does this sub-domain not cover)?
- Where do step definitions live? (Per prior-project convention: in the saltelli crate whose behavior is being TCK'd, not under `tck/`.)

## What goes here

Per the prior-project-wide convention:

- **Observable behavior from outside.** A `.feature` describes what the user / caller / reviewer can see, not how it's implemented. A spec that survives reimplementation in another language (Python, MATLAB) is the bar.
- **One sub-domain per directory.** Avoid mega-features bundling many concerns; bias toward small, scenario-focused features.
- **Step definitions in the consuming crate.** `tck/saltelli/sobol-estimator/features/saltelli2010.feature` has step definitions in `crates/saltelli-estimators/tests/sobol_estimator_tck.rs`, not under `tck/`.
- **`.feature` files name their canonical test functions explicitly.** Ishigami, Sobol' G with `aᵢ = …`, Morris-test with the Morris 1991 §4 setup, Borgonovo bimodal — these are the field's canonical battery (per `decisions/2026-04-28-saltelli-tck-posture.md`); features cite them by name and source.

## What does NOT go here

- **Inner property tests** (Layer 2 of the validation strategy) — lives in each saltelli crate's `tests/` directory. Property-based tests via `proptest`, model-free identity tests (Sᵢ ≤ S_Tᵢ, Σ Sᵢ ≤ 1, μ*ᵢ ≥ |μᵢ|, dummy-parameter floor), and structural-invariance tests are not Gherkin-shaped — they're randomized-input asserts.
- **Frozen-CSV SALib differential** (Layer 3) — reference data lives in `crates/saltelli-validation/reference/salib_outputs/`; assertions live in `crates/saltelli-{estimators,samplers}/tests/differential_*.rs`. Not under `tck/`.
- **Convergence-rate + criterion benches** (Layer 4) — convergence-rate tests live in `crates/saltelli-validation/tests/convergence_*.rs`; benches live in `crates/saltelli-*/benches/`. Not under `tck/`.
- **cargo-mutants survivor catalog** — lives in `crates/saltelli-validation/reference/mutants_inspected.md`. Not under `tck/`.

## Why the four-layer split

A Gherkin spec is a *behavior description* — what the system does, in language a non-implementer can read. It's the right shape for "given a Saltelli matrix of size N for the Ishigami function, when I run the Saltelli2010 estimator, then `S_1` is within tolerance of the analytic value." It's the wrong shape for "for any model `f` and any sample matrix, `S_i ≤ S_Ti`" (model-free identity — a randomized-input property, not a behavior) or for "as N grows from 2¹² to 2¹⁶, RMSE decays as 1/√N within ε" (a convergence-rate test — not a per-input behavior).

Layer 1 (Gherkin TCK, here) gates **scenario-level behavior**.
Layer 2 (`proptest` + identity tests, in-crate) gates **property-level claims**.
Layer 3 (frozen CSVs, in-crate) gates **agreement with the field's reference implementation**.
Layer 4 (convergence + benches + mutants, in-crate) gates **statistical asymptotics, performance, and meta-rigor**.

Each layer catches a different bug class. Per the reviewer-affordance contract in `decisions/2026-04-28-saltelli-tck-posture.md`, every estimator PR ships evidence at all four layers — Gherkin for the headline scenario, identity tests for the model-free claims, differential CSV for SALib agreement, convergence-rate for the asymptotics. The reviewer (Patrick or the code-review subagent) checks for the artifacts; the math is verified by the field's own consensus battery.

## See also

- `decisions/2026-04-28-saltelli-tck-posture.md` — the validation discipline this directory operationalizes.
- `decisions/2026-04-28-tck-harness-port-substrate.md` — the harness this directory rides on.
- `tck/README.md` — prior-project-wide TCK posture.
- `tck/audit-envelope/` / `tck/audit-seal/` / `tck/blob/` — sibling sub-domains, exemplar layouts.
- `plans/0002-saltelli-roadmap.md` — the phase plan; each PR opens (or extends) one sub-domain here.
- `rust_salib_crate_research.md` § 9 — sky-side validation spec; the four layers extend it.
