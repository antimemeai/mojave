use crate::types::{ChartSignal, ControlLimits, SpcError};

#[derive(Debug, Clone)]
pub struct CusumConfig {
    pub limits: ControlLimits,
    pub k: f64,
    pub h: f64,
}

impl CusumConfig {
    #[must_use]
    pub fn default_for(limits: ControlLimits) -> Self {
        Self {
            limits,
            k: 0.5,
            h: 5.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CusumChart {
    mu_0: f64,
    sigma: f64,
    k: f64,
    h: f64,
    c_plus: f64,
    c_minus: f64,
    n: usize,
    initial_c_plus: f64,
    initial_c_minus: f64,
}

impl CusumChart {
    pub fn new(config: CusumConfig) -> Result<Self, SpcError> {
        Self::new_with_head_start(config, 0.0)
    }

    pub(crate) fn new_with_head_start(
        config: CusumConfig,
        head_start: f64,
    ) -> Result<Self, SpcError> {
        if config.k <= 0.0 {
            return Err(SpcError::NonPositiveParam {
                name: "k",
                value: config.k,
            });
        }
        if config.h <= 0.0 {
            return Err(SpcError::NonPositiveParam {
                name: "h",
                value: config.h,
            });
        }
        Ok(Self {
            mu_0: config.limits.mu_0,
            sigma: config.limits.sigma,
            k: config.k,
            h: config.h,
            c_plus: head_start,
            c_minus: head_start,
            n: 0,
            initial_c_plus: head_start,
            initial_c_minus: head_start,
        })
    }

    pub fn observe(&mut self, x: f64) -> ChartSignal {
        debug_assert!(x.is_finite());
        let z = (x - self.mu_0) / self.sigma;
        self.c_plus = f64::max(0.0, self.c_plus + z - self.k);
        self.c_minus = f64::max(0.0, self.c_minus - z - self.k);
        self.n += 1;

        if self.c_plus > self.h {
            ChartSignal::OutOfControl {
                statistic: self.c_plus,
                observation_index: self.n - 1,
            }
        } else if self.c_minus > self.h {
            ChartSignal::OutOfControl {
                statistic: self.c_minus,
                observation_index: self.n - 1,
            }
        } else {
            ChartSignal::InControl
        }
    }

    pub fn reset(&mut self) {
        self.c_plus = self.initial_c_plus;
        self.c_minus = self.initial_c_minus;
        self.n = 0;
    }

    #[must_use]
    pub fn c_plus(&self) -> f64 {
        self.c_plus
    }

    #[must_use]
    pub fn c_minus(&self) -> f64 {
        self.c_minus
    }

    #[must_use]
    pub fn n_observations(&self) -> usize {
        self.n
    }
}
