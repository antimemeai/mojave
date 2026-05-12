//! `salib-validation` — analytic test functions with closed-form
//! sensitivity indices, plus frozen `SALib` reference data, for the
//! saltelli subsystem.
//!
//! # The canonical battery
//!
//! Functions hosted here (per `rust_salib_crate_research.md` § 9.1 and the
//! GSA literature):
//!
//! | Function                  | Source                               | Why it matters |
//! |---------------------------|--------------------------------------|----------------|
//! | Ishigami                  | Ishigami-Homma 1990; Saltelli 2008   | X₃ has zero first-order, nonzero total — total-order canary |
//! | Sobol' G (tunable)        | Saltelli-Sobol' 1995                 | Tunable factor strengths via aᵢ |
//! | Sobol' G\*                | Saltelli et al. 2010                 | G with shifts/exponents |
//! | Bratley family            | Bratley-Fox 1988                     | Integration baselines |
//! | Morris test (20-factor)   | Morris 1991 §4                       | 5 strong / 5 interacting / 10 negligible |
//! | Borgonovo bimodal         | Borgonovo 2007                       | Multi-modal output; variance fails, δ succeeds |
//! | Linear k-factor           | trivial                              | SRC validity: pure linear ⇒ SRC = √Sᵢ exactly |
//! | Oakley-O'Hagan            | Oakley-O'Hagan 2004                  | Kriging / GP SA standard |
//!
//! Each function ships with its closed-form analytic-indices module
//! (where closed forms exist) — these are the ground truth that the
//! estimator PRs converge to under the reviewer-affordance contract.
//!
//! # The pattern
//!
//! Every analytic test function lives in its own module
//! (`salib_validation::ishigami`, `salib_validation::sobol_g`,
//! …) and exports three pieces:
//!
//! - **`fn name(x: &[f64]) -> f64`** — the function evaluation. Pure
//!   `[f64]` slice for inputs; ndarray adapters wait for the
//!   sampler crate (PR 5+ of `plans/0002-saltelli-roadmap.md`).
//! - **`fn analytic_indices(params...) -> SobolIndicesAnalytic`** — the
//!   closed-form Sobol' decomposition. The ground truth that
//!   estimator PRs converge to.
//! - **`fn input_distribution(...) -> Problem`** — the matching
//!   `Problem` with the function's canonical input space (e.g.,
//!   `Uniform(-π, π)` for Ishigami, `Uniform(0, 1)` for Sobol' G).
//!   Lets samplers drive the function without each test rewiring its
//!   factor list.
//!
//! Per `decisions/2026-04-28-salib-validation-pattern.md`.
//!
//! # Frozen `SALib` differential reference
//!
//! `reference/salib_outputs/*.csv` (lands with the first estimator PR
//! that needs it, PR 7 of `plans/0002-saltelli-roadmap.md`) holds
//! `SALib` outputs for each (sampler, estimator, RNG, N, seed) tuple
//! in the test suite. Two tolerance regimes per
//! `decisions/2026-04-28-saltelli-tck-posture.md` Layer 3: byte-exact
//! (~1e-12) under pinned MT19937 via the `salib-compat` feature,
//! MC-noise (`k / √N`) when only the spec is pinned. Live `PyO3`
//! rebaselining mode is a future weekly-CI path (bead — lands once
//! the first frozen CSV is committed).
//!
//! # Crate boundaries
//!
//! Dependency: `salib-core` (for `Problem` / `Distribution` / `ProblemBuilder`).
//! Hosted here so estimators and samplers can both pull it in for
//! tests + benches without pulling each other. workspace-agnostic.
//!
//! # Status
//!
//! 2026-04-28: PR 4 ships [`ishigami`] (full first + total-order
//! analytic) and [`sobol_g`] (first-order analytic; total-order
//! deferred per
//! `decisions/2026-04-28-salib-validation-pattern.md`). Morris-test
//! lands alongside the Morris estimator (PR 8 of
//! `plans/0002-saltelli-roadmap.md`); its analytic ground truth is
//! EE-style (μ, μ*, σ), shape-mismatched with
//! `SobolIndicesAnalytic`. Bratley / Borgonovo / Oakley-O'Hagan land
//! as their respective estimators do.
//!
//! See also:
//! - `decisions/2026-04-28-salib-validation-pattern.md`
//! - `decisions/2026-04-28-saltelli-tck-posture.md`
//! - `plans/0002-saltelli-roadmap.md` Phase B
//! - `rust_salib_crate_research.md` § 9

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod analytic;
pub mod ishigami;
pub mod morris_test;
pub mod sobol_g;

pub use analytic::{MorrisEffectsAnalytic, SobolIndicesAnalytic};
