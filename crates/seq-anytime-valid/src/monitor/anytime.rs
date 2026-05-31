use crate::error::SeqError;
use crate::types::{DataFamily, EvidenceSnapshot, MsprtConfig};

pub struct AnytimeMonitor {
    theta_0: f64,
    mixing_variance: f64,
    alpha: f64,
    data_family: DataFamily,
    n: usize,
    /// Running sum of (x_i - theta_0).
    running_sum: f64,
    /// Running sum of x_i (for confidence sequence).
    raw_sum: f64,
    /// Running sum of (x_i - running_mean)^2 via Welford's algorithm (M2).
    welford_m2: f64,
}

impl AnytimeMonitor {
    pub fn new(config: MsprtConfig, alpha: f64) -> Result<Self, SeqError> {
        if config.mixing_variance <= 0.0 {
            return Err(SeqError::InvalidMixingVariance(config.mixing_variance));
        }
        if alpha <= 0.0 || alpha >= 1.0 {
            return Err(SeqError::InvalidAlpha(alpha));
        }
        Ok(Self {
            theta_0: config.theta_0,
            mixing_variance: config.mixing_variance,
            alpha,
            data_family: config.family,
            n: 0,
            running_sum: 0.0,
            raw_sum: 0.0,
            welford_m2: 0.0,
        })
    }

    pub fn update(&mut self, observation: f64) -> Result<EvidenceSnapshot, SeqError> {
        if !observation.is_finite() {
            return Err(SeqError::NonFiniteInput(observation));
        }

        // Update running statistics (Welford's online algorithm for variance)
        let old_mean = if self.n > 0 {
            self.raw_sum / self.n as f64
        } else {
            0.0
        };
        self.n += 1;
        self.running_sum += observation - self.theta_0;
        self.raw_sum += observation;
        let new_mean = self.raw_sum / self.n as f64;
        self.welford_m2 += (observation - old_mean) * (observation - new_mean);

        let n_f = self.n as f64;
        let tau_sq = self.mixing_variance;

        // mSPRT log-LR: -0.5*ln(1 + n*tau^2) + n^2*xbar^2*tau^2 / (2*(1 + n*tau^2))
        // where xbar = running_sum / n (centered at theta_0)
        let x_bar = self.running_sum / n_f;
        let denom = 1.0 + n_f * tau_sq;
        let log_lr = -0.5 * denom.ln() + n_f.powi(2) * x_bar.powi(2) * tau_sq / (2.0 * denom);

        // Always-valid p-value: min(1, 1/Lambda_n)
        let lr = log_lr.exp();
        let avp = (1.0 / lr).min(1.0);

        // Confidence sequence: xbar ± sigma * sqrt(2*(1+1/n)*ln(sqrt(n+1)/alpha)/n)
        //
        // Sigma selection dispatches on DataFamily:
        //   - Bernoulli: sigma=0.5 (conservative upper bound; max std dev for Bernoulli)
        //   - Normal(known_variance=Some(v)): sigma = sqrt(v)
        //   - Normal(known_variance=None): Welford online estimate (no anytime-valid guarantee)
        let sigma = match self.data_family {
            DataFamily::Bernoulli => 0.5,
            DataFamily::Normal {
                known_variance: Some(v),
            } => v.sqrt().max(1e-10),
            DataFamily::Normal {
                known_variance: None,
            } => {
                let variance = if self.n > 1 {
                    self.welford_m2 / n_f
                } else {
                    0.0
                };
                variance.sqrt().max(1e-10)
            }
        };
        let width =
            sigma * (2.0 * (1.0 + 1.0 / n_f) * ((n_f + 1.0).sqrt() / self.alpha).ln() / n_f).sqrt();
        let raw_mean = self.raw_sum / n_f;
        let cs = (raw_mean - width, raw_mean + width);

        Ok(EvidenceSnapshot {
            log_likelihood_ratio: log_lr,
            n_observations: self.n,
            always_valid_p: Some(avp),
            confidence_interval: Some(cs),
            e_value: Some(log_lr.exp()),
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn anytime_monitor_tracks_evidence() {
        let config = MsprtConfig {
            theta_0: 0.0,
            mixing_variance: 1.0,
            family: DataFamily::Normal {
                known_variance: Some(1.0),
            },
            max_samples: None,
        };
        let mut monitor = AnytimeMonitor::new(config, 0.05).unwrap();
        let snap1 = monitor.update(2.0).unwrap();
        let snap2 = monitor.update(2.0).unwrap();
        assert_eq!(snap2.n_observations, 2);
        assert!(
            snap2.always_valid_p.unwrap() <= snap1.always_valid_p.unwrap(),
            "p-value should decrease with consistent evidence"
        );
    }

    #[test]
    fn invalid_mixing_variance_rejected() {
        let config = MsprtConfig {
            theta_0: 0.0,
            mixing_variance: -1.0,
            family: DataFamily::Normal {
                known_variance: Some(1.0),
            },
            max_samples: None,
        };
        assert!(matches!(
            AnytimeMonitor::new(config, 0.05),
            Err(SeqError::InvalidMixingVariance(_))
        ));
    }

    #[test]
    fn invalid_alpha_rejected() {
        let config = MsprtConfig {
            theta_0: 0.0,
            mixing_variance: 1.0,
            family: DataFamily::Normal {
                known_variance: Some(1.0),
            },
            max_samples: None,
        };
        assert!(matches!(
            AnytimeMonitor::new(config, 1.5),
            Err(SeqError::InvalidAlpha(_))
        ));
    }

    #[test]
    fn non_finite_observation_rejected() {
        let config = MsprtConfig {
            theta_0: 0.0,
            mixing_variance: 1.0,
            family: DataFamily::Normal {
                known_variance: Some(1.0),
            },
            max_samples: None,
        };
        let mut monitor = AnytimeMonitor::new(config, 0.05).unwrap();
        assert!(matches!(
            monitor.update(f64::NAN),
            Err(SeqError::NonFiniteInput(_))
        ));
    }
}
