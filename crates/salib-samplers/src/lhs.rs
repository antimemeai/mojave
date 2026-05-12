//! Latin Hypercube Sampling — the simplest stratified sampler.
//!
//! Per McKay-Beckman-Conover 1979. For each dimension `j` independently:
//!
//! 1. Permute `(0, 1, …, n-1)` randomly via Fisher-Yates using the
//!    `ChaCha20Rng` derived from the shared `RngState`.
//! 2. For each row `i`, set `X[i, j]` such that exactly one sample
//!    falls in each stratification cell `[k/n, (k+1)/n)`.
//!
//! Two variants:
//! - **Classic** (`LhsKind::Classic`) — `X[i, j] = (perm[i] + u) / n`
//!   where `u ~ Uniform(0, 1)`. Each cell contains a uniformly-random
//!   point.
//! - **Centered** (`LhsKind::Centered`) — `X[i, j] = (perm[i] + 0.5) / n`.
//!   Each cell contains its center; deterministic given the permutation.
//!
//! # Why both
//!
//! Classic is the canonical LHS; centered is widely used as a coarser
//! deterministic alternative when the random-offset within each
//! stratification cell isn't load-bearing (e.g., when the SA
//! estimator only cares about the rank-stratification structure).
//! Per sky-claude's spec § 4.1 and the LHS implementations in
//! `egobox-doe` and `SALib`.
//!
//! # Determinism
//!
//! Per dimension, the RNG advance is deterministic:
//! 1. Fisher-Yates draws `n - 1` `u32` values for the permutation.
//! 2. Classic: `n` `u32` values are reduced to `f64` via
//!    `(u32::MAX as f64 + 1.0)` for the `u` offset; centered has no
//!    extra RNG draws.
//!
//! Same `RngState` in → bit-identical `Array2<f64>` out.
//! Configuration hash is stable across calls.

use ndarray::Array2;
use rand::RngCore;
use salib_core::RngState;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::sampler::Sampler;

/// LHS variant. Closed enum, `#[non_exhaustive]`. Future variants:
/// `Maximin` (via `egobox-doe`), `Orthogonal` (Tang 1993),
/// `Replicated` (multiple independent LHS draws). Each lands via
/// follow-on ADRs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[serde(tag = "kind")]
pub enum LhsKind {
    /// Random offset within each stratification cell:
    /// `X[i, j] = (perm[i] + u) / n` with `u ~ U(0, 1)`.
    Classic,
    /// Centered cell value: `X[i, j] = (perm[i] + 0.5) / n`.
    Centered,
}

/// Latin Hypercube Sampler.
///
/// Construct via [`LhsSampler::classic`], [`LhsSampler::centered`],
/// or [`LhsSampler::with_kind`]. `#[non_exhaustive]` blocks
/// struct-literal construction; future fields (e.g., `n_replications`
/// for replicated LHS) land non-breaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct LhsSampler {
    /// Number of factors (output columns).
    pub dim: usize,
    /// LHS variant.
    pub kind: LhsKind,
}

impl LhsSampler {
    /// Classic LHS sampler. Random offset within each stratification
    /// cell.
    #[must_use]
    pub fn classic(dim: usize) -> Self {
        Self {
            dim,
            kind: LhsKind::Classic,
        }
    }

    /// Centered LHS sampler. Cell-center values; deterministic given
    /// the permutation.
    #[must_use]
    pub fn centered(dim: usize) -> Self {
        Self {
            dim,
            kind: LhsKind::Centered,
        }
    }

    /// Construct with explicit `LhsKind`.
    #[must_use]
    pub fn with_kind(dim: usize, kind: LhsKind) -> Self {
        Self { dim, kind }
    }

    /// LHS variant.
    #[must_use]
    pub fn kind(&self) -> LhsKind {
        self.kind
    }
}

impl Sampler for LhsSampler {
    fn dim(&self) -> usize {
        self.dim
    }

