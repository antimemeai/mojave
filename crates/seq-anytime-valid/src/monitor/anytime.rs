use crate::error::SeqError;
use crate::evidence::{confseq, msprt};
use crate::types::{EvidenceSnapshot, MsprtConfig};

pub struct AnytimeMonitor {
    theta_0: f64,
    mixing_variance: f64,
    alpha: f64,
    observations: Vec<f64>,
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
            observations: Vec::new(),
        })
    }

    pub fn update(&mut self, observation: f64) -> Result<EvidenceSnapshot, SeqError> {
        if !observation.is_finite() {
            return Err(SeqError::NonFiniteInput(observation));
        }
        self.observations.push(observation);

        let log_lr =
            msprt::gaussian_msprt_log_lr(&self.observations, self.theta_0, self.mixing_variance)?;
        let avp = msprt::always_valid_p(&self.observations, self.theta_0, self.mixing_variance)?;
        let cs = confseq::normal_mixture_cs(&self.observations, self.alpha)?;

        Ok(EvidenceSnapshot {
            log_likelihood_ratio: log_lr,
            n_observations: self.observations.len(),
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
    use crate::types::DataFamily;

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
