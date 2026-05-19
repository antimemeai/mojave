use rand::seq::SliceRandom as _;
use rand::{Rng as _, SeedableRng as _};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};

use crate::information;
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ExposureControl {
    None,
    MaxExposures(u64),
    ConditionalProbability { threshold: u64, accept_rate: f64 },
}

/// The result of item selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionResult {
    pub selected_ids: Vec<ItemId>,
    pub seed: u64,
    pub strategy_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SelectionStrategy {
    /// Uniform random draw. Each eligible item equally likely.
    UniformRandom {
        n_items: usize,
        exposure_control: ExposureControl,
    },

    /// Stratified random: guarantee at least `items_per_domain` from each
    /// content domain, fill remainder uniformly. If total quota exceeds
    /// available items, selects as many as possible.
    StratifiedRandom {
        items_per_domain: usize,
        total_items: usize,
        exposure_control: ExposureControl,
    },

    /// Select items maximizing Fisher information at a specific ability
    /// estimate θ. This is the standard CAT max-info criterion.
    /// Still randomized: samples from top-k most informative items
    /// (k = `oversample_k`) rather than taking the single best, to
    /// maintain unpredictability.
    MaxFisherInfo {
        n_items: usize,
        theta: f64,
        oversample_k: usize,
        exposure_control: ExposureControl,
    },

    /// Minimax: select items that maximize the minimum Fisher information
    /// across the ability range [theta_min, theta_max]. Robust against an
    /// adversary who can choose to be tested at any ability level.
    /// This is the game-theoretic optimal: the evaluator's mixed strategy
    /// that is hardest to game regardless of the agent's true ability.
    Minimax {
        n_items: usize,
        theta_min: f64,
        theta_max: f64,
        theta_grid: usize,
        exposure_control: ExposureControl,
    },
}

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
        } => select_uniform(pool, *n_items, *exposure_control, &mut rng, seed),

        SelectionStrategy::StratifiedRandom {
            items_per_domain,
            total_items,
            exposure_control,
        } => select_stratified(
            pool,
            *items_per_domain,
            *total_items,
            *exposure_control,
            &mut rng,
            seed,
        ),

        SelectionStrategy::MaxFisherInfo {
            n_items,
            theta,
            oversample_k,
            exposure_control,
        } => select_max_info(
            pool,
            *n_items,
            *theta,
            *oversample_k,
            *exposure_control,
            &mut rng,
            seed,
        ),

        SelectionStrategy::Minimax {
            n_items,
            theta_min,
            theta_max,
            theta_grid,
            exposure_control,
        } => select_minimax(
            pool,
            *n_items,
            *theta_min,
            *theta_max,
            *theta_grid,
            *exposure_control,
            &mut rng,
            seed,
        ),
    }
}

fn select_uniform(
    pool: &ItemPool,
    n_items: usize,
    exposure_control: ExposureControl,
    rng: &mut ChaCha20Rng,
    seed: u64,
) -> Result<SelectionResult, SelectionError> {
    let eligible = eligible_items(pool.items(), exposure_control, rng);
    if eligible.len() < n_items {
        return Err(SelectionError::InsufficientItems {
            requested: n_items,
            available: eligible.len(),
        });
    }
    let mut indices: Vec<usize> = (0..eligible.len()).collect();
    indices.shuffle(rng);
    let selected: Vec<ItemId> = indices[..n_items]
        .iter()
        .map(|&i| eligible[i].id.clone())
        .collect();
    Ok(SelectionResult {
        selected_ids: selected,
        seed,
        strategy_name: "uniform-random".into(),
    })
}

