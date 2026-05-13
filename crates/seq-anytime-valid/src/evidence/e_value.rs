use crate::error::SeqError;
use crate::types::Decision;

/// Product of independent e-values (optional continuation property).
pub fn product_e_value(e_values: &[f64]) -> Result<f64, SeqError> {
    if e_values.is_empty() {
        return Ok(1.0);
    }
    for &e in e_values {
        if !e.is_finite() || e < 0.0 {
            return Err(SeqError::NonFiniteInput(e));
        }
    }
    Ok(e_values.iter().product())
}

/// Convert e-value to conservative p-value: p = 1/E.
pub fn e_to_p(e_value: f64) -> f64 {
    if e_value <= 0.0 {
        return 1.0;
    }
    (1.0 / e_value).min(1.0)
}

/// Threshold decision: reject if E >= 1/alpha.
pub fn threshold_decision(e_value: f64, alpha: f64) -> Decision {
    let threshold = 1.0 / alpha;
    if e_value >= threshold {
        Decision::Reject
    } else {
        Decision::Continue
    }
}

/// Compute e-value from log-likelihood ratio (for simple H0).
pub fn lr_to_e_value(log_lr: f64) -> f64 {
    log_lr.exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_e_values() {
        let product = product_e_value(&[3.0, 4.0]).unwrap();
        assert!((product - 12.0).abs() < 1e-10);
    }

    #[test]
    fn e_to_p_conversion() {
        let p = e_to_p(25.0);
        assert!((p - 0.04).abs() < 1e-10);
    }

    #[test]
    fn threshold_rejects_above() {
        assert_eq!(threshold_decision(25.0, 0.05), Decision::Reject);
    }

    #[test]
    fn threshold_continues_below() {
        assert_eq!(threshold_decision(15.0, 0.05), Decision::Continue);
    }

    #[test]
    fn empty_product_is_one() {
        assert!((product_e_value(&[]).unwrap() - 1.0).abs() < 1e-10);
    }
}
