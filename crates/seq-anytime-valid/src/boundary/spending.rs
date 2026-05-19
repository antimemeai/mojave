use crate::boundary::obf::normal_quantile;
use crate::boundary::wald;
use crate::error::SeqError;

/// Pocock-type spending function: alpha*(t) = alpha * ln(1 + (e-1)*t)
pub fn pocock_spending(t: f64, alpha: f64) -> f64 {
    alpha * (1.0 + (std::f64::consts::E - 1.0) * t).ln()
}

/// OBF-type spending function: alpha*(t) = 2 - 2*Phi(z_{alpha/2} / sqrt(t))
pub fn obf_spending(t: f64, alpha: f64) -> f64 {
    if t <= 0.0 {
        return 0.0;
    }
    let z = normal_quantile(1.0 - alpha / 2.0);
    2.0 * (1.0 - normal_cdf(z / t.sqrt()))
}

/// Compute incremental alpha spent at look k given information fractions.
/// Returns the boundary (z-value) at look k.
pub fn spending_boundary(
    k: usize,
    info_fractions: &[f64],
    alpha: f64,
    spending_fn: &dyn Fn(f64, f64) -> f64,
) -> Result<f64, SeqError> {
    if info_fractions.is_empty() {
        return Err(SeqError::InvalidLooks(0));
    }
    if k == 0 || k > info_fractions.len() {
        return Err(SeqError::LookOutOfRange {
            k,
            total: info_fractions.len(),
        });
    }
    wald::validate_error_rates(alpha, 0.5)?;

    let cumulative_k = spending_fn(info_fractions[k - 1], alpha);
    let cumulative_prev = if k > 1 {
        spending_fn(info_fractions[k - 2], alpha)
    } else {
        0.0
    };
    let incremental = (cumulative_k - cumulative_prev).clamp(f64::EPSILON, alpha);
    // Convert incremental alpha to z-boundary (two-sided).
    // If incremental is effectively zero (clamped to EPSILON), the boundary will be
    // very large but finite, preventing infinity from floating-point rounding.
    Ok(normal_quantile(1.0 - incremental / 2.0))
}

/// Standard normal CDF via error function approximation.
pub(crate) fn normal_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

/// Error function approximation (Abramowitz & Stegun 7.1.26).
fn erf(x: f64) -> f64 {
    let a1 = 0.254_829_592;
    let a2 = -0.284_496_736;
    let a3 = 1.421_413_741;
    let a4 = -1.453_152_027;
    let a5 = 1.061_405_429;
    let p = 0.327_591_1;
    let sign = if x >= 0.0 { 1.0 } else { -1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
    sign * y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pocock_spending_at_t1_equals_alpha() {
        let alpha = 0.05;
        let s = pocock_spending(1.0, alpha);
        assert!((s - alpha).abs() < 1e-10, "spending(1) = alpha, got {s}");
    }

    #[test]
    fn pocock_spending_at_t0_equals_zero() {
        let s = pocock_spending(0.0, 0.05);
        assert!((s - 0.0).abs() < 1e-10, "spending(0) = 0, got {s}");
    }

    #[test]
    fn obf_spending_at_t1_equals_alpha() {
        let alpha = 0.05;
        let s = obf_spending(1.0, alpha);
        assert!(
            (s - alpha).abs() < 0.001,
            "OBF spending(1) ~ alpha, got {s}"
        );
    }

    #[test]
    fn obf_spending_at_t0_equals_zero() {
        let s = obf_spending(0.0, 0.05);
        assert!((s - 0.0).abs() < 1e-10);
    }

    #[test]
    fn spending_is_monotone() {
        let alpha = 0.05;
        let mut prev = 0.0;
        for i in 1..=100 {
            let t = i as f64 / 100.0;
            let s = pocock_spending(t, alpha);
            assert!(s >= prev - 1e-10, "spending not monotone at t={t}");
            prev = s;
        }
    }

    #[test]
    fn spending_boundary_produces_valid_z() {
        let fractions = vec![0.25, 0.50, 0.75, 1.0];
        let b = spending_boundary(1, &fractions, 0.05, &pocock_spending).unwrap();
        assert!(
            b > 0.0 && b.is_finite(),
            "boundary should be positive finite, got {b}"
        );
    }
}
