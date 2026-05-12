//! Deterministic f64 reductions for saltelli — fixed-tree pairwise
//! folds whose output is bit-identical regardless of rayon
//! partitioning.
//!
//! # The float-associativity problem
//!
//! `f64 + f64` is not associative. If rayon partitions a length-N
//! vector differently across runs (different machine, different
//! rayon version, different available cores), `xs.par_iter().sum()`
//! produces different bit-level f64 values. For Sobol' indices
//! computed as `(1/N) Σ f(B)_j (f(A_Bⁱ)_j - f(A)_j)`, that means
//! bit-different indices across runs of the same seed. Unacceptable
//! for ledger byte-exactness (per
//! `decisions/2026-04-28-saltelli-ledger-composition.md`).
//!
//! # The defense — block-then-tree
//!
//! 1. Partition `xs` into fixed-size [`BLOCK`] chunks.
//! 2. Each chunk reduces sequentially via [`tree_sum`] — pairwise
//!    fold, halving in place each round, deterministic in-block
//!    order. Per-chunk sums are written to a `Vec<f64>` at known
//!    indices via [`rayon::slice::ParallelSlice::par_chunks`] +
//!    `map(...).collect()`. `par_chunks` is an
//!    `IndexedParallelIterator` — output order is fixed by chunk
//!    index regardless of which thread computed which chunk.
//! 3. The per-chunk sum vector reduces via the same [`tree_sum`].
//!    Sequential, pairwise, deterministic.
//!
//! Result: bit-identical to a single-threaded `tree_sum(xs)`
//! regardless of rayon thread count. Pinned by
//! `tck/saltelli/rng-determinism/features/tree_fold_invariance.feature`.
//!
//! # Why these primitives, not `par_iter().sum()`
//!
//! `clippy::disallowed_methods` extension in workspace `clippy.toml`
//! bans `rayon::iter::ParallelIterator::{sum, reduce, reduce_with,
//! fold}` for any saltelli crate that opts in via
//! `#![deny(clippy::disallowed_methods)]`. `salib-core` opts in
//! at the crate root. Future saltelli crates opt in as they land.
//! See `decisions/2026-04-28-saltelli-rng-determinism.md` § "Banned
//! methods."
//!
//! # Cost
//!
//! ~5–10% slower than `par_iter().sum()` per
//! `rust_salib_crate_research.md` § 6.1 (one extra pass over per-block
//! sums). Accepted for byte-exact reproducibility under the
//! "pay-in-disk-not-speed" posture in `CLAUDE.md` § "Way of working."

use rayon::prelude::*;

/// Per-chunk size for the parallel block-then-tree reduction. Fixed;
/// changing it would change reduction-tree shape and therefore
/// bit-level results. Future workload-tuned per-`Experiment` blocks
/// would land via ADR.
pub const BLOCK: usize = 1 << 12;

/// Sequential pairwise tree-fold sum. O(N) work, O(log N) reduction
/// depth. Bit-identical to [`par_tree_sum`] regardless of rayon
/// thread count.
///
/// Edge cases: empty input → 0.0; length-1 input → the single
/// element verbatim.
#[must_use]
pub fn tree_sum(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    if xs.len() == 1 {
        return xs[0];
    }
    let mut buf: Vec<f64> = xs.to_vec();
    while buf.len() > 1 {
        let pairs = buf.len() / 2;
        let trailing = buf.len() % 2 == 1;
        for i in 0..pairs {
            buf[i] = buf[2 * i] + buf[2 * i + 1];
        }
        if trailing {
            buf[pairs] = buf[2 * pairs];
            buf.truncate(pairs + 1);
        } else {
            buf.truncate(pairs);
        }
    }
    buf[0]
}

/// Parallel block-then-tree sum. Partitions `xs` into [`BLOCK`]-sized
/// chunks, reduces each chunk via [`tree_sum`] in parallel via rayon,
/// then reduces the per-chunk sums via [`tree_sum`] sequentially.
/// Bit-identical to `tree_sum(xs)` regardless of rayon thread count
/// or worker partition pattern.
#[must_use]
pub fn par_tree_sum(xs: &[f64]) -> f64 {
    if xs.len() <= BLOCK {
        return tree_sum(xs);
    }
    let block_sums: Vec<f64> = xs.par_chunks(BLOCK).map(tree_sum).collect();
    tree_sum(&block_sums)
}

