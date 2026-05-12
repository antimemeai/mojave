//! Sobol' low-discrepancy quasi-Monte-Carlo sampler — vendored
//! Joe-Kuo direction numbers + Antonov-Saleev gray-code recursion.
//!
//! # Mathematical background
//!
//! The Sobol' sequence is a deterministic quasi-random sequence
//! parameterized by a primitive polynomial and a set of "direction
//! numbers" per dimension. For `d`-dimensional Sobol', dimension 1
//! uses the trivial direction numbers `v_i = 1 << (RES - i)`
//! (filling the unit interval bit-by-bit); dimensions ≥ 2 use the
//! tabulated Joe-Kuo `(s, a, m_1..m_s)` triples per
//! `data/new-joe-kuo-6.{100,1000}` to generate `v_i` via the
//! Joe-Kuo recurrence:
//!
//! ```text
//! v_i = m_i << (RES - i)                                  for i ≤ s
//! v_i = (v_{i-s} >> s) XOR v_{i-s} XOR Σ_k a_k · v_{i-k}    for i > s
//! ```
//!
//! Generation uses the Antonov-Saleev gray-code recursion: with
//! `c(k) = number of trailing zeros in k`,
//!
//! ```text
//! x_0 = 0
//! x_k = x_{k-1} XOR v_{c(k) + 1}
//! ```
//!
//! Each subsequent point flips at most one direction-number's worth
//! of bits — `O(1)` per draw. f64 normalization is `x_int as f64 / 2^32`.
//!
//! # Vendoring
//!
//! `data/new-joe-kuo-6.100` and `data/new-joe-kuo-6.1000` are vendored
//! verbatim from <https://web.maths.unsw.edu.au/~fkuo/sobol/>.
//! BSD-3-Clause attribution in `data/LICENSE.joe-kuo`. Per
//! `decisions/2026-04-29-saltelli-sobol-sampler.md`.
//!
//! # Skip-first
//!
//! Sobol' point 0 is exactly the origin `(0, 0, …, 0)`. Owen 2020
//! ("On dropping the first Sobol' point") recommends skipping it
//! because the all-zeros origin biases convergence at small N.
//! Default `skip_first = true`. Set `false` for `SALib`-compat
//! comparisons (older `SALib` versions include the origin).

use ndarray::Array2;
use salib_core::RngState;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::sampler::Sampler;

/// Parsed Joe-Kuo direction-number row for a single dimension.
/// Generated from the vendored `data/new-joe-kuo-6.*` files at
/// build time (see `build.rs`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct JoeKuoD6Dim {
    /// Dimension index (starts at 2; dimension 1 is the trivial case).
    #[allow(dead_code)]
    pub d: u16,
    /// Encoded coefficient bits of the primitive polynomial.
    pub a: u32,
    /// Initial direction-number values `m_1..m_s`.
    pub m: &'static [u32],
}

include!(concat!(env!("OUT_DIR"), "/joe_kuo_d6_data.rs"));

/// 32-bit resolution for the integer state. `2^32` distinct points
/// per dim per Sobol' sequence; sufficient for any realistic SA
/// workload (typical N ≤ 2^20 = ~1M).
const RES: u32 = 32;

/// Selects which vendored Joe-Kuo dimension table to use. Closed
/// enum, `#[non_exhaustive]`. Future variants may include `Extended`
/// (21,201 dims) when needed; the data file is already documented in
/// the upstream Joe-Kuo distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[serde(tag = "kind")]
pub enum SobolDimSet {
    /// 100 dimensions; 4.5 KB vendored.
    Minimal,
    /// 1000 dimensions; 62 KB vendored.
    Standard,
}

impl SobolDimSet {
    fn dims(self) -> &'static [JoeKuoD6Dim] {
        match self {
            Self::Minimal => JOE_KUO_D6_MINIMAL_DATA,
            Self::Standard => JOE_KUO_D6_STANDARD_DATA,
        }
    }

    fn max_dims(self) -> usize {
        // The first dimension is the trivial case (not in the table);
        // the data covers dims 2..=N+1.
        self.dims().len() + 1
    }
}

/// Sobol' quasi-random low-discrepancy sampler.
///
/// `#[non_exhaustive]` — future fields (`scrambling: SobolScrambling`
/// for Owen-hash scrambling per Burley 2020) land non-breaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SobolSampler {
    /// Output column count.
    pub dim: usize,
    /// Which vendored Joe-Kuo dim table to use.
    pub dim_set: SobolDimSet,
    /// Drop the all-zeros origin point. Owen 2020 default = `true`;
    /// set `false` for older `SALib`-compat behavior.
    pub skip_first: bool,
}

