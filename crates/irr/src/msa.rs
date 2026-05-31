//! MSA (Measurement System Analysis) gauge discrimination diagnostics.
//!
//! Implements the AIAG MSA Manual 4th edition formulas for evaluating
//! whether a measurement system (judge/rater) can distinguish between
//! performance levels:
//!
//! - **ndc** (Number of Distinct Categories): `floor(1.41 * sigma_parts / sigma_gauge_rr)`
//!   AIAG requires ndc >= 5 for an adequate measurement system.
//!
//! - **P/T ratio** (Precision-to-Tolerance): `6 * sigma_gauge_rr / tolerance`
//!   Typically P/T < 0.10 is excellent, < 0.30 is adequate, >= 0.30 is inadequate.
//!
//! These diagnostics answer "can the judge distinguish performance levels?" --
//! a question that IRR agreement statistics (kappa, alpha, etc.) cannot answer.

use serde::{Deserialize, Serialize};

/// Error type for MSA diagnostic computations.
#[derive(Debug, thiserror::Error)]
pub enum MsaError {
    #[error("sigma_parts must be positive, got {0}")]
    InvalidSigmaParts(f64),
    #[error("sigma_gauge_rr must be positive, got {0}")]
    InvalidSigmaGaugeRr(f64),
    #[error("tolerance must be positive, got {0}")]
    InvalidTolerance(f64),
}

/// Result of MSA gauge discrimination diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[must_use]
pub struct MsaDiagnostics {
    /// Number of distinct categories the gauge can discriminate.
    /// AIAG formula: floor(1.41 * sigma_parts / sigma_gauge_rr).
    pub ndc: usize,
    /// Whether the gauge meets AIAG ndc >= 5 criterion.
    pub ndc_adequate: bool,
}

/// Result of P/T ratio computation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[must_use]
pub struct PtRatioDiagnostics {
    /// Precision-to-tolerance ratio: 6 * sigma_gauge_rr / tolerance.
    pub p_t_ratio: f64,
    /// Whether the gauge is adequate (P/T < 0.30).
    pub pt_adequate: bool,
}

/// Compute the number of distinct categories (ndc) from AIAG MSA formula.
///
/// ndc = floor(1.41 * sigma_parts / sigma_gauge_rr)
///
/// AIAG MSA Manual 4th ed requires ndc >= 5 for the measurement system
/// to be considered adequate for process control.
///
/// # Arguments
/// - `sigma_parts`: standard deviation of part-to-part variation
/// - `sigma_gauge_rr`: standard deviation of gauge repeatability & reproducibility
pub fn ndc(sigma_parts: f64, sigma_gauge_rr: f64) -> Result<MsaDiagnostics, MsaError> {
    if sigma_parts <= 0.0 || !sigma_parts.is_finite() {
        return Err(MsaError::InvalidSigmaParts(sigma_parts));
    }
    if sigma_gauge_rr <= 0.0 || !sigma_gauge_rr.is_finite() {
        return Err(MsaError::InvalidSigmaGaugeRr(sigma_gauge_rr));
    }

    let ndc_val = (1.41 * sigma_parts / sigma_gauge_rr).floor() as usize;
    Ok(MsaDiagnostics {
        ndc: ndc_val,
        ndc_adequate: ndc_val >= 5,
    })
}

/// Compute the precision-to-tolerance ratio (P/T).
///
/// P/T = 6 * sigma_gauge_rr / tolerance
///
/// Thresholds (AIAG convention):
/// - P/T < 0.10: excellent
/// - P/T < 0.30: adequate
/// - P/T >= 0.30: inadequate
///
/// # Arguments
/// - `sigma_gauge_rr`: standard deviation of gauge repeatability & reproducibility
/// - `tolerance`: specification tolerance width (USL - LSL)
pub fn pt_ratio(sigma_gauge_rr: f64, tolerance: f64) -> Result<PtRatioDiagnostics, MsaError> {
    if sigma_gauge_rr <= 0.0 || !sigma_gauge_rr.is_finite() {
        return Err(MsaError::InvalidSigmaGaugeRr(sigma_gauge_rr));
    }
    if tolerance <= 0.0 || !tolerance.is_finite() {
        return Err(MsaError::InvalidTolerance(tolerance));
    }

    let p_t = 6.0 * sigma_gauge_rr / tolerance;
    Ok(PtRatioDiagnostics {
        p_t_ratio: p_t,
        pt_adequate: p_t < 0.30,
    })
}

/// Compute both ndc and P/T ratio in a single call.
///
/// Convenience function combining [`ndc`] and [`pt_ratio`].
pub fn msa_diagnostics(
    sigma_parts: f64,
    sigma_gauge_rr: f64,
    tolerance: f64,
) -> Result<(MsaDiagnostics, PtRatioDiagnostics), MsaError> {
    let ndc_result = ndc(sigma_parts, sigma_gauge_rr)?;
    let pt_result = pt_ratio(sigma_gauge_rr, tolerance)?;
    Ok((ndc_result, pt_result))
}
