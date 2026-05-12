//! Iman-Conover dependent-input correlation transformation
//! (Iman & Conover 1982; Mara-Tarantola-Annoni 2015 § 3.2 cites it
//! as one of the two practical sampling strategies for
//! dependent-input GSA).
//!
//! # What this transforms
//!
//! Given `(N, d)` **independent** marginal samples (rows are
//! observations; column `j` is a sample from the `j`-th factor's
//! marginal `F_j`), and a target `(d, d)` correlation matrix, the
//! transformation produces an `(N, d)` matrix with:
//!
//! - **Same marginals** as the input — column `j`'s multiset of
//!   values is preserved (only the row-ordering changes).
//! - **Spearman rank correlation matching the target matrix**
//!   asymptotically as `N → ∞`.
//!
//! Existing sensitivity estimators consume the transformed samples
//! unchanged. The whole "dependent inputs" extension is one
//! function call away.
//!
//! # Algorithm — Iman-Conover 1982
//!
//! 1. Cholesky-decompose the target: `R = L · Lᵀ` (`L` lower
//!    triangular).
//! 2. Generate `Z ∈ ℝ^{N × d}` of i.i.d. standard normals.
//! 3. Compute `Y = Z · Lᵀ`. For Gaussian `Z`, `Y` has Pearson
//!    correlation matrix `R` in the population limit.
//! 4. For each column `j`, compute the rank vector `r_j` of `Y`'s
//!    `j`-th column.
//! 5. Sort the input's `j`-th column ascending into `X̃_j` and
//!    place `X̃_j[r_j[k] - 1]` at row `k`. The output column has
//!    the same marginal as the input but its ordinal pattern
//!    matches `Y`'s.
//!
//! Asymptotically the output's *Spearman* rank correlation matches
//! `R` (because rank patterns are preserved through the rank-
//! based reorder); for samples with Gaussian-shaped marginals,
//! Spearman ≈ Pearson modulo the `2 sin(πρ/6)` factor of the
//! Gaussian copula (Liu-Kiureghian 1986).
//!
//! # What `target_rank_correlation` should be
//!
//! The function induces *Spearman* (rank) correlation matching the
//! supplied matrix. For a downstream user who knows their target
//! Pearson correlation `C` and wants the IC output to land on
//! Pearson `C` exactly, they need to pre-convert via the Liu-
//! Kiureghian or Li 2008 mapping (the inverse of the Gaussian-
//! copula Spearman→Pearson factor). For most workloads where
//! "small to moderate correlation" is the rough specification,
//! supplying the desired correlation directly works to within a
//! few percent — and far better than ignoring dependence entirely.
//! Mara 2015 § 3.2 documents this caveat.
//!
//! # Determinism
//!
//! Same `RngState` in → bit-identical output. The internal `Z`
//! draw and the rank-reorder are deterministic w.r.t. the input
//! state.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::items_after_statements,
    clippy::needless_range_loop,
    clippy::uninlined_format_args
)]

use ndarray::Array2;
use rand::RngCore;
use salib_core::{Distribution, RngState};

/// Errors from [`iman_conover_transform`].
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ImanConoverError {
    #[error("iman-conover: independent_samples must have ≥ 1 row, got 0")]
    EmptyInput,
    #[error("iman-conover: d must be ≥ 1, got 0 cols")]
    ZeroD,
    #[error("iman-conover: target_rank_correlation must be {d}×{d} square, got {rows}×{cols}")]
    NonSquareTarget { d: usize, rows: usize, cols: usize },
    #[error(
        "iman-conover: target_rank_correlation must be symmetric (entry [{i},{j}] differs from [{j},{i}] by {diff})"
    )]
    NonSymmetricTarget { i: usize, j: usize, diff: f64 },
    #[error(
        "iman-conover: target_rank_correlation must have unit diagonal (entry [{i},{i}] = {value})"
    )]
    NonUnitDiagonal { i: usize, value: f64 },
    #[error(
        "iman-conover: target_rank_correlation is not positive-definite (Cholesky failed at index {i})"
    )]
    NotPositiveDefinite { i: usize },
}

