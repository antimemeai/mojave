use crate::boundary::obf::normal_quantile;
use crate::boundary::wald;
use crate::error::SeqError;

/// Pocock boundary: find constant c such that P(reject at any of K looks) = alpha
/// under H0, where test statistic Z_k ~ N(0, 1) with Cov(Z_j, Z_k) = sqrt(j/k).
///
/// Uses bisection on the rejection probability.
pub fn boundary(total_looks: usize, alpha: f64) -> Result<f64, SeqError> {
    if total_looks == 0 {
        return Err(SeqError::InvalidLooks(0));
    }
    wald::validate_error_rates(alpha, 0.5)?;
    if total_looks == 1 {
        return Ok(normal_quantile(1.0 - alpha / 2.0));
    }
    // Bisection: find c such that rejection_prob(c, K) = alpha
    let mut lo = 0.5_f64;
    let mut hi = 6.0_f64;
    for _ in 0..100 {
        let mid = (lo + hi) / 2.0;
        let p = rejection_probability_mc(mid, total_looks, 200_000);
        if p > alpha {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok((lo + hi) / 2.0)
}

/// All K boundaries (they're all equal for Pocock).
pub fn boundaries(total_looks: usize, alpha: f64) -> Result<Vec<f64>, SeqError> {
    let c = boundary(total_looks, alpha)?;
    Ok(vec![c; total_looks])
}

/// Monte Carlo estimate of rejection probability for equal boundary c
/// with K equally-spaced looks.
fn rejection_probability_mc(c: f64, k: usize, n_reps: usize) -> f64 {
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use rand_distr::{Distribution, StandardNormal};

    let mut rng = StdRng::seed_from_u64(42);
    let normal = StandardNormal;
    let mut rejections = 0usize;

    for _ in 0..n_reps {
        let mut cumsum = 0.0_f64;
        let mut rejected = false;
        for look in 1..=k {
            let x: f64 = normal.sample(&mut rng);
            cumsum += x;
            let z = cumsum / (look as f64).sqrt();
            if z.abs() >= c {
                rejected = true;
                break;
            }
        }
        if rejected {
            rejections += 1;
        }
    }
    rejections as f64 / n_reps as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pocock_k1_equals_z_alpha_half() {
        let c = boundary(1, 0.05).unwrap();
        let z = normal_quantile(0.975);
        assert!(
            (c - z).abs() < 0.02,
            "K=1 Pocock should be z_0.975 ~ 1.96, got {c}"
        );
    }

    #[test]
    fn pocock_k2_known_value() {
        // Pocock 1977 Table 1: K=2, alpha=0.05 two-sided => c ~ 2.178
        let c = boundary(2, 0.05).unwrap();
        assert!((c - 2.178).abs() < 0.05, "K=2 Pocock ~ 2.178, got {c}");
    }

    #[test]
    fn pocock_k3_known_value() {
        // Pocock 1977 Table 1: K=3, alpha=0.05 two-sided => c ~ 2.289
        let c = boundary(3, 0.05).unwrap();
        assert!((c - 2.289).abs() < 0.05, "K=3 Pocock ~ 2.289, got {c}");
    }

    #[test]
    fn pocock_boundaries_all_equal() {
        let bs = boundaries(4, 0.05).unwrap();
        for b in &bs {
            assert!(
                (b - bs[0]).abs() < 1e-10,
                "Pocock boundaries should all be equal"
            );
        }
    }
}
