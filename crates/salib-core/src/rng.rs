//! `RngState` — multi-stream `ChaCha20` with deterministic salt-derived
//! forking. The replay-determinism foundation for every saltelli
//! sampler and estimator.
//!
//! # The state
//!
//! Four fields:
//! - `algorithm`: closed enum, `ChaCha20` only today (future
//!   `Pcg64` / `Xoshiro256pp` behind a `fast-rng` feature).
//! - `seed: [u8; 32]`: 256-bit seed.
//! - `stream: u64`: 2⁶⁴ independent streams per seed (`ChaCha20Rng::set_stream`).
//! - `word_pos: u128`: position within the stream
//!   (`ChaCha20Rng::get_word_pos` / `set_word_pos`); enables mid-flight
//!   snapshot + resumption.
//!
//! Recording all four into the audit envelope's `context` payload (per
//! `decisions/2026-04-28-saltelli-ledger-composition.md`) lets a
//! verifier reconstruct any saltelli campaign's RNG stream from
//! scratch.
//!
//! # Forking
//!
//! [`RngState::fork`] derives a child stream from a salt:
//!
//! ```text
//! child.stream   = parent.stream XOR u64::from_le_bytes(SHA-256(parent.stream || salt)[..8])
//! child.word_pos = 0
//! child.seed     = parent.seed
//! ```
//!
//! Pure function of `(parent.stream, salt)` and the parent's seed —
//! `parent.fork(b"block-7")` always yields the same child regardless
//! of process, machine, rayon thread count, or wall-clock time. This
//! is what makes parallel sampling deterministic: rayon workers fork
//! by block index, the resulting per-block streams are stable across
//! runs, and the sample matrix is bit-identical regardless of how
//! work was distributed.
//!
//! Why XOR-with-mix instead of replace-with-mix: the child's stream
//! depends on *both* `parent.stream` and `salt`, not just `salt`. Two
//! distinct parents that happen to fork with the same salt produce
//! distinct children — important for nested forking patterns where
//! the same salt vocabulary recurs at multiple levels.
//!
//! Why SHA-256 and not Blake3: SHA-256 is already in the workspace
//! dep graph (`workspace-audit-seal::chain`), FIPS 180-4 reference
//! protocol, and indistinguishable from Blake3 at this small input
//! size (≪64 bytes). Blake3 lands in a later PR when saltelli has
//! a tree-mode-parallel-hashing use case (e.g. content-addressing a
//! 10⁶-row sample matrix). See
//! `decisions/2026-04-28-saltelli-rng-determinism.md` § "Rationale"
//! for the full justification.

use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The set of RNG algorithms saltelli supports. Closed enum +
/// `#[non_exhaustive]`; future variants land non-breaking via the
/// same protocol that `workspace_core::Action` uses.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RngAlgorithm {
    /// `ChaCha20` (RFC 7539). 256-bit seed, 2⁶⁴ streams per seed,
    /// 2¹²⁸ word-positions per stream. The default.
    #[default]
    ChaCha20,
}

/// The serializable RNG state. The single source of truth for any
/// saltelli campaign's randomness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RngState {
    pub algorithm: RngAlgorithm,
    pub seed: [u8; 32],
    pub stream: u64,
    pub word_pos: u128,
}

