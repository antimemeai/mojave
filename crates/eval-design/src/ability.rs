use crate::information::prob_correct;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EstimationMethod {
    Mle,
    Eap { prior_mean: f64, prior_sd: f64 },
}

#[derive(Debug, Clone)]
pub struct AbilityEstimate {
    pub theta: f64,
    pub standard_error: f64,
    pub method: EstimationMethod,
    pub n_items: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct AdminItem {
    pub difficulty: f64,
    pub discrimination: f64,
    pub response: f64,
}

const MLE_MAX_ITER: usize = 50;
const MLE_TOLERANCE: f64 = 1e-6;
const MLE_BOUNDS: (f64, f64) = (-6.0, 6.0);

const EAP_GRID_POINTS: usize = 61;
const EAP_GRID_RANGE: (f64, f64) = (-4.0, 4.0);

/// MLE via Newton-Raphson for 2PL IRT.
///
/// Returns (theta_hat, standard_error). If the likelihood is flat
/// (all correct or all incorrect with few items), returns None.
pub fn mle(items: &[AdminItem]) -> Option<AbilityEstimate> {
    if items.is_empty() {
        return None;
    }
    // All-correct or all-incorrect — MLE undefined (complete separation)
    let sum_resp: f64 = items.iter().map(|i| i.response).sum();
    if sum_resp < f64::EPSILON || (sum_resp - items.len() as f64).abs() < f64::EPSILON {
        return None;
    }

    let mut theta = 0.0;

    for _ in 0..MLE_MAX_ITER {
        let (first_deriv, neg_second_deriv) = derivatives(theta, items);

        if neg_second_deriv < f64::EPSILON {
            return None;
        }

        let delta = first_deriv / neg_second_deriv;
        theta += delta;
        theta = theta.clamp(MLE_BOUNDS.0, MLE_BOUNDS.1);

        if delta.abs() < MLE_TOLERANCE {
            let se = 1.0 / neg_second_deriv.sqrt();
            return Some(AbilityEstimate {
                theta,
                standard_error: se,
                method: EstimationMethod::Mle,
                n_items: items.len(),
            });
        }
    }

    // Didn't converge — return last estimate with SE
    let (_, neg_second_deriv) = derivatives(theta, items);
    let se = if neg_second_deriv > f64::EPSILON {
        1.0 / neg_second_deriv.sqrt()
    } else {
        f64::INFINITY
    };
    Some(AbilityEstimate {
        theta,
        standard_error: se,
        method: EstimationMethod::Mle,
        n_items: items.len(),
    })
}

/// EAP (Expected A Posteriori) via quadrature with normal prior.
///
/// Uses the log-sum-exp trick to prevent underflow when many items
/// cause log-posterior values to be extremely negative at most grid points.
pub fn eap(items: &[AdminItem], prior_mean: f64, prior_sd: f64) -> AbilityEstimate {
    let step = (EAP_GRID_RANGE.1 - EAP_GRID_RANGE.0) / (EAP_GRID_POINTS as f64 - 1.0);

    // First pass: compute log-posteriors and find the maximum
    let mut log_posteriors = [0.0_f64; EAP_GRID_POINTS];
    let mut max_log_posterior = f64::NEG_INFINITY;

    for (i, lp) in log_posteriors.iter_mut().enumerate() {
        let theta = EAP_GRID_RANGE.0 + step * i as f64;

        // Log-likelihood
        let log_l: f64 = items
            .iter()
            .map(|item| {
                let p = prob_correct(theta, item.difficulty, item.discrimination);
                let p_clamped = p.clamp(1e-15, 1.0 - 1e-15);
                item.response * p_clamped.ln() + (1.0 - item.response) * (1.0 - p_clamped).ln()
            })
            .sum();

        // Log-prior (normal)
        let z = (theta - prior_mean) / prior_sd;
        let log_prior = -0.5 * z * z;

        *lp = log_l + log_prior;
        if *lp > max_log_posterior {
            max_log_posterior = *lp;
        }
    }

    // Second pass: exponentiate with log-sum-exp trick (subtract max before exp)
    let mut numerator = 0.0;
    let mut denominator = 0.0;
    let mut second_moment = 0.0;

    for (i, lp) in log_posteriors.iter().enumerate() {
        let theta = EAP_GRID_RANGE.0 + step * i as f64;
        let w = (lp - max_log_posterior).exp();

        numerator += theta * w;
        second_moment += theta * theta * w;
        denominator += w;
    }

    if denominator < f64::MIN_POSITIVE {
        return AbilityEstimate {
            theta: prior_mean,
            standard_error: prior_sd,
            method: EstimationMethod::Eap {
                prior_mean,
                prior_sd,
            },
            n_items: items.len(),
        };
    }

    let theta_hat = numerator / denominator;
    let variance = (second_moment / denominator) - theta_hat * theta_hat;
    let se = if variance > 0.0 {
        variance.sqrt()
    } else {
        prior_sd
    };

    AbilityEstimate {
        theta: theta_hat,
        standard_error: se,
        method: EstimationMethod::Eap {
            prior_mean,
            prior_sd,
        },
        n_items: items.len(),
    }
}

/// Compute ability estimate using the specified method.
pub fn estimate(items: &[AdminItem], method: EstimationMethod) -> AbilityEstimate {
    match method {
        EstimationMethod::Mle => mle(items).unwrap_or_else(|| {
            // Fall back to EAP(0,1) when MLE is undefined
            eap(items, 0.0, 1.0)
        }),
        EstimationMethod::Eap {
            prior_mean,
            prior_sd,
        } => eap(items, prior_mean, prior_sd),
    }
}

/// Fisher information at theta from a set of administered items.
pub fn total_information_at(theta: f64, items: &[AdminItem]) -> f64 {
    items
        .iter()
        .map(|item| {
            // AdminItem discrimination is already validated at construction time
            // via ItemMetadata, so this will not fail for valid sessions.
            // Use the raw formula directly to avoid needing ItemMetadata construction.
            let p = prob_correct(theta, item.difficulty, item.discrimination);
            item.discrimination * item.discrimination * p * (1.0 - p)
        })
        .sum()
}

fn derivatives(theta: f64, items: &[AdminItem]) -> (f64, f64) {
    let mut first = 0.0;
    let mut neg_second = 0.0;
    for item in items {
        let p = prob_correct(theta, item.difficulty, item.discrimination);
        let a = item.discrimination;
        first += a * (item.response - p);
        neg_second += a * a * p * (1.0 - p);
    }
    (first, neg_second)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(difficulty: f64, discrimination: f64, response: f64) -> AdminItem {
        AdminItem {
            difficulty,
            discrimination,
            response,
        }
    }

    #[test]
    fn mle_empty_returns_none() {
        assert!(mle(&[]).is_none());
    }

    #[test]
    fn mle_single_correct_returns_none() {
        assert!(mle(&[item(0.0, 1.0, 1.0)]).is_none());
    }

    #[test]
    fn mle_mixed_responses_converges() {
        let items = vec![
            item(-1.0, 1.0, 1.0),
            item(0.0, 1.0, 1.0),
            item(1.0, 1.0, 0.0),
            item(2.0, 1.0, 0.0),
        ];
        let est = mle(&items).unwrap();
        // With these responses, ability should be near 0.5 (between items 2 and 3)
        assert!(est.theta > -1.0 && est.theta < 2.0);
        assert!(est.standard_error > 0.0);
        assert!(est.standard_error < 5.0);
    }

    #[test]
    fn mle_high_ability() {
        let items = vec![
            item(-2.0, 1.5, 1.0),
            item(-1.0, 1.5, 1.0),
            item(0.0, 1.5, 1.0),
            item(1.0, 1.5, 1.0),
            item(2.0, 1.5, 0.0),
            item(3.0, 1.5, 0.0),
        ];
        let est = mle(&items).unwrap();
        assert!(est.theta > 0.5 && est.theta < 3.0);
    }

    #[test]
    fn eap_with_no_items_returns_prior() {
        let est = eap(&[], 0.5, 1.0);
        assert!((est.theta - 0.5).abs() < 0.1);
        assert!((est.standard_error - 1.0).abs() < 0.1);
    }

    #[test]
    fn eap_shrinks_toward_prior() {
        let items = vec![item(0.0, 1.0, 1.0)];
        let est_zero_prior = eap(&items, 0.0, 1.0);
        let est_high_prior = eap(&items, 2.0, 1.0);
        // With same data but different prior, EAP should differ
        assert!(est_high_prior.theta > est_zero_prior.theta);
    }

    #[test]
    fn eap_converges_with_data() {
        let items = vec![
            item(-1.0, 1.5, 1.0),
            item(0.0, 1.5, 1.0),
            item(1.0, 1.5, 0.0),
            item(2.0, 1.5, 0.0),
        ];
        let est = eap(&items, 0.0, 1.0);
        // Should be near 0.5
        assert!(est.theta > -0.5 && est.theta < 1.5);
        // SE should shrink with more items
        assert!(est.standard_error < 1.0);
    }

    #[test]
    fn estimate_mle_fallback_to_eap() {
        // All correct with 1 item — MLE undefined, should fallback
        let items = vec![item(0.0, 1.0, 1.0)];
        let est = estimate(&items, EstimationMethod::Mle);
        // Should get an EAP estimate (fallback)
        assert!(est.theta.is_finite());
    }

    #[test]
    fn se_decreases_with_more_items() {
        let items_few = vec![item(0.0, 1.5, 1.0), item(1.0, 1.5, 0.0)];
        let items_many = vec![
            item(-1.0, 1.5, 1.0),
            item(0.0, 1.5, 1.0),
            item(0.5, 1.5, 1.0),
            item(1.0, 1.5, 0.0),
            item(1.5, 1.5, 0.0),
            item(2.0, 1.5, 0.0),
        ];
        let est_few = eap(&items_few, 0.0, 1.0);
        let est_many = eap(&items_many, 0.0, 1.0);
        assert!(est_many.standard_error < est_few.standard_error);
    }

    #[test]
    fn total_information_at_sums() {
        let items = vec![item(0.0, 2.0, 1.0), item(0.0, 1.0, 0.0)];
        let info = total_information_at(0.0, &items);
        // At theta=b, I = a^2/4. So 4/4 + 1/4 = 1.25
        assert!((info - 1.25).abs() < 1e-10);
    }
}