    fn unit_sample(&self, n: usize, rng: &mut RngState) -> Array2<f64> {
        let mut chacha = rng.clone().into_chacha();

        let mut out = Array2::<f64>::zeros((n, self.dim));
        if n == 0 || self.dim == 0 {
            *rng = RngState::snapshot(&chacha, rng);
            return out;
        }

        #[allow(clippy::cast_precision_loss)]
        let n_f = n as f64;
        // Reciprocal of (u32::MAX + 1) for converting random u32 to
        // U(0, 1). Same conversion `rand`'s `f64` Open01 uses.
        let u32_norm = 1.0_f64 / (f64::from(u32::MAX) + 1.0);

        // Per-dimension Fisher-Yates permutation + cell evaluation.
        // Fixed iteration order: dim-major, within-dim row-by-row.
        // The order matches the bit-identity property in the TCK.
        let mut perm: Vec<usize> = (0..n).collect();
        for j in 0..self.dim {
            // Reset the permutation buffer to identity.
            for (idx, slot) in perm.iter_mut().enumerate() {
                *slot = idx;
            }
            // Fisher-Yates: i from n-1 down to 1, swap perm[i] with
            // perm[k] where k ~ Uniform{0..=i}. We draw u32 and
            // reduce mod (i+1) — the standard approach. n-1 draws
            // total.
            for i in (1..n).rev() {
                #[allow(clippy::cast_possible_truncation)]
                let k = (chacha.next_u32() as usize) % (i + 1);
                perm.swap(i, k);
            }

            // Fill column j.
            match self.kind {
                LhsKind::Classic => {
                    for i in 0..n {
                        #[allow(clippy::cast_precision_loss)]
                        let perm_i = perm[i] as f64;
                        let u = f64::from(chacha.next_u32()) * u32_norm;
                        out[[i, j]] = (perm_i + u) / n_f;
                    }
                }
                LhsKind::Centered => {
                    for i in 0..n {
                        #[allow(clippy::cast_precision_loss)]
                        let perm_i = perm[i] as f64;
                        out[[i, j]] = (perm_i + 0.5) / n_f;
                    }
                }
            }
        }

        *rng = RngState::snapshot(&chacha, rng);
        out
    }

    fn config_hash(&self) -> [u8; 32] {
        #[allow(clippy::expect_used)]
        let bytes = serde_json::to_vec(self)
            .expect("serializing LhsSampler to JSON cannot fail (all plain data)");
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        hasher.finalize().into()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::cast_precision_loss)]
mod tests {
    use super::*;

    fn fresh_rng(stream: u64) -> RngState {
        RngState::from_parts([0x42; 32], stream, 0)
    }

    // ── Constructors ────────────────────────────────────────────────

    #[test]
    fn classic_sets_kind_classic() {
        let s = LhsSampler::classic(5);
        assert_eq!(s.dim, 5);
        assert_eq!(s.kind, LhsKind::Classic);
    }

    #[test]
    fn centered_sets_kind_centered() {
        let s = LhsSampler::centered(3);
        assert_eq!(s.dim, 3);
        assert_eq!(s.kind, LhsKind::Centered);
    }

    #[test]
    fn with_kind_preserves_explicit_kind() {
        let s = LhsSampler::with_kind(7, LhsKind::Centered);
        assert_eq!(s.kind, LhsKind::Centered);
        let s = LhsSampler::with_kind(7, LhsKind::Classic);
        assert_eq!(s.kind, LhsKind::Classic);
    }

    #[test]
    fn kind_accessor_returns_kind() {
        assert_eq!(LhsSampler::classic(3).kind(), LhsKind::Classic);
        assert_eq!(LhsSampler::centered(3).kind(), LhsKind::Centered);
    }

    // ── Output shape ────────────────────────────────────────────────

    #[test]
    fn unit_sample_output_shape_matches_n_by_dim() {
        let s = LhsSampler::classic(4);
        let mut rng = fresh_rng(0);
        let m = s.unit_sample(64, &mut rng);
        assert_eq!(m.shape(), &[64, 4]);
    }

    #[test]
    fn unit_sample_zero_rows_returns_empty_matrix() {
        let s = LhsSampler::classic(3);
        let mut rng = fresh_rng(0);
        let m = s.unit_sample(0, &mut rng);
        assert_eq!(m.shape(), &[0, 3]);
    }

    #[test]
    fn unit_sample_zero_dim_returns_empty_columns() {
        let s = LhsSampler::classic(0);
        let mut rng = fresh_rng(0);
        let m = s.unit_sample(8, &mut rng);
        assert_eq!(m.shape(), &[8, 0]);
    }

