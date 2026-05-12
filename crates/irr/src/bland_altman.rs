//! Bland-Altman limits of agreement analysis.
//!
//! Assesses agreement between two measurement methods via mean difference
//! and 95% limits of agreement (mean +/- 1.96 * SD).
//!
//! Reference: Bland & Altman (1986), "Statistical methods for assessing
//! agreement between two methods of clinical measurement", The Lancet.

/// Result of a Bland-Altman limits-of-agreement analysis.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[must_use]
pub struct BlandAltmanResult {
    /// Mean of the differences (x - y).
    pub mean_diff: f64,
    /// Standard deviation of the differences (sample SD, n-1 denominator).
    pub sd_diff: f64,
    /// Lower limit of agreement: mean_diff - 1.96 * sd_diff.
    pub lower_loa: f64,
    /// Upper limit of agreement: mean_diff + 1.96 * sd_diff.
    pub upper_loa: f64,
    /// Number of paired observations.
    pub n: usize,
}

/// Errors that can occur during Bland-Altman analysis.
#[derive(Debug, thiserror::Error)]
pub enum BlandAltmanError {
    /// Input vectors have different lengths.
    #[error("inputs must have equal length: got {len_x} and {len_y}")]
    LengthMismatch {
        /// Length of the first input.
        len_x: usize,
        /// Length of the second input.
        len_y: usize,
    },

    /// Fewer than 2 paired observations were provided.
    #[error("need at least 2 paired observations")]
    TooFewObservations,

    /// All differences are identical (zero variance), so LoA are undefined.
    #[error("zero variance in differences")]
    ZeroVariance,

    /// One or more input values are NaN or infinity.
    #[error("inputs must be finite (no NaN or infinity)")]
    NonFiniteInput,
}

/// Compute Bland-Altman limits of agreement for paired measurements.
///
/// # Algorithm
/// 1. Check lengths match, n >= 2.
/// 2. Compute differences: d_i = x_i - y_i.
/// 3. Compute mean of differences.
/// 4. Compute sample variance (n-1 denominator).
/// 5. If variance < 1e-15, return `ZeroVariance`.
/// 6. SD = sqrt(variance).
/// 7. Lower LoA = mean - 1.96 * SD.
/// 8. Upper LoA = mean + 1.96 * SD.
///
/// # Errors
/// - [`BlandAltmanError::LengthMismatch`] if `x.len() != y.len()`.
/// - [`BlandAltmanError::TooFewObservations`] if `n < 2`.
/// - [`BlandAltmanError::NonFiniteInput`] if any value is NaN or infinity.
/// - [`BlandAltmanError::ZeroVariance`] if all differences are equal.
pub fn agreement(x: &[f64], y: &[f64]) -> Result<BlandAltmanResult, BlandAltmanError> {
    let len_x = x.len();
    let len_y = y.len();

    if len_x != len_y {
        return Err(BlandAltmanError::LengthMismatch { len_x, len_y });
    }

    let n = len_x;
    if n < 2 {
        return Err(BlandAltmanError::TooFewObservations);
    }

    if x.iter().chain(y.iter()).any(|v| !v.is_finite()) {
        return Err(BlandAltmanError::NonFiniteInput);
    }

    // Compute differences
    let diffs: Vec<f64> = x.iter().zip(y.iter()).map(|(xi, yi)| xi - yi).collect();

    // Mean of differences
    let sum: f64 = diffs.iter().sum();
    let mean_diff = sum / n as f64;

    // Sample variance (n-1 denominator)
    let var: f64 = diffs.iter().map(|d| (d - mean_diff).powi(2)).sum::<f64>() / (n as f64 - 1.0);

    if var.abs() < 1e-15 {
        return Err(BlandAltmanError::ZeroVariance);
    }

    let sd_diff = var.sqrt();
    let lower_loa = mean_diff - 1.96 * sd_diff;
    let upper_loa = mean_diff + 1.96 * sd_diff;

    Ok(BlandAltmanResult {
        mean_diff,
        sd_diff,
        lower_loa,
        upper_loa,
        n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_mismatch() {
        let err = agreement(&[1.0, 2.0], &[1.0]).unwrap_err();
        assert!(matches!(err, BlandAltmanError::LengthMismatch { .. }));
    }

    #[test]
    fn too_few() {
        let err = agreement(&[1.0], &[2.0]).unwrap_err();
        assert!(matches!(err, BlandAltmanError::TooFewObservations));
    }

    #[test]
    fn zero_variance() {
        let err = agreement(&[1.0, 2.0, 3.0], &[2.0, 3.0, 4.0]).unwrap_err();
        assert!(matches!(err, BlandAltmanError::ZeroVariance));
    }
}
