//! Saltelli `(A, B, A_Bⁱ)` matrix construction (radial design,
//! Saltelli 2010) — the design pattern Sobol'-style sensitivity
//! estimators consume.
//!
//! # The radial construction
//!
//! Given a base 2d-dimensional sample of size N (drawn from any
//! `Sampler` configured for `2d` columns), split into two halves:
//!
//! - `A` = first `d` columns (`N × d`).
//! - `B` = last `d` columns (`N × d`).
//!
//! Then for each factor `i ∈ 0..d`, build `A_Bⁱ` by cloning `A`
//! and replacing its `i`-th column with `B`'s `i`-th column. Result:
//! `d` matrices each `N × d`. The `(A, B, A_Bⁱ)` bundle is the input
//! every variance-based Sobol' estimator (Saltelli2010, Jansen1999,
//! Janon2014, Owen2013) consumes — see PR 7 of
//! `plans/0002-saltelli-roadmap.md`.
//!
//! Optionally, with `second_order = true`, also build `d` matrices
//! `B_Aⁱ` (the symmetric construction with B and A swapped); enables
//! second-order interaction-index estimation. Total cost grows from
//! `N·(d+2)` to `N·(2d+2)` model evaluations.
//!
//! # Why radial
//!
//! Saltelli 2010 ("Variance based sensitivity analysis of model
//! output. Design and estimator for the total sensitivity index")
//! showed that the radial design — splitting a single 2d-dim base
//! sample — gives lower MAE for total-order indices than the
//! original (Saltelli 2002) design that uses two independent d-dim
//! samples. `SALib`'s default since ~2018 has been the radial design.
//!
//! Per `decisions/2026-04-29-saltelli-matrix-construction.md`, PR 6
//! ships **only the radial design**. The original design is deferred
//! to a follow-on PR; it requires per-sampler-class handling
//! (LHS forks `RngState`; Sobol' is RNG-deterministic and would need a
//! `start_index` field to get a second independent sample). Radial
//! works cleanly on both LHS and Sobol' as a single code path.
//!
//! # Sampler dim contract
//!
//! `sampler.dim()` must equal `2 * d` where `d` is the SA factor
//! count. A 7-factor problem with radial Saltelli requires a
//! 14-dim sampler. The function validates this; odd `sampler.dim()`
//! returns `SaltelliError::OddBaseDim`.

use ndarray::Array2;
use salib_core::RngState;

use crate::sampler::Sampler;

/// The output of `build_saltelli_matrix`. Carries the base matrices
/// (`a`, `b`) and the column-replaced derivatives. `#[non_exhaustive]`
/// — future fields (e.g., the recorded pre-draw `RngState` for
/// audit-replay) land non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SaltelliMatrix {
    /// Sample size (rows in each matrix).
    pub n: usize,
    /// Factor dimension (columns in each matrix). Equals
    /// `sampler.dim() / 2`.
    pub dim: usize,
    /// `n × dim` first-half base matrix.
    pub a: Array2<f64>,
    /// `n × dim` second-half base matrix.
    pub b: Array2<f64>,
    /// `dim` matrices, each `n × dim`. `a_b[i]` is `a` with column
    /// `i` replaced by `b.column(i)`.
    pub a_b: Vec<Array2<f64>>,
    /// `dim` matrices for second-order indices: `b_a[i]` is `b` with
    /// column `i` replaced by `a.column(i)`. `None` if not requested.
    pub b_a: Option<Vec<Array2<f64>>>,
}

impl SaltelliMatrix {
    /// Total number of model evaluations the SA campaign requires.
    /// Equals `n·(d+2)` for first+total only, `n·(2d+2)` if also
    /// computing second-order.
    #[must_use]
    pub fn total_evaluations(&self) -> usize {
        let base = self.n * (self.dim + 2);
        if self.b_a.is_some() {
            base + self.n * self.dim
        } else {
            base
        }
    }
}

