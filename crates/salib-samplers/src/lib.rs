//! `salib-samplers` — sampling designs for the saltelli sensitivity-
//! analysis subsystem.
//!
//! Two surfaces:
//!
//! - **`Sampler` trait** — i.i.d.-shaped samplers producing unit-cube
//!   samples in `[0, 1)^d` (LHS, Sobol'). Downstream code maps the
//!   unit samples through factor distributions via
//!   `Distribution::quantile` (per
//!   `decisions/2026-04-28-saltelli-problem-shape.md`).
//! - **Free-function constructors** for designs whose output is
//!   intrinsically blocked or carries load-bearing metadata that
//!   doesn't fit the `Sampler::unit_sample(n) -> (n, d)` shape:
//!   `build_saltelli_matrix` (the `(A, B, A_Bⁱ)` construction),
//!   `build_morris_trajectories` (OAT trajectories with factor-
//!   permutation order), and `build_fast_design` (search-curve
//!   blocks with frequency / phase metadata).
//!
//! # Implementations
//!
//! - [`LhsSampler`] — Latin Hypercube Sampling, classic and centered
//!   variants. Per McKay-Beckman-Conover 1979. PR 5 of
//!   `plans/0002-saltelli-roadmap.md`.
//!
//! Future implementations (per the plan):
//! - `SobolSampler` (PR 5b — gated on bead `workspace-kss` for the
//!   Joe-Kuo direction-number table legal review).
//! - Saltelli `(A, B, A_Bⁱ)` matrix construction (PR 6) — wraps any
//!   `Sampler` to produce the SA matrix design.
//! - Morris trajectories (PR 8).
//! - FAST / eFAST / RBD-FAST search-curve designs (Phase C).
//! - Maximin / Orthogonal / Replicated LHS (follow-on).
//!
//! # Determinism
//!
//! Every sampler advances the input `RngState`'s `word_pos` by the
//! exact number of bytes consumed; same `RngState` in → bit-identical
//! `Array2<f64>` out. Pinned by
//! `tck/saltelli/lhs-sampler/features/lhs_determinism.feature` and
//! analogous TCK directories for each future sampler.
//!
//! # Crate boundaries
//!
//! Depends on `salib-core`. workspace-agnostic.
//!
//! See also:
//! - `decisions/2026-04-28-saltelli-lhs-sampler.md`
//! - `decisions/2026-04-28-saltelli-tck-posture.md`
//! - `plans/0002-saltelli-roadmap.md` Phase B
//! - `rust_salib_crate_research.md` § 4

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod fast;
pub mod iman_conover;
pub mod lhs;
pub mod morris;
pub mod owen_matrix;
pub mod plackett_burman;
pub mod saltelli_matrix;
pub mod sampler;
pub mod sobol;

pub use fast::{build_fast_design, FastDesign, FastError};
pub use iman_conover::{iman_conover_transform, ImanConoverError};
pub use lhs::{LhsKind, LhsSampler};
pub use morris::{
    build_grouped_morris_trajectories, build_morris_trajectories, MorrisError, MorrisTrajectories,
};
pub use owen_matrix::{build_owen_matrix, OwenMatrix, OwenMatrixError};
pub use plackett_burman::{build_plackett_burman, PbError, PlackettBurmanDesign};
pub use saltelli_matrix::{build_saltelli_matrix, SaltelliError, SaltelliMatrix};
pub use sampler::Sampler;
pub use sobol::{SobolDimSet, SobolSampler};
