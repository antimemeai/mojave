use crate::error::SeqError;
use crate::evidence::msprt;
use crate::types::{BernoulliMsprtConfig, EvidenceSnapshot};

pub struct BernoulliMonitor {
    p0: f64,
    beta_a: f64,
    beta_b: f64,
    alpha: f64,
    n: usize,
    successes: f64,
}

impl BernoulliMonitor {
    pub fn new(config: BernoulliMsprtConfig, alpha: f64) -> Result<Self, SeqError> {
        if config.p0 <= 0.0 || config.p0 >= 1.0 {
            return Err(SeqError::InvalidNullProportion(config.p0));
        }
        if config.beta_a <= 0.0 || config.beta_b <= 0.0 {
            return Err(SeqError::InvalidBetaParams {
                a: config.beta_a,
                b: config.beta_b,
            });
        }
        if alpha <= 0.0 || alpha >= 1.0 {
            return Err(SeqError::InvalidAlpha(alpha));
        }
        Ok(Self {
            p0: config.p0,
            beta_a: config.beta_a,
            beta_b: config.beta_b,
            alpha,
            n: 0,
            successes: 0.0,
        })
    }

    pub fn update(&mut self, observation: f64) -> Result<EvidenceSnapshot, SeqError> {
        if !observation.is_finite() || !(0.0..=1.0).contains(&observation) {
            return Err(SeqError::InvalidBernoulliObservation(observation));
        }

        self.n += 1;
        self.successes += observation;
        let failures = self.n as f64 - self.successes;

        let log_lr = msprt::lnbeta(self.successes + self.beta_a, failures + self.beta_b)
            - msprt::lnbeta(self.beta_a, self.beta_b)
            - self.successes * self.p0.ln()
            - failures * (1.0 - self.p0).ln();

        let lr = log_lr.exp();
        let avp = (1.0 / lr).min(1.0);

        Ok(EvidenceSnapshot {
            log_likelihood_ratio: log_lr,
            n_observations: self.n,
            always_valid_p: Some(avp),
            confidence_interval: None,
            e_value: Some(lr),
        })
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    pub fn n_observations(&self) -> usize {
        self.n
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn monitor_tracks_evidence() {
        let config = BernoulliMsprtConfig {
            p0: 0.25,
            beta_a: 1.0,
            beta_b: 1.0,
            max_samples: None,
        };
        let mut monitor = BernoulliMonitor::new(config, 0.05).unwrap();
        let snap1 = monitor.update(1.0).unwrap();
        assert_eq!(snap1.n_observations, 1);
        let snap2 = monitor.update(1.0).unwrap();
        assert_eq!(snap2.n_observations, 2);
        assert!(
            snap2.always_valid_p.unwrap() <= snap1.always_valid_p.unwrap(),
            "p should decrease with consistent successes"
        );
    }

    #[test]
    fn monitor_p_stays_high_under_null() {
        let config = BernoulliMsprtConfig {
            p0: 0.50,
            beta_a: 1.0,
            beta_b: 1.0,
            max_samples: None,
        };
        let mut monitor = BernoulliMonitor::new(config, 0.05).unwrap();
        for i in 0..100 {
            let obs = if i % 2 == 0 { 1.0 } else { 0.0 };
            monitor.update(obs).unwrap();
        }
        let snap = monitor.update(1.0).unwrap();
        assert!(
            snap.always_valid_p.unwrap() > 0.05,
            "p should stay high under null"
        );
    }

    #[test]
    fn monitor_rejects_invalid_observation() {
        let config = BernoulliMsprtConfig {
            p0: 0.25,
            beta_a: 1.0,
            beta_b: 1.0,
            max_samples: None,
        };
        let mut monitor = BernoulliMonitor::new(config, 0.05).unwrap();
        assert!(matches!(
            monitor.update(1.5),
            Err(SeqError::InvalidBernoulliObservation(_))
        ));
    }

    #[test]
    fn monitor_rejects_invalid_config() {
        let config = BernoulliMsprtConfig {
            p0: 0.0,
            beta_a: 1.0,
            beta_b: 1.0,
            max_samples: None,
        };
        assert!(matches!(
            BernoulliMonitor::new(config, 0.05),
            Err(SeqError::InvalidNullProportion(_))
        ));
    }
}
