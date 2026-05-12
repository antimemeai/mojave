//! Owen 2013 three-vector sampling design for the small-`Sᵢ`-regime
//! Sobol' estimator.
//!
//! Per Owen 2013 ("Better estimation of small Sobol' sensitivity
//! indices", ACM TOMACS). Owen's "Correlation 2" estimator uses
//! **three** independent random vectors `(x, y, z)` instead of the
//! two `(A, B)` used by Saltelli 2010. This module provides the
//! corresponding sampling design — `(A, B, C, A_Cⁱ, B_Aⁱ)`.
//!
//! # The matrix
//!
//! For `d` factors and `n` samples:
//!
//! - `A`, `B`, `C` — three independent `(n, d)` random sample
//!   matrices. Drawn from a single `(n, 3d)` sampler call and split
//!   into thirds.
//! - `A_Cⁱ` — `A` with column `i` replaced by `C`'s column `i`. One
//!   per factor.
//! - `B_Aⁱ` — `B` with column `i` replaced by `A`'s column `i`. One
//!   per factor.
//!
//! Total model evaluations consumed by the estimator:
//! `n · (3 + 2d)` (`A`, `B`, `C` baseline + `A_Cⁱ` and `B_Aⁱ` per
//! factor). For comparison, `SaltelliMatrix` requires `n · (d + 2)`.
//! Owen pays ~2× the model-eval cost for substantially tighter MC
//! variance on small `Sᵢ` factors (Owen 2013 § 5: `O(ε⁴)` vs
//! `O(ε²)` variance in the "total insensitivity" limit).
//!
//! # Why a separate type from `SaltelliMatrix`
//!
//! Owen's third matrix `C` and the `A_Cⁱ` / `B_Aⁱ` hybrids do not
//! map cleanly onto the `(A, B, A_Bⁱ)` Saltelli design. Mixing them
//! in one struct via optional fields (`C: Option<Array2<f64>>`,
//! `a_c: Option<Vec<Array2<f64>>>`) is feasible but muddles type
//! semantics — code consuming a `SaltelliMatrix` shouldn't need to
//! handle the Owen-only branches. Cleaner: separate `OwenMatrix`
//! type with its own constructor.
//!
//! # Determinism
//!
//! `build_owen_matrix` calls the sampler **once** with `dim = 3·d`
//! and partitions the output into `A | B | C`. Same `RngState` in →
//! bit-identical matrix out.

#![allow(clippy::many_single_char_names, clippy::similar_names)]

use ndarray::Array2;
use salib_core::RngState;

use crate::sampler::Sampler;

/// Owen 2013 three-vector sampling design for the
/// `estimate_owen` first-order Sobol' estimator.
///
/// `#[non_exhaustive]` — future fields (`recorded_rng_state` for
/// audit replay) land non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct OwenMatrix {
    /// Number of samples per matrix.
    pub n: usize,
    /// Number of factors.
    pub dim: usize,
    /// First base random matrix `(n, d)`.
    pub a: Array2<f64>,
    /// Second base random matrix `(n, d)`, independent of `A`.
    pub b: Array2<f64>,
    /// Third base random matrix `(n, d)`, independent of `A` and
    /// `B`. Owen's "z" series.
    pub c: Array2<f64>,
    /// Hybrid: `A` with column `i` replaced by `C`'s column `i`.
    /// Length `d`; `a_c[i]` has shape `(n, d)`.
    pub a_c: Vec<Array2<f64>>,
    /// Hybrid: `B` with column `i` replaced by `A`'s column `i`.
    /// Length `d`.
    pub b_a: Vec<Array2<f64>>,
}

impl OwenMatrix {
    /// Total model evaluations needed by `estimate_owen`:
    /// `n · (3 + 2d)`.
    #[must_use]
    pub fn total_evaluations(&self) -> usize {
        self.n * (3 + 2 * self.dim)
    }
}

/// Errors from [`build_owen_matrix`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum OwenMatrixError {
    #[error("Owen: n must be ≥ 1, got 0")]
    ZeroN,
    #[error("Owen: sampler dim {dim} is not divisible by 3 (expected 3·d)")]
    NotDivisibleByThree { dim: usize },
}