fn select_stratified(
    pool: &ItemPool,
    items_per_domain: usize,
    total_items: usize,
    exposure_control: ExposureControl,
    rng: &mut ChaCha20Rng,
    seed: u64,
) -> Result<SelectionResult, SelectionError> {
    let eligible = eligible_items(pool.items(), exposure_control, rng);
    let domains = pool.domains();
    let mut selected: Vec<ItemId> = Vec::new();

    for domain in &domains {
        let domain_items: Vec<&ItemMetadata> = eligible
            .iter()
            .filter(|i| i.content_domain == *domain)
            .copied()
            .collect();
        let take = items_per_domain.min(domain_items.len());
        let mut indices: Vec<usize> = (0..domain_items.len()).collect();
        indices.shuffle(rng);
        for &idx in indices.iter().take(take) {
            selected.push(domain_items[idx].id.clone());
        }
    }

    // If we overshot the budget, shuffle then truncate —
    // no domain-ordering bias in who gets cut.
    if selected.len() > total_items {
        selected.shuffle(rng);
        selected.truncate(total_items);
    }

    // Fill remaining slots from the pool uniformly.
    if selected.len() < total_items {
        let remaining: Vec<&ItemMetadata> = eligible
            .iter()
            .filter(|i| !selected.contains(&i.id))
            .copied()
            .collect();
        let need = (total_items - selected.len()).min(remaining.len());
        let mut indices: Vec<usize> = (0..remaining.len()).collect();
        indices.shuffle(rng);
        for &idx in indices.iter().take(need) {
            selected.push(remaining[idx].id.clone());
        }
    }

    if selected.is_empty() {
        return Err(SelectionError::ExhaustedByExposureControl);
    }

    Ok(SelectionResult {
        selected_ids: selected,
        seed,
        strategy_name: "stratified-random".into(),
    })
}

