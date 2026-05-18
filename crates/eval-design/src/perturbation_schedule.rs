use rand::{Rng as _, SeedableRng as _};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};

use crate::item_pool::ItemId;

/// A perturbation assignment for one item in one evaluation run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerturbationAssignment {
    pub item_id: ItemId,
    pub perturbation_seed: u64,
    pub family_index: usize,
}

/// Configuration for perturbation schedule generation.
///
/// The schedule assigns each selected item a perturbation seed and family,
/// ensuring the perturbation pattern varies across runs (anti-gaming)
/// while remaining deterministic within a run (reproducibility).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ScheduleConfig {
    /// Number of perturbation families available (e.g., 3 for format/paraphrase/multi-turn).
    pub n_families: usize,
    /// Fraction of items that receive a perturbation (0.0..=1.0).
    /// Remaining items serve as unperturbed controls.
    pub perturbation_rate: f64,
}

/// A complete perturbation schedule for one evaluation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerturbationSchedule {
    pub assignments: Vec<PerturbationAssignment>,
    pub control_items: Vec<ItemId>,
    pub run_seed: u64,
}

impl PerturbationSchedule {
    #[must_use]
    pub fn n_perturbed(&self) -> usize {
        self.assignments.len()
    }

    #[must_use]
    pub fn n_control(&self) -> usize {
        self.control_items.len()
    }

    #[must_use]
    pub fn assignment_for(&self, id: &ItemId) -> Option<&PerturbationAssignment> {
        self.assignments.iter().find(|a| &a.item_id == id)
    }

