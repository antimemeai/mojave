use crate::error::SeqError;
use crate::types::EvidenceSnapshot;

/// Hedged capital confidence sequence for [0,1]-bounded data.
///
/// Implements the betting-based confidence sequence from Waudby-Smith & Ramdas
/// (2024, Theorem 3) using predictable plug-in bet sizing. Provides
/// anytime-valid confidence intervals with no distributional assumptions
/// beyond boundedness.
pub struct BettingMonitor {
    alpha: f64,
    truncation: f64,
    grid: Vec<f64>,
    log_kplus: Vec<f64>,
    log_kminus: Vec<f64>,
    sum_x: f64,
    sum_sq: f64,
    n: usize,
    threshold: f64,
}

impl BettingMonitor {
    /// Create a new `BettingMonitor`.
    ///
    /// # Arguments
    /// * `alpha` - significance level, must be in (0, 1)
    /// * `grid_size` - number of grid points for CI inversion, must be >= 10
    ///
    /// # Errors
    /// Returns `SeqError::InvalidAlpha` if alpha is not in (0, 1).
    /// Returns `SeqError::InvalidGridSize` if grid_size < 10.
    pub fn new(alpha: f64, grid_size: usize) -> Result<Self, SeqError> {
        if alpha <= 0.0 || alpha >= 1.0 {
            return Err(SeqError::InvalidAlpha(alpha));
        }
        if grid_size < 10 {
            return Err(SeqError::InvalidGridSize(grid_size));
        }

        let eps = 1e-4;
        let grid: Vec<f64> = (0..grid_size)
            .map(|i| eps + (1.0 - 2.0 * eps) * (i as f64) / ((grid_size - 1) as f64))
            .collect();

        let log_kplus = vec![0.0; grid_size];
        let log_kminus = vec![0.0; grid_size];

        Ok(Self {
            alpha,
            truncation: 0.5,
            grid,
            log_kplus,
            log_kminus,
            sum_x: 0.0,
            sum_sq: 0.0,
            n: 0,
            threshold: (1.0 / alpha).ln(),
        })
    }

    /// Update the monitor with a new observation in [0, 1].
    ///
    /// Returns an `EvidenceSnapshot` containing the current confidence interval.
    ///
    /// # Errors
    /// Returns `SeqError::InvalidBettingObservation` if the observation is
    /// outside [0, 1] or non-finite.
    pub fn update(&mut self, observation: f64) -> Result<EvidenceSnapshot, SeqError> {
        if !observation.is_finite() || !(0.0..=1.0).contains(&observation) {
            return Err(SeqError::InvalidBettingObservation(observation));
        }

        // Predictable estimates (computed BEFORE incorporating this observation)
        let hat_mu = (0.5 + self.sum_x) / (self.n as f64 + 1.0);
        let hat_var = (0.25 + self.sum_sq) / (self.n as f64 + 1.0);

        // Update running statistics
        self.sum_sq += (observation - hat_mu).powi(2);
        self.sum_x += observation;
        self.n += 1;

        let n_f = self.n as f64;
        let c = self.truncation;

        // Compute tilde_lambda (predictable plug-in bet size)
        let tilde_lambda =
            (2.0 * (2.0 / self.alpha).ln() / (hat_var * n_f.max(1.0) * (n_f + 2.0).ln())).sqrt();

        // Update log-wealth for each grid point
        for j in 0..self.grid.len() {
            let m = self.grid[j];
            let diff = observation - m;

            // Truncated bet sizes
            let lam_plus = tilde_lambda.min(c / m);
            let lam_minus = tilde_lambda.min(c / (1.0 - m));

            // Update in log space
            let arg_plus = 1.0 + lam_plus * diff;
            if arg_plus > 0.0 {
                self.log_kplus[j] += arg_plus.ln();
            } else {
                // Wealth goes to zero (log = -infinity)
                self.log_kplus[j] = f64::NEG_INFINITY;
            }

            let arg_minus = 1.0 - lam_minus * diff;
            if arg_minus > 0.0 {
                self.log_kminus[j] += arg_minus.ln();
            } else {
                self.log_kminus[j] = f64::NEG_INFINITY;
            }
        }

        // CI inversion: find m_lo and m_hi
        let ln_half = (0.5_f64).ln();
        let ci = self.invert_ci(ln_half);

        Ok(EvidenceSnapshot {
            log_likelihood_ratio: 0.0,
            n_observations: self.n,
            always_valid_p: None,
            confidence_interval: Some(ci),
            e_value: None,
        })
    }

