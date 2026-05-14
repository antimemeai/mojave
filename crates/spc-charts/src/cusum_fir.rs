use crate::cusum::CusumConfig;
use crate::types::{ChartSignal, ControlLimits, SpcError};

#[derive(Debug, Clone)]
pub struct FirCusumConfig {
    pub limits: ControlLimits,
    pub k: f64,
    pub h: f64,
    pub head_start: f64,
}

impl FirCusumConfig {
    #[must_use]
    pub fn default_for(limits: ControlLimits) -> Self {
        Self {
            limits,
            k: 0.5,
            h: 5.0,
            head_start: 2.5, // h/2
        }
    }
}

#[derive(Debug, Clone)]
pub struct FirCusumChart {
    inner: crate::cusum::CusumChart,
}

impl FirCusumChart {
    pub fn new(config: FirCusumConfig) -> Result<Self, SpcError> {
        if config.head_start < 0.0 {
            return Err(SpcError::NonPositiveParam {
                name: "head_start",
                value: config.head_start,
            });
        }
        let cusum_config = CusumConfig {
            limits: config.limits,
            k: config.k,
            h: config.h,
        };
        let inner = crate::cusum::CusumChart::new_with_head_start(cusum_config, config.head_start)?;
        Ok(Self { inner })
    }

    pub fn observe(&mut self, x: f64) -> ChartSignal {
        self.inner.observe(x)
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }

    #[must_use]
    pub fn c_plus(&self) -> f64 {
        self.inner.c_plus()
    }

    #[must_use]
    pub fn c_minus(&self) -> f64 {
        self.inner.c_minus()
    }

    #[must_use]
    pub fn n_observations(&self) -> usize {
        self.inner.n_observations()
    }
}