impl RngState {
    /// Construct a fresh state from a 32-byte seed. `ChaCha20`, stream
    /// 0, `word_pos` 0.
    #[must_use]
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            algorithm: RngAlgorithm::ChaCha20,
            seed,
            stream: 0,
            word_pos: 0,
        }
    }

    /// Construct from explicit fields. Only path into a non-default
    /// `(stream, word_pos)` outside of `fork` and `snapshot`.
    #[must_use]
    pub fn from_parts(seed: [u8; 32], stream: u64, word_pos: u128) -> Self {
        Self {
            algorithm: RngAlgorithm::ChaCha20,
            seed,
            stream,
            word_pos,
        }
    }

    /// Derive a child state from a salt. The child shares the
    /// parent's seed; the child stream is
    /// `parent.stream XOR u64::from_le_bytes(SHA-256(parent.stream || salt)[..8])`.
    /// The child's `word_pos` is reset to 0.
    ///
    /// Pure function of `(parent.stream, parent.seed, salt)`. Same
    /// inputs always produce equal outputs — the field-equality TCK
    /// scenario in
    /// `tck/saltelli/rng-determinism/features/multi_stream_chacha.feature`
    /// pins this.
    #[must_use]
    pub fn fork(&self, salt: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(self.stream.to_le_bytes());
        hasher.update(salt);
        let digest = hasher.finalize();
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&digest[..8]);
        let mix = u64::from_le_bytes(buf);
        Self {
            algorithm: self.algorithm,
            seed: self.seed,
            stream: self.stream ^ mix,
            word_pos: 0,
        }
    }

    /// Construct a `ChaCha20Rng` initialized to this state's
    /// `(seed, stream, word_pos)`. The handed-out RNG is detached
    /// from this `RngState` — mutations to the returned RNG do not
    /// update `self`.
    #[must_use]
    pub fn into_chacha(self) -> ChaCha20Rng {
        let mut rng = ChaCha20Rng::from_seed(self.seed);
        rng.set_stream(self.stream);
        rng.set_word_pos(self.word_pos);
        rng
    }

    /// Snapshot a `ChaCha20Rng`'s current `(stream, word_pos)` back
    /// into a serializable `RngState`, preserving `algorithm` and
    /// `seed` from `parent`. Used to record mid-flight RNG state
    /// for resumption / audit.
    ///
    /// `parent` carries the `seed` because `ChaCha20Rng` does not
    /// expose its seed once constructed; the caller is responsible
    /// for handing a `parent` whose seed matches the RNG's actual
    /// seed (typically the same `RngState` that produced the RNG via
    /// `into_chacha`).
    #[must_use]
    pub fn snapshot(rng: &ChaCha20Rng, parent: &RngState) -> Self {
        Self {
            algorithm: parent.algorithm,
            seed: parent.seed,
            stream: rng.get_stream(),
            word_pos: rng.get_word_pos(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::rand_core::RngCore;

    fn seed_bytes(b: u8) -> [u8; 32] {
        [b; 32]
    }

    fn draw_n(rng: &mut ChaCha20Rng, n: usize) -> Vec<u8> {
        let mut buf = vec![0u8; n];
        rng.fill_bytes(&mut buf);
        buf
    }

    #[test]
    fn from_seed_defaults_stream_and_word_pos_to_zero() {
        let s = RngState::from_seed(seed_bytes(0x42));
        assert_eq!(s.stream, 0);
        assert_eq!(s.word_pos, 0);
        assert_eq!(s.algorithm, RngAlgorithm::ChaCha20);
        assert_eq!(s.seed, seed_bytes(0x42));
    }

    #[test]
    fn same_state_produces_identical_bytes() {
        let s = RngState::from_parts(seed_bytes(0x42), 7, 0);
        let mut a = s.clone().into_chacha();
        let mut b = s.into_chacha();
        assert_eq!(draw_n(&mut a, 1024), draw_n(&mut b, 1024));
    }

    #[test]
    fn fork_with_same_salt_is_deterministic() {
        let parent = RngState::from_parts(seed_bytes(0x42), 0, 0);
        let c1 = parent.fork(b"block-0");
        let c2 = parent.fork(b"block-0");
        assert_eq!(c1, c2);
    }

    #[test]
    fn fork_with_distinct_salts_produces_distinct_streams() {
        let parent = RngState::from_parts(seed_bytes(0x42), 0, 0);
        let c1 = parent.fork(b"block-0");
        let c2 = parent.fork(b"block-1");
        assert_ne!(c1.stream, c2.stream);
        let mut r1 = c1.into_chacha();
        let mut r2 = c2.into_chacha();
        assert_ne!(draw_n(&mut r1, 1024), draw_n(&mut r2, 1024));
    }

    #[test]
    fn fork_resets_child_word_pos() {
        let parent = RngState::from_parts(seed_bytes(0x42), 100, 999);
        let child = parent.fork(b"any");
        assert_eq!(child.word_pos, 0);
        assert_eq!(child.seed, parent.seed);
    }

    #[test]
    fn fork_xors_with_mix_so_distinct_parents_produce_distinct_children_under_same_salt() {
        let p1 = RngState::from_parts(seed_bytes(0x42), 1, 0);
        let p2 = RngState::from_parts(seed_bytes(0x42), 2, 0);
        let c1 = p1.fork(b"same-salt");
        let c2 = p2.fork(b"same-salt");
        assert_ne!(c1.stream, c2.stream);
    }

    #[test]
    fn snapshot_round_trips_word_pos_and_stream() {
        let s = RngState::from_parts(seed_bytes(0x42), 0, 0);
        let mut r = s.clone().into_chacha();
        let _ = draw_n(&mut r, 8192);
        let snap = RngState::snapshot(&r, &s);
        // The snapshot's word_pos reflects the post-draw position.
        assert_eq!(snap.word_pos, r.get_word_pos());
        // Drawing 1024 more from the original vs from a fresh chacha
        // initialized from the snapshot must agree.
        let mut original_continued = r;
        let mut resumed = snap.into_chacha();
        assert_eq!(
            draw_n(&mut original_continued, 1024),
            draw_n(&mut resumed, 1024)
        );
    }

    #[test]
    fn rngstate_serde_round_trip() {
        let s = RngState::from_parts(seed_bytes(0x42), 12345, 67890);
        let json = serde_json::to_string(&s).expect("serialize");
        let back: RngState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, s);
    }

    // ── RngAlgorithm sanity ───────────────────────────────────────────

    #[test]
    fn rng_algorithm_default_is_chacha20() {
        assert_eq!(RngAlgorithm::default(), RngAlgorithm::ChaCha20);
    }

    #[test]
    fn rng_algorithm_serde_round_trip() {
        let json = serde_json::to_string(&RngAlgorithm::ChaCha20).expect("serialize");
        let back: RngAlgorithm = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, RngAlgorithm::ChaCha20);
    }

    // ── RngState construction equivalences ────────────────────────────

    #[test]
    fn from_seed_equals_from_parts_with_zero_stream_and_zero_word_pos() {
        let a = RngState::from_seed(seed_bytes(0x99));
        let b = RngState::from_parts(seed_bytes(0x99), 0, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn from_parts_preserves_all_four_fields() {
        let s = RngState::from_parts([0xab; 32], 9_999_999, 12_345_678_901_234_567_890_u128);
        assert_eq!(s.algorithm, RngAlgorithm::ChaCha20);
        assert_eq!(s.seed, [0xab; 32]);
        assert_eq!(s.stream, 9_999_999);
        assert_eq!(s.word_pos, 12_345_678_901_234_567_890_u128);
    }

    // ── into_chacha / get_stream + get_word_pos round-trips ─────────

    #[test]
    fn into_chacha_initializes_stream_and_word_pos_correctly() {
        let s = RngState::from_parts(seed_bytes(0x42), 1234, 5678);
        let rng = s.clone().into_chacha();
        assert_eq!(rng.get_stream(), s.stream);
        assert_eq!(rng.get_word_pos(), s.word_pos);
    }

    #[test]
    fn into_chacha_clones_so_state_is_unchanged_by_draws() {
        let s = RngState::from_parts(seed_bytes(0x42), 0, 0);
        let mut r = s.clone().into_chacha();
        let _ = draw_n(&mut r, 4096);
        // s itself is untouched (s.word_pos == 0 still); the consumed
        // RNG is detached.
        assert_eq!(s.word_pos, 0);
    }

    // ── Fork: deeper properties ───────────────────────────────────────

    #[test]
    fn fork_is_pure_under_parent_stream_and_seed() {
        // Two parents with identical (stream, seed) and any word_pos
        // value must produce the same fork — fork ignores parent.word_pos.
        let p1 = RngState::from_parts(seed_bytes(0x42), 7, 0);
        let p2 = RngState::from_parts(seed_bytes(0x42), 7, 999);
        assert_eq!(p1.fork(b"some-salt"), p2.fork(b"some-salt"));
    }

    #[test]
    fn fork_keys_off_parent_stream_not_just_seed() {
        // Two parents with same seed but different streams must
        // produce different forks even with the same salt.
        let p1 = RngState::from_parts(seed_bytes(0x42), 0, 0);
        let p2 = RngState::from_parts(seed_bytes(0x42), 1, 0);
        assert_ne!(p1.fork(b"x"), p2.fork(b"x"));
    }

    #[test]
    fn fork_keys_off_seed_not_just_stream() {
        // Two parents with same stream but different seeds must
        // produce different forks even with the same salt — actually,
        // fork derivation today is `parent.stream ^ SHA256(parent.stream || salt)[..8]`,
        // which is *seed-independent* in the stream value; the child's
        // seed differs because it's copied from the parent. So
        // assert seed equality, not stream equality.
        let p1 = RngState::from_parts(seed_bytes(0x42), 5, 0);
        let p2 = RngState::from_parts(seed_bytes(0xbb), 5, 0);
        let c1 = p1.fork(b"x");
        let c2 = p2.fork(b"x");
        assert_eq!(c1.seed, p1.seed);
        assert_eq!(c2.seed, p2.seed);
        // Stream value coincides because the SHA256 input is
        // (parent.stream, salt) — neither side carries the seed.
        assert_eq!(c1.stream, c2.stream);
        // Bytes drawn from the chacha differ because the seed differs.
        let mut r1 = c1.into_chacha();
        let mut r2 = c2.into_chacha();
        assert_ne!(draw_n(&mut r1, 1024), draw_n(&mut r2, 1024));
    }

    #[test]
    fn fork_with_empty_salt_is_deterministic() {
        let parent = RngState::from_parts(seed_bytes(0x42), 0, 0);
        let c1 = parent.fork(b"");
        let c2 = parent.fork(b"");
        assert_eq!(c1, c2);
    }

    #[test]
    fn fork_with_empty_salt_differs_from_fork_with_nonempty_salt() {
        let parent = RngState::from_parts(seed_bytes(0x42), 0, 0);
        let c_empty = parent.fork(b"");
        let c_one = parent.fork(b"x");
        assert_ne!(c_empty.stream, c_one.stream);
    }

    #[test]
    fn fork_is_not_commutative_under_double_fork() {
        // grandchild via fork-then-fork — order of salts matters.
        let parent = RngState::from_parts(seed_bytes(0x42), 0, 0);
        let g_ab = parent.fork(b"a").fork(b"b");
        let g_ba = parent.fork(b"b").fork(b"a");
        assert_ne!(g_ab.stream, g_ba.stream);
    }

    #[test]
    fn fork_chains_of_same_salt_strictly_descend_to_distinct_streams() {
        // parent → fork(s) → fork(s).fork(s) → fork(s).fork(s).fork(s)
        // Each generation gets a new stream value (modulo astronomical
        // SHA-256 collisions).
        let p = RngState::from_parts(seed_bytes(0x42), 0, 0);
        let g1 = p.fork(b"s");
        let g2 = g1.fork(b"s");
        let g3 = g2.fork(b"s");
        assert_ne!(p.stream, g1.stream);
        assert_ne!(g1.stream, g2.stream);
        assert_ne!(g2.stream, g3.stream);
        // And no transitive aliasing:
        assert_ne!(p.stream, g2.stream);
        assert_ne!(p.stream, g3.stream);
        assert_ne!(g1.stream, g3.stream);
    }

    #[test]
    fn fork_long_salt_is_handled() {
        // Long salt (>>SHA-256 block size) — sha2 absorbs it normally.
        let parent = RngState::from_parts(seed_bytes(0x42), 0, 0);
        let long_salt = vec![0xab; 4096];
        let c1 = parent.fork(&long_salt);
        let c2 = parent.fork(&long_salt);
        assert_eq!(c1, c2);
    }

    #[test]
    fn fork_one_byte_salt_difference_changes_stream() {
        // SHA-256 cascade ensures any salt bit difference flips ~half
        // the digest bits; stream values must differ.
        let parent = RngState::from_parts(seed_bytes(0x42), 0, 0);
        let c1 = parent.fork(b"abcdefgh");
        let c2 = parent.fork(b"abcdefgi");
        assert_ne!(c1.stream, c2.stream);
    }

    // ── Snapshot semantics ────────────────────────────────────────────

    #[test]
    fn snapshot_at_word_pos_zero_equals_a_fresh_state() {
        let s = RngState::from_parts(seed_bytes(0x42), 0, 0);
        let r = s.clone().into_chacha();
        let snap = RngState::snapshot(&r, &s);
        assert_eq!(snap, s);
    }

    #[test]
    fn snapshot_records_post_draw_word_pos() {
        let s = RngState::from_parts(seed_bytes(0x42), 0, 0);
        let mut r = s.clone().into_chacha();
        let _ = draw_n(&mut r, 4096);
        let snap = RngState::snapshot(&r, &s);
        assert!(snap.word_pos > 0);
        assert_eq!(snap.word_pos, r.get_word_pos());
    }

    #[test]
    fn snapshot_can_be_round_tripped_through_serde() {
        let s = RngState::from_parts(seed_bytes(0x42), 0, 0);
        let mut r = s.clone().into_chacha();
        let _ = draw_n(&mut r, 2048);
        let snap = RngState::snapshot(&r, &s);
        let json = serde_json::to_string(&snap).expect("serialize");
        let back: RngState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, snap);
    }

    #[test]
    fn snapshot_matches_set_word_pos_resumption() {
        // Snapshot at any word_pos K, then resume produces the same
        // bytes as continuing the original RNG.
        let s = RngState::from_parts(seed_bytes(0x42), 0, 0);
        let mut r = s.clone().into_chacha();
        let _ = draw_n(&mut r, 12_345);
        let snap = RngState::snapshot(&r, &s);
        let mut r_resumed = snap.into_chacha();
        // Compare the next 4096 bytes.
        let cont = draw_n(&mut r, 4096);
        let resumed = draw_n(&mut r_resumed, 4096);
        assert_eq!(cont, resumed);
    }

    // ── Cross-stream isolation ────────────────────────────────────────

    #[test]
    fn distinct_streams_give_independent_byte_sequences() {
        // Streams 0..4 with the same seed produce mutually distinct
        // first-1024-byte draws.
        let seed = seed_bytes(0x42);
        let mut draws: Vec<Vec<u8>> = Vec::new();
        for stream in 0..4u64 {
            let mut r = RngState::from_parts(seed, stream, 0).into_chacha();
            draws.push(draw_n(&mut r, 1024));
        }
        for i in 0..4 {
            for j in (i + 1)..4 {
                assert_ne!(draws[i], draws[j], "stream {i} == stream {j}");
            }
        }
    }

    #[test]
    fn streams_with_distinct_seeds_are_independent() {
        let mut r1 = RngState::from_parts(seed_bytes(0xaa), 0, 0).into_chacha();
        let mut r2 = RngState::from_parts(seed_bytes(0xbb), 0, 0).into_chacha();
        assert_ne!(draw_n(&mut r1, 1024), draw_n(&mut r2, 1024));
    }
}