/// Build an Owen three-vector sampling matrix for a `d`-factor
/// problem with `n` samples per matrix.
///
/// `sampler.dim()` must equal `3 · d` — the sampler delivers
/// `(n, 3d)` unit-cube samples in a single call, partitioned into
/// `(A | B | C)`. This guarantees that `A`, `B`, `C` are statistically
/// independent under the sampler's design (LHS / Sobol' / etc.).
///
/// # Errors
///
/// - [`OwenMatrixError::ZeroN`] if `n == 0`.
/// - [`OwenMatrixError::NotDivisibleByThree`] if the sampler's
///   dimension is not a multiple of 3.
pub fn build_owen_matrix(
    sampler: &dyn Sampler,
    n: usize,
    rng: &mut RngState,
) -> Result<OwenMatrix, OwenMatrixError> {
    if n == 0 {
        return Err(OwenMatrixError::ZeroN);
    }
    let total_dim = sampler.dim();
    if !total_dim.is_multiple_of(3) {
        return Err(OwenMatrixError::NotDivisibleByThree { dim: total_dim });
    }
    let d = total_dim / 3;

    // Single sampler call to keep A, B, C jointly produced under
    // the same RNG draw — preserves the sampler's stratification
    // properties (LHS) or low-discrepancy (Sobol').
    let big = sampler.unit_sample(n, rng);

    let mut a = Array2::<f64>::zeros((n, d));
    let mut b = Array2::<f64>::zeros((n, d));
    let mut c = Array2::<f64>::zeros((n, d));
    for i in 0..n {
        for j in 0..d {
            a[[i, j]] = big[[i, j]];
            b[[i, j]] = big[[i, j + d]];
            c[[i, j]] = big[[i, j + 2 * d]];
        }
    }

    let a_c: Vec<Array2<f64>> = (0..d)
        .map(|i| {
            let mut m = a.clone();
            for k in 0..n {
                m[[k, i]] = c[[k, i]];
            }
            m
        })
        .collect();

    let b_a: Vec<Array2<f64>> = (0..d)
        .map(|i| {
            let mut m = b.clone();
            for k in 0..n {
                m[[k, i]] = a[[k, i]];
            }
            m
        })
        .collect();

    Ok(OwenMatrix {
        n,
        dim: d,
        a,
        b,
        c,
        a_c,
        b_a,
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::LhsSampler;

    fn fresh_rng() -> RngState {
        RngState::from_seed([0x42; 32])
    }

    #[test]
    fn zero_n_errors() {
        let s = LhsSampler::classic(9); // 3·d = 9, d = 3
        let mut rng = fresh_rng();
        let err = build_owen_matrix(&s, 0, &mut rng).unwrap_err();
        assert_eq!(err, OwenMatrixError::ZeroN);
    }

    #[test]
    fn non_divisible_dim_errors() {
        let s = LhsSampler::classic(7); // 7 is not divisible by 3
        let mut rng = fresh_rng();
        let err = build_owen_matrix(&s, 64, &mut rng).unwrap_err();
        assert!(matches!(err, OwenMatrixError::NotDivisibleByThree { .. }));
    }

    #[test]
    fn output_shapes_correct() {
        let s = LhsSampler::classic(9); // 3·3
        let mut rng = fresh_rng();
        let m = build_owen_matrix(&s, 64, &mut rng).unwrap();
        assert_eq!(m.n, 64);
        assert_eq!(m.dim, 3);
        assert_eq!(m.a.shape(), &[64, 3]);
        assert_eq!(m.b.shape(), &[64, 3]);
        assert_eq!(m.c.shape(), &[64, 3]);
        assert_eq!(m.a_c.len(), 3);
        assert_eq!(m.b_a.len(), 3);
        for i in 0..3 {
            assert_eq!(m.a_c[i].shape(), &[64, 3]);
            assert_eq!(m.b_a[i].shape(), &[64, 3]);
        }
    }

    #[test]
    fn a_c_replaces_only_column_i() {
        let s = LhsSampler::classic(9);
        let mut rng = fresh_rng();
        let m = build_owen_matrix(&s, 32, &mut rng).unwrap();
        for i in 0..3 {
            for k in 0..32 {
                for j in 0..3 {
                    if j == i {
                        // Column i should match C.
                        assert_eq!(m.a_c[i][[k, j]], m.c[[k, j]]);
                    } else {
                        // Other columns should match A.
                        assert_eq!(m.a_c[i][[k, j]], m.a[[k, j]]);
                    }
                }
            }
        }
    }

    #[test]
    fn b_a_replaces_only_column_i() {
        let s = LhsSampler::classic(9);
        let mut rng = fresh_rng();
        let m = build_owen_matrix(&s, 32, &mut rng).unwrap();
        for i in 0..3 {
            for k in 0..32 {
                for j in 0..3 {
                    if j == i {
                        assert_eq!(m.b_a[i][[k, j]], m.a[[k, j]]);
                    } else {
                        assert_eq!(m.b_a[i][[k, j]], m.b[[k, j]]);
                    }
                }
            }
        }
    }

    #[test]
    fn total_evaluations_matches_owen_cost() {
        let s = LhsSampler::classic(9); // d=3
        let mut rng = fresh_rng();
        let m = build_owen_matrix(&s, 100, &mut rng).unwrap();
        // Owen cost: n · (3 + 2d) = 100 · 9 = 900.
        assert_eq!(m.total_evaluations(), 900);
    }

    #[test]
    fn determinism_same_seed_yields_identical_matrix() {
        let s = LhsSampler::classic(9);
        let mut rng_a = fresh_rng();
        let mut rng_b = fresh_rng();
        let m1 = build_owen_matrix(&s, 64, &mut rng_a).unwrap();
        let m2 = build_owen_matrix(&s, 64, &mut rng_b).unwrap();
        assert_eq!(m1.a, m2.a);
        assert_eq!(m1.b, m2.b);
        assert_eq!(m1.c, m2.c);
    }

    #[test]
    fn a_b_c_are_distinct_under_lhs() {
        // LHS gives independent uniform marginals across columns;
        // the three sub-matrices should not be element-wise equal.
        let s = LhsSampler::classic(9);
        let mut rng = fresh_rng();
        let m = build_owen_matrix(&s, 64, &mut rng).unwrap();
        assert_ne!(m.a, m.b);
        assert_ne!(m.b, m.c);
        assert_ne!(m.a, m.c);
    }
}
