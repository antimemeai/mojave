use serde::{Deserialize, Serialize};

/// Named weight schemes for categorical agreement coefficients.
///
/// These follow the definitions in Gwet (2014) "Handbook of Inter-Rater Reliability"
/// and are consumed by AC2/AC3 weighted agreement computations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeightScheme {
    /// Unweighted (nominal): w[i][j] = 1 if i==j, 0 otherwise.
    Identity,
    /// Linear weights: w[i][j] = 1 - |i-j| / (q-1).
    Linear,
    /// Quadratic weights: w[i][j] = 1 - (i-j)^2 / (q-1)^2.
    Quadratic,
    /// Ordinal weights: w[i][j] = 1 - (n_between - 1) * n_between / (q * (q-1)),
    /// where n_between = |i - j| + 1.
    Ordinal,
}

/// A square, symmetric weight matrix over a set of ordered categories.
#[derive(Debug, Clone)]
pub struct WeightMatrix {
    /// Row-major weight values; `weights[i][j]` is the agreement weight
    /// between category `categories[i]` and category `categories[j]`.
    pub weights: Vec<Vec<f64>>,
    /// The ordered category labels this matrix spans.
    pub categories: Vec<u32>,
}

/// Errors arising from weight matrix construction or validation.
#[derive(Debug, thiserror::Error)]
pub enum WeightError {
    #[error("weight matrix must be square: got {rows}x{cols}")]
    NotSquare { rows: usize, cols: usize },
    #[error("weight matrix must be symmetric")]
    NotSymmetric,
    #[error("diagonal entries must be 1.0")]
    DiagonalNotOne,
    #[error("weights must be non-negative")]
    NegativeWeight,
    #[error("dimension mismatch: {n_cats} categories but {n_weights}x{n_weights} weights")]
    DimensionMismatch { n_cats: usize, n_weights: usize },
}

const TOL: f64 = 1e-12;

impl WeightMatrix {
    /// Build a weight matrix from a named scheme over the given categories.
    ///
    /// Categories are sorted internally; the ordering determines the distance
    /// metric used by Linear, Quadratic, and Ordinal schemes.
    pub fn from_scheme(categories: &[u32], scheme: WeightScheme) -> Self {
        let mut cats: Vec<u32> = categories.to_vec();
        cats.sort_unstable();
        cats.dedup();

        let q = cats.len();

        let weights = match scheme {
            WeightScheme::Identity => identity_weights(q),
            WeightScheme::Linear => linear_weights(q),
            WeightScheme::Quadratic => quadratic_weights(q),
            WeightScheme::Ordinal => ordinal_weights(q),
        };

        Self {
            weights,
            categories: cats,
        }
    }

    /// Build a weight matrix from a user-supplied matrix.
    ///
    /// Validates: square, symmetric (within 1e-12), diagonal = 1.0 (within 1e-12),
    /// all entries non-negative, and dimension matches `categories.len()`.
    pub fn custom(categories: &[u32], weights: Vec<Vec<f64>>) -> Result<Self, WeightError> {
        let rows = weights.len();
        // Check square
        for (i, row) in weights.iter().enumerate() {
            if row.len() != rows {
                return Err(WeightError::NotSquare {
                    rows,
                    cols: row.len(),
                });
            }
            // Check diagonal
            if (row[i] - 1.0).abs() > TOL {
                return Err(WeightError::DiagonalNotOne);
            }
        }

        // Check non-negative and symmetric
        for (i, row_i) in weights.iter().enumerate() {
            for (j, &w_ij) in row_i.iter().enumerate() {
                if w_ij < -TOL {
                    return Err(WeightError::NegativeWeight);
                }
                if (w_ij - weights[j][i]).abs() > TOL {
                    return Err(WeightError::NotSymmetric);
                }
            }
        }

        // Check dimension match
        let mut cats: Vec<u32> = categories.to_vec();
        cats.sort_unstable();
        cats.dedup();
        if cats.len() != rows {
            return Err(WeightError::DimensionMismatch {
                n_cats: cats.len(),
                n_weights: rows,
            });
        }

        Ok(Self {
            weights,
            categories: cats,
        })
    }
}

/// Identity (unweighted / nominal): w[i][j] = 1 if i==j, 0 otherwise.
fn identity_weights(q: usize) -> Vec<Vec<f64>> {
    let mut w = vec![vec![0.0; q]; q];
    for (i, row) in w.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    w
}

