//! `salib-cli` — operator-facing command surface for the saltelli
//! sensitivity-analysis subsystem. Library half of the `saltelli`
//! binary; the binary entry point at `src/main.rs` is a thin clap-driven
//! wrapper.
//!
//! # Subcommand surface
//!
//! Per `rust_salib_crate_research.md` § 12 and `plans/0002-saltelli-roadmap.md`:
//!
//! - `saltelli sample <problem.yaml> --sampler=<sobol|lhs|saltelli|morris|fast> --n=<N> --seed=<s>` — emit a sample matrix.
//! - `saltelli run <experiment.yaml>` — drive an end-to-end campaign (sample, evaluate via subprocess or library, analyze).
//! - `saltelli analyze <samples.parquet> <outputs.parquet> --estimator=<saltelli2010|jansen|janon|owen|morris|...>` — analyze pre-computed (X, y) pairs.
//!
//! Library entry points are `pub` so harness code, integration tests, and
//! the future `saltelli-workspace` shim (Phase E of `plans/0002-saltelli-roadmap.md`)
//! can drive saltelli without exec'ing the binary — same posture as
//! `crates/workspace-verify/` (per `decisions/2026-04-28-verify-cli.md`).
//!
//! # Crate boundaries
//!
//! Depends on the four saltelli library crates (`-core`, `-samplers`,
//! `-estimators`, `-validation`). workspace-agnostic — the integration
//! shim that maps `saltelli run` results into workspace's audit envelope
//! is `saltelli-workspace`, a separate crate per
//! `decisions/2026-04-28-saltelli-ledger-composition.md`.
//!
//! # Status
//!
//! Pre-code (2026-04-28). PR 1 (`feat/saltelli-scaffold`) ships this empty
//! library + a stub binary. The actual subcommand surface lands in Phase B
//! and beyond as the underlying samplers / estimators / validation lands.
//!
//! See also:
//! - `decisions/2026-04-28-saltelli-where-and-naming.md`
//! - `decisions/2026-04-28-saltelli-tck-posture.md`
//! - `plans/0002-saltelli-roadmap.md`
//! - `rust_salib_crate_research.md` § 12

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
