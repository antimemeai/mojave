use crate::cusum::{CusumChart, CusumConfig};
use crate::types::{ChartSignal, ControlLimits, SpcError};

#[derive(Debug, Clone)]
pub struct CombinedConfig {
    pub cusum: CusumConfig,
    pub shewhart_k: f64,
}

impl CombinedConfig {
    #[must_use]
    pub fn default_for(limits: ControlLimits) -> Self {
        Self {
            cusum: CusumConfig::default_for(limits),
            shewhart_k: 3.5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CombinedChart {
    cusum: CusumChart,
    shewhart_k: f64,
    mu_0: f64,
    sigma: f64,
    n: usize,
}

impl CombinedChart {
    pub fn new(config: CombinedConfig) -> Result<Self, SpcError> {
        if config.shewhart_k <= 0.0 {
            return Err(SpcError::NonPositiveParam {
                name: "shewhart_k",
                value: config.shewhart_k,
            });
        }
        let mu_0 = config.cusum.limits.mu_0;
        let sigma = config.cusum.limits.sigma;
        let cusum = CusumChart::new(config.cusum)?;
        Ok(Self {
            cusum,
            shewhart_k: config.shewhart_k,
            mu_0,
            sigma,
            n: 0,
        })
    }

    pub fn observe(&mut self, x: f64) -> Result<ChartSignal, SpcError> {
        if !x.is_finite() {
            return Err(SpcError::NonFiniteInput(x));
        }
        self.n += 1;
        let z = (x - self.mu_0) / self.sigma;

        // Shewhart check first (instantaneous large shift).
        if z.abs() > self.shewhart_k {
            // Still update CUSUM state for consistency.
            self.cusum.observe(x)?;
            return Ok(ChartSignal::OutOfControl {
                statistic: z,
                observation_index: self.n - 1,
            });
        }

        // CUSUM check (sustained small shift).
        let cusum_signal = self.cusum.observe(x)?;
        if cusum_signal.is_out_of_control() {
            return Ok(cusum_signal);
        }

        Ok(ChartSignal::InControl)
    }

    pub fn reset(&mut self) {
        self.cusum.reset();
        self.n = 0;
    }

    #[must_use]
    pub fn cusum_state(&self) -> (f64, f64) {
        (self.cusum.c_plus(), self.cusum.c_minus())
    }

    #[must_use]
    pub fn n_observations(&self) -> usize {
        self.n
    }
}