/// Apply the Iman-Conover correlation transformation to a sample
/// matrix.
///
/// `independent_samples` is `(N, d)` — column `j` is `N` i.i.d.
/// draws from factor `j`'s marginal. `target_rank_correlation` is
/// `(d, d)` — symmetric, unit diagonal, positive-definite.
/// `rng` is consumed deterministically and advanced.
///
/// Returns an `(N, d)` matrix with the same per-column marginals as
/// `independent_samples` but a Spearman rank-correlation matrix
/// approximating `target_rank_correlation`.
///
/// # Errors
///
/// - [`ImanConoverError::EmptyInput`] if `N == 0`.
/// - [`ImanConoverError::ZeroD`] if `d == 0`.
/// - [`ImanConoverError::NonSquareTarget`] if the correlation matrix
///   is not `d × d`.
/// - [`ImanConoverError::NonSymmetricTarget`] if any off-diagonal pair
///   differs by more than `1e-9`.
/// - [`ImanConoverError::NonUnitDiagonal`] if any diagonal entry
///   differs from `1.0` by more than `1e-9`.
/// - [`ImanConoverError::NotPositiveDefinite`] if the Cholesky pivot
///   becomes non-positive — caller should sanitize the matrix or
///   use a nearest-PD projection.
pub fn iman_conover_transform(
    independent_samples: &Array2<f64>,
    target_rank_correlation: &Array2<f64>,
    rng: &mut RngState,
) -> Result<Array2<f64>, ImanConoverError> {
    let n = independent_samples.nrows();
    let d = independent_samples.ncols();
    if n == 0 {
        return Err(ImanConoverError::EmptyInput);
    }
    if d == 0 {
        return Err(ImanConoverError::ZeroD);
    }

    // Validate target shape.
    if target_rank_correlation.nrows() != d || target_rank_correlation.ncols() != d {
        return Err(ImanConoverError::NonSquareTarget {
            d,
            rows: target_rank_correlation.nrows(),
            cols: target_rank_correlation.ncols(),
        });
    }
    for i in 0..d {
        let diag = target_rank_correlation[[i, i]];
        if (diag - 1.0).abs() > 1e-9 {
            return Err(ImanConoverError::NonUnitDiagonal { i, value: diag });
        }
        for j in (i + 1)..d {
            let upper = target_rank_correlation[[i, j]];
            let lower = target_rank_correlation[[j, i]];
            if (upper - lower).abs() > 1e-9 {
                return Err(ImanConoverError::NonSymmetricTarget {
                    i,
                    j,
                    diff: upper - lower,
                });
            }
        }
    }

    // Cholesky `R = L · Lᵀ`. Hand-rolled for d ≤ ~few hundred where
    // it's faster than pulling in nalgebra and where salib-core's
    // dep-graph is intentionally light.
    let l = cholesky(target_rank_correlation)?;

    // Generate Z ∈ ℝ^{N×d} of i.i.d. standard normals via the
    // existing salib-core Distribution surface.
    let mut chacha = rng.clone().into_chacha();
    let normal = Distribution::Normal {
        mu: 0.0,
        sigma: 1.0,
    };
    let mut z = Array2::<f64>::zeros((n, d));
    for i in 0..n {
        for j in 0..d {
            let u = uniform_01(&mut chacha);
            z[[i, j]] = normal.quantile(u);
        }
    }
    *rng = RngState::snapshot(&chacha, rng);

    // Y = Z · Lᵀ. For row i, factor j: Y[i,j] = Σ_k Z[i,k] · L[j,k].
    // (L is stored as a lower-triangular d×d.)
    let mut y = Array2::<f64>::zeros((n, d));
    for i in 0..n {
        for j in 0..d {
            let mut acc = 0.0_f64;
            for k in 0..=j {
                acc += z[[i, k]] * l[[j, k]];
            }
            y[[i, j]] = acc;
        }
    }

    // For each column, compute the rank pattern of Y and reorder
    // the corresponding input column to match.
    let mut out = Array2::<f64>::zeros((n, d));
    let mut col_buf = vec![0.0_f64; n];
    let mut sorted_input = vec![0.0_f64; n];
    for j in 0..d {
        for i in 0..n {
            col_buf[i] = y[[i, j]];
        }
        let ranks = ordinal_ranks(&col_buf);

        for i in 0..n {
            sorted_input[i] = independent_samples[[i, j]];
        }
        sorted_input.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // ranks[i] ∈ 1..=N — place sorted_input[ranks[i] - 1] at row i.
        for i in 0..n {
            out[[i, j]] = sorted_input[ranks[i] - 1];
        }
    }

    Ok(out)
}