    // ── Output range ────────────────────────────────────────────────

    #[test]
    fn classic_output_in_zero_one_open() {
        let s = LhsSampler::classic(3);
        let mut rng = fresh_rng(0);
        let m = s.unit_sample(128, &mut rng);
        for &v in &m {
            assert!((0.0..1.0).contains(&v), "out-of-range LHS value {v}");
        }
    }

    #[test]
    fn centered_output_in_zero_one_open() {
        let s = LhsSampler::centered(3);
        let mut rng = fresh_rng(0);
        let m = s.unit_sample(128, &mut rng);
        for &v in &m {
            assert!(
                (0.0..1.0).contains(&v),
                "out-of-range centered LHS value {v}"
            );
        }
    }

    // ── Stratification: one sample per cell per column ─────────────

    #[test]
    fn classic_each_column_has_one_sample_per_stratification_cell() {
        // For each column, `floor(value * n)` should be a permutation
        // of {0, 1, …, n-1}. This is the load-bearing LHS property.
        let n = 32;
        let dim = 5;
        let s = LhsSampler::classic(dim);
        let mut rng = fresh_rng(0);
        let m = s.unit_sample(n, &mut rng);
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let cell_index = |x: f64| (x * n as f64).floor() as usize;
        for j in 0..dim {
            let mut cells: Vec<usize> = (0..n).map(|i| cell_index(m[[i, j]])).collect();
            cells.sort_unstable();
            let expected: Vec<usize> = (0..n).collect();
            assert_eq!(cells, expected, "column {j} stratification");
        }
    }

    #[test]
    fn centered_each_column_has_exact_cell_centers() {
        // Centered LHS: each column's set of values, sorted, equals
        // {0.5/n, 1.5/n, …, (n-0.5)/n}.
        let n = 16;
        let dim = 3;
        let s = LhsSampler::centered(dim);
        let mut rng = fresh_rng(0);
        let m = s.unit_sample(n, &mut rng);
        for j in 0..dim {
            let mut col: Vec<f64> = (0..n).map(|i| m[[i, j]]).collect();
            col.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let expected: Vec<f64> = (0..n).map(|k| (k as f64 + 0.5) / n as f64).collect();
            for (got, want) in col.iter().zip(expected.iter()) {
                assert!(
                    (got - want).abs() < 1e-12,
                    "col {j}: got {got}, want {want}"
                );
            }
        }
    }

    // ── Determinism ─────────────────────────────────────────────────

    #[test]
    fn unit_sample_is_deterministic_under_same_rng_state() {
        let s = LhsSampler::classic(4);
        let mut r1 = fresh_rng(7);
        let mut r2 = fresh_rng(7);
        let m1 = s.unit_sample(64, &mut r1);
        let m2 = s.unit_sample(64, &mut r2);
        assert_eq!(m1, m2);
        // RNG advance should also be identical.
        assert_eq!(r1, r2);
    }

    #[test]
    fn unit_sample_different_streams_produce_different_matrices() {
        let s = LhsSampler::classic(3);
        let mut r1 = fresh_rng(1);
        let mut r2 = fresh_rng(2);
        let m1 = s.unit_sample(32, &mut r1);
        let m2 = s.unit_sample(32, &mut r2);
        assert_ne!(m1, m2);
    }

    #[test]
    fn unit_sample_advances_rng_word_pos() {
        let s = LhsSampler::classic(3);
        let mut rng = fresh_rng(0);
        let initial_word_pos = rng.word_pos;
        let _m = s.unit_sample(32, &mut rng);
        assert!(
            rng.word_pos > initial_word_pos,
            "word_pos {} did not advance from {}",
            rng.word_pos,
            initial_word_pos
        );
    }

    #[test]
    fn unit_sample_zero_n_does_not_consume_rng() {
        // n=0: no permutation draws, no offset draws — but we still
        // snapshot the RNG. Snapshot of an unmodified RNG preserves
        // word_pos.
        let s = LhsSampler::classic(3);
        let mut rng = fresh_rng(0);
        let initial_word_pos = rng.word_pos;
        let _m = s.unit_sample(0, &mut rng);
        assert_eq!(rng.word_pos, initial_word_pos);
    }

