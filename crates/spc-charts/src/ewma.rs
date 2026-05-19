use crate::types::{ChartSignal, ControlLimits, SpcError};

#[derive(Debug, Clone)]
pub struct EwmaConfig {
    pub limits: ControlLimits,
    pub lambda: f64,
    pub l_sigma: f64,
}

impl EwmaConfig {
    #[must_use]
    pub fn default_for(limits: ControlLimits) -> Self {
        Self {
            limits,
            lambda: 0.2,
            l_sigma: 2.962,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EwmaChart {
    mu_0: f64,
    sigma: f64,
    lambda: f64,
    l_sigma: f64,
    z: f64,
    n: usize,
    one_minus_lambda: f64,
    lambda_ratio: f64,
}

impl EwmaChart {
    pub fn new(config: EwmaConfig) -> Result<Self, SpcError> {
        if config.lambda <= 0.0 || config.lambda > 1.0 {
            return Err(SpcError::InvalidLambda(config.lambda));
        }
        if config.l_sigma <= 0.0 {
            return Err(SpcError::NonPositiveParam {
                name: "l_sigma",
                value: config.l_sigma,
            });
        }
        let one_minus_lambda = 1.0 - config.lambda;
        let lambda_ratio = config.lambda / (2.0 - config.lambda);
        Ok(Self {
            mu_0: config.limits.mu_0,
            sigma: config.limits.sigma,
            lambda: config.lambda,
            l_sigma: config.l_sigma,
            z: config.limits.mu_0,
            n: 0,
            one_minus_lambda,
            lambda_ratio,
        })
    }

    pub fn observe(&mut self, x: f64) -> Result<ChartSignal, SpcError> {
        if !x.is_finite() {
            return Err(SpcError::NonFiniteInput(x));
        }
        self.n += 1;
        self.z = self.lambda * x + self.one_minus_lambda * self.z;

        // For large n, (1-lambda)^(2n) is indistinguishable from 0 within f64
        // precision. Use steady-state formula (time_factor = 1.0) to avoid
        // integer overflow in powi when 2*n exceeds i32::MAX.
        let time_factor = if self.n > 1000 {
            1.0
        } else {
            1.0 - self.one_minus_lambda.powi(2 * self.n as i32)
        };
        let limit_width = self.l_sigma * self.sigma * (self.lambda_ratio * time_factor).sqrt();
        let ucl = self.mu_0 + limit_width;
        let lcl = self.mu_0 - limit_width;

        if self.z > ucl || self.z < lcl {
            Ok(ChartSignal::OutOfControl {
                statistic: self.z,
                observation_index: self.n - 1,
            })
        } else {
            Ok(ChartSignal::InControl)
        }
    }

    pub fn reset(&mut self) {
        self.z = self.mu_0;
        self.n = 0;
    }

    #[must_use]
    pub fn z(&self) -> f64 {
        self.z
    }

    #[must_use]
    pub fn n_observations(&self) -> usize {
        self.n
    }
}
