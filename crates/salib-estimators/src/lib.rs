//! `salib-estimators` — sensitivity-index estimators for the
//! saltelli subsystem.
//!
//! # Shipped
//!
//! - **Saltelli 2010 first-order + total-order Sobol'** (`saltelli2010`
//!   module, PR 7). Per Saltelli et al. 2010 Eq c (first-order) +
//!   Jansen 1999 Eq f (total-order).
//! - **Percentile bootstrap CIs** (`bootstrap` module, PR 7). Matches
//!   `SALib`'s default; `BCa` deferred. Designed-sample (Saltelli 2010)
//!   path.
//! - **Generic given-data percentile bootstrap** (`bootstrap_given_data`
//!   module). Wraps any given-data estimator (Sobol', regression,
//!   Borgonovo δ, QOSA, PAWN, RBD-FAST) with row-resampled percentile
//!   CIs; foundation for Tier 2 bootstrap-CI wiring in
//!   `workspace-sensitivity`.
//! - **Morris elementary effects** μ, μ*, σ (`morris` module, PR 8 +
//!   PR 8.6). Morris 1991 + Campolongo 2007.
//! - **FAST / eFAST spectral estimator** (`fast` module, PR 9b).
//!   Saltelli-Tarantola-Chan 1999 — both `Sᵢ` and `Sᵀᵢ` via the
//!   complementary frequency set.
//! - **RBD-FAST estimator** (`rbd_fast` module, PR 10). Tarantola
//!   2006 + Plischke 2010 bias correction. Given-data first-order.
//! - **Borgonovo δ** (`borgonovo` module, PR 11). Plischke-Borgonovo-
//!   Smith 2013 KDE-based moment-independent index.
//! - **PAWN** (`pawn` module, PR 12). Pianosi-Wagener 2015/2018
//!   Kolmogorov-Smirnov moment-independent index.
//! - **DGSM with Poincaré bound** (`dgsm` module, PR 13).
//!   Sobol-Kucherenko 2009 derivative-based total-order upper bound.
//! - **Regression-based** (`regression` module, PR 14).
//!   SRC/SRRC/PCC/PRCC + R² diagnostics.
//! - **Given-data Sobol'** (`given_data_sobol` module, PR 14b).
//!   Plischke-Borgonovo-Smith 2013 partition + law-of-total-variance.
//! - **Janon 2014 + Jansen 1999 + Owen 2013** (`janon`, `jansen`,
//!   `owen` modules, PR 15 — Phase D). Three alternative first-
//!   order estimators with different efficiency / cost trade-offs.
//! - **Crossed G-theory p x i x r** (`g_theory` module).
//!   Balanced model / item / judge reliability decomposition with
//!   `g_coefficient` and `phi_coefficient` diagnostics.
//!
//! # Future
//!
//! Per `plans/0003-saltelli-phase-d.md`:
//!
//! - Phase D: PCE with sparse LARS (PR 16); Shapley effects (PR 17);
//!   active subspaces (PR 18); QOSA + dependent-input (PR 19).
//!
//! # Determinism
//!
//! All sums route through `salib_core::reduce::tree_*` —
//! bit-identical regardless of rayon partitioning. Bootstrap RNG
//! draws use `ChaCha20Rng` derived from the caller's `RngState`.
//!
//! # Crate boundaries
//!
//! Depends on `salib-core` (RNG, reductions) and
//! `salib-samplers` (`SaltelliMatrix`). workspace-agnostic.
//!
//! See:
//! - `decisions/2026-04-29-saltelli-saltelli2010-estimator.md`
//! - `decisions/2026-04-28-saltelli-tck-posture.md`
//! - `plans/0002-saltelli-roadmap.md` (Phases A–C)
//! - `plans/0003-saltelli-phase-d.md` (Phase D)
//! - `rust_salib_crate_research.md`

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod anova;
pub mod bootstrap;
pub mod bootstrap_given_data;
pub mod borgonovo;
pub mod dgsm;
pub mod fast;
pub mod g_theory;
pub mod given_data_sobol;
pub mod janon;
pub mod jansen;
pub mod morris;
pub mod owen;
pub mod pawn;
pub mod qosa;
pub mod rbd_fast;
pub mod regression;
pub mod saltelli2010;
pub mod sobol_indices;

pub use anova::{
    bootstrap_anova_three_way, bootstrap_anova_two_way, estimate_anova_three_way,
    estimate_anova_three_way_with_bootstrap, estimate_anova_two_way,
    estimate_anova_two_way_with_bootstrap, AnovaBootstrapError, AnovaError, AnovaThreeWayResult,
    AnovaTwoWayResult,
};
pub use bootstrap::estimate_saltelli2010_with_bootstrap;
pub use bootstrap_given_data::{
    bootstrap_given_data, BootstrapCi, BootstrapGivenDataError, BoxedEstimatorError,
};
pub use borgonovo::{estimate_borgonovo_delta, BorgonovoError, BorgonovoIndices};
pub use dgsm::{
    estimate_dgsm, finite_difference_gradients, poincare_constant, DgsmError, DgsmIndices, FdKind,
    PoincareError,
};
pub use fast::{estimate_fast, FastEstimatorError, FastIndices};
pub use g_theory::{
    bootstrap_g_theory_pir, estimate_g_theory_pir, estimate_g_theory_pir_with_bootstrap,
    project_g_theory_d_study, DStudyPoint, GTheoryBootstrapError, GTheoryDesign, GTheoryError,
    GTheoryResult,
};
pub use given_data_sobol::{estimate_given_data_sobol, GivenDataSobolError, GivenDataSobolIndices};
pub use janon::{estimate_janon, JanonIndices};
pub use jansen::{estimate_jansen, JansenIndices};
pub use morris::{estimate_morris_effects, EmptyError, MorrisEffects};
pub use owen::{estimate_owen, OwenIndices};
pub use pawn::{estimate_pawn, PawnError, PawnIndices};
pub use qosa::{estimate_qosa, QosaError, QosaIndices};
pub use rbd_fast::{estimate_rbd_fast, RbdFastError, RbdFastIndices};
pub use regression::{estimate_regression_indices, RegressionError, RegressionIndices};
pub use saltelli2010::estimate_saltelli2010;
pub use sobol_indices::{BootstrapMethod, SobolIndices, SobolIndicesWithCi};