/// Sequential pairwise tree-fold dot product. Produces
/// `Σᵢ aᵢ·bᵢ` in tree order. Bit-identical to [`par_tree_dot`].
///
/// Panics if `a.len() != b.len()`.
#[must_use]
pub fn tree_dot(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len(), "tree_dot: length mismatch");
    if a.is_empty() {
        return 0.0;
    }
    let products: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x * y).collect();
    tree_sum(&products)
}

/// Parallel block-then-tree dot product. Same partitioning logic as
/// [`par_tree_sum`]; bit-identical to [`tree_dot`].
///
/// Panics if `a.len() != b.len()`.
#[must_use]
pub fn par_tree_dot(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len(), "par_tree_dot: length mismatch");
    if a.len() <= BLOCK {
        return tree_dot(a, b);
    }
    let block_sums: Vec<f64> = a
        .par_chunks(BLOCK)
        .zip(b.par_chunks(BLOCK))
        .map(|(ac, bc)| tree_dot(ac, bc))
        .collect();
    tree_sum(&block_sums)
}

/// Sequential two-pass unbiased sample variance, routed through
/// [`tree_sum`] in both passes. Returns `0.0` for inputs of length
/// `< 2`.
///
/// Two-pass to avoid accumulator dependence on traversal order:
/// pass 1 computes `μ = tree_sum(xs) / N`; pass 2 computes
/// `Σ (xᵢ - μ)² / (N - 1)` via `tree_sum`.
#[must_use]
pub fn tree_var(xs: &[f64]) -> f64 {
    let n = xs.len();
    if n < 2 {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let n_f = n as f64;
    let mean = tree_sum(xs) / n_f;
    let centered_sq: Vec<f64> = xs
        .iter()
        .map(|&x| {
            let d = x - mean;
            d * d
        })
        .collect();
    #[allow(clippy::cast_precision_loss)]
    let denom = (n - 1) as f64;
    tree_sum(&centered_sq) / denom
}

/// Parallel two-pass unbiased sample variance. Mean via
/// [`par_tree_sum`]; centered-square via parallel `map` (preserves
/// index order on `IndexedParallelIterator`); then the centered-square
/// reduction via [`par_tree_sum`].
#[must_use]
pub fn par_tree_var(xs: &[f64]) -> f64 {
    let n = xs.len();
    if n < 2 {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let n_f = n as f64;
    let mean = par_tree_sum(xs) / n_f;
    let centered_sq: Vec<f64> = xs
        .par_iter()
        .map(|&x| {
            let d = x - mean;
            d * d
        })
        .collect();
    #[allow(clippy::cast_precision_loss)]
    let denom = (n - 1) as f64;
    par_tree_sum(&centered_sq) / denom
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::cast_precision_loss
)]
mod tests {
    use super::*;

    fn linear_vec(n: usize) -> Vec<f64> {
        (0..n).map(|i| (i as f64) * 0.5).collect()
    }

    #[test]
    fn tree_sum_empty_is_zero() {
        assert_eq!(tree_sum(&[]), 0.0);
    }

    #[test]
    fn tree_sum_single_is_passthrough() {
        assert_eq!(tree_sum(&[3.14]), 3.14);
    }

    #[test]
    fn tree_sum_pair_is_sum() {
        assert_eq!(tree_sum(&[1.0, 2.0]), 3.0);
    }

    #[test]
    fn tree_sum_odd_length_handles_trailing_element() {
        // [1, 2, 3, 4, 5] → tree fold:
        // round 1: [1+2, 3+4, 5] = [3, 7, 5]
        // round 2: [3+7, 5]      = [10, 5]
        // round 3: [10+5]        = [15]
        assert_eq!(tree_sum(&[1.0, 2.0, 3.0, 4.0, 5.0]), 15.0);
    }

    #[test]
    fn tree_sum_matches_arithmetic_progression_closed_form() {
        let n = 1000usize;
        let xs = linear_vec(n);
        // Σ 0.5·i for i in 0..n  =  0.5 · n·(n-1)/2
        #[allow(clippy::cast_precision_loss)]
        let expected = 0.5 * (n as f64) * ((n - 1) as f64) / 2.0;
        let got = tree_sum(&xs);
        assert!(
            (got - expected).abs() < 1e-9,
            "tree_sum {got} vs closed form {expected}"
        );
    }

    #[test]
    fn par_tree_sum_short_input_falls_through_to_tree_sum() {
        let xs = linear_vec(100);
        assert_eq!(par_tree_sum(&xs), tree_sum(&xs));
    }

    #[test]
    fn par_tree_sum_long_input_matches_tree_sum_bitwise() {
        let xs = linear_vec(BLOCK * 16 + 137);
        let p = par_tree_sum(&xs);
        let s = tree_sum(&xs);
        assert!(
            p.to_bits() == s.to_bits(),
            "par_tree_sum != tree_sum bitwise"
        );
    }

    #[test]
    fn par_tree_sum_is_self_consistent_across_reruns() {
        let xs = linear_vec(BLOCK * 8);
        let a = par_tree_sum(&xs);
        let b = par_tree_sum(&xs);
        assert!(a.to_bits() == b.to_bits());
    }

    #[test]
    fn tree_dot_matches_naive_dot_for_short_inputs() {
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![2.0, 3.0, 4.0, 5.0];
        // 2 + 6 + 12 + 20 = 40
        assert_eq!(tree_dot(&a, &b), 40.0);
    }

    #[test]
    #[should_panic(expected = "length mismatch")]
    fn tree_dot_length_mismatch_panics() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        let _ = tree_dot(&a, &b);
    }

    #[test]
    fn par_tree_dot_long_input_matches_tree_dot_bitwise() {
        let a = linear_vec(BLOCK * 4 + 11);
        let b: Vec<f64> = a.iter().rev().copied().collect();
        let p = par_tree_dot(&a, &b);
        let s = tree_dot(&a, &b);
        assert!(p.to_bits() == s.to_bits());
    }

    #[test]
    fn tree_var_constant_input_is_zero() {
        let xs = vec![3.0; 1000];
        let v = tree_var(&xs);
        assert!(v.abs() < 1e-12, "constant input variance {v} not ~0");
    }

    #[test]
    fn tree_var_matches_unbiased_formula_for_simple_input() {
        // [1, 2, 3, 4, 5]: mean = 3, sum (xi-3)^2 = 4+1+0+1+4 = 10,
        // unbiased / (5-1) = 2.5
        let xs = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let v = tree_var(&xs);
        assert!((v - 2.5).abs() < 1e-12, "got {v}");
    }

    #[test]
    fn par_tree_var_matches_tree_var_bitwise_long_input() {
        let xs = linear_vec(BLOCK * 4 + 11);
        let p = par_tree_var(&xs);
        let s = tree_var(&xs);
        assert!(p.to_bits() == s.to_bits());
    }

    #[test]
    fn tree_var_short_input_returns_zero_below_n_two() {
        assert_eq!(tree_var(&[]), 0.0);
        assert_eq!(tree_var(&[42.0]), 0.0);
    }

    // ── Tree-shape and ordering properties ────────────────────────────

    #[test]
    fn tree_sum_two_elements_uses_single_addition() {
        // Tree-fold of [a, b] is exactly a + b — no extra rounds.
        let a = 1.234_567_8_f64;
        let b = 9.876_543_2_f64;
        assert_eq!(tree_sum(&[a, b]).to_bits(), (a + b).to_bits());
    }

    #[test]
    fn tree_sum_four_elements_pairs_then_pairs() {
        // [a, b, c, d] → round 1: [a+b, c+d] → round 2: [(a+b)+(c+d)].
        let a = 1.0;
        let b = 2.0;
        let c = 3.0;
        let d = 4.0;
        let expected = (a + b) + (c + d);
        assert_eq!(tree_sum(&[a, b, c, d]).to_bits(), expected.to_bits());
    }

    #[test]
    fn tree_sum_distinguishes_naive_left_fold_under_catastrophic_cancellation() {
        // Adversarial: 1e16 then a million 1.0s. Naive left-fold yields
        // 1e16 (each `1.0` truncates against `1e16`); tree fold pairs
        // the 1.0s up first, accumulating 2^k partial sums until they
        // grow large enough to register against `1e16`.
        let mut xs = Vec::with_capacity(1 + (1 << 20));
        xs.push(1e16);
        xs.extend(std::iter::repeat_n(1.0_f64, 1 << 20));
        let naive: f64 = xs.iter().copied().sum();
        let tree = tree_sum(&xs);
        // Naive is "stuck" at 1e16; tree picks up the 1.0s.
        assert!(
            tree > naive,
            "tree_sum {tree} should exceed naive sum {naive} on this input"
        );
        // True total is 1e16 + 2^20 = 10000000001048576.0.
        let exact = 1e16 + ((1u64 << 20) as f64);
        assert_eq!(tree.to_bits(), exact.to_bits());
    }

    #[test]
    fn tree_sum_zero_vector_is_zero() {
        let xs = vec![0.0_f64; 1024];
        assert_eq!(tree_sum(&xs), 0.0);
    }

    #[test]
    fn tree_sum_negative_values_sum_correctly() {
        let xs = vec![-1.0, -2.0, -3.0, -4.0];
        assert_eq!(tree_sum(&xs), -10.0);
    }

    #[test]
    fn tree_sum_alternating_signs_cancels() {
        let xs = vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
        assert_eq!(tree_sum(&xs), 0.0);
    }

    #[test]
    fn tree_sum_handles_lengths_through_block_boundary() {
        // Sweep N around the BLOCK boundary so par_tree_sum exercises
        // both the short-circuit branch (≤ BLOCK) and the multi-block
        // branch.
        for n in [
            1, 2, 3, 7, 16, 100, 1023, 1024, 4095, 4096, 4097, 8191, 8192, 8193,
        ] {
            let xs = linear_vec(n);
            assert_eq!(
                par_tree_sum(&xs).to_bits(),
                tree_sum(&xs).to_bits(),
                "par_tree_sum != tree_sum at N={n}"
            );
        }
    }

    // ── Algebraic properties: tree_sum ───────────────────────────────

    #[test]
    fn tree_sum_scaled_input_scales_output_for_finite_factor() {
        // Pure scaling: tree_sum(c·xs) = c · tree_sum(xs) within FP.
        let xs = linear_vec(2048);
        let c = 7.5_f64;
        let scaled: Vec<f64> = xs.iter().map(|x| c * x).collect();
        let lhs = tree_sum(&scaled);
        let rhs = c * tree_sum(&xs);
        assert!(
            (lhs - rhs).abs() <= 1e-9 * lhs.abs().max(rhs.abs()),
            "tree_sum scaling: lhs={lhs} rhs={rhs}"
        );
    }

    #[test]
    fn tree_sum_concat_equals_sum_of_subsums_for_powers_of_two() {
        // For lengths that are exact powers of two, splitting at the
        // half boundary yields a partition-sum identity:
        //   tree_sum(xs[..N/2]) + tree_sum(xs[N/2..]) == tree_sum(xs)
        // (bit-exact, because the tree fold of a 2^k vector pairs the
        // first half then the second half then reduces them.)
        for k in [4, 8, 12] {
            let n = 1usize << k;
            let xs = linear_vec(n);
            let half = n / 2;
            let lhs = tree_sum(&xs[..half]) + tree_sum(&xs[half..]);
            let rhs = tree_sum(&xs);
            assert_eq!(
                lhs.to_bits(),
                rhs.to_bits(),
                "split-half identity at N=2^{k}"
            );
        }
    }

    // ── Algebraic properties: tree_dot ───────────────────────────────

    #[test]
    fn tree_dot_with_zero_vector_is_zero() {
        let a = linear_vec(1024);
        let zero = vec![0.0_f64; 1024];
        assert_eq!(tree_dot(&a, &zero), 0.0);
    }

    #[test]
    fn tree_dot_is_commutative_bitwise() {
        // tree_dot(a, b) == tree_dot(b, a) bit-exactly because the
        // products vector is element-wise commutative under f64
        // multiplication, and both calls reduce via the same tree.
        let a = linear_vec(4096);
        let b: Vec<f64> = a.iter().rev().copied().collect();
        assert_eq!(tree_dot(&a, &b).to_bits(), tree_dot(&b, &a).to_bits());
    }

    #[test]
    fn tree_dot_scales_with_either_argument() {
        let a = linear_vec(2048);
        let b = linear_vec(2048);
        let c = 3.0_f64;
        let scaled_a: Vec<f64> = a.iter().map(|x| c * x).collect();
        let lhs = tree_dot(&scaled_a, &b);
        let rhs = c * tree_dot(&a, &b);
        assert!((lhs - rhs).abs() <= 1e-9 * lhs.abs().max(rhs.abs()));
    }

    #[test]
    fn tree_dot_of_empty_pair_is_zero() {
        assert_eq!(tree_dot(&[], &[]), 0.0);
    }

    #[test]
    fn tree_dot_of_unit_vectors_is_inner_product() {
        let a = vec![3.0];
        let b = vec![4.0];
        assert_eq!(tree_dot(&a, &b), 12.0);
    }

    #[test]
    fn tree_dot_squares_match_tree_sum_of_squares() {
        // tree_dot(a, a) is the sum-of-squares; check it agrees with
        // tree_sum over the squared vector.
        let a = linear_vec(1024);
        let squares: Vec<f64> = a.iter().map(|x| x * x).collect();
        let lhs = tree_dot(&a, &a);
        let rhs = tree_sum(&squares);
        assert_eq!(lhs.to_bits(), rhs.to_bits());
    }

    // ── Algebraic properties: tree_var ───────────────────────────────

    #[test]
    fn tree_var_is_translation_invariant() {
        // var(xs + c) == var(xs) within FP — translation doesn't
        // change spread.
        let xs = linear_vec(2048);
        let c = 100.0_f64;
        let shifted: Vec<f64> = xs.iter().map(|x| x + c).collect();
        let v_xs = tree_var(&xs);
        let v_shift = tree_var(&shifted);
        assert!(
            (v_xs - v_shift).abs() <= 1e-7 * v_xs.abs().max(v_shift.abs()),
            "translation invariance: var(xs)={v_xs} var(xs+c)={v_shift}"
        );
    }

    #[test]
    fn tree_var_scales_quadratically() {
        // var(c · xs) == c² · var(xs) within FP.
        let xs = linear_vec(2048);
        let c = 4.0_f64;
        let scaled: Vec<f64> = xs.iter().map(|x| c * x).collect();
        let lhs = tree_var(&scaled);
        let rhs = c * c * tree_var(&xs);
        assert!(
            (lhs - rhs).abs() <= 1e-9 * lhs.abs().max(rhs.abs()),
            "var quadratic scaling: lhs={lhs} rhs={rhs}"
        );
    }

    #[test]
    fn tree_var_uses_bessel_correction_n_minus_one() {
        // [1, 2]: mean = 1.5, sum (xi-μ)² = 0.25 + 0.25 = 0.5,
        // unbiased / (2-1) = 0.5  (vs the biased estimator: 0.5 / 2 = 0.25).
        let xs = vec![1.0, 2.0];
        let v = tree_var(&xs);
        assert!((v - 0.5).abs() < 1e-12, "Bessel-corrected var; got {v}");
    }

    #[test]
    fn tree_var_of_two_equal_values_is_zero() {
        let xs = vec![5.0, 5.0];
        assert_eq!(tree_var(&xs), 0.0);
    }

    #[test]
    fn tree_var_of_centered_dataset_matches_naive_unbiased() {
        // Centered [-1, 0, 1]: mean = 0, sum_sq = 2, var = 2/2 = 1.
        let xs = vec![-1.0, 0.0, 1.0];
        let v = tree_var(&xs);
        assert!((v - 1.0).abs() < 1e-12, "got {v}");
    }

    #[test]
    fn tree_var_matches_naive_var_for_uniform_grid() {
        // 0..n at unit spacing. Mean = (n-1)/2; variance =
        // n(n+1)/12 · (1/(n-1)) -- but easier: compute the naive
        // unbiased variance directly and compare.
        let n = 100usize;
        let xs: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let mean: f64 = xs.iter().copied().sum::<f64>() / n as f64;
        let naive_var: f64 = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
        let v = tree_var(&xs);
        assert!(
            (v - naive_var).abs() <= 1e-9 * v.abs().max(naive_var.abs()),
            "tree_var {v} vs naive {naive_var}"
        );
    }

    // ── Parallel-vs-sequential bit-equivalence ───────────────────────

    #[test]
    fn par_tree_sum_matches_tree_sum_for_zero_vector() {
        let xs = vec![0.0_f64; BLOCK * 4];
        assert_eq!(par_tree_sum(&xs).to_bits(), tree_sum(&xs).to_bits());
    }

    #[test]
    fn par_tree_sum_matches_tree_sum_for_negative_values() {
        let xs: Vec<f64> = (0..BLOCK * 3).map(|i| -(i as f64) * 0.25).collect();
        assert_eq!(par_tree_sum(&xs).to_bits(), tree_sum(&xs).to_bits());
    }

    #[test]
    fn par_tree_dot_handles_lengths_through_block_boundary() {
        for n in [1, 2, 1023, 1024, 4095, 4096, 4097, 8193] {
            let a = linear_vec(n);
            let b: Vec<f64> = a.iter().map(|x| x + 1.0).collect();
            assert_eq!(
                par_tree_dot(&a, &b).to_bits(),
                tree_dot(&a, &b).to_bits(),
                "par_tree_dot != tree_dot at N={n}"
            );
        }
    }

    #[test]
    fn par_tree_var_handles_lengths_through_block_boundary() {
        for n in [2, 100, 1024, 4096, 4097, 8192] {
            let xs = linear_vec(n);
            assert_eq!(
                par_tree_var(&xs).to_bits(),
                tree_var(&xs).to_bits(),
                "par_tree_var != tree_var at N={n}"
            );
        }
    }

    // ── Edge cases that don't fit the categorical buckets ────────────

    #[test]
    fn tree_sum_of_three_elements_pairs_first_two_then_adds_third() {
        // [a, b, c] → round 1: [a+b, c] → round 2: [(a+b)+c].
        let a = 1e10_f64;
        let b = 1e-10_f64;
        let c = -1e10_f64;
        // Naive left fold: ((a+b)+c) — same shape as our tree, so they
        // agree. The point of this test is to pin the *tree shape*:
        // the first round pairs (a, b) and carries c forward.
        let expected = (a + b) + c;
        assert_eq!(tree_sum(&[a, b, c]).to_bits(), expected.to_bits());
    }

    #[test]
    fn tree_sum_handles_subnormals() {
        // Sums of subnormals should still respect the tree shape and
        // not round to zero (we're careful to add small numbers
        // pairwise, not against a giant accumulator).
        let small = f64::MIN_POSITIVE / 2.0; // a subnormal
        let xs = vec![small; 8];
        let s = tree_sum(&xs);
        assert!(s > 0.0);
        assert_eq!(s.to_bits(), (8.0 * small).to_bits());
    }

    #[test]
    fn par_tree_sum_one_block_exact() {
        // Length exactly BLOCK: par_tree_sum's short-circuit branch
        // takes the sequential tree_sum path.
        let xs = linear_vec(BLOCK);
        assert_eq!(par_tree_sum(&xs).to_bits(), tree_sum(&xs).to_bits());
    }

    #[test]
    fn par_tree_sum_many_blocks_with_tail() {
        // Length BLOCK*8 + 1: the partial trailing block (length 1)
        // exercises tree_sum's len==1 short-circuit inside the per-
        // chunk reduction.
        let xs = linear_vec(BLOCK * 8 + 1);
        assert_eq!(par_tree_sum(&xs).to_bits(), tree_sum(&xs).to_bits());
    }

    #[test]
    fn par_tree_dot_with_self_equals_sum_of_squares() {
        let a = linear_vec(BLOCK * 3 + 7);
        let p = par_tree_dot(&a, &a);
        let s_seq = tree_dot(&a, &a);
        assert_eq!(p.to_bits(), s_seq.to_bits());
    }

    #[test]
    fn tree_dot_distributes_over_addition_within_fp() {
        // a · (b + c) ≈ a·b + a·c within FP.
        let a = linear_vec(1024);
        let b = linear_vec(1024);
        let c: Vec<f64> = (0..1024_i32).map(|i| f64::from(i) * 0.1).collect();
        let bc: Vec<f64> = b.iter().zip(c.iter()).map(|(x, y)| x + y).collect();
        let lhs = tree_dot(&a, &bc);
        let rhs = tree_dot(&a, &b) + tree_dot(&a, &c);
        assert!(
            (lhs - rhs).abs() <= 1e-9 * lhs.abs().max(rhs.abs()),
            "lhs={lhs} rhs={rhs}"
        );
    }
}