impl SobolSampler {
    /// Standard Sobol' (1000-dim Joe-Kuo table) with skip-first
    /// enabled per Owen 2020.
    ///
    /// # Panics
    ///
    /// On `dim == 0` or `dim > 1000`.
    #[must_use]
    pub fn standard(dim: usize) -> Self {
        Self::with(dim, SobolDimSet::Standard, true)
    }

    /// Minimal Sobol' (100-dim Joe-Kuo table) with skip-first enabled.
    ///
    /// # Panics
    ///
    /// On `dim == 0` or `dim > 100`.
    #[must_use]
    pub fn minimal(dim: usize) -> Self {
        Self::with(dim, SobolDimSet::Minimal, true)
    }

    /// Construct with explicit settings.
    ///
    /// # Panics
    ///
    /// On `dim == 0` or `dim > dim_set.max_dims()`.
    #[must_use]
    pub fn with(dim: usize, dim_set: SobolDimSet, skip_first: bool) -> Self {
        assert!(dim > 0, "Sobol: dim must be ≥ 1");
        let max = dim_set.max_dims();
        assert!(dim <= max, "Sobol: dim {dim} exceeds {dim_set:?} max {max}");
        Self {
            dim,
            dim_set,
            skip_first,
        }
    }

    /// Toggle skip-first.
    #[must_use]
    pub fn with_skip_first(mut self, skip_first: bool) -> Self {
        self.skip_first = skip_first;
        self
    }

    /// Compute the direction-number vector `v[1..=RES]` for one
    /// dimension. For dimension index `j` (0-indexed; `j=0` is the
    /// trivial dim 1), the resulting `v[i]` are pre-shifted to the
    /// high bits so the integer state representation directly
    /// produces the f64 fraction via `x as f64 / 2^32`.
    // Joe-Kuo direction-number recurrence. The body is genuinely
    // index-based (it's the published recurrence), and `i ≤ RES = 32`
    // fits in `u32` by construction — bypass the iterator-pattern
    // and cast-truncation lints with scope-limited allows.
    #[allow(clippy::needless_range_loop, clippy::cast_possible_truncation)]
    fn direction_numbers(&self, j: usize) -> [u32; RES as usize + 1] {
        let mut v = [0u32; RES as usize + 1];
        if j == 0 {
            // Dimension 1: m_i = 1 for all i. v[i] = 1 << (RES - i).
            for i in 1..=(RES as usize) {
                v[i] = 1u32 << (RES - i as u32);
            }
            return v;
        }
        // j >= 1: index into the Joe-Kuo table at position j - 1.
        let table = self.dim_set.dims();
        let dim_data = &table[j - 1];
        let s = dim_data.m.len();
        let a = dim_data.a;

        // v[i] = m[i-1] << (RES - i) for i in 1..=s.
        for i in 1..=s {
            v[i] = dim_data.m[i - 1] << (RES - i as u32);
        }
        // v[i] for i > s: Joe-Kuo recurrence.
        // v_i = (v_{i-s} >> s) XOR v_{i-s} XOR Σ_{k=1..s-1} a_k · v_{i-k}
        // where a_k is bit (s - 1 - k) of `a` (Joe-Kuo encoding;
        // see Joe-Kuo 2008 paper).
        for i in (s + 1)..=(RES as usize) {
            let mut value = (v[i - s] >> s) ^ v[i - s];
            for k in 1..s {
                let a_k = (a >> (s - 1 - k)) & 1;
                if a_k != 0 {
                    value ^= v[i - k];
                }
            }
            v[i] = value;
        }
        v
    }
}

impl Sampler for SobolSampler {
    fn dim(&self) -> usize {
        self.dim
    }

