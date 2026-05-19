use crate::boundary::wald::{self, SprtBoundaries};
use crate::error::SeqError;
use crate::evidence::likelihood;
use crate::types::{DataFamily, Decision, EvidenceSnapshot, SprtConfig, SprtVariant};

pub struct SprtMonitor {
    config: SprtConfig,
    boundaries: SprtBoundaries,
    cumulative_log_lr: f64,
    n_observations: usize,
    decided: Option<Decision>,
    /// Tracks the boosted supermartingale value for `SprtVariant::Boosted`.
    /// `None` for non-boosted variants.
    boosted_process: Option<f64>,
}

impl SprtMonitor {
    pub fn new(config: SprtConfig) -> Result<Self, SeqError> {
        let boundaries = match config.variant {
            SprtVariant::Approximate => wald::approximate(config.alpha, config.beta)?,
            SprtVariant::Conservative => wald::conservative(config.alpha, config.beta)?,
            SprtVariant::Boosted => wald::conservative(config.alpha, config.beta)?,
        };
        if (config.theta_0 - config.theta_1).abs() < f64::EPSILON {
            return Err(SeqError::DegenerateHypotheses);
        }
        let boosted_process = match config.variant {
            SprtVariant::Boosted => Some(1.0),
            _ => None,
        };
        Ok(Self {
            config,
            boundaries,
            cumulative_log_lr: 0.0,
            n_observations: 0,
            decided: None,
            boosted_process,
        })
    }

    pub fn update(&mut self, observation: f64) -> Result<Decision, SeqError> {
        likelihood::validate_observation(observation)?;
        if let Some(d) = self.decided {
            return Ok(d);
        }
        let log_lr = match self.config.family {
            DataFamily::Bernoulli => {
                likelihood::bernoulli_log_lr(observation, self.config.theta_0, self.config.theta_1)?
            }
            DataFamily::Normal { known_variance } => {
                let sigma_sq = known_variance.unwrap_or(1.0);
                likelihood::normal_log_lr(
                    observation,
                    self.config.theta_0,
                    self.config.theta_1,
                    sigma_sq,
                )
            }
        };
        self.cumulative_log_lr += log_lr;
        self.n_observations += 1;

        // Boosted variant: apply truncation in ratio space (Fischer 2024).
        // Power-one test — only rejects, never accepts.
        if let Some(ref mut m_boost) = self.boosted_process {
            let lr_factor = log_lr.exp();
            let truncated =
                crate::boundary::boosted::truncation(lr_factor, *m_boost, self.config.alpha);
            *m_boost *= truncated;
            let decision = if *m_boost >= 1.0 / self.config.alpha {
                Decision::Reject
            } else {
                Decision::Continue
            };
            if decision != Decision::Continue {
                self.decided = Some(decision);
            }
            return Ok(decision);
        }

        let decision = if self.cumulative_log_lr >= self.boundaries.log_upper_a {
            Decision::Reject
        } else if self.cumulative_log_lr <= self.boundaries.log_lower_b {
            Decision::Accept
        } else {
            Decision::Continue
        };
        if decision != Decision::Continue {
            self.decided = Some(decision);
        }
        Ok(decision)
    }

    pub fn snapshot(&self) -> EvidenceSnapshot {
        EvidenceSnapshot {
            log_likelihood_ratio: self.cumulative_log_lr,
            n_observations: self.n_observations,
            always_valid_p: None,
            confidence_interval: None,
            e_value: None,
        }
    }
}

pub fn sprt_decide(config: &SprtConfig, observations: &[f64]) -> Result<Decision, SeqError> {
    likelihood::validate_observations(observations)?;
    let mut monitor = SprtMonitor::new(config.clone())?;
    let mut last_decision = Decision::Continue;
    for &obs in observations {
        last_decision = monitor.update(obs)?;
        if last_decision != Decision::Continue {
            return Ok(last_decision);
        }
    }
    Ok(last_decision)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn monitor_rejects_on_strong_evidence() {
        let config = SprtConfig {
            theta_0: 0.3,
            theta_1: 0.7,
            alpha: 0.05,
            beta: 0.10,
            variant: SprtVariant::Approximate,
            family: DataFamily::Bernoulli,
        };
        let mut monitor = SprtMonitor::new(config).unwrap();
        for _ in 0..50 {
            let d = monitor.update(1.0).unwrap();
            if d == Decision::Reject {
                return;
            }
        }
        panic!("expected rejection after 50 successes");
    }

    #[test]
    fn monitor_accepts_on_null_evidence() {
        let config = SprtConfig {
            theta_0: 0.3,
            theta_1: 0.7,
            alpha: 0.05,
            beta: 0.10,
            variant: SprtVariant::Approximate,
            family: DataFamily::Bernoulli,
        };
        let mut monitor = SprtMonitor::new(config).unwrap();
        for _ in 0..50 {
            let d = monitor.update(0.0).unwrap();
            if d == Decision::Accept {
                return;
            }
        }
        panic!("expected acceptance after 50 failures");
    }

    #[test]
    fn stateless_matches_stateful() {
        let config = SprtConfig {
            theta_0: 0.3,
            theta_1: 0.7,
            alpha: 0.05,
            beta: 0.10,
            variant: SprtVariant::Approximate,
            family: DataFamily::Bernoulli,
        };
        let obs: Vec<f64> = vec![1.0; 20];
        let stateless = sprt_decide(&config, &obs).unwrap();
        let mut monitor = SprtMonitor::new(config).unwrap();
        let mut stateful = Decision::Continue;
        for &o in &obs {
            stateful = monitor.update(o).unwrap();
            if stateful != Decision::Continue {
                break;
            }
        }
        assert_eq!(stateless, stateful);
    }
}
