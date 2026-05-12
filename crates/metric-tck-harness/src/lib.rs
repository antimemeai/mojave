//! `metric-tck-harness` — test-only helpers for Workspace crates.
//!
//! **Dep invariant:** other crates MUST declare this only in
//! `[dev-dependencies]`, never in `[dependencies]`. Test-only
//! scaffolding shipping into production builds inflates binary size
//! and pulls in test-only code paths attackers can target. Mirrors
//! substrate's `firecrew-test` invariant; Workspace will enforce
//! via grep-lint when crate count justifies it.
//!
//! Modules:
//!
//! - [`gherkin`] — homegrown Gherkin parser + scenario runner for
//!   Workspace-wide TCK harnesses. Ported verbatim from substrate's
//!   `firecrew-test::gherkin` on 2026-04-28. See
//!   `decisions/2026-04-28-tck-harness-port-substrate.md`.
//!
//! Future modules will accumulate as Workspace's test surface
//! grows — proptest strategies for canonical-bytes round-trips,
//! fixture loaders for `sherlock`-shaped object batches, chaos /
//! fault-injection helpers for sink testing, etc.
//!
//! Per substrate's pattern (and the `clippy.toml` cap), helper
//! modules stay focused; if a helper grows beyond ~300 lines or
//! pulls in heavyweight deps, it earns its own crate rather than
//! bloating this one.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod gherkin;
