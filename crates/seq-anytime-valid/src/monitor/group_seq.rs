use crate::boundary::{obf, pocock, spending};
use crate::error::SeqError;
use crate::types::{BoundaryType, Decision, GroupSeqConfig, SpendingFunctionType};

pub struct GroupSeqMonitor {
    config: GroupSeqConfig,
    boundaries: Vec<f64>,
    current_look: usize,
}

impl GroupSeqMonitor {
    pub fn new(config: GroupSeqConfig) -> Result<Self, SeqError> {
        let boundaries = compute_boundaries(&config)?;
        Ok(Self {
            config,
            boundaries,
            current_look: 0,
        })
    }

    pub fn update_batch(&mut self, z_statistic: f64) -> Result<Decision, SeqError> {
        if self.current_look >= self.config.total_looks {
            return Err(SeqError::LookOutOfRange {
                k: self.current_look + 1,
                total: self.config.total_looks,
            });
        }
        let boundary = self.boundaries[self.current_look];
        self.current_look += 1;

        if z_statistic.abs() >= boundary {
            Ok(Decision::Reject)
        } else if self.current_look >= self.config.total_looks {
            Ok(Decision::Accept)
        } else {
            Ok(Decision::Continue)
        }
    }

    pub fn boundaries(&self) -> &[f64] {
        &self.boundaries
    }
}

pub fn compute_boundaries(config: &GroupSeqConfig) -> Result<Vec<f64>, SeqError> {
    match &config.boundary_type {
        BoundaryType::Pocock => pocock::boundaries(config.total_looks, config.alpha),
        BoundaryType::OBrienFleming => obf::boundaries(config.total_looks, config.alpha),
        BoundaryType::LanDeMets(sf) => {
            let fractions: Vec<f64> = (1..=config.total_looks)
                .map(|k| k as f64 / config.total_looks as f64)
                .collect();
            let sf_fn: Box<dyn Fn(f64, f64) -> f64> = match sf {
                SpendingFunctionType::PocockType => Box::new(spending::pocock_spending),
                SpendingFunctionType::OBrienFlemingType => Box::new(spending::obf_spending),
            };
            (1..=config.total_looks)
                .map(|k| spending::spending_boundary(k, &fractions, config.alpha, &*sf_fn))
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_seq_monitor_rejects_at_boundary() {
        let config = GroupSeqConfig {
            total_looks: 3,
            alpha: 0.05,
            beta: 0.20,
            boundary_type: BoundaryType::OBrienFleming,
        };
        let mut monitor = GroupSeqMonitor::new(config).unwrap_or_else(|e| panic!("{e}"));
        let d = monitor.update_batch(5.0).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(d, Decision::Reject);
    }

    #[test]
    fn group_seq_monitor_accepts_at_final_look() {
        let config = GroupSeqConfig {
            total_looks: 2,
            alpha: 0.05,
            beta: 0.20,
            boundary_type: BoundaryType::OBrienFleming,
        };
        let mut monitor = GroupSeqMonitor::new(config).unwrap_or_else(|e| panic!("{e}"));
        let _ = monitor.update_batch(0.5).unwrap_or_else(|e| panic!("{e}"));
        let d = monitor.update_batch(0.5).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(d, Decision::Accept);
    }
}
