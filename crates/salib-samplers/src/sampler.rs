//! The `Sampler` trait — produces unit-cube `[0, 1)^d` sample
//! matrices that downstream code maps through factor distributions
//! via `Distribution::quantile` (per
//! `decisions/2026-04-28-saltelli-problem-shape.md`).
//!
//! # Why a trait, not a closed enum
//!
//! Closed-enum samplers would force every estimator that consumes
//! samples to match-arm over the sampler kind. Trait-object
//! samplers (or generic-bounded samplers) compose freely with the
//! Saltelli `(A, B, A_Bⁱ)` construction — per sky-claude's spec
//! § 4.3, the construction works on *any* base sampler. LHS, Sobol',
//! Halton — all produce unit samples that the construction stitches
//! together.
//!
//! The trait is `Send + Sync` so samplers can be passed across rayon
//! parallel boundaries when sampling becomes the parallelism target
//! (PR 5+ work, not yet exercised).
//!
//! # The `config_hash` contract
//!
//! Identifies the *configuration* of the sampler — dim, kind, any
//! tunable parameters — not its *output*. Two samplers with the same
//! `config_hash` produce the same output sequence given the same
//! `RngState` input. Used by saltelli's audit envelope (per
//! `decisions/2026-04-28-saltelli-ledger-composition.md`) to record
//! sampler identity inside `context` payloads alongside the seed.
//!
//! SHA-256 over canonical-JSON of the sampler's config struct.
//! Mirrors `Problem::content_hash`.
//!
//! # Determinism
//!
//! `unit_sample` takes `&mut RngState` and *advances* it through the
//! draws. Same `RngState` in → bit-identical `Array2<f64>` out, with
//! `RngState::word_pos` advanced to reflect the consumed bytes. The
//! caller can `RngState::snapshot` before / after to record the
//! pre- and post-draw RNG identity for audit replay.

use ndarray::Array2;
use salib_core::RngState;

/// A sampler that produces unit-cube samples in `[0, 1)^d`.
///
/// Implementations (PR 5):
/// - [`crate::LhsSampler`] — classic and centered Latin Hypercube Sampling.
///
/// Future implementations (per `plans/0002-saltelli-roadmap.md`):
/// - `SobolSampler` (PR 5b — gated on bead `workspace-kss` for the
///   Joe-Kuo direction-number table legal review).
/// - Saltelli matrix construction wraps any `Sampler` (PR 6).
/// - FAST / eFAST / RBD-FAST (Phase C).
pub trait Sampler: Send + Sync {
    /// Number of factors. The output of [`unit_sample`](Self::unit_sample)
    /// has shape `(n, dim())`.
    fn dim(&self) -> usize;

    /// Generate an `n × dim` matrix of unit-cube samples in
    /// `[0, 1)^dim`. Pure: same `(self_config, rng_state)` → same
    /// output. Advances `rng` to reflect bytes consumed; the caller
    /// can snapshot before and after to record the draw range.
    fn unit_sample(&self, n: usize, rng: &mut RngState) -> Array2<f64>;

    /// SHA-256 over canonical-JSON of the sampler's configuration.
    /// Identifies the *configuration* (dim, kind, tunable
    /// parameters), not the output. Two samplers with the same
    /// `config_hash` produce the same output given the same RNG
    /// state.
    fn config_hash(&self) -> [u8; 32];
}
