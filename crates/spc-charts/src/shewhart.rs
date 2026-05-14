use crate::types::{ChartSignal, ControlLimits, SpcError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShewhartRule {
    WE1,
    WE2,
    WE3,
    WE4,
}

#[derive(Debug, Clone)]
pub struct ShewhartConfig {
    pub limits: ControlLimits,
    pub k_sigma: f64,
    pub rules: Vec<ShewhartRule>,
}

impl ShewhartConfig {
    #[must_use]
    pub fn default_for(limits: ControlLimits) -> Self {
        Self {
            limits,
            k_sigma: 3.0,
            rules: vec![ShewhartRule::WE1],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShewhartChart {
    config: ShewhartConfig,
    /// Upper control limit (retained for future plotting/reporting).
    #[allow(dead_code)]
    ucl: f64,
    /// Lower control limit (retained for future plotting/reporting).
    #[allow(dead_code)]
    lcl: f64,
    sigma: f64,
    mu_0: f64,
    n: usize,
    history: Vec<f64>,
}

impl ShewhartChart {
    pub fn new(config: ShewhartConfig) -> Result<Self, SpcError> {
        if config.k_sigma <= 0.0 {
            return Err(SpcError::NonPositiveParam {
                name: "k_sigma",
                value: config.k_sigma,
            });
        }
        let mu_0 = config.limits.mu_0;
        let sigma = config.limits.sigma;
        let ucl = mu_0 + config.k_sigma * sigma;
        let lcl = mu_0 - config.k_sigma * sigma;
        Ok(Self {
            config,
            ucl,
            lcl,
            sigma,
            mu_0,
            n: 0,
            history: Vec::new(),
        })
    }

    pub fn observe(&mut self, x: f64) -> ChartSignal {
        debug_assert!(x.is_finite());
        self.n += 1;
        let z = (x - self.mu_0) / self.sigma;
        self.history.push(z);

        for &rule in &self.config.rules {
            if self.check_rule(rule) {
                return ChartSignal::OutOfControl {
                    statistic: z,
                    observation_index: self.n - 1,
                };
            }
        }

        if z.abs() > 2.0 {
            ChartSignal::Warning { statistic: z }
        } else {
            ChartSignal::InControl
        }
    }

    pub fn reset(&mut self) {
        self.n = 0;
        self.history.clear();
    }

    #[must_use]
    pub fn n_observations(&self) -> usize {
        self.n
    }

    fn check_rule(&self, rule: ShewhartRule) -> bool {
        let h = &self.history;
        let n = h.len();
        match rule {
            ShewhartRule::WE1 => n >= 1 && h[n - 1].abs() > self.config.k_sigma,
            ShewhartRule::WE2 => {
                if n < 2 {
                    return false;
                }
                let last3: Vec<f64> = h[n.saturating_sub(3)..].to_vec();
                let above = last3.iter().filter(|&&z| z > 2.0).count();
                let below = last3.iter().filter(|&&z| z < -2.0).count();
                above >= 2 || below >= 2
            }
            ShewhartRule::WE3 => {
                if n < 4 {
                    return false;
                }
                let last5: Vec<f64> = h[n.saturating_sub(5)..].to_vec();
                let above = last5.iter().filter(|&&z| z > 1.0).count();
                let below = last5.iter().filter(|&&z| z < -1.0).count();
                above >= 4 || below >= 4
            }
            ShewhartRule::WE4 => {
                if n < 8 {
                    return false;
                }
                let last8 = &h[n - 8..];
                let all_above = last8.iter().all(|&z| z > 0.0);
                let all_below = last8.iter().all(|&z| z < 0.0);
                all_above || all_below
            }
        }
    }
}