fn select_max_info(
    pool: &ItemPool,
    n_items: usize,
    theta: f64,
    oversample_k: usize,
    exposure_control: ExposureControl,
    rng: &mut ChaCha20Rng,
    seed: u64,
) -> Result<SelectionResult, SelectionError> {
    let eligible = eligible_items(pool.items(), exposure_control, rng);
    if eligible.is_empty() {
        return Err(SelectionError::ExhaustedByExposureControl);
    }
    if eligible.len() <= n_items {
        let selected: Vec<ItemId> = eligible.iter().map(|i| i.id.clone()).collect();
        return Ok(SelectionResult {
            selected_ids: selected,
            seed,
            strategy_name: "max-fisher-info".into(),
        });
    }

    // Sort by Fisher information at θ (descending).
    let mut scored: Vec<(usize, f64)> = eligible
        .iter()
        .enumerate()
        .map(|(idx, item)| (idx, information::fisher_information(theta, item)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Take top-k, then randomly sample n_items from that top-k.
    // This maintains measurement efficiency while adding unpredictability.
    let k = oversample_k.max(n_items).min(eligible.len());
    let mut top_k_indices: Vec<usize> = scored[..k].iter().map(|(idx, _)| *idx).collect();
    top_k_indices.shuffle(rng);
    let selected: Vec<ItemId> = top_k_indices[..n_items]
        .iter()
        .map(|&idx| eligible[idx].id.clone())
        .collect();

    Ok(SelectionResult {
        selected_ids: selected,
        seed,
        strategy_name: "max-fisher-info".into(),
    })
}

fn select_minimax(
    pool: &ItemPool,
    n_items: usize,
    theta_min: f64,
    theta_max: f64,
    theta_grid: usize,
    exposure_control: ExposureControl,
    rng: &mut ChaCha20Rng,
    seed: u64,
) -> Result<SelectionResult, SelectionError> {
    let eligible = eligible_items(pool.items(), exposure_control, rng);
    if eligible.is_empty() {
        return Err(SelectionError::ExhaustedByExposureControl);
    }
    if eligible.len() <= n_items {
        let selected: Vec<ItemId> = eligible.iter().map(|i| i.id.clone()).collect();
        return Ok(SelectionResult {
            selected_ids: selected,
            seed,
            strategy_name: "minimax".into(),
        });
    }

    // Greedy minimax: iteratively add the item that maximizes the
    // minimum information across the θ grid.
    let grid: Vec<f64> = if theta_grid <= 1 {
        vec![(theta_min + theta_max) / 2.0]
    } else {
        let step = (theta_max - theta_min) / (theta_grid - 1) as f64;
        (0..theta_grid)
            .map(|i| theta_min + step * i as f64)
            .collect()
    };

    let mut selected_indices: Vec<usize> = Vec::with_capacity(n_items);
    let mut available: Vec<usize> = (0..eligible.len()).collect();

    // Running information at each grid point from already-selected items.
    let mut grid_info: Vec<f64> = vec![0.0; grid.len()];

    for _ in 0..n_items {
        if available.is_empty() {
            break;
        }

        let mut best_candidate = 0;
        let mut best_min_info = f64::NEG_INFINITY;

        for (pos, &idx) in available.iter().enumerate() {
            // If we add this item, what's the new minimum across the grid?
            let mut min_info = f64::INFINITY;
            for (g, &theta) in grid.iter().enumerate() {
                let new_info = grid_info[g] + information::fisher_information(theta, eligible[idx]);
                if new_info < min_info {
                    min_info = new_info;
                }
            }
            if min_info > best_min_info {
                best_min_info = min_info;
                best_candidate = pos;
            }
        }

        let chosen_idx = available[best_candidate];
        selected_indices.push(chosen_idx);

        // Update running grid info.
        for (g, &theta) in grid.iter().enumerate() {
            grid_info[g] += information::fisher_information(theta, eligible[chosen_idx]);
        }

        available.swap_remove(best_candidate);
    }

    // Shuffle the final selection to remove ordering information.
    selected_indices.shuffle(rng);
    let selected: Vec<ItemId> = selected_indices
        .iter()
        .map(|&idx| eligible[idx].id.clone())
        .collect();

    Ok(SelectionResult {
        selected_ids: selected,
        seed,
        strategy_name: "minimax".into(),
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_pool::{ItemId, ItemMetadata, ItemPool};

    fn sample_pool() -> ItemPool {
        ItemPool::new(vec![
            ItemMetadata::new(ItemId::new("t1"), 0.5, 1.0, "math".into()).unwrap(),
            ItemMetadata::new(ItemId::new("t2"), 0.7, 1.2, "math".into()).unwrap(),
            ItemMetadata::new(ItemId::new("t3"), 0.3, 0.8, "code".into()).unwrap(),
            ItemMetadata::new(ItemId::new("t4"), 0.9, 1.5, "code".into()).unwrap(),
            ItemMetadata::new(ItemId::new("t5"), 0.6, 1.1, "reasoning".into()).unwrap(),
            ItemMetadata::new(ItemId::new("t6"), 0.4, 0.9, "reasoning".into()).unwrap(),
        ])
        .unwrap()
    }

    fn wide_pool() -> ItemPool {
        let items: Vec<ItemMetadata> = (0..20)
            .map(|i| {
                let difficulty = -2.0 + (i as f64) * 0.2;
                let discrimination = 0.5 + (i as f64) * 0.1;
                let domain = match i % 3 {
                    0 => "math",
                    1 => "code",
                    _ => "reasoning",
                };
                ItemMetadata::new(
                    ItemId::new(format!("w{i}")),
                    difficulty,
                    discrimination,
                    domain.into(),
                )
                .unwrap()
            })
            .collect();
        ItemPool::new(items).unwrap()
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
            ItemMetadata::new(ItemId::new("t1"), 0.5, 1.0, "math".into()).unwrap(),
            ItemMetadata::new(ItemId::new("t2"), 0.7, 1.2, "math".into()).unwrap(),
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
    fn stratified_truncation_is_unbiased() {
        // With 3 domains × 2 items_per_domain = 6, but total_items = 4,
        // we need to drop 2. Verify no domain is always sacrificed.
        let pool = sample_pool();
        let strat = SelectionStrategy::StratifiedRandom {
            items_per_domain: 2,
            total_items: 4,
            exposure_control: ExposureControl::None,
        };
        let mut domain_counts = std::collections::HashMap::new();
        for seed in 0..100 {
            let result = select(&pool, &strat, seed).unwrap();
            for id in &result.selected_ids {
                let domain = pool.get(id).unwrap().content_domain.clone();
                *domain_counts.entry(domain).or_insert(0u32) += 1;
            }
        }
        // Each domain should appear a reasonable number of times.
        for count in domain_counts.values() {
            assert!(*count > 50, "domain under-represented: {domain_counts:?}");
        }
    }

    #[test]
    fn max_fisher_info_selects_n_items() {
        let pool = wide_pool();
        let result = select(
            &pool,
            &SelectionStrategy::MaxFisherInfo {
                n_items: 5,
                theta: 0.0,
                oversample_k: 10,
                exposure_control: ExposureControl::None,
            },
            42,
        )
        .unwrap();
        assert_eq!(result.selected_ids.len(), 5);
    }

    #[test]
    fn max_fisher_info_is_deterministic() {
        let pool = wide_pool();
        let strat = SelectionStrategy::MaxFisherInfo {
            n_items: 5,
            theta: 0.0,
            oversample_k: 10,
            exposure_control: ExposureControl::None,
        };
        let a = select(&pool, &strat, 42).unwrap();
        let b = select(&pool, &strat, 42).unwrap();
        assert_eq!(a.selected_ids, b.selected_ids);
    }

    #[test]
    fn max_fisher_info_favors_items_near_theta() {
        // Items with difficulty near θ=0.0 should be selected more often.
        let pool = wide_pool();
        let strat = SelectionStrategy::MaxFisherInfo {
            n_items: 5,
            theta: 0.0,
            oversample_k: 8,
            exposure_control: ExposureControl::None,
        };
        let result = select(&pool, &strat, 42).unwrap();
        let avg_difficulty: f64 = result
            .selected_ids
            .iter()
            .map(|id| pool.get(id).unwrap().difficulty)
            .sum::<f64>()
            / 5.0;
        // Average difficulty of selected items should be near 0.
        assert!(
            avg_difficulty.abs() < 1.0,
            "expected items near θ=0, got avg difficulty {avg_difficulty}"
        );
    }

    #[test]
    fn max_fisher_info_with_oversample_adds_randomness() {
        let pool = wide_pool();
        let strat = SelectionStrategy::MaxFisherInfo {
            n_items: 3,
            theta: 0.0,
            oversample_k: 10,
            exposure_control: ExposureControl::None,
        };
        let a = select(&pool, &strat, 1).unwrap();
        let b = select(&pool, &strat, 2).unwrap();
        // With oversample_k > n_items, different seeds should give different selections
        assert_ne!(a.selected_ids, b.selected_ids);
    }

    #[test]
    fn minimax_selects_n_items() {
        let pool = wide_pool();
        let result = select(
            &pool,
            &SelectionStrategy::Minimax {
                n_items: 5,
                theta_min: -2.0,
                theta_max: 2.0,
                theta_grid: 21,
                exposure_control: ExposureControl::None,
            },
            42,
        )
        .unwrap();
        assert_eq!(result.selected_ids.len(), 5);
    }

    #[test]
    fn minimax_is_deterministic() {
        let pool = wide_pool();
        let strat = SelectionStrategy::Minimax {
            n_items: 5,
            theta_min: -2.0,
            theta_max: 2.0,
            theta_grid: 21,
            exposure_control: ExposureControl::None,
        };
        let a = select(&pool, &strat, 42).unwrap();
        let b = select(&pool, &strat, 42).unwrap();
        assert_eq!(a.selected_ids, b.selected_ids);
    }

    #[test]
    fn minimax_spreads_difficulty() {
        // Minimax should select items spanning the difficulty range
        // to provide good information everywhere, not cluster.
        let pool = wide_pool();
        let result = select(
            &pool,
            &SelectionStrategy::Minimax {
                n_items: 5,
                theta_min: -2.0,
                theta_max: 2.0,
                theta_grid: 21,
                exposure_control: ExposureControl::None,
            },
            42,
        )
        .unwrap();
        let difficulties: Vec<f64> = result
            .selected_ids
            .iter()
            .map(|id| pool.get(id).unwrap().difficulty)
            .collect();
        let min_d = difficulties.iter().copied().fold(f64::INFINITY, f64::min);
        let max_d = difficulties
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let spread = max_d - min_d;
        assert!(
            spread > 0.5,
            "minimax should spread items across difficulty range, got spread={spread}"
        );
    }

    #[test]
    fn minimax_outperforms_uniform_on_worst_case() {
        let pool = wide_pool();
        let minimax_result = select(
            &pool,
            &SelectionStrategy::Minimax {
                n_items: 5,
                theta_min: -2.0,
                theta_max: 2.0,
                theta_grid: 21,
                exposure_control: ExposureControl::None,
            },
            42,
        )
        .unwrap();
        let uniform_result = select(
            &pool,
            &SelectionStrategy::UniformRandom {
                n_items: 5,
                exposure_control: ExposureControl::None,
            },
            42,
        )
        .unwrap();

        let minimax_items: Vec<&ItemMetadata> = minimax_result
            .selected_ids
            .iter()
            .map(|id| pool.get(id).unwrap())
            .collect();
        let uniform_items: Vec<&ItemMetadata> = uniform_result
            .selected_ids
            .iter()
            .map(|id| pool.get(id).unwrap())
            .collect();

        let minimax_worst = information::min_information_over_range(&minimax_items, -2.0, 2.0, 21);
        let uniform_worst = information::min_information_over_range(&uniform_items, -2.0, 2.0, 21);

        assert!(
            minimax_worst >= uniform_worst,
            "minimax worst-case ({minimax_worst:.3}) should >= uniform worst-case ({uniform_worst:.3})"
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
