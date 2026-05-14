//! `salib-core` — foundational types for the saltelli sensitivity-
//! analysis subsystem. Owns the experiment-level vocabulary that every
//! other saltelli crate composes against: [`rng::RngState`] (multi-
//! stream `ChaCha20` with deterministic salt-derived forking), the
//! [`reduce`] reduction primitives ([`reduce::tree_sum`] /
//! [`reduce::par_tree_sum`] / [`reduce::tree_dot`] /
//! [`reduce::par_tree_dot`] / [`reduce::tree_var`] /
//! [`reduce::par_tree_var`]), and (in later PRs) `Problem` / `Factor` /
//! the closed `Distribution` enum.
//!
//! # Subsystem identity
//!
//! Saltelli is workspace's global-sensitivity-analysis (GSA) substrate. It
//! is **not** AI software, **not** an evaluator of LLMs, **not** a
//! perturbation engine. It is the measurement instrument the perturbation
//! engine (Phase 6 of `plans/0001-overall.md`) commissions and consumes.
//! Same architectural family as Python's `SALib`, R's `sensitivity`, and
//! MATLAB's `UQLab` — designed to slot under those packages' verification
//! batteries (Ishigami, Sobol' G, Morris-test, Borgonovo bimodal,
//! Oakley-O'Hagan), per
//! `decisions/2026-04-28-saltelli-tck-posture.md`.
//!
//! # Determinism foundation
//!
//! `salib-core` is the determinism floor every saltelli sampler and
//! estimator stands on:
//!
//! - [`rng::RngState`] is the serializable RNG identity. Recording it
//!   into the audit envelope's `context` payload (per
//!   `decisions/2026-04-28-saltelli-ledger-composition.md`) lets a
//!   verifier reconstruct any saltelli campaign's RNG stream from
//!   scratch.
//! - [`reduce`] reductions defeat the float-associativity non-
//!   determinism that naive `par_iter().sum()` over `f64` produces
//!   under rayon. Bit-identical regardless of thread count.
//! - The crate-level `#![deny(clippy::disallowed_methods)]` plus the
//!   workspace `clippy.toml` extension enforces the "no
//!   `rayon::iter::ParallelIterator::{sum, reduce, reduce_with, fold}`
//!   on f64" discipline.
//!
//! See `decisions/2026-04-28-saltelli-rng-determinism.md` for the
//! full rationale.
//!
//! # Crate boundaries
//!
//! `salib-core` is **standalone**: no path dep on
//! `workspace-*`. The integration shim that maps SA results into
//! workspace's audit envelope is `saltelli-workspace` (Phase E of
//! `plans/0002-saltelli-roadmap.md`); only it crosses the saltelli /
//! workspace boundary. See
//! `decisions/2026-04-28-saltelli-ledger-composition.md`.
//!
//! # Status
//!
//! 2026-04-28: PR 2 (`feat/saltelli-rng-determinism`) ships [`rng`] +
//! [`reduce`] + the outer Gherkin TCK at
//! `tck/saltelli/rng-determinism/`. PR 3 adds `Problem` /
//! `Distribution` / `Factor` per `plans/0002-saltelli-roadmap.md`
//! Phase A.
//!
//! See also:
//! - `decisions/2026-04-28-saltelli-where-and-naming.md`
//! - `decisions/2026-04-28-saltelli-ledger-composition.md`
//! - `decisions/2026-04-28-saltelli-tck-posture.md`
//! - `decisions/2026-04-28-saltelli-rng-determinism.md`
//! - `plans/0002-saltelli-roadmap.md`
//! - `rust_salib_crate_research.md`

#![forbid(unsafe_code)]
#![deny(clippy::disallowed_methods)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod distribution;
pub mod problem;
pub mod reduce;
pub mod rng;

pub use distribution::Distribution;
pub use problem::{BuildError, Factor, FactorKind, Group, Problem, ProblemBuilder};
pub use reduce::{par_tree_dot, par_tree_sum, par_tree_var, tree_dot, tree_sum, tree_var, BLOCK};
pub use rng::{RngAlgorithm, RngState};
