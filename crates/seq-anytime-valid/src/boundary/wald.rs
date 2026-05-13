use crate::error::SeqError;

/// Wald (1945) SPRT boundary pair.
///
/// All four fields are stored so callers can work in either the
/// ratio space (`upper_a`, `lower_b`) or the log-ratio space
/// (`log_upper_a`, `log_lower_b`) without redundant recomputation.
#[derive(Debug, Clone, Copy)]
pub struct SprtBoundaries {
    /// Upper (rejection of H0) boundary in ratio space: A.
    pub upper_a: f64,
    /// Lower (acceptance of H0 / rejection of H1) boundary in ratio space: B.
    pub lower_b: f64,
    /// Natural log of `upper_a`.
    pub log_upper_a: f64,
    /// Natural log of `lower_b`.
    pub log_lower_b: f64,
}

/// Validate that `alpha` and `beta` are both in (0, 1) and that their
/// sum is strictly less than 1.
///
/// # Errors
/// - [`SeqError::InvalidAlpha`] — `alpha` is not in (0, 1).
/// - [`SeqError::InvalidBeta`]  — `beta`  is not in (0, 1).
/// - [`SeqError::AlphaBetaSum`] — `alpha + beta >= 1`.
pub fn validate_error_rates(alpha: f64, beta: f64) -> Result<(), SeqError> {
    if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
        return Err(SeqError::InvalidAlpha(alpha));
    }
    if !beta.is_finite() || beta <= 0.0 || beta >= 1.0 {
        return Err(SeqError::InvalidBeta(beta));
    }
    if alpha + beta >= 1.0 {
        return Err(SeqError::AlphaBetaSum);
    }
    Ok(())
}

/// Wald (1945) *approximate* SPRT boundaries.
///
/// ```text
/// A = (1 - beta) / alpha
/// B = beta / (1 - alpha)
/// ```
///
/// These are Wald's original approximation; actual Type-I and Type-II
/// error rates are at most `alpha` and `beta` respectively (the
/// inequality is due to the overshoot of the random walk past the
/// boundaries).
///
/// # Errors
/// See [`validate_error_rates`].
pub fn approximate(alpha: f64, beta: f64) -> Result<SprtBoundaries, SeqError> {
    validate_error_rates(alpha, beta)?;
    let upper_a = (1.0 - beta) / alpha;
    let lower_b = beta / (1.0 - alpha);
    Ok(SprtBoundaries {
        upper_a,
        lower_b,
        log_upper_a: upper_a.ln(),
        log_lower_b: lower_b.ln(),
    })
}

/// Wald (1945) *conservative* SPRT boundaries.
///
/// ```text
/// A = 1 / alpha
/// B = beta
/// ```
///
/// These guarantee the stated error rates without relying on the
/// approximation that the random walk hits the boundary exactly.
///
/// # Errors
/// See [`validate_error_rates`].
pub fn conservative(alpha: f64, beta: f64) -> Result<SprtBoundaries, SeqError> {
    validate_error_rates(alpha, beta)?;
    let upper_a = 1.0 / alpha;
    let lower_b = beta;
    Ok(SprtBoundaries {
        upper_a,
        lower_b,
        log_upper_a: upper_a.ln(),
        log_lower_b: lower_b.ln(),
    })
}