/// Linear weights: w[i][j] = 1 - |i-j| / (q-1).
/// For q == 1, all weights are 1.0 (single category).
fn linear_weights(q: usize) -> Vec<Vec<f64>> {
    if q <= 1 {
        return vec![vec![1.0; q]; q];
    }
    let denom = (q - 1) as f64;
    let mut w = vec![vec![0.0; q]; q];
    for (i, row) in w.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = 1.0 - (i.abs_diff(j) as f64) / denom;
        }
    }
    w
}

/// Quadratic weights: w[i][j] = 1 - (i-j)^2 / (q-1)^2.
/// For q == 1, all weights are 1.0.
fn quadratic_weights(q: usize) -> Vec<Vec<f64>> {
    if q <= 1 {
        return vec![vec![1.0; q]; q];
    }
    let denom = ((q - 1) as f64).powi(2);
    let mut w = vec![vec![0.0; q]; q];
    for (i, row) in w.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            let diff = (i as f64) - (j as f64);
            *cell = 1.0 - diff.powi(2) / denom;
        }
    }
    w
}

/// Ordinal weights: w[i][j] = 1 - (n_between - 1) * n_between / (q * (q-1)),
/// where n_between = |i - j| + 1 (number of categories between i and j inclusive).
/// For q == 1, all weights are 1.0.
fn ordinal_weights(q: usize) -> Vec<Vec<f64>> {
    if q <= 1 {
        return vec![vec![1.0; q]; q];
    }
    let denom = (q as f64) * ((q - 1) as f64);
    let mut w = vec![vec![0.0; q]; q];
    for (i, row) in w.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            let n_between = (i.abs_diff(j) + 1) as f64;
            *cell = 1.0 - (n_between - 1.0) * n_between / denom;
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_3x3() {
        let wm = WeightMatrix::from_scheme(&[1, 2, 3], WeightScheme::Identity);
        assert_eq!(wm.weights[0][0], 1.0);
        assert_eq!(wm.weights[0][1], 0.0);
        assert_eq!(wm.weights[1][2], 0.0);
        assert_eq!(wm.weights[2][2], 1.0);
    }

    #[test]
    fn linear_4_categories() {
        let wm = WeightMatrix::from_scheme(&[1, 2, 3, 4], WeightScheme::Linear);
        // q = 4, denom = 3
        assert!((wm.weights[0][0] - 1.0).abs() < 1e-15);
        assert!((wm.weights[0][1] - 2.0 / 3.0).abs() < 1e-15);
        assert!((wm.weights[0][2] - 1.0 / 3.0).abs() < 1e-15);
        assert!((wm.weights[0][3] - 0.0).abs() < 1e-15);
    }

    #[test]
    fn quadratic_4_categories() {
        let wm = WeightMatrix::from_scheme(&[1, 2, 3, 4], WeightScheme::Quadratic);
        // q = 4, denom = 9
        assert!((wm.weights[0][0] - 1.0).abs() < 1e-15);
        assert!((wm.weights[0][1] - (1.0 - 1.0 / 9.0)).abs() < 1e-15);
        assert!((wm.weights[0][2] - (1.0 - 4.0 / 9.0)).abs() < 1e-15);
        assert!((wm.weights[0][3] - 0.0).abs() < 1e-15);
    }

    #[test]
    fn ordinal_4_categories() {
        let wm = WeightMatrix::from_scheme(&[1, 2, 3, 4], WeightScheme::Ordinal);
        // q = 4, denom = 12
        // w[0][0]: n_between=1, (0*1)/12 = 0 → 1.0
        assert!((wm.weights[0][0] - 1.0).abs() < 1e-15);
        // w[0][1]: n_between=2, (1*2)/12 = 2/12 → 1 - 1/6
        assert!((wm.weights[0][1] - (1.0 - 2.0 / 12.0)).abs() < 1e-15);
        // w[0][2]: n_between=3, (2*3)/12 = 6/12 → 0.5
        assert!((wm.weights[0][2] - 0.5).abs() < 1e-15);
        // w[0][3]: n_between=4, (3*4)/12 = 1.0 → 0.0
        assert!((wm.weights[0][3] - 0.0).abs() < 1e-15);
    }

    #[test]
    fn symmetry_all_schemes() {
        for scheme in [
            WeightScheme::Identity,
            WeightScheme::Linear,
            WeightScheme::Quadratic,
            WeightScheme::Ordinal,
        ] {
            let wm = WeightMatrix::from_scheme(&[1, 2, 3, 4, 5], scheme);
            let q = wm.categories.len();
            for i in 0..q {
                for j in 0..q {
                    assert!(
                        (wm.weights[i][j] - wm.weights[j][i]).abs() < 1e-15,
                        "scheme {:?} not symmetric at [{i}][{j}]",
                        scheme
                    );
                }
            }
        }
    }

    #[test]
    fn diagonal_all_ones() {
        for scheme in [
            WeightScheme::Identity,
            WeightScheme::Linear,
            WeightScheme::Quadratic,
            WeightScheme::Ordinal,
        ] {
            let wm = WeightMatrix::from_scheme(&[1, 2, 3, 4, 5], scheme);
            for i in 0..wm.categories.len() {
                assert!(
                    (wm.weights[i][i] - 1.0).abs() < 1e-15,
                    "scheme {:?} diagonal [{i}][{i}] != 1.0",
                    scheme
                );
            }
        }
    }

    #[test]
    fn single_category() {
        for scheme in [
            WeightScheme::Identity,
            WeightScheme::Linear,
            WeightScheme::Quadratic,
            WeightScheme::Ordinal,
        ] {
            let wm = WeightMatrix::from_scheme(&[42], scheme);
            assert_eq!(wm.weights, vec![vec![1.0]]);
        }
    }

    #[test]
    fn duplicate_categories_deduped() {
        let wm = WeightMatrix::from_scheme(&[3, 1, 2, 1, 3], WeightScheme::Linear);
        assert_eq!(wm.categories, vec![1, 2, 3]);
        assert_eq!(wm.weights.len(), 3);
    }

    #[test]
    fn custom_valid() {
        let w = vec![
            vec![1.0, 0.5, 0.0],
            vec![0.5, 1.0, 0.5],
            vec![0.0, 0.5, 1.0],
        ];
        let wm = WeightMatrix::custom(&[1, 2, 3], w).unwrap();
        assert_eq!(wm.categories, vec![1, 2, 3]);
        assert_eq!(wm.weights[0][1], 0.5);
    }

    #[test]
    fn custom_not_square() {
        let w = vec![vec![1.0, 0.5], vec![0.5, 1.0, 0.0]];
        let err = WeightMatrix::custom(&[1, 2], w).unwrap_err();
        assert!(matches!(err, WeightError::NotSquare { rows: 2, cols: 3 }));
    }

    #[test]
    fn custom_not_symmetric() {
        let w = vec![vec![1.0, 0.3], vec![0.5, 1.0]];
        let err = WeightMatrix::custom(&[1, 2], w).unwrap_err();
        assert!(matches!(err, WeightError::NotSymmetric));
    }

    #[test]
    fn custom_diagonal_not_one() {
        let w = vec![vec![0.9, 0.5], vec![0.5, 1.0]];
        let err = WeightMatrix::custom(&[1, 2], w).unwrap_err();
        assert!(matches!(err, WeightError::DiagonalNotOne));
    }

    #[test]
    fn custom_negative_weight() {
        let w = vec![vec![1.0, -0.1], vec![-0.1, 1.0]];
        let err = WeightMatrix::custom(&[1, 2], w).unwrap_err();
        assert!(matches!(err, WeightError::NegativeWeight));
    }

    #[test]
    fn custom_dimension_mismatch() {
        let w = vec![vec![1.0, 0.5], vec![0.5, 1.0]];
        let err = WeightMatrix::custom(&[1, 2, 3], w).unwrap_err();
        assert!(matches!(
            err,
            WeightError::DimensionMismatch {
                n_cats: 3,
                n_weights: 2
            }
        ));
    }

    #[test]
    fn weights_in_zero_one_range() {
        for scheme in [
            WeightScheme::Linear,
            WeightScheme::Quadratic,
            WeightScheme::Ordinal,
        ] {
            let wm = WeightMatrix::from_scheme(&[1, 2, 3, 4, 5], scheme);
            let q = wm.categories.len();
            for i in 0..q {
                for j in 0..q {
                    assert!(
                        wm.weights[i][j] >= -1e-15 && wm.weights[i][j] <= 1.0 + 1e-15,
                        "scheme {:?} weight [{i}][{j}] = {} out of [0,1]",
                        scheme,
                        wm.weights[i][j]
                    );
                }
            }
        }
    }
}