    #[must_use]
    pub fn is_control(&self, id: &ItemId) -> bool {
        self.control_items.contains(id)
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum ScheduleError {
    #[error("perturbation rate must be in [0.0, 1.0], got {0}")]
    InvalidRate(f64),
    #[error("n_families must be > 0")]
    NoFamilies,
    #[error("no items provided")]
    NoItems,
}

/// Generate a perturbation schedule for the given item IDs.
///
/// Each run gets a different random assignment of items to perturbation
/// families and seeds, while maintaining a control group. This prevents
/// an agent from learning which perturbation to expect on which item.
pub fn generate_schedule(
    item_ids: &[ItemId],
    config: &ScheduleConfig,
    run_seed: u64,
) -> Result<PerturbationSchedule, ScheduleError> {
    if item_ids.is_empty() {
        return Err(ScheduleError::NoItems);
    }
    if config.n_families == 0 {
        return Err(ScheduleError::NoFamilies);
    }
    if !(0.0..=1.0).contains(&config.perturbation_rate) {
        return Err(ScheduleError::InvalidRate(config.perturbation_rate));
    }

    let mut rng = ChaCha20Rng::seed_from_u64(run_seed);
    let mut assignments = Vec::new();
    let mut control_items = Vec::new();

    for item_id in item_ids {
        let r: f64 = rng.random();
        if r < config.perturbation_rate {
            let perturbation_seed: u64 = rng.random();
            let family_index: usize = rng.random_range(0..config.n_families);
            assignments.push(PerturbationAssignment {
                item_id: item_id.clone(),
                perturbation_seed,
                family_index,
            });
        } else {
            control_items.push(item_id.clone());
        }
    }

    Ok(PerturbationSchedule {
        assignments,
        control_items,
        run_seed,
    })
}

/// Generate N schedules that together cover the full item pool with
/// perturbations at least `min_coverage` fraction of the time.
///
/// This is the core anti-gaming mechanism: across multiple runs, every
/// item gets perturbed under different conditions, making it impossible
/// to optimize for a specific perturbation pattern.
pub fn generate_schedule_series(
    item_ids: &[ItemId],
    config: &ScheduleConfig,
    base_seed: u64,
    n_runs: usize,
) -> Result<Vec<PerturbationSchedule>, ScheduleError> {
    let mut schedules = Vec::with_capacity(n_runs);
    let mut seed_rng = ChaCha20Rng::seed_from_u64(base_seed);
    for _ in 0..n_runs {
        let run_seed: u64 = seed_rng.random();
        let schedule = generate_schedule(item_ids, config, run_seed)?;
        schedules.push(schedule);
    }
    Ok(schedules)
}

/// Compute the empirical perturbation coverage: for each item, what
/// fraction of runs included it as a perturbed (non-control) item?
#[must_use]
pub fn coverage_report(
    schedules: &[PerturbationSchedule],
    item_ids: &[ItemId],
) -> Vec<(ItemId, f64)> {
    if schedules.is_empty() {
        return item_ids.iter().map(|id| (id.clone(), 0.0)).collect();
    }
    let n = schedules.len() as f64;
    item_ids
        .iter()
        .map(|id| {
            let perturbed_count = schedules
                .iter()
                .filter(|s| s.assignment_for(id).is_some())
                .count();
            (id.clone(), perturbed_count as f64 / n)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item_ids() -> Vec<ItemId> {
        (0..10).map(|i| ItemId::new(format!("t{i}"))).collect()
    }

    fn default_config() -> ScheduleConfig {
        ScheduleConfig {
            n_families: 3,
            perturbation_rate: 0.7,
        }
    }

    #[test]
    fn schedule_is_deterministic() {
        let ids = item_ids();
        let config = default_config();
        let a = generate_schedule(&ids, &config, 42).unwrap();
        let b = generate_schedule(&ids, &config, 42).unwrap();
        assert_eq!(a.assignments.len(), b.assignments.len());
        for (aa, bb) in a.assignments.iter().zip(b.assignments.iter()) {
            assert_eq!(aa.item_id, bb.item_id);
            assert_eq!(aa.perturbation_seed, bb.perturbation_seed);
            assert_eq!(aa.family_index, bb.family_index);
        }
    }

    #[test]
    fn schedule_varies_with_seed() {
        let ids = item_ids();
        let config = default_config();
        let a = generate_schedule(&ids, &config, 1).unwrap();
        let b = generate_schedule(&ids, &config, 2).unwrap();
        let a_perturbed: Vec<_> = a.assignments.iter().map(|a| &a.item_id).collect();
        let b_perturbed: Vec<_> = b.assignments.iter().map(|a| &a.item_id).collect();
        assert_ne!(a_perturbed, b_perturbed);
    }

    #[test]
    fn all_items_accounted_for() {
        let ids = item_ids();
        let config = default_config();
        let schedule = generate_schedule(&ids, &config, 42).unwrap();
        let total = schedule.n_perturbed() + schedule.n_control();
        assert_eq!(total, ids.len());
    }

    #[test]
    fn family_indices_in_range() {
        let ids = item_ids();
        let config = default_config();
        let schedule = generate_schedule(&ids, &config, 42).unwrap();
        for a in &schedule.assignments {
            assert!(a.family_index < config.n_families);
        }
    }

    #[test]
    fn zero_perturbation_rate_all_control() {
        let ids = item_ids();
        let config = ScheduleConfig {
            n_families: 3,
            perturbation_rate: 0.0,
        };
        let schedule = generate_schedule(&ids, &config, 42).unwrap();
        assert_eq!(schedule.n_perturbed(), 0);
        assert_eq!(schedule.n_control(), ids.len());
    }

    #[test]
    fn full_perturbation_rate_no_control() {
        let ids = item_ids();
        let config = ScheduleConfig {
            n_families: 3,
            perturbation_rate: 1.0,
        };
        let schedule = generate_schedule(&ids, &config, 42).unwrap();
        assert_eq!(schedule.n_perturbed(), ids.len());
        assert_eq!(schedule.n_control(), 0);
    }

    #[test]
    fn invalid_rate_rejected() {
        let ids = item_ids();
        let config = ScheduleConfig {
            n_families: 3,
            perturbation_rate: 1.5,
        };
        assert!(generate_schedule(&ids, &config, 42).is_err());
    }

    #[test]
    fn no_families_rejected() {
        let ids = item_ids();
        let config = ScheduleConfig {
            n_families: 0,
            perturbation_rate: 0.5,
        };
        assert!(generate_schedule(&ids, &config, 42).is_err());
    }

    #[test]
    fn series_generates_n_schedules() {
        let ids = item_ids();
        let config = default_config();
        let series = generate_schedule_series(&ids, &config, 42, 10).unwrap();
        assert_eq!(series.len(), 10);
    }

    #[test]
    fn coverage_report_converges_to_rate() {
        let ids = item_ids();
        let config = ScheduleConfig {
            n_families: 3,
            perturbation_rate: 0.7,
        };
        let series = generate_schedule_series(&ids, &config, 42, 1000).unwrap();
        let report = coverage_report(&series, &ids);
        for (_id, coverage) in &report {
            assert!(
                (0.6..=0.8).contains(coverage),
                "expected ~0.7 coverage, got {coverage}"
            );
        }
    }

    #[test]
    fn assignment_for_finds_perturbed_item() {
        let ids = item_ids();
        let config = ScheduleConfig {
            n_families: 3,
            perturbation_rate: 1.0,
        };
        let schedule = generate_schedule(&ids, &config, 42).unwrap();
        let assignment = schedule.assignment_for(&ids[0]);
        assert!(assignment.is_some());
    }

    #[test]
    fn is_control_identifies_unperturbed() {
        let ids = item_ids();
        let config = ScheduleConfig {
            n_families: 3,
            perturbation_rate: 0.0,
        };
        let schedule = generate_schedule(&ids, &config, 42).unwrap();
        assert!(schedule.is_control(&ids[0]));
    }
}