    /// Invert the wealth process to find the confidence interval.
    ///
    /// The CI is { m : K_t^{+-}(m) < 1/alpha }, where
    /// K_t^{+-}(m) = max{ 0.5 * K_t^+(m), 0.5 * K_t^-(m) }.
    ///
    /// In log space: max(log_kplus + ln(0.5), log_kminus + ln(0.5)) < ln(1/alpha).
    fn invert_ci(&self, ln_half: f64) -> (f64, f64) {
        let grid_len = self.grid.len();

        // Find lower bound: scan from left, find first m where hedged capital < threshold
        let mut lo = self.grid[0];
        for j in 0..grid_len {
            let hedged_log = (self.log_kplus[j] + ln_half).max(self.log_kminus[j] + ln_half);
            if hedged_log < self.threshold {
                lo = self.grid[j];
                break;
            }
        }

        // Find upper bound: scan from right, find first m where hedged capital < threshold
        let mut hi = self.grid[grid_len - 1];
        for j in (0..grid_len).rev() {
            let hedged_log = (self.log_kplus[j] + ln_half).max(self.log_kminus[j] + ln_half);
            if hedged_log < self.threshold {
                hi = self.grid[j];
                break;
            }
        }

        (lo, hi)
    }

    /// Returns the current number of observations.
    pub fn n_observations(&self) -> usize {
        self.n
    }

    /// Returns the significance level.
    pub fn alpha(&self) -> f64 {
        self.alpha
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn construction_validates_alpha() {
        assert!(matches!(
            BettingMonitor::new(0.0, 100),
            Err(SeqError::InvalidAlpha(_))
        ));
        assert!(matches!(
            BettingMonitor::new(1.0, 100),
            Err(SeqError::InvalidAlpha(_))
        ));
        assert!(matches!(
            BettingMonitor::new(-0.5, 100),
            Err(SeqError::InvalidAlpha(_))
        ));
    }

    #[test]
    fn construction_validates_grid_size() {
        assert!(matches!(
            BettingMonitor::new(0.05, 5),
            Err(SeqError::InvalidGridSize(_))
        ));
        assert!(matches!(
            BettingMonitor::new(0.05, 9),
            Err(SeqError::InvalidGridSize(_))
        ));
        assert!(BettingMonitor::new(0.05, 10).is_ok());
    }

    #[test]
    fn rejects_out_of_range_observation() {
        let mut monitor = BettingMonitor::new(0.05, 100).unwrap();
        assert!(matches!(
            monitor.update(1.5),
            Err(SeqError::InvalidBettingObservation(_))
        ));
        assert!(matches!(
            monitor.update(-0.1),
            Err(SeqError::InvalidBettingObservation(_))
        ));
        assert!(matches!(
            monitor.update(f64::NAN),
            Err(SeqError::InvalidBettingObservation(_))
        ));
    }

    #[test]
    fn accepts_boundary_observations() {
        let mut monitor = BettingMonitor::new(0.05, 100).unwrap();
        assert!(monitor.update(0.0).is_ok());
        assert!(monitor.update(1.0).is_ok());
    }

    #[test]
    fn returns_confidence_interval() {
        let mut monitor = BettingMonitor::new(0.05, 100).unwrap();
        let snap = monitor.update(0.5).unwrap();
        assert!(snap.confidence_interval.is_some());
        let (lo, hi) = snap.confidence_interval.unwrap();
        assert!(lo < hi, "CI should have positive width");
    }
}
