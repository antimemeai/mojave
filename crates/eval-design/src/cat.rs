use rand::SeedableRng as _;
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};

use crate::ability::{
    estimate, total_information_at, AbilityEstimate, AdminItem, EstimationMethod,
};
use crate::information::fisher_information;
use crate::item_pool::{ItemId, ItemMetadata, ItemPool};
use crate::selection::{ExposureControl, SelectionStrategy};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CatConfig {
    pub estimation_method: CatEstimationMethod,
    pub selection: CatSelection,
    pub stopping: StoppingRule,
    pub exposure_control: ExposureControl,
    pub seed: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CatEstimationMethod {
    Mle,
    Eap { prior_mean: f64, prior_sd: f64 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CatSelection {
    MaxInfo {
        oversample_k: usize,
    },
    Minimax {
        theta_min: f64,
        theta_max: f64,
        theta_grid: usize,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StoppingRule {
    pub max_items: usize,
    pub min_items: usize,
    pub se_threshold: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CatStatus {
    SelectNext,
    Terminated,
}

#[derive(Debug, Clone)]
pub struct CatSession {
    config: CatConfig,
    administered: Vec<AdminItem>,
    administered_ids: Vec<ItemId>,
    current_estimate: AbilityEstimate,
    status: CatStatus,
    rng: ChaCha20Rng,
    step_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatResult {
    pub theta: f64,
    pub standard_error: f64,
    pub n_items: usize,
    pub administered_ids: Vec<ItemId>,
    pub theta_history: Vec<f64>,
}

#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum CatError {
    #[error("no eligible items remain in pool")]
    PoolExhausted,
    #[error("session already terminated")]
    AlreadyTerminated,
    #[error("item {0:?} not found in pool")]
    ItemNotFound(ItemId),
}

impl CatSession {
    pub fn new(config: CatConfig) -> Self {
        let rng = ChaCha20Rng::seed_from_u64(config.seed);
        let method = to_estimation_method(config.estimation_method);
        let initial = estimate(&[], method);
        Self {
            config,
            administered: Vec::new(),
            administered_ids: Vec::new(),
            current_estimate: initial,
            status: CatStatus::SelectNext,
            rng,
            step_count: 0,
        }
    }

    pub fn status(&self) -> CatStatus {
        self.status
    }

    pub fn current_estimate(&self) -> &AbilityEstimate {
        &self.current_estimate
    }

    pub fn n_administered(&self) -> usize {
        self.administered.len()
    }

    pub fn administered_ids(&self) -> &[ItemId] {
        &self.administered_ids
    }

    /// Select the next item to administer from the pool.
    pub fn next_item(&mut self, pool: &ItemPool) -> Result<ItemId, CatError> {
        if self.status == CatStatus::Terminated {
            return Err(CatError::AlreadyTerminated);
        }

        let theta = self.current_estimate.theta;
        let eligible = self.eligible_items(pool);
        if eligible.is_empty() {
            return Err(CatError::PoolExhausted);
        }

        let selected_id = match self.config.selection {
            CatSelection::MaxInfo { oversample_k } => {
                self.select_max_info(&eligible, theta, oversample_k)
            }
            CatSelection::Minimax {
                theta_min,
                theta_max,
                theta_grid,
            } => self.select_minimax(&eligible, theta_min, theta_max, theta_grid),
        };

        Ok(selected_id)
    }

    /// Record a response to the given item and update the ability estimate.
    /// Returns the new status (SelectNext or Terminated).
    pub fn record_response(
        &mut self,
        pool: &ItemPool,
        item_id: &ItemId,
        response: f64,
    ) -> Result<CatStatus, CatError> {
        if self.status == CatStatus::Terminated {
            return Err(CatError::AlreadyTerminated);
        }

        let item = pool
            .get(item_id)
            .ok_or_else(|| CatError::ItemNotFound(item_id.clone()))?;

        self.administered.push(AdminItem {
            difficulty: item.difficulty,
            discrimination: item.discrimination,
            response,
        });
        self.administered_ids.push(item_id.clone());

        let method = to_estimation_method(self.config.estimation_method);
        self.current_estimate = estimate(&self.administered, method);
        self.step_count += 1;

        if self.should_stop() {
            self.status = CatStatus::Terminated;
        }

        Ok(self.status)
    }

    /// Get the final result. Only meaningful after termination.
    pub fn result(&self) -> CatResult {
        CatResult {
            theta: self.current_estimate.theta,
            standard_error: self.current_estimate.standard_error,
            n_items: self.administered.len(),
            administered_ids: self.administered_ids.clone(),
            theta_history: Vec::new(),
        }
    }

    fn should_stop(&self) -> bool {
        let n = self.administered.len();
        if n < self.config.stopping.min_items {
            return false;
        }
        if n >= self.config.stopping.max_items {
            return true;
        }
        self.current_estimate.standard_error <= self.config.stopping.se_threshold
    }

    fn eligible_items<'a>(&mut self, pool: &'a ItemPool) -> Vec<&'a ItemMetadata> {
        use rand::Rng as _;

        pool.items()
            .iter()
            .filter(|item| {
                // Not already administered
                if self.administered_ids.contains(&item.id) {
                    return false;
                }
                // Exposure control
                match self.config.exposure_control {
                    ExposureControl::None => true,
                    ExposureControl::MaxExposures(max) => item.exposure_count < max,
                    ExposureControl::ConditionalProbability {
                        threshold,
                        accept_rate,
                    } => {
                        if item.exposure_count < threshold {
                            true
                        } else {
                            let r: f64 = self.rng.random();
                            r < accept_rate
                        }
                    }
                }
            })
            .collect()
    }

    fn select_max_info(
        &mut self,
        eligible: &[&ItemMetadata],
        theta: f64,
        oversample_k: usize,
    ) -> ItemId {
        use rand::seq::SliceRandom as _;

        let mut scored: Vec<(usize, f64)> = eligible
            .iter()
            .enumerate()
            .map(|(idx, item)| (idx, fisher_information(theta, item)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let k = oversample_k.max(1).min(eligible.len());
        let mut top_k: Vec<usize> = scored[..k].iter().map(|(idx, _)| *idx).collect();
        top_k.shuffle(&mut self.rng);
        eligible[top_k[0]].id.clone()
    }

    fn select_minimax(
        &mut self,
        eligible: &[&ItemMetadata],
        theta_min: f64,
        theta_max: f64,
        theta_grid: usize,
    ) -> ItemId {
        use rand::seq::SliceRandom as _;

        let grid: Vec<f64> = if theta_grid <= 1 {
            vec![(theta_min + theta_max) / 2.0]
        } else {
            let step = (theta_max - theta_min) / (theta_grid - 1) as f64;
            (0..theta_grid)
                .map(|i| theta_min + step * i as f64)
                .collect()
        };

        // Current information at each grid point from already-administered items
        let base_info: Vec<f64> = grid
            .iter()
            .map(|&theta| total_information_at(theta, &self.administered))
            .collect();

        // Score each item by its minimum information gain across the grid
        let mut scored: Vec<(usize, f64)> = eligible
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let mut min_info = f64::INFINITY;
                for (g, &theta) in grid.iter().enumerate() {
                    let new_info = base_info[g] + fisher_information(theta, item);
                    if new_info < min_info {
                        min_info = new_info;
                    }
                }
                (idx, min_info)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Top-k randomization for game-theoretic unpredictability
        let k = 3_usize.min(eligible.len());
        let mut top_k: Vec<usize> = scored[..k].iter().map(|(idx, _)| *idx).collect();
        top_k.shuffle(&mut self.rng);
        eligible[top_k[0]].id.clone()
    }
}

/// Run a complete CAT session given a pool and pre-determined responses.
/// Useful for simulation and testing.
pub fn simulate_cat(
    pool: &ItemPool,
    config: CatConfig,
    responses: &[(ItemId, f64)],
) -> Result<CatResult, CatError> {
    let mut session = CatSession::new(config);
    for (item_id, response) in responses {
        if session.status() == CatStatus::Terminated {
            break;
        }
        session.record_response(pool, item_id, *response)?;
    }
    Ok(session.result())
}

/// Run a full adaptive CAT session where items are selected adaptively.
/// The `response_fn` is called with each selected item to get the response.
pub fn run_cat<F>(
    pool: &ItemPool,
    config: CatConfig,
    mut response_fn: F,
) -> Result<CatResult, CatError>
where
    F: FnMut(&ItemId, &ItemMetadata) -> f64,
{
    let mut session = CatSession::new(config);
    loop {
        if session.status() == CatStatus::Terminated {
            break;
        }
        let item_id = session.next_item(pool)?;
        let item = pool
            .get(&item_id)
            .ok_or_else(|| CatError::ItemNotFound(item_id.clone()))?;
        let response = response_fn(&item_id, item);
        session.record_response(pool, &item_id, response)?;
    }
    Ok(session.result())
}

fn to_estimation_method(m: CatEstimationMethod) -> EstimationMethod {
    match m {
        CatEstimationMethod::Mle => EstimationMethod::Mle,
        CatEstimationMethod::Eap {
            prior_mean,
            prior_sd,
        } => EstimationMethod::Eap {
            prior_mean,
            prior_sd,
        },
    }
}

/// Convenience: create a selection strategy suitable for the current CAT state.
/// This bridges CAT config into eval-design's static selection API.
pub fn cat_selection_strategy(
    config: &CatConfig,
    current_theta: f64,
    n_items: usize,
) -> SelectionStrategy {
    match config.selection {
        CatSelection::MaxInfo { oversample_k } => SelectionStrategy::MaxFisherInfo {
            n_items,
            theta: current_theta,
            oversample_k,
            exposure_control: config.exposure_control,
        },
        CatSelection::Minimax {
            theta_min,
            theta_max,
            theta_grid,
        } => SelectionStrategy::Minimax {
            n_items,
            theta_min,
            theta_max,
            theta_grid,
            exposure_control: config.exposure_control,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information::prob_correct;
    use crate::item_pool::{ItemId, ItemMetadata, ItemPool};

    fn test_pool() -> ItemPool {
        let items: Vec<ItemMetadata> = (0..20)
            .map(|i| {
                let difficulty = -2.0 + (i as f64) * 0.2;
                let discrimination = 1.0 + (i as f64) * 0.05;
                ItemMetadata::new(
                    ItemId::new(format!("cat{i}")),
                    difficulty,
                    discrimination,
                    "general".into(),
                )
                .unwrap()
            })
            .collect();
        ItemPool::new(items).unwrap()
    }

    fn default_config() -> CatConfig {
        CatConfig {
            estimation_method: CatEstimationMethod::Eap {
                prior_mean: 0.0,
                prior_sd: 1.0,
            },
            selection: CatSelection::MaxInfo { oversample_k: 3 },
            stopping: StoppingRule {
                max_items: 10,
                min_items: 3,
                se_threshold: 0.4,
            },
            exposure_control: ExposureControl::None,
            seed: 42,
        }
    }

    #[test]
    fn session_starts_in_select_next() {
        let session = CatSession::new(default_config());
        assert_eq!(session.status(), CatStatus::SelectNext);
        assert_eq!(session.n_administered(), 0);
    }

    #[test]
    fn next_item_returns_valid_item() {
        let pool = test_pool();
        let mut session = CatSession::new(default_config());
        let item_id = session.next_item(&pool).unwrap();
        assert!(pool.get(&item_id).is_some());
    }

    #[test]
    fn record_response_updates_estimate() {
        let pool = test_pool();
        let mut session = CatSession::new(default_config());
        let item_id = session.next_item(&pool).unwrap();
        session.record_response(&pool, &item_id, 1.0).unwrap();
        assert_eq!(session.n_administered(), 1);
        assert!(session.current_estimate().theta.is_finite());
    }

    #[test]
    fn session_terminates_at_max_items() {
        let pool = test_pool();
        let config = CatConfig {
            stopping: StoppingRule {
                max_items: 5,
                min_items: 1,
                se_threshold: 0.01, // impossibly tight — forces max_items stop
            },
            ..default_config()
        };
        let mut session = CatSession::new(config);
        for _ in 0..5 {
            let item_id = session.next_item(&pool).unwrap();
            session.record_response(&pool, &item_id, 1.0).unwrap();
        }
        assert_eq!(session.status(), CatStatus::Terminated);
        assert_eq!(session.n_administered(), 5);
    }

    #[test]
    fn session_terminates_on_se_threshold() {
        let pool = test_pool();
        let config = CatConfig {
            stopping: StoppingRule {
                max_items: 20,
                min_items: 3,
                se_threshold: 0.6, // generous — should stop early
            },
            ..default_config()
        };
        let mut session = CatSession::new(config);
        // Give mixed responses so MLE/EAP is well-defined
        let responses = [1.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0];
        for &r in &responses {
            if session.status() == CatStatus::Terminated {
                break;
            }
            let item_id = session.next_item(&pool).unwrap();
            session.record_response(&pool, &item_id, r).unwrap();
        }
        // Should have stopped before using all 20 items
        assert!(session.n_administered() < 20);
    }

    #[test]
    fn no_item_repeated() {
        let pool = test_pool();
        let mut session = CatSession::new(default_config());
        let mut seen = std::collections::HashSet::new();
        for _ in 0..10 {
            if session.status() == CatStatus::Terminated {
                break;
            }
            let item_id = session.next_item(&pool).unwrap();
            assert!(seen.insert(item_id.clone()), "item {item_id:?} repeated");
            session.record_response(&pool, &item_id, 1.0).unwrap();
        }
    }

    #[test]
    fn run_cat_with_known_ability() {
        let pool = test_pool();
        let true_theta = 0.5;
        let config = CatConfig {
            stopping: StoppingRule {
                max_items: 15,
                min_items: 5,
                se_threshold: 0.5,
            },
            ..default_config()
        };

        let result = run_cat(&pool, config, |_id, item| {
            let p = prob_correct(true_theta, item.difficulty, item.discrimination);
            if p > 0.5 {
                1.0
            } else {
                0.0
            }
        })
        .unwrap();

        // Estimate should be in the ballpark of true ability
        assert!(
            (result.theta - true_theta).abs() < 2.0,
            "expected θ near {true_theta}, got {}",
            result.theta
        );
        assert!(result.n_items >= 5);
    }

    #[test]
    fn run_cat_deterministic() {
        let pool = test_pool();
        let config = default_config();

        let result1 = run_cat(&pool, config.clone(), |_id, item| {
            if item.difficulty < 0.0 {
                1.0
            } else {
                0.0
            }
        })
        .unwrap();

        let result2 = run_cat(
            &pool,
            config,
            |_id, item| {
                if item.difficulty < 0.0 {
                    1.0
                } else {
                    0.0
                }
            },
        )
        .unwrap();

        assert_eq!(result1.administered_ids, result2.administered_ids);
        assert!((result1.theta - result2.theta).abs() < 1e-10);
    }

    #[test]
    fn minimax_selection_works() {
        let pool = test_pool();
        let config = CatConfig {
            selection: CatSelection::Minimax {
                theta_min: -2.0,
                theta_max: 2.0,
                theta_grid: 11,
            },
            ..default_config()
        };
        let result = run_cat(
            &pool,
            config,
            |_id, item| {
                if item.difficulty < 0.0 {
                    1.0
                } else {
                    0.0
                }
            },
        )
        .unwrap();
        assert!(result.n_items >= 3);
        assert!(result.theta.is_finite());
    }

    #[test]
    fn terminated_session_rejects_more_items() {
        let pool = test_pool();
        let config = CatConfig {
            stopping: StoppingRule {
                max_items: 3,
                min_items: 1,
                se_threshold: 0.001, // impossibly tight — forces max_items stop
            },
            ..default_config()
        };
        let mut session = CatSession::new(config);
        for _ in 0..3 {
            let item_id = session.next_item(&pool).unwrap();
            session.record_response(&pool, &item_id, 1.0).unwrap();
        }
        assert_eq!(session.status(), CatStatus::Terminated);
        assert!(session.next_item(&pool).is_err());
    }

    #[test]
    fn simulate_cat_respects_termination() {
        let pool = test_pool();
        let config = CatConfig {
            stopping: StoppingRule {
                max_items: 5,
                min_items: 1,
                se_threshold: 0.01,
            },
            ..default_config()
        };
        // Provide more responses than max_items
        let responses: Vec<(ItemId, f64)> = (0..20)
            .map(|i| {
                (
                    ItemId::new(format!("cat{i}")),
                    if i < 10 { 1.0 } else { 0.0 },
                )
            })
            .collect();
        let result = simulate_cat(&pool, config, &responses).unwrap();
        assert!(result.n_items <= 5);
    }
}
