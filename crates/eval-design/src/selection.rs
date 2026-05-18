use rand::seq::SliceRandom as _;
use rand::{Rng as _, SeedableRng as _};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};

use crate::item_pool::{ItemId, ItemMetadata, ItemPool, PoolError};

#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum SelectionError {
    #[error(transparent)]
    Pool(#[from] PoolError),
    #[error("requested {requested} items but pool contains only {available}")]
    InsufficientItems { requested: usize, available: usize },
    #[error("no items remain after applying exposure control")]
    ExhaustedByExposureControl,
}

/// Controls how often individual items can appear across evaluation runs.
/// Prevents gaming by ensuring no single item becomes predictable.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ExposureControl {
    /// No exposure limit — any item can be selected any number of times.
    None,
    /// Hard cap: items with `exposure_count >= max` are excluded from selection.
    MaxExposures(u64),
    /// Sympson-Hetter style: items with exposure_count >= threshold have
    /// their selection probability reduced by the given factor (0.0..1.0).
    ConditionalProbability { threshold: u64, accept_rate: f64 },
}

/// A randomized item selection strategy that provides anti-gaming properties.
///
/// Core principle from game-theoretic eval design: if the evaluator's item
/// selection is deterministic, a model developer can optimize specifically
/// for those items. Randomized selection forces genuine capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SelectionStrategy {
    /// Uniform random: each item equally likely. Simplest anti-gaming baseline.
    UniformRandom {
        n_items: usize,
        exposure_control: ExposureControl,
    },
    /// Stratified random: ensures coverage across content domains.
    /// Selects `items_per_domain` items from each domain, then fills
    /// remaining slots uniformly from the full pool.
    StratifiedRandom {
        items_per_domain: usize,
        total_items: usize,
        exposure_control: ExposureControl,
    },
    /// Information-weighted: items with higher discrimination are more
    /// likely to be selected. Balances measurement efficiency against
    /// unpredictability.
    InformationWeighted {
        n_items: usize,
        exposure_control: ExposureControl,
    },
}

/// The result of item selection: which items were chosen and why.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionResult {
    pub selected_ids: Vec<ItemId>,
    pub seed: u64,
    pub strategy_name: String,
}

/// Select items from the pool using the given strategy and seed.
pub fn select(
    pool: &ItemPool,
    strategy: &SelectionStrategy,
    seed: u64,
) -> Result<SelectionResult, SelectionError> {
    let mut rng = ChaCha20Rng::seed_from_u64(seed);

    match strategy {
        SelectionStrategy::UniformRandom {
            n_items,
            exposure_control,
        } => {
            let eligible = eligible_items(pool.items(), *exposure_control, &mut rng);
            if eligible.len() < *n_items {
                return Err(SelectionError::InsufficientItems {
                    requested: *n_items,
                    available: eligible.len(),
                });
            }
            let mut indices: Vec<usize> = (0..eligible.len()).collect();
            indices.shuffle(&mut rng);
            let selected: Vec<ItemId> = indices[..*n_items]
                .iter()
                .map(|&i| eligible[i].id.clone())
                .collect();
            Ok(SelectionResult {
                selected_ids: selected,
                seed,
                strategy_name: "uniform-random".into(),
            })
        }
        SelectionStrategy::StratifiedRandom {
            items_per_domain,
            total_items,
            exposure_control,
        } => {
            let eligible = eligible_items(pool.items(), *exposure_control, &mut rng);
            let domains = pool.domains();
            let mut selected: Vec<ItemId> = Vec::new();

            for domain in &domains {
                let domain_items: Vec<&ItemMetadata> = eligible
                    .iter()
                    .filter(|i| i.content_domain == *domain)
                    .copied()
                    .collect();
                let take = (*items_per_domain).min(domain_items.len());
                let mut indices: Vec<usize> = (0..domain_items.len()).collect();
                indices.shuffle(&mut rng);
                for &idx in indices.iter().take(take) {
                    selected.push(domain_items[idx].id.clone());
                }
            }

            if selected.len() < *total_items {
                let remaining: Vec<&ItemMetadata> = eligible
                    .iter()
                    .filter(|i| !selected.contains(&i.id))
                    .copied()
                    .collect();
                let need = (*total_items - selected.len()).min(remaining.len());
                let mut indices: Vec<usize> = (0..remaining.len()).collect();
                indices.shuffle(&mut rng);
                for &idx in indices.iter().take(need) {
                    selected.push(remaining[idx].id.clone());
                }
            }

            selected.truncate(*total_items);

            if selected.is_empty() {
                return Err(SelectionError::ExhaustedByExposureControl);
            }

            Ok(SelectionResult {
                selected_ids: selected,
                seed,
                strategy_name: "stratified-random".into(),
            })
        }
        SelectionStrategy::InformationWeighted {
            n_items,
            exposure_control,
        } => {
            let eligible = eligible_items(pool.items(), *exposure_control, &mut rng);
            if eligible.is_empty() {
                return Err(SelectionError::ExhaustedByExposureControl);
            }
            if eligible.len() <= *n_items {
                let selected: Vec<ItemId> = eligible.iter().map(|i| i.id.clone()).collect();
                return Ok(SelectionResult {
                    selected_ids: selected,
                    seed,
                    strategy_name: "information-weighted".into(),
                });
            }

            let selected = weighted_sample_without_replacement(&eligible, *n_items, &mut rng);

            Ok(SelectionResult {
                selected_ids: selected,
                seed,
                strategy_name: "information-weighted".into(),
            })
        }
    }
}

