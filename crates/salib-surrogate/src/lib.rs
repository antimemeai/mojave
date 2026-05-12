//! `salib-surrogate` — surrogate models for the saltelli
//! sensitivity-analysis subsystem.
//!
//! # Phase D scope (per `plans/0003-saltelli-phase-d.md`)
//!
//! - **PR 16a (this PR)** — scaffold + univariate orthogonal
//!   polynomial bases (Legendre, Hermite, Laguerre, Jacobi) +
//!   multi-index enumeration with total-degree truncation.
//! - **PR 16b** — full-PCE OLS solver + Sudret 2008 closed-form
//!   Sobol' indices from PCE coefficients.
//! - **PR 16c** — sparse LARS adaptive PCE (Blatman-Sudret 2011).
//! - **PR 18** — active subspaces (Constantine 2014).
//!
//! # Why a separate crate from `salib-estimators`
//!
//! Surrogate models build a *function approximation* (PCE, Kriging,
//! ridge approximations) and then derive sensitivity indices from
//! the surrogate analytically. The dataflow is fundamentally
//! different from the direct-MC estimators in `salib-estimators`:
//! samples → surrogate → indices, vs samples → indices.
//! Active subspaces likewise live here because they project the
//! input space before surrogate fitting.
//!
//! # Crate boundaries
//!
//! Depends on `salib-core` (`Distribution`, RNG, reductions).
//! workspace-agnostic.
//!
//! # Determinism
//!
//! Polynomial evaluation and multi-index enumeration are pure;
//! same input → bit-identical output. PCE OLS (PR 16b) routes
//! through `salib-core`'s `tree_*` reductions for reduction
//! determinism.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod active_subspace;
pub mod multi_index;
pub mod pce;
pub mod polynomial;
pub mod sparse_pce;

pub use active_subspace::{compute_active_subspace, ActiveSubspace, ActiveSubspaceError};
pub use multi_index::{enumerate_hyperbolic, enumerate_total_degree, MultiIndex, MultiIndexError};
pub use pce::{fit_full_pce, sobol_indices_from_pce, PceError, PolynomialChaos, SobolFromPce};
pub use polynomial::{evaluate, norm_squared, PolynomialFamily};
pub use sparse_pce::{fit_sparse_pce, SparseFitDiagnostic, SparseSolver, TruncationScheme};