/// Lower-triangular Cholesky `R = L · Lᵀ`. Pivots that fall to
/// `≤ 1e-15` trip [`ImanConoverError::NotPositiveDefinite`] — this
/// guards against `sqrt` of a tiny non-positive pivot (the proper-
/// non-PD case where a true negative would make `sqrt` produce `NaN`,
/// plus the rank-deficient case where the pivot is exactly zero).
fn cholesky(r: &Array2<f64>) -> Result<Array2<f64>, ImanConoverError> {
    let d = r.nrows();
    let mut l = Array2::<f64>::zeros((d, d));
    for i in 0..d {
        for j in 0..=i {
            let mut s = r[[i, j]];
            for k in 0..j {
                s -= l[[i, k]] * l[[j, k]];
            }
            if i == j {
                if s <= 1e-15 {
                    return Err(ImanConoverError::NotPositiveDefinite { i });
                }
                l[[i, j]] = s.sqrt();
            } else {
                l[[i, j]] = s / l[[j, j]];
            }
        }
    }
    Ok(l)
}

/// Ordinal ranking with stable tie-break by input order. Output is
/// `1..=N` (1-indexed). Mirrors the workspace convention from
/// `borgonovo::ordinal_ranks` to keep rank semantics consistent;
/// re-implemented here because we don't pull `salib-estimators`
/// into `salib-samplers`'s dep graph.
fn ordinal_ranks(data: &[f64]) -> Vec<usize> {
    let n = data.len();
    let mut indexed: Vec<(usize, f64)> = data.iter().copied().enumerate().collect();
    indexed.sort_by(|(ia, va), (ib, vb)| {
        va.partial_cmp(vb)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(ia.cmp(ib))
    });
    let mut ranks = vec![0_usize; n];
    for (rank_minus_one, &(orig_idx, _)) in indexed.iter().enumerate() {
        ranks[orig_idx] = rank_minus_one + 1;
    }
    ranks
}

