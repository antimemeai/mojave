#![cfg(feature = "g-theory")]

use crate::types::{ControlLimits, SpcError};
use salib_estimators::GTheoryResult;

/// Convert G-theory variance components into SPC control limits.
///
/// The "universe score" standard error from G-theory:
///   σ² = σ²_pi/n_i + σ²_pr/n_r + σ²_pir/(n_i·n_r)
///
/// This excludes σ²_p (person variance = signal) and includes only
/// the error facets (noise floor). The caller provides `grand_mean`
/// from baseline runs.
#[allow(clippy::cast_precision_loss)]
pub fn control_limits_from_g_theory(
    result: &GTheoryResult,
    grand_mean: f64,
    n_items: usize,
    n_raters: usize,
) -> Result<ControlLimits, SpcError> {
    let ni = n_items as f64;
    let nr = n_raters as f64;
    let sigma_sq = result.sigma_pi / ni + result.sigma_pr / nr + result.sigma_pir / (ni * nr);
    if sigma_sq <= 0.0 {
        return Err(SpcError::NonPositiveSigma(sigma_sq.sqrt()));
    }
    ControlLimits::new(grand_mean, sigma_sq.sqrt())
}
