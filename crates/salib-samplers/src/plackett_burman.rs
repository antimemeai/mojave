//! Plackett-Burman fractional factorial design sampler.
//!
//! Constructs two-level screening designs via cyclic shift of known
//! generating vectors (Plackett & Burman, 1946). Supports dimensions
//! 1..=23 (N up to 24 runs).
//!
//! The design matrix has entries in {-1, +1} and satisfies the
//! orthogonality property X'X = N·I for the main-effect columns.

use ndarray::Array2;
use thiserror::Error;

/// A Plackett-Burman two-level design matrix.
#[derive(Debug, Clone)]
pub struct PlackettBurmanDesign {
    /// N × d matrix with entries in {-1.0, +1.0}.
    pub matrix: Array2<f64>,
    /// Number of runs (rows). Always a multiple of 4.
    pub n_runs: usize,
    /// Number of factors (columns).
    pub dim: usize,
}

/// Errors from Plackett-Burman construction.
#[derive(Debug, Clone, Error)]
pub enum PbError {
    /// Dimension must be at least 1.
    #[error("dimension must be at least 1, got 0")]
    ZeroDim,
    /// Dimension exceeds the largest known generating vector (N=24 → d≤23).
    #[error("dimension {0} exceeds maximum supported 23")]
    DimTooLarge(usize),
}

/// Build a Plackett-Burman design for `dim` factors.
///
/// Returns an N × dim matrix where N = next multiple of 4 ≥ dim + 1.
/// All entries are in {-1.0, +1.0}.
pub fn build_plackett_burman(dim: usize) -> Result<PlackettBurmanDesign, PbError> {
    if dim == 0 {
        return Err(PbError::ZeroDim);
    }
    if dim > 23 {
        return Err(PbError::DimTooLarge(dim));
    }
    let n = next_multiple_of_4(dim + 1);
    let gen = generating_vector(n);
    let mut matrix = Array2::<f64>::zeros((n, dim));

    // First row: first `dim` elements of the generating vector.
    for j in 0..dim {
        matrix[[0, j]] = gen[j];
    }

    // Rows 1..N-1: cyclic left-shift of gen.
    for i in 1..(n - 1) {
        for j in 0..dim {
            matrix[[i, j]] = gen[(j + i) % (n - 1)];
        }
    }

    // Last row: all -1.
    for j in 0..dim {
        matrix[[n - 1, j]] = -1.0;
    }

    Ok(PlackettBurmanDesign {
        matrix,
        n_runs: n,
        dim,
    })
}

/// Smallest multiple of 4 that is ≥ `min_val`.
fn next_multiple_of_4(min_val: usize) -> usize {
    let rem = min_val % 4;
    if rem == 0 {
        min_val
    } else {
        min_val + (4 - rem)
    }
}

/// Known Plackett-Burman generating vectors for standard run counts.
///
/// Each vector has N-1 elements of {-1.0, +1.0}.
fn generating_vector(n: usize) -> Vec<f64> {
    let p = 1.0_f64;
    let m = -1.0_f64;
    match n {
        4 => vec![p, m, p],
        8 => vec![p, p, p, m, p, m, m],
        12 => vec![p, p, m, p, p, p, m, m, m, p, m],
        16 => vec![p, p, p, p, m, p, m, p, p, m, m, p, m, m, m],
        20 => vec![p, p, m, p, p, m, m, m, m, p, m, p, m, p, p, p, p, m, m],
        24 => vec![
            p, p, p, p, p, m, p, m, p, p, m, m, p, p, m, m, p, m, p, m, m, m, m,
        ],
        _ => panic!("no generating vector for N={n} (should be unreachable)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pb_dim3_gives_4_runs() {
        let pb = build_plackett_burman(3).unwrap();
        assert_eq!(pb.n_runs, 4);
        assert_eq!(pb.dim, 3);
        for &v in pb.matrix.iter() {
            assert!(v == 1.0 || v == -1.0);
        }
    }

    #[test]
    fn pb_orthogonality() {
        let pb = build_plackett_burman(7).unwrap();
        let xt = pb.matrix.t();
        let xtx = xt.dot(&pb.matrix);
        for i in 0..pb.dim {
            assert!((xtx[[i, i]] - pb.n_runs as f64).abs() < 1e-10);
        }
    }

    #[test]
    fn pb_zero_dim_error() {
        assert!(matches!(build_plackett_burman(0), Err(PbError::ZeroDim)));
    }

    #[test]
    fn pb_dim_too_large_error() {
        assert!(matches!(
            build_plackett_burman(24),
            Err(PbError::DimTooLarge(24))
        ));
    }

    #[test]
    fn pb_all_entries_are_plus_minus_one() {
        for d in 1..=23 {
            let pb = build_plackett_burman(d).unwrap();
            for &v in pb.matrix.iter() {
                assert!(v == 1.0 || v == -1.0, "dim={d}: got {v}");
            }
        }
    }

    #[test]
    fn pb_n_runs_is_correct() {
        // d=1 → N=4, d=3 → N=4, d=4 → N=8, d=7 → N=8, d=11 → N=12
        assert_eq!(build_plackett_burman(1).unwrap().n_runs, 4);
        assert_eq!(build_plackett_burman(3).unwrap().n_runs, 4);
        assert_eq!(build_plackett_burman(4).unwrap().n_runs, 8);
        assert_eq!(build_plackett_burman(7).unwrap().n_runs, 8);
        assert_eq!(build_plackett_burman(11).unwrap().n_runs, 12);
        assert_eq!(build_plackett_burman(15).unwrap().n_runs, 16);
        assert_eq!(build_plackett_burman(19).unwrap().n_runs, 20);
        assert_eq!(build_plackett_burman(23).unwrap().n_runs, 24);
    }
}