/// Uniform `[0, 1)` from a `u32` draw — same conversion the other
/// samplers use.
fn uniform_01(chacha: &mut rand_chacha::ChaCha20Rng) -> f64 {
    let u32_norm = 1.0_f64 / (f64::from(u32::MAX) + 1.0);
    f64::from(chacha.next_u32()) * u32_norm
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use super::*;
    use ndarray::array;

    fn identity(d: usize) -> Array2<f64> {
        let mut m = Array2::<f64>::zeros((d, d));
        for i in 0..d {
            m[[i, i]] = 1.0;
        }
        m
    }

    fn random_uniform_samples(n: usize, d: usize) -> Array2<f64> {
        let mut x = Array2::<f64>::zeros((n, d));
        for j in 0..d {
            let mut state: u64 = 0x9E37_79B9_7F4A_7C15_u64.wrapping_mul((j as u64).wrapping_add(1));
            for i in 0..n {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                let u = (state >> 33) as f64 / ((u64::MAX >> 33) as f64 + 1.0);
                x[[i, j]] = u;
            }
        }
        x
    }

    fn pearson_correlation(x: &Array2<f64>, i: usize, j: usize) -> f64 {
        let n = x.nrows() as f64;
        let mean_i: f64 = (0..x.nrows()).map(|k| x[[k, i]]).sum::<f64>() / n;
        let mean_j: f64 = (0..x.nrows()).map(|k| x[[k, j]]).sum::<f64>() / n;
        let mut num = 0.0;
        let mut sxx = 0.0;
        let mut syy = 0.0;
        for k in 0..x.nrows() {
            let dx = x[[k, i]] - mean_i;
            let dy = x[[k, j]] - mean_j;
            num += dx * dy;
            sxx += dx * dx;
            syy += dy * dy;
        }
        num / (sxx * syy).sqrt()
    }

    // ── Validation ────────────────────────────────────────────────

    #[test]
    fn empty_input_errors() {
        let x = Array2::<f64>::zeros((0, 3));
        let r = identity(3);
        let mut rng = RngState::from_seed([0; 32]);
        assert_eq!(
            iman_conover_transform(&x, &r, &mut rng).unwrap_err(),
            ImanConoverError::EmptyInput
        );
    }

    #[test]
    fn zero_d_errors() {
        let x = Array2::<f64>::zeros((10, 0));
        let r = Array2::<f64>::zeros((0, 0));
        let mut rng = RngState::from_seed([0; 32]);
        assert_eq!(
            iman_conover_transform(&x, &r, &mut rng).unwrap_err(),
            ImanConoverError::ZeroD
        );
    }

    #[test]
    fn non_square_target_errors() {
        let x = Array2::<f64>::zeros((10, 3));
        let r = Array2::<f64>::zeros((3, 4));
        let mut rng = RngState::from_seed([0; 32]);
        assert!(matches!(
            iman_conover_transform(&x, &r, &mut rng).unwrap_err(),
            ImanConoverError::NonSquareTarget { .. }
        ));
    }

    #[test]
    fn non_symmetric_target_errors() {
        let x = Array2::<f64>::zeros((10, 2));
        let r = array![[1.0, 0.5], [0.4, 1.0]]; // [0,1] = 0.5 vs [1,0] = 0.4
        let mut rng = RngState::from_seed([0; 32]);
        assert!(matches!(
            iman_conover_transform(&x, &r, &mut rng).unwrap_err(),
            ImanConoverError::NonSymmetricTarget { .. }
        ));
    }

    #[test]
    fn non_unit_diagonal_errors() {
        let x = Array2::<f64>::zeros((10, 2));
        let r = array![[0.5, 0.0], [0.0, 1.0]];
        let mut rng = RngState::from_seed([0; 32]);
        assert!(matches!(
            iman_conover_transform(&x, &r, &mut rng).unwrap_err(),
            ImanConoverError::NonUnitDiagonal { .. }
        ));
    }

    #[test]
    fn not_positive_definite_errors() {
        let x = Array2::<f64>::zeros((10, 2));
        // Off-diagonal of 1.0 with unit diagonal makes it singular
        // (rank 1 — not positive definite).
        let r = array![[1.0, 1.0], [1.0, 1.0]];
        let mut rng = RngState::from_seed([0; 32]);
        assert!(matches!(
            iman_conover_transform(&x, &r, &mut rng).unwrap_err(),
            ImanConoverError::NotPositiveDefinite { .. }
        ));
    }

    // ── Marginal preservation ─────────────────────────────────────

    #[test]
    fn output_columns_have_same_multiset_as_input() {
        let n = 100;
        let d = 3;
        let x = random_uniform_samples(n, d);
        let r = array![[1.0, 0.5, 0.0], [0.5, 1.0, 0.3], [0.0, 0.3, 1.0]];
        let mut rng = RngState::from_seed([0; 32]);
        let out = iman_conover_transform(&x, &r, &mut rng).unwrap();

        // Each output column should be a permutation of the input column.
        for j in 0..d {
            let mut input_col: Vec<f64> = (0..n).map(|i| x[[i, j]]).collect();
            let mut output_col: Vec<f64> = (0..n).map(|i| out[[i, j]]).collect();
            input_col.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            output_col.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            for k in 0..n {
                assert!(
                    (input_col[k] - output_col[k]).abs() < 1e-12,
                    "factor {j}: sorted output[{k}] = {} differs from input = {}",
                    output_col[k],
                    input_col[k]
                );
            }
        }
    }

    // ── Identity correlation: output approximates independence ───

    #[test]
    fn identity_target_yields_near_zero_pairwise_correlation() {
        let n = 2000;
        let d = 3;
        let x = random_uniform_samples(n, d);
        let r = identity(d);
        let mut rng = RngState::from_seed([0; 32]);
        let out = iman_conover_transform(&x, &r, &mut rng).unwrap();
        // Pairwise Pearson should be near 0 (uniform marginals,
        // independent rank patterns by construction).
        for i in 0..d {
            for j in (i + 1)..d {
                let rho = pearson_correlation(&out, i, j);
                assert!(
                    rho.abs() < 0.1,
                    "ρ({i},{j}) = {rho:.3} should be ≈ 0 under identity target",
                    i = i,
                    j = j,
                );
            }
        }
    }

    // ── Correlated normals: output approximates target correlation ─

    #[test]
    fn correlated_normals_recover_target_pearson_correlation() {
        // Input is independent N(0, 1) marginals. Apply IC with
        // target ρ between factors 0 and 1. Output should have
        // Pearson ρ ≈ target between those factors.
        let n = 5000;
        let target_rho = 0.6;
        let r = array![
            [1.0, target_rho, 0.0],
            [target_rho, 1.0, 0.0],
            [0.0, 0.0, 1.0]
        ];

        // Generate independent N(0, 1) samples via the same path
        // the IC procedure uses internally — we want input marginals
        // to be Gaussian so Spearman ≈ Pearson.
        use rand::SeedableRng;
        let mut chacha = rand_chacha::ChaCha20Rng::from_seed([7; 32]);
        let normal = Distribution::Normal {
            mu: 0.0,
            sigma: 1.0,
        };
        let mut x = Array2::<f64>::zeros((n, 3));
        for i in 0..n {
            for j in 0..3 {
                let u = uniform_01(&mut chacha);
                x[[i, j]] = normal.quantile(u);
            }
        }

        let mut rng = RngState::from_seed([42; 32]);
        let out = iman_conover_transform(&x, &r, &mut rng).unwrap();

        let realized_rho = pearson_correlation(&out, 0, 1);
        assert!(
            (realized_rho - target_rho).abs() < 0.05,
            "realized ρ({}, {}) = {realized_rho:.3}, target = {target_rho}",
            0,
            1
        );
        // Factor 2 should remain near-uncorrelated with the others.
        let rho_02 = pearson_correlation(&out, 0, 2);
        let rho_12 = pearson_correlation(&out, 1, 2);
        assert!(rho_02.abs() < 0.1, "ρ(0, 2) = {rho_02:.3} should be ≈ 0");
        assert!(rho_12.abs() < 0.1, "ρ(1, 2) = {rho_12:.3} should be ≈ 0");
    }

    // ── Determinism ───────────────────────────────────────────────

    #[test]
    fn same_rng_yields_identical_output() {
        let n = 100;
        let x = random_uniform_samples(n, 3);
        let r = array![[1.0, 0.4, 0.2], [0.4, 1.0, 0.1], [0.2, 0.1, 1.0]];
        let mut rng_a = RngState::from_seed([5; 32]);
        let mut rng_b = RngState::from_seed([5; 32]);
        let a = iman_conover_transform(&x, &r, &mut rng_a).unwrap();
        let b = iman_conover_transform(&x, &r, &mut rng_b).unwrap();
        for i in 0..n {
            for j in 0..3 {
                assert_eq!(a[[i, j]], b[[i, j]]);
            }
        }
    }

    // ── Cholesky correctness sanity ───────────────────────────────

    #[test]
    fn cholesky_recovers_known_matrix() {
        // R = [[1, 0.5], [0.5, 1]]
        // L = [[1, 0], [0.5, sqrt(0.75)]]
        // L · Lᵀ = [[1, 0.5], [0.5, 0.25 + 0.75]] = R. ✓
        let r = array![[1.0, 0.5], [0.5, 1.0]];
        let l = cholesky(&r).unwrap();
        assert!((l[[0, 0]] - 1.0).abs() < 1e-12);
        assert!(l[[0, 1]].abs() < 1e-12);
        assert!((l[[1, 0]] - 0.5).abs() < 1e-12);
        assert!((l[[1, 1]] - 0.75_f64.sqrt()).abs() < 1e-12);
    }

    // ── Ordinal ranks sanity ──────────────────────────────────────

    #[test]
    fn ordinal_ranks_assigns_one_through_n() {
        let data = [3.0, 1.0, 2.0];
        let ranks = ordinal_ranks(&data);
        assert_eq!(ranks, vec![3, 1, 2]);
    }

    #[test]
    fn ordinal_ranks_breaks_ties_by_input_order() {
        let data = [2.0, 1.0, 2.0];
        let ranks = ordinal_ranks(&data);
        // 1.0 → rank 1; the two 2.0s break tie by input order:
        // index 0 gets rank 2, index 2 gets rank 3.
        assert_eq!(ranks, vec![2, 1, 3]);
    }
}