    fn unit_sample(&self, n: usize, rng: &mut RngState) -> Array2<f64> {
        // Unscrambled Sobol' is a deterministic QMC sequence — no
        // RNG draws. The `&mut RngState` is part of the trait shape
        // and will be consumed when Owen-hash scrambling lands. For
        // now we snapshot it untouched so callers see the same
        // word_pos in/out.
        let chacha = rng.clone().into_chacha();

        let mut out = Array2::<f64>::zeros((n, self.dim));
        if n == 0 || self.dim == 0 {
            *rng = RngState::snapshot(&chacha, rng);
            return out;
        }

        // Per-dimension precomputed direction numbers.
        let dir_nums: Vec<[u32; RES as usize + 1]> =
            (0..self.dim).map(|j| self.direction_numbers(j)).collect();

        // Antonov-Saleev gray-code, per dim independently. The
        // `state[j]` carries the integer XOR accumulator; at each
        // point k we XOR in the direction number at `c(k) + 1` where
        // c(k) = trailing zeros of k.
        let mut state = vec![0u32; self.dim];

        // Production index — caller-visible point counter. We emit
        // points with `gen_idx = 0..n` (or 1..=n if `skip_first`)
        // and the generator step advances `state` per gen_idx.
        // Point at output row `i` corresponds to gen_idx = i (or
        // i + 1 with skip_first).
        let start_gen = usize::from(self.skip_first);
        for row in 0..n {
            let gen_idx = start_gen + row;
            if gen_idx == 0 {
                // gen_idx = 0 is the all-zeros origin. State is
                // already 0; just emit zeros.
                for j in 0..self.dim {
                    out[[row, j]] = 0.0;
                }
                continue;
            }
            // c = trailing zeros of gen_idx.
            let c = gen_idx.trailing_zeros() as usize;
            // XOR direction number v[c+1] into each dim's state.
            for j in 0..self.dim {
                state[j] ^= dir_nums[j][c + 1];
                #[allow(clippy::cast_precision_loss)]
                let frac = f64::from(state[j]) / (f64::from(u32::MAX) + 1.0);
                out[[row, j]] = frac;
            }
        }

        *rng = RngState::snapshot(&chacha, rng);
        out
    }