fn eligible_items<'a>(
    items: &'a [ItemMetadata],
    control: ExposureControl,
    rng: &mut ChaCha20Rng,
) -> Vec<&'a ItemMetadata> {
    match control {
        ExposureControl::None => items.iter().collect(),
        ExposureControl::MaxExposures(max) => {
            items.iter().filter(|i| i.exposure_count < max).collect()
        }
        ExposureControl::ConditionalProbability {
            threshold,
            accept_rate,
        } => items
            .iter()
            .filter(|i| {
                if i.exposure_count < threshold {
                    true
                } else {
                    let r: f64 = rng.random();
                    r < accept_rate
                }
            })
            .collect(),
    }
}

fn weighted_sample_without_replacement(
    items: &[&ItemMetadata],
    n: usize,
    rng: &mut ChaCha20Rng,
) -> Vec<ItemId> {
    let mut weights: Vec<f64> = items
        .iter()
        .map(|i| i.discrimination.abs().max(0.01))
        .collect();
    let mut selected = Vec::with_capacity(n);
    let mut available: Vec<usize> = (0..items.len()).collect();

    for _ in 0..n {
        if available.is_empty() {
            break;
        }
        let total: f64 = available.iter().map(|&i| weights[i]).sum();
        if total <= 0.0 {
            break;
        }
        let mut r: f64 = rng.random::<f64>() * total;
        let mut chosen_pos = 0;
        for (pos, &idx) in available.iter().enumerate() {
            r -= weights[idx];
            if r <= 0.0 {
                chosen_pos = pos;
                break;
            }
        }
        let chosen_idx = available[chosen_pos];
        selected.push(items[chosen_idx].id.clone());
        weights[chosen_idx] = 0.0;
        available.swap_remove(chosen_pos);
    }

    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_pool::{ItemId, ItemMetadata, ItemPool};

    fn sample_pool() -> ItemPool {
        ItemPool::new(vec![
            ItemMetadata::new(ItemId::new("t1"), 0.5, 1.0, "math".into()),
            ItemMetadata::new(ItemId::new("t2"), 0.7, 1.2, "math".into()),
            ItemMetadata::new(ItemId::new("t3"), 0.3, 0.8, "code".into()),
            ItemMetadata::new(ItemId::new("t4"), 0.9, 1.5, "code".into()),
            ItemMetadata::new(ItemId::new("t5"), 0.6, 1.1, "reasoning".into()),
            ItemMetadata::new(ItemId::new("t6"), 0.4, 0.9, "reasoning".into()),
        ])
        .unwrap()
    }

    #[test]
    fn uniform_random_selects_n_items() {
        let pool = sample_pool();
        let result = select(
            &pool,
            &SelectionStrategy::UniformRandom {
                n_items: 3,
                exposure_control: ExposureControl::None,
            },
            42,
        )
        .unwrap();
        assert_eq!(result.selected_ids.len(), 3);
    }

    #[test]
    fn uniform_random_is_deterministic() {
        let pool = sample_pool();
        let strat = SelectionStrategy::UniformRandom {
            n_items: 3,
            exposure_control: ExposureControl::None,
        };
        let a = select(&pool, &strat, 42).unwrap();
        let b = select(&pool, &strat, 42).unwrap();
        assert_eq!(a.selected_ids, b.selected_ids);
    }

    #[test]
    fn uniform_random_varies_with_seed() {
        let pool = sample_pool();
        let strat = SelectionStrategy::UniformRandom {
            n_items: 3,
            exposure_control: ExposureControl::None,
        };
        let a = select(&pool, &strat, 1).unwrap();
        let b = select(&pool, &strat, 2).unwrap();
        assert_ne!(a.selected_ids, b.selected_ids);
    }

    #[test]
    fn uniform_random_insufficient_items() {
        let pool = sample_pool();
        let result = select(
            &pool,
            &SelectionStrategy::UniformRandom {
                n_items: 100,
                exposure_control: ExposureControl::None,
            },
            42,
        );
        assert!(matches!(
            result,
            Err(SelectionError::InsufficientItems { .. })
        ));
    }

    #[test]
    fn max_exposure_excludes_overexposed() {
        let mut items = vec![
            ItemMetadata::new(ItemId::new("t1"), 0.5, 1.0, "math".into()),
            ItemMetadata::new(ItemId::new("t2"), 0.7, 1.2, "math".into()),
        ];
        items[0].exposure_count = 5;
        let pool = ItemPool::new(items).unwrap();
        let result = select(
            &pool,
            &SelectionStrategy::UniformRandom {
                n_items: 1,
                exposure_control: ExposureControl::MaxExposures(5),
            },
            42,
        )
        .unwrap();
        assert_eq!(result.selected_ids, vec![ItemId::new("t2")]);
    }

    #[test]
    fn stratified_covers_all_domains() {
        let pool = sample_pool();
        let result = select(
            &pool,
            &SelectionStrategy::StratifiedRandom {
                items_per_domain: 1,
                total_items: 4,
                exposure_control: ExposureControl::None,
            },
            42,
        )
        .unwrap();
        let selected_domains: Vec<&str> = result
            .selected_ids
            .iter()
            .map(|id| pool.get(id).unwrap().content_domain.as_str())
            .collect();
        assert!(selected_domains.contains(&"math"));
        assert!(selected_domains.contains(&"code"));
        assert!(selected_domains.contains(&"reasoning"));
    }

    #[test]
    fn information_weighted_selects_n_items() {
        let pool = sample_pool();
        let result = select(
            &pool,
            &SelectionStrategy::InformationWeighted {
                n_items: 3,
                exposure_control: ExposureControl::None,
            },
            42,
        )
        .unwrap();
        assert_eq!(result.selected_ids.len(), 3);
    }

    #[test]
    fn information_weighted_is_deterministic() {
        let pool = sample_pool();
        let strat = SelectionStrategy::InformationWeighted {
            n_items: 3,
            exposure_control: ExposureControl::None,
        };
        let a = select(&pool, &strat, 42).unwrap();
        let b = select(&pool, &strat, 42).unwrap();
        assert_eq!(a.selected_ids, b.selected_ids);
    }

    #[test]
    fn information_weighted_favors_high_discrimination() {
        let items = vec![
            ItemMetadata::new(ItemId::new("low"), 0.5, 0.1, "x".into()),
            ItemMetadata::new(ItemId::new("high"), 0.5, 10.0, "x".into()),
        ];
        let pool = ItemPool::new(items).unwrap();
        let strat = SelectionStrategy::InformationWeighted {
            n_items: 1,
            exposure_control: ExposureControl::None,
        };
        let mut high_count = 0;
        for seed in 0..100 {
            let result = select(&pool, &strat, seed).unwrap();
            if result.selected_ids[0] == ItemId::new("high") {
                high_count += 1;
            }
        }
        assert!(
            high_count > 80,
            "high-discrimination item should be selected most often, got {high_count}/100"
        );
    }

    #[test]
    fn no_duplicate_selections() {
        let pool = sample_pool();
        let result = select(
            &pool,
            &SelectionStrategy::UniformRandom {
                n_items: 5,
                exposure_control: ExposureControl::None,
            },
            42,
        )
        .unwrap();
        let mut ids = result.selected_ids.clone();
        ids.sort_by(|a, b| a.0.cmp(&b.0));
        ids.dedup_by(|a, b| a.0 == b.0);
        assert_eq!(ids.len(), result.selected_ids.len());
    }
}