    #[test]
    fn unit_sample_centered_advances_rng_only_for_permutation() {
        // Centered uses no per-cell offset draws — only the
        // Fisher-Yates draws (n-1 per dim).
        let s_classic = LhsSampler::classic(2);
        let s_centered = LhsSampler::centered(2);
        let n = 32;

        let mut r_classic = fresh_rng(0);
        let mut r_centered = fresh_rng(0);
        let _m1 = s_classic.unit_sample(n, &mut r_classic);
        let _m2 = s_centered.unit_sample(n, &mut r_centered);
        // Classic consumes more bytes (offset draws), so its word_pos
        // is strictly larger.
        assert!(r_classic.word_pos > r_centered.word_pos);
    }

    // ── config_hash ──────────────────────────────────────────────────

    #[test]
    fn config_hash_is_stable_across_calls() {
        let s = LhsSampler::classic(5);
        let h1 = s.config_hash();
        let h2 = s.config_hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn config_hash_distinct_for_different_dims() {
        let s1 = LhsSampler::classic(5);
        let s2 = LhsSampler::classic(6);
        assert_ne!(s1.config_hash(), s2.config_hash());
    }

    #[test]
    fn config_hash_distinct_for_different_kinds() {
        let s1 = LhsSampler::classic(5);
        let s2 = LhsSampler::centered(5);
        assert_ne!(s1.config_hash(), s2.config_hash());
    }

    #[test]
    fn config_hash_returns_thirty_two_bytes() {
        let s = LhsSampler::classic(3);
        assert_eq!(s.config_hash().len(), 32);
    }

    // ── Sampler trait impls ────────────────────────────────────────

    #[test]
    fn dim_returns_constructor_dim() {
        assert_eq!(LhsSampler::classic(7).dim(), 7);
        assert_eq!(LhsSampler::centered(11).dim(), 11);
    }

    // ── Edge cases ──────────────────────────────────────────────────

    #[test]
    fn unit_sample_n_one_returns_single_row() {
        let s = LhsSampler::classic(3);
        let mut rng = fresh_rng(0);
        let m = s.unit_sample(1, &mut rng);
        assert_eq!(m.shape(), &[1, 3]);
        // With n=1, perm is identity ([0]); each cell is [0, 1).
        for j in 0..3 {
            assert!(m[[0, j]] >= 0.0 && m[[0, j]] < 1.0);
        }
    }

    #[test]
    fn unit_sample_n_one_centered_returns_half() {
        let s = LhsSampler::centered(2);
        let mut rng = fresh_rng(0);
        let m = s.unit_sample(1, &mut rng);
        // Centered with n=1, perm[0]=0: value = (0 + 0.5)/1 = 0.5.
        assert_eq!(m[[0, 0]], 0.5);
        assert_eq!(m[[0, 1]], 0.5);
    }

    #[test]
    fn unit_sample_dim_one_handles_single_column() {
        let s = LhsSampler::classic(1);
        let mut rng = fresh_rng(0);
        let m = s.unit_sample(16, &mut rng);
        assert_eq!(m.shape(), &[16, 1]);
        // Stratification holds.
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let cell_index = |x: f64| (x * 16.0).floor() as usize;
        let mut cells: Vec<usize> = (0..16).map(|i| cell_index(m[[i, 0]])).collect();
        cells.sort_unstable();
        assert_eq!(cells, (0..16).collect::<Vec<_>>());
    }

    #[test]
    fn unit_sample_large_n_performs() {
        // Sanity check N=1024 doesn't choke.
        let s = LhsSampler::classic(8);
        let mut rng = fresh_rng(0);
        let m = s.unit_sample(1024, &mut rng);
        assert_eq!(m.shape(), &[1024, 8]);
    }

    // ── Independence across dimensions ──────────────────────────────

    #[test]
    fn classic_columns_are_independent_under_distinct_dims() {
        // Two independent dims should give different permutations
        // (probability of identical perms ≪ 1 for n=64).
        let s = LhsSampler::classic(2);
        let mut rng = fresh_rng(0);
        let m = s.unit_sample(64, &mut rng);
        let col0: Vec<f64> = (0..64).map(|i| m[[i, 0]]).collect();
        let col1: Vec<f64> = (0..64).map(|i| m[[i, 1]]).collect();
        assert_ne!(col0, col1, "columns should be independently permuted");
    }
}
