use crate::boundary::wald;
use crate::error::SeqError;

/// O'Brien-Fleming critical values at look k of K.
/// c_k = C * sqrt(K / k) where C = z_{alpha/2} for the overall alpha.
pub fn boundary(k: usize, total_looks: usize, alpha: f64) -> Result<f64, SeqError> {
    if total_looks == 0 {
        return Err(SeqError::InvalidLooks(0));
    }
    if k == 0 || k > total_looks {
        return Err(SeqError::LookOutOfRange {
            k,
            total: total_looks,
        });
    }
    wald::validate_error_rates(alpha, 0.5)?;
    let z = normal_quantile(1.0 - alpha / 2.0);
    Ok(z * (total_looks as f64 / k as f64).sqrt())
}

/// All K boundaries at once.
pub fn boundaries(total_looks: usize, alpha: f64) -> Result<Vec<f64>, SeqError> {
    (1..=total_looks)
        .map(|k| boundary(k, total_looks, alpha))
        .collect()
}

/// Standard normal quantile (inverse CDF) via rational approximation.
/// Abramowitz & Stegun 26.2.23, accurate to ~4.5e-4.
pub(crate) fn normal_quantile(p: f64) -> f64 {
    if p <= 0.0 || p >= 1.0 {
        if p <= 0.0 {
            return f64::NEG_INFINITY;
        }
        return f64::INFINITY;
    }
    if (p - 0.5).abs() < f64::EPSILON {
        return 0.0;
    }
    let sign;
    let pp;
    if p < 0.5 {
        pp = p;
        sign = -1.0;
    } else {
        pp = 1.0 - p;
        sign = 1.0;
    }
    let t = (-2.0 * pp.ln()).sqrt();
    let c0 = 2.515_517;
    let c1 = 0.802_853;
    let c2 = 0.010_328;
    let d1 = 1.432_788;
    let d2 = 0.189_269;
    let d3 = 0.001_308;
    sign * (t - (c0 + c1 * t + c2 * t * t) / (1.0 + d1 * t + d2 * t * t + d3 * t * t * t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obf_at_k_equals_total_is_nearly_z_alpha_half() {
        let z = normal_quantile(0.975);
        let b = boundary(5, 5, 0.05).unwrap();
        assert!(
            (b - z).abs() < 0.01,
            "at final look OBF ~ z_alpha/2, got {b} vs {z}"
        );
    }

    #[test]
    fn obf_boundaries_decrease() {
        let bs = boundaries(5, 0.05).unwrap();
        for i in 1..bs.len() {
            assert!(
                bs[i] <= bs[i - 1],
                "OBF boundaries should decrease: {} > {}",
                bs[i],
                bs[i - 1]
            );
        }
    }

    #[test]
    fn obf_at_k1_equals_single_look() {
        let b = boundary(1, 1, 0.05).unwrap();
        let z = normal_quantile(0.975);
        assert!((b - z).abs() < 0.01);
    }

    #[test]
    fn look_out_of_range() {
        assert!(matches!(
            boundary(0, 5, 0.05),
            Err(SeqError::LookOutOfRange { .. })
        ));
        assert!(matches!(
            boundary(6, 5, 0.05),
            Err(SeqError::LookOutOfRange { .. })
        ));
    }
}
