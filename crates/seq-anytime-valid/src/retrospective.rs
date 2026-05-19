use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::error::SeqError;
use crate::monitor::bernoulli::BernoulliMonitor;
use crate::types::BernoulliMsprtConfig;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoppingTimeResult {
    pub stopped: bool,
    pub stopping_n: Option<usize>,
    pub final_p: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PermutationSummary {
    pub n_permutations: usize,
    pub n_stopped: usize,
    pub stopping_times: Vec<StoppingTimeResult>,
}

pub fn permutation_stopping_times(
    outcomes: &[f64],
    config: BernoulliMsprtConfig,
    alpha: f64,
    n_permutations: usize,
    seed: u64,
) -> Result<PermutationSummary, SeqError> {
    if outcomes.is_empty() {
        return Err(SeqError::EmptyObservations);
    }
    for &x in outcomes {
        if !x.is_finite() || !(0.0..=1.0).contains(&x) {
            return Err(SeqError::InvalidBernoulliObservation(x));
        }
    }

    let mut rng = StdRng::seed_from_u64(seed);
    let mut results = Vec::with_capacity(n_permutations);
    let mut n_stopped = 0;

    for _ in 0..n_permutations {
        let mut shuffled: Vec<f64> = outcomes.to_vec();
        shuffled.shuffle(&mut rng);

        let mut monitor = BernoulliMonitor::new(config.clone(), alpha)?;
        let mut stopped = false;
        let mut stopping_n = None;
        let mut final_p = 1.0;

        for (i, &obs) in shuffled.iter().enumerate() {
            let snap = monitor.update(obs)?;
            final_p = snap.always_valid_p.unwrap_or(1.0);
            if final_p <= alpha {
                stopped = true;
                stopping_n = Some(i + 1);
                break;
            }
        }

        if stopped {
            n_stopped += 1;
        }
        results.push(StoppingTimeResult {
            stopped,
            stopping_n,
            final_p,
        });
    }

    Ok(PermutationSummary {
        n_permutations,
        n_stopped,
        stopping_times: results,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn returns_correct_count() {
        let outcomes: Vec<f64> = [vec![1.0; 50], vec![0.0; 50]].concat();
        let config = BernoulliMsprtConfig {
            p0: 0.25,
            beta_a: 1.0,
            beta_b: 1.0,
            max_samples: None,
        };
        let result = permutation_stopping_times(&outcomes, config, 0.05, 100, 42).unwrap();
        assert_eq!(result.stopping_times.len(), 100);
        assert_eq!(result.n_permutations, 100);
    }

    #[test]
    fn strong_signal_mostly_stops() {
        let outcomes: Vec<f64> = [vec![1.0; 80], vec![0.0; 20]].concat();
        let config = BernoulliMsprtConfig {
            p0: 0.25,
            beta_a: 1.0,
            beta_b: 1.0,
            max_samples: None,
        };
        let result = permutation_stopping_times(&outcomes, config, 0.05, 100, 42).unwrap();
        assert!(
            result.n_stopped >= 95,
            "80% success rate should stop most permutations, got {}",
            result.n_stopped
        );
    }

    #[test]
    fn null_data_rarely_stops() {
        let outcomes: Vec<f64> = [vec![1.0; 25], vec![0.0; 75]].concat();
        let config = BernoulliMsprtConfig {
            p0: 0.25,
            beta_a: 1.0,
            beta_b: 1.0,
            max_samples: None,
        };
        let result = permutation_stopping_times(&outcomes, config, 0.05, 100, 42).unwrap();
        assert!(
            result.n_stopped < 10,
            "null data should rarely stop, got {}",
            result.n_stopped
        );
    }

    #[test]
    fn deterministic_with_same_seed() {
        let outcomes: Vec<f64> = [vec![1.0; 50], vec![0.0; 50]].concat();
        let config = BernoulliMsprtConfig {
            p0: 0.25,
            beta_a: 1.0,
            beta_b: 1.0,
            max_samples: None,
        };
        let r1 = permutation_stopping_times(&outcomes, config.clone(), 0.05, 50, 42).unwrap();
        let r2 = permutation_stopping_times(&outcomes, config, 0.05, 50, 42).unwrap();
        for (a, b) in r1.stopping_times.iter().zip(r2.stopping_times.iter()) {
            assert_eq!(a.stopped, b.stopped);
            assert_eq!(a.stopping_n, b.stopping_n);
        }
    }
}