/// Errors from `build_saltelli_matrix`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SaltelliError {
    /// `n` must be at least 1; a zero-row Saltelli matrix is
    /// degenerate.
    #[error("Saltelli: n must be ≥ 1, got 0")]
    ZeroN,
    /// Radial design requires `sampler.dim()` even (so it splits
    /// cleanly into A and B halves).
    #[error("Saltelli: radial design requires even sampler.dim(), got {dim}")]
    OddBaseDim { dim: usize },
}

/// Build a Saltelli `(A, B, A_Bⁱ)` matrix bundle (radial design,
/// Saltelli 2010). Wraps any `Sampler` configured with
/// `dim = 2 * d` columns.
///
/// # Errors
///
/// - `SaltelliError::ZeroN` if `n == 0`.
/// - `SaltelliError::OddBaseDim` if `sampler.dim()` is odd.
///
/// # Determinism
///
/// Pure under `(sampler, rng)`. Same `Sampler` config + same
/// `RngState` in → bit-identical `SaltelliMatrix` out. The
/// underlying `Sampler::unit_sample` advances `rng` per its own
/// rules (LHS consumes per-block-and-cell bytes; unscrambled Sobol'
/// consumes nothing).
// `n`, `d`, and the loop indices `i`, `j` are the canonical SA
// notation in Saltelli 2010; the per-dim `a`, `b`, `m` shorthands
// match. The `many_single_char_names` lint is noisy here.
#[allow(clippy::many_single_char_names)]
pub fn build_saltelli_matrix(
    sampler: &dyn Sampler,
    n: usize,
    second_order: bool,
    rng: &mut RngState,
) -> Result<SaltelliMatrix, SaltelliError> {
    if n == 0 {
        return Err(SaltelliError::ZeroN);
    }
    let base_dim = sampler.dim();
    if !base_dim.is_multiple_of(2) {
        return Err(SaltelliError::OddBaseDim { dim: base_dim });
    }
    let d = base_dim / 2;

    // Single base draw of n × 2d. Splitting downstream.
    let base = sampler.unit_sample(n, rng);

    // Split into halves by column-by-column copy. ndarray's `s!`
    // slicing macro contains `unsafe` and conflicts with the
    // workspace `forbid(unsafe_code)`; explicit column copies are
    // safe and only marginally slower.
    let mut a = Array2::<f64>::zeros((n, d));
    let mut b = Array2::<f64>::zeros((n, d));
    for j in 0..d {
        a.column_mut(j).assign(&base.column(j));
        b.column_mut(j).assign(&base.column(j + d));
    }

    // Build A_Bⁱ matrices.
    let mut a_b: Vec<Array2<f64>> = Vec::with_capacity(d);
    for i in 0..d {
        let mut m = a.clone();
        let col_b = b.column(i).to_owned();
        m.column_mut(i).assign(&col_b);
        a_b.push(m);
    }

    // Optionally B_Aⁱ.
    let b_a = if second_order {
        let mut v: Vec<Array2<f64>> = Vec::with_capacity(d);
        for i in 0..d {
            let mut m = b.clone();
            let col_a = a.column(i).to_owned();
            m.column_mut(i).assign(&col_a);
            v.push(m);
        }
        Some(v)
    } else {
        None
    };

    Ok(SaltelliMatrix {
        n,
        dim: d,
        a,
        b,
        a_b,
        b_a,
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::lhs::LhsSampler;
    use crate::sobol::SobolSampler;

    fn fresh_rng() -> RngState {
        RngState::from_seed([0x42; 32])
    }

    // ── Sampler-dim validation ──────────────────────────────────────

    #[test]
    fn n_zero_returns_zero_n_error() {
        let s = LhsSampler::classic(4);
        let mut rng = fresh_rng();
        let err = build_saltelli_matrix(&s, 0, false, &mut rng).unwrap_err();
        assert_eq!(err, SaltelliError::ZeroN);
    }

    #[test]
    fn odd_sampler_dim_returns_odd_base_dim_error() {
        let s = LhsSampler::classic(5);
        let mut rng = fresh_rng();
        let err = build_saltelli_matrix(&s, 64, false, &mut rng).unwrap_err();
        assert_eq!(err, SaltelliError::OddBaseDim { dim: 5 });
    }

    // ── Output shape ────────────────────────────────────────────────

    #[test]
    fn lhs_base_produces_correct_shapes() {
        // sampler.dim() = 6 → d = 3.
        let s = LhsSampler::classic(6);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 64, false, &mut rng).unwrap();
        assert_eq!(m.n, 64);
        assert_eq!(m.dim, 3);
        assert_eq!(m.a.shape(), &[64, 3]);
        assert_eq!(m.b.shape(), &[64, 3]);
        assert_eq!(m.a_b.len(), 3);
        for ab_i in &m.a_b {
            assert_eq!(ab_i.shape(), &[64, 3]);
        }
        assert!(m.b_a.is_none());
    }

    #[test]
    fn sobol_base_produces_correct_shapes() {
        // sampler.dim() = 8 → d = 4.
        let s = SobolSampler::standard(8);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 32, false, &mut rng).unwrap();
        assert_eq!(m.dim, 4);
        assert_eq!(m.a.shape(), &[32, 4]);
        assert_eq!(m.b.shape(), &[32, 4]);
        assert_eq!(m.a_b.len(), 4);
    }

    #[test]
    fn second_order_populates_b_a() {
        let s = LhsSampler::classic(6);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 32, true, &mut rng).unwrap();
        let b_a = m.b_a.as_ref().expect("b_a should be Some");
        assert_eq!(b_a.len(), 3);
        for ba_i in b_a {
            assert_eq!(ba_i.shape(), &[32, 3]);
        }
    }

    // ── Column-replacement structure ────────────────────────────────

    #[test]
    fn a_b_i_replaces_column_i_with_b_column_i() {
        let s = LhsSampler::classic(6);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 32, false, &mut rng).unwrap();
        for (i, ab_i) in m.a_b.iter().enumerate() {
            // Column i should equal B's column i.
            for row in 0..m.n {
                assert_eq!(
                    ab_i[[row, i]],
                    m.b[[row, i]],
                    "a_b[{i}] row {row} col {i}: expected b[{row},{i}]"
                );
            }
            // All other columns should equal A's columns.
            for j in 0..m.dim {
                if j == i {
                    continue;
                }
                for row in 0..m.n {
                    assert_eq!(
                        ab_i[[row, j]],
                        m.a[[row, j]],
                        "a_b[{i}] row {row} col {j}: expected a[{row},{j}]"
                    );
                }
            }
        }
    }

    #[test]
    fn b_a_i_replaces_column_i_with_a_column_i() {
        let s = LhsSampler::classic(6);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 16, true, &mut rng).unwrap();
        let b_a = m.b_a.as_ref().unwrap();
        for (i, ba_i) in b_a.iter().enumerate() {
            for row in 0..m.n {
                assert_eq!(ba_i[[row, i]], m.a[[row, i]]);
            }
            for j in 0..m.dim {
                if j == i {
                    continue;
                }
                for row in 0..m.n {
                    assert_eq!(ba_i[[row, j]], m.b[[row, j]]);
                }
            }
        }
    }

    #[test]
    fn a_and_b_are_disjoint_halves_of_base_sobol_sample() {
        // For Sobol' (deterministic), we can verify A and B are
        // exactly the first-half and second-half columns of the
        // 2d-dim base sample.
        let s = SobolSampler::standard(6).with_skip_first(false);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 16, false, &mut rng).unwrap();
        // Independently re-draw the base 2d sample.
        let mut rng2 = fresh_rng();
        let base = s.unit_sample(16, &mut rng2);
        for row in 0..16 {
            for col in 0..3 {
                assert_eq!(m.a[[row, col]], base[[row, col]]);
                assert_eq!(m.b[[row, col]], base[[row, col + 3]]);
            }
        }
    }

    // ── Determinism ─────────────────────────────────────────────────

    #[test]
    fn same_sampler_same_rng_produces_identical_matrix_lhs() {
        let s = LhsSampler::classic(6);
        let mut r1 = fresh_rng();
        let mut r2 = fresh_rng();
        let m1 = build_saltelli_matrix(&s, 64, false, &mut r1).unwrap();
        let m2 = build_saltelli_matrix(&s, 64, false, &mut r2).unwrap();
        assert_eq!(m1.a, m2.a);
        assert_eq!(m1.b, m2.b);
        for (a1, a2) in m1.a_b.iter().zip(m2.a_b.iter()) {
            assert_eq!(a1, a2);
        }
    }

    #[test]
    fn same_sampler_same_rng_produces_identical_matrix_sobol() {
        let s = SobolSampler::standard(8);
        let mut r1 = fresh_rng();
        let mut r2 = fresh_rng();
        let m1 = build_saltelli_matrix(&s, 32, false, &mut r1).unwrap();
        let m2 = build_saltelli_matrix(&s, 32, false, &mut r2).unwrap();
        assert_eq!(m1.a, m2.a);
        assert_eq!(m1.b, m2.b);
    }

    #[test]
    fn distinct_lhs_streams_produce_different_matrices() {
        let s = LhsSampler::classic(4);
        let mut r1 = RngState::from_parts([0; 32], 1, 0);
        let mut r2 = RngState::from_parts([0; 32], 2, 0);
        let m1 = build_saltelli_matrix(&s, 32, false, &mut r1).unwrap();
        let m2 = build_saltelli_matrix(&s, 32, false, &mut r2).unwrap();
        assert_ne!(m1.a, m2.a);
    }

    // ── total_evaluations ───────────────────────────────────────────

    #[test]
    fn total_evaluations_first_total_is_n_times_d_plus_two() {
        // n=64, d=3 → 64 · 5 = 320 evaluations (A + B + 3 A_Bⁱ).
        let s = LhsSampler::classic(6);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 64, false, &mut rng).unwrap();
        assert_eq!(m.total_evaluations(), 64 * 5);
    }

    #[test]
    fn total_evaluations_with_second_order_is_n_times_two_d_plus_two() {
        // n=32, d=3 → 32 · 8 = 256 evaluations (A + B + 3 A_Bⁱ + 3 B_Aⁱ).
        let s = LhsSampler::classic(6);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 32, true, &mut rng).unwrap();
        assert_eq!(m.total_evaluations(), 32 * 8);
    }

    // ── Sampler dim → d derivation ──────────────────────────────────

    #[test]
    fn dim_is_half_of_sampler_dim() {
        for d in [1usize, 2, 5, 10, 50] {
            let s = LhsSampler::classic(2 * d);
            let mut rng = fresh_rng();
            let m = build_saltelli_matrix(&s, 16, false, &mut rng).unwrap();
            assert_eq!(m.dim, d);
            assert_eq!(m.a_b.len(), d);
        }
    }

    // ── Edge cases ──────────────────────────────────────────────────

    #[test]
    fn minimum_d_is_one_with_two_dim_sampler() {
        // sampler.dim() = 2 → d = 1. One factor.
        let s = LhsSampler::classic(2);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 32, false, &mut rng).unwrap();
        assert_eq!(m.dim, 1);
        assert_eq!(m.a.shape(), &[32, 1]);
        assert_eq!(m.a_b.len(), 1);
        // a_b[0] should equal b (full column replacement).
        assert_eq!(m.a_b[0], m.b);
    }

    #[test]
    fn n_one_returns_one_row_matrices() {
        let s = LhsSampler::classic(4);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 1, false, &mut rng).unwrap();
        assert_eq!(m.n, 1);
        assert_eq!(m.a.shape(), &[1, 2]);
    }

    #[test]
    fn large_n_and_d() {
        let s = SobolSampler::standard(20); // d = 10
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 1024, false, &mut rng).unwrap();
        assert_eq!(m.dim, 10);
        assert_eq!(m.a_b.len(), 10);
        assert_eq!(m.total_evaluations(), 1024 * 12);
    }

    // ── A and B independence ────────────────────────────────────────

    #[test]
    fn a_and_b_differ_when_sampler_produces_nontrivial_output() {
        // For any non-degenerate sampler with dim >= 2, A and B
        // (the two halves) should differ — otherwise the Saltelli
        // construction is meaningless.
        let s = LhsSampler::classic(4);
        let mut rng = fresh_rng();
        let m = build_saltelli_matrix(&s, 32, false, &mut rng).unwrap();
        assert_ne!(m.a, m.b);
    }
}