    fn config_hash(&self) -> [u8; 32] {
        #[allow(clippy::expect_used)]
        let bytes = serde_json::to_vec(self)
            .expect("serializing SobolSampler to JSON cannot fail (all plain data)");
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        hasher.finalize().into()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn fresh_rng() -> RngState {
        RngState::from_seed([0; 32])
    }

    // ── Constructors ─────────────────────────────────────────────────

    #[test]
    fn standard_uses_standard_dim_set() {
        let s = SobolSampler::standard(5);
        assert_eq!(s.dim, 5);
        assert_eq!(s.dim_set, SobolDimSet::Standard);
        assert!(s.skip_first);
    }

    #[test]
    fn minimal_uses_minimal_dim_set() {
        let s = SobolSampler::minimal(5);
        assert_eq!(s.dim_set, SobolDimSet::Minimal);
    }

    #[test]
    fn with_skip_first_toggles() {
        let s = SobolSampler::standard(3).with_skip_first(false);
        assert!(!s.skip_first);
    }

    #[test]
    #[should_panic(expected = "dim must be ≥ 1")]
    fn zero_dim_panics() {
        let _ = SobolSampler::standard(0);
    }

    #[test]
    #[should_panic(expected = "exceeds Minimal max")]
    fn dim_above_minimal_max_panics() {
        let _ = SobolSampler::minimal(101);
    }

    #[test]
    #[should_panic(expected = "exceeds Standard max")]
    fn dim_above_standard_max_panics() {
        let _ = SobolSampler::standard(1001);
    }

    #[test]
    fn standard_max_dim_succeeds() {
        let s = SobolSampler::standard(1000);
        assert_eq!(s.dim, 1000);
    }

    // ── Output shape ─────────────────────────────────────────────────

    #[test]
    fn output_shape_matches_n_by_dim() {
        let s = SobolSampler::standard(4);
        let mut rng = fresh_rng();
        let m = s.unit_sample(64, &mut rng);
        assert_eq!(m.shape(), &[64, 4]);
    }

    #[test]
    fn zero_n_returns_empty_matrix() {
        let s = SobolSampler::standard(3);
        let mut rng = fresh_rng();
        let m = s.unit_sample(0, &mut rng);
        assert_eq!(m.shape(), &[0, 3]);
    }

    // ── Output range ─────────────────────────────────────────────────

    #[test]
    fn output_in_zero_one_open_with_skip_first() {
        let s = SobolSampler::standard(5).with_skip_first(true);
        let mut rng = fresh_rng();
        let m = s.unit_sample(256, &mut rng);
        for &v in &m {
            assert!((0.0..1.0).contains(&v), "out-of-range {v}");
        }
    }

    #[test]
    fn output_includes_origin_without_skip_first() {
        // With skip_first=false, row 0 is the all-zeros origin.
        let s = SobolSampler::standard(3).with_skip_first(false);
        let mut rng = fresh_rng();
        let m = s.unit_sample(8, &mut rng);
        for j in 0..3 {
            assert_eq!(m[[0, j]], 0.0, "row 0 dim {j} should be 0");
        }
    }

    // ── Known Sobol' values for dimension 1 ─────────────────────────

    #[test]
    fn dim_one_first_eight_match_canonical_sequence() {
        // Unscrambled Sobol' dim 1 (the trivial dim with all m_i=1)
        // produces:
        //   point 0: 0.0
        //   point 1: 0.5
        //   point 2: 0.75
        //   point 3: 0.25
        //   point 4: 0.375
        //   point 5: 0.875
        //   point 6: 0.625
        //   point 7: 0.125
        // Standard textbook values (e.g. Saltelli Primer 2008 §4).
        let s = SobolSampler::standard(1).with_skip_first(false);
        let mut rng = fresh_rng();
        let m = s.unit_sample(8, &mut rng);
        let expected = [0.0, 0.5, 0.75, 0.25, 0.375, 0.875, 0.625, 0.125];
        for (i, &want) in expected.iter().enumerate() {
            assert_eq!(
                m[[i, 0]],
                want,
                "dim 1 point {i}: got {}, want {want}",
                m[[i, 0]]
            );
        }
    }

    #[test]
    fn dim_one_skip_first_drops_origin() {
        // With skip_first=true, the origin (point 0 = 0.0) is
        // dropped; the sequence starts at point 1 = 0.5.
        let s = SobolSampler::standard(1).with_skip_first(true);
        let mut rng = fresh_rng();
        let m = s.unit_sample(7, &mut rng);
        let expected = [0.5, 0.75, 0.25, 0.375, 0.875, 0.625, 0.125];
        for (i, &want) in expected.iter().enumerate() {
            assert_eq!(m[[i, 0]], want);
        }
    }

    // ── Determinism ─────────────────────────────────────────────────

    #[test]
    fn same_config_same_rngstate_produces_identical_output() {
        let s = SobolSampler::standard(5);
        let mut r1 = fresh_rng();
        let mut r2 = fresh_rng();
        let m1 = s.unit_sample(128, &mut r1);
        let m2 = s.unit_sample(128, &mut r2);
        assert_eq!(m1, m2);
    }

    #[test]
    fn unscrambled_does_not_consume_rng_word_pos() {
        let s = SobolSampler::standard(3);
        let mut rng = fresh_rng();
        let initial = rng.word_pos;
        let _m = s.unit_sample(64, &mut rng);
        // Unscrambled Sobol' makes no RNG draws. word_pos unchanged.
        assert_eq!(rng.word_pos, initial);
    }

    #[test]
    fn output_independent_of_rngstate_under_unscrambled() {
        // Unscrambled Sobol' is a deterministic QMC sequence — the
        // RngState argument is unused. Different streams produce
        // identical output.
        let s = SobolSampler::standard(4);
        let mut r1 = RngState::from_parts([0; 32], 1, 0);
        let mut r2 = RngState::from_parts([0; 32], 999, 0);
        let m1 = s.unit_sample(64, &mut r1);
        let m2 = s.unit_sample(64, &mut r2);
        assert_eq!(m1, m2);
    }

    // ── Stratification properties ───────────────────────────────────

    #[test]
    fn first_n_points_are_well_distributed_in_unit_interval() {
        // For N = 2^k with k ≤ RES, the first N Sobol' points
        // partition [0, 1) into N exact bins per dimension. Each bin
        // contains exactly one point.
        let n = 1024usize;
        let s = SobolSampler::standard(3).with_skip_first(false);
        let mut rng = fresh_rng();
        let m = s.unit_sample(n, &mut rng);
        #[allow(
            clippy::cast_sign_loss,
            clippy::cast_possible_truncation,
            clippy::cast_precision_loss
        )]
        let bin = |v: f64| (v * n as f64).floor() as usize;
        for j in 0..3 {
            let mut bins: Vec<usize> = (0..n).map(|i| bin(m[[i, j]])).collect();
            bins.sort_unstable();
            let expected: Vec<usize> = (0..n).collect();
            assert_eq!(bins, expected, "dim {j} stratification");
        }
    }

    // ── config_hash ─────────────────────────────────────────────────

    #[test]
    fn config_hash_stable_across_calls() {
        let s = SobolSampler::standard(5);
        assert_eq!(s.config_hash(), s.config_hash());
    }

    #[test]
    fn config_hash_differs_for_different_dims() {
        assert_ne!(
            SobolSampler::standard(5).config_hash(),
            SobolSampler::standard(6).config_hash()
        );
    }

    #[test]
    fn config_hash_differs_for_different_dim_sets() {
        // Same dim but different dim_set should hash differently.
        // (Both Minimal and Standard support dim=5.)
        assert_ne!(
            SobolSampler::standard(5).config_hash(),
            SobolSampler::minimal(5).config_hash()
        );
    }

    #[test]
    fn config_hash_differs_for_different_skip_first() {
        let s1 = SobolSampler::standard(5).with_skip_first(true);
        let s2 = SobolSampler::standard(5).with_skip_first(false);
        assert_ne!(s1.config_hash(), s2.config_hash());
    }

    #[test]
    fn config_hash_returns_thirty_two_bytes() {
        assert_eq!(SobolSampler::standard(3).config_hash().len(), 32);
    }

    // ── Sampler trait ──────────────────────────────────────────────

    #[test]
    fn dim_returns_constructor_dim() {
        assert_eq!(SobolSampler::standard(7).dim(), 7);
    }

    // ── Dim 2 cross-check ──────────────────────────────────────────

    #[test]
    fn dim_two_first_eight_match_canonical_values() {
        // Joe-Kuo dim 2 (table row 1: s=1, a=0, m=[1]).
        // v_1 = 1 << 31 = 0.5
        // v_2 = (v_1 >> 1) XOR v_1 = 0x40000000 XOR 0x80000000 = 0xC0000000 = 0.75
        // v_3 = (v_2 >> 1) XOR v_2 = 0x60000000 XOR 0xC0000000 = 0xA0000000 = 0.625
        // ...
        // Sequence with skip_first=false:
        //   point 0: x = 0.0
        //   point 1: x = 0 XOR v_1 = 0.5
        //   point 2: x = 0.5 XOR v_2 = 0.5 XOR 0.75 = bit 0x80000000 ^ 0xC0000000 = 0x40000000 = 0.25
        //   point 3: x = 0.25 XOR v_1 = 0.25 XOR 0.5 = bit 0x40000000 ^ 0x80000000 = 0xC0000000 = 0.75
        //   point 4: x = 0.75 XOR v_3 = 0xC0000000 ^ 0xA0000000 = 0x60000000 = 0.375
        //   point 5: x = 0.375 XOR v_1 = 0x60000000 ^ 0x80000000 = 0xE0000000 = 0.875
        //   point 6: x = 0.875 XOR v_2 = 0xE0000000 ^ 0xC0000000 = 0x20000000 = 0.125
        //   point 7: x = 0.125 XOR v_1 = 0x20000000 ^ 0x80000000 = 0xA0000000 = 0.625
        let s = SobolSampler::standard(2).with_skip_first(false);
        let mut rng = fresh_rng();
        let m = s.unit_sample(8, &mut rng);
        // Verify dim 1 first (already covered above in single-dim test).
        // For dim 2:
        let expected_dim2 = [0.0, 0.5, 0.25, 0.75, 0.375, 0.875, 0.125, 0.625];
        for (i, &want) in expected_dim2.iter().enumerate() {
            assert_eq!(
                m[[i, 1]],
                want,
                "dim 2 point {i}: got {}, want {want}",
                m[[i, 1]]
            );
        }
    }

    // ── Edge cases ──────────────────────────────────────────────────

    #[test]
    fn dim_one_alone_produces_one_column() {
        let s = SobolSampler::standard(1);
        let mut rng = fresh_rng();
        let m = s.unit_sample(16, &mut rng);
        assert_eq!(m.shape(), &[16, 1]);
    }

    #[test]
    fn large_n_within_resolution_bounds() {
        // N = 2^16: well within RES=32.
        let s = SobolSampler::standard(3);
        let mut rng = fresh_rng();
        let m = s.unit_sample(65536, &mut rng);
        assert_eq!(m.shape(), &[65536, 3]);
        for &v in &m {
            assert!((0.0..1.0).contains(&v));
        }
    }

    // ── Direction-number internals (private helper coverage) ───────

    #[test]
    #[allow(clippy::needless_range_loop)]
    fn direction_numbers_dim_one_are_powers_of_two() {
        let s = SobolSampler::standard(1);
        let v = s.direction_numbers(0);
        // v[i] = 1 << (32 - i) for i in 1..=32. Index-based loop is
        // the natural form for "verify v at every position."
        for i in 1..=32 {
            assert_eq!(v[i], 1u32 << (32 - i));
        }
    }

    #[test]
    fn direction_numbers_dim_two_match_joe_kuo_recurrence() {
        // dim 2: s=1, a=0, m=[1]. v[1] = 1 << 31. For i > 1:
        // v_i = (v_{i-1} >> 1) XOR v_{i-1}.
        let s = SobolSampler::standard(2);
        let v = s.direction_numbers(1);
        assert_eq!(v[1], 1u32 << 31);
        #[allow(clippy::needless_range_loop)]
        for i in 2..=32 {
            let want = (v[i - 1] >> 1) ^ v[i - 1];
            assert_eq!(v[i], want, "v[{i}]");
        }
    }
}
