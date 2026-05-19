use crate::types::{ChartSignal, SpcError};

pub trait EValueSource {
    fn e_value(&self, observation: f64) -> f64;
}

#[derive(Debug, Clone)]
pub struct GaussianEValue {
    pub mu_0: f64,
    pub sigma: f64,
    pub mixing_variance: f64,
}

impl GaussianEValue {
    pub fn new(mu_0: f64, sigma: f64, mixing_variance: f64) -> Result<Self, SpcError> {
        if sigma <= 0.0 {
            return Err(SpcError::NonPositiveSigma(sigma));
        }
        if mixing_variance <= 0.0 {
            return Err(SpcError::NonPositiveParam {
                name: "mixing_variance",
                value: mixing_variance,
            });
        }
        Ok(Self {
            mu_0,
            sigma,
            mixing_variance,
        })
    }
}

impl EValueSource for GaussianEValue {
    fn e_value(&self, observation: f64) -> f64 {
        // Single-observation MSPRT log-LR for N(mu_0, sigma^2) with
        // mixing prior N(0, tau^2) on the standardized effect:
        //   log(e) = -0.5 * ln(1 + tau^2) + z^2 * tau^2 / (2*(1+tau^2))
        // where z = (x - mu_0) / sigma, tau^2 = mixing_variance.
        let z = (observation - self.mu_0) / self.sigma;
        let tau_sq = self.mixing_variance;
        let log_e = -0.5 * (1.0 + tau_sq).ln() + z * z * tau_sq / (2.0 * (1.0 + tau_sq));
        log_e.exp()
    }
}

#[derive(Debug, Clone)]
pub enum EDetectorWindow {
    Growing,
    Fixed { width: usize },
}

#[derive(Debug, Clone)]
pub struct EDetectorConfig {
    pub alpha: f64,
    pub window: EDetectorWindow,
}

impl EDetectorConfig {
    #[must_use]
    pub fn default_growing() -> Self {
        Self {
            alpha: 0.05,
            window: EDetectorWindow::Growing,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EDetector<E: EValueSource> {
    source: E,
    threshold: f64,
    window: EDetectorWindow,
    m: f64,
    n: usize,
    ring: Vec<f64>,
    ring_pos: usize,
}

impl<E: EValueSource> EDetector<E> {
    pub fn new(config: EDetectorConfig, source: E) -> Result<Self, SpcError> {
        if config.alpha <= 0.0 || config.alpha >= 1.0 {
            return Err(SpcError::InvalidAlpha(config.alpha));
        }
        if let EDetectorWindow::Fixed { width } = &config.window {
            if *width == 0 {
                return Err(SpcError::InvalidWindowWidth(0));
            }
        }
        let threshold = 1.0 / config.alpha;
        let ring = match &config.window {
            EDetectorWindow::Growing => Vec::new(),
            EDetectorWindow::Fixed { width } => vec![1.0; *width],
        };
        Ok(Self {
            source,
            threshold,
            window: config.window,
            m: 1.0,
            n: 0,
            ring,
            ring_pos: 0,
        })
    }

    pub fn observe(&mut self, x: f64) -> Result<ChartSignal, SpcError> {
        if !x.is_finite() {
            return Err(SpcError::NonFiniteInput(x));
        }
        self.n += 1;
        let e = self.source.e_value(x);

        match &self.window {
            EDetectorWindow::Growing => {
                self.m = f64::max(1.0, self.m * e);
            }
            EDetectorWindow::Fixed { width } => {
                let w = *width;
                let old = self.ring[self.ring_pos];
                self.ring[self.ring_pos] = e;
                self.ring_pos = (self.ring_pos + 1) % w;
                if self.n <= w {
                    self.m *= e;
                } else {
                    self.m = self.m / old * e;
                }
            }
        }

        if self.m >= self.threshold {
            Ok(ChartSignal::OutOfControl {
                statistic: self.m,
                observation_index: self.n - 1,
            })
        } else {
            Ok(ChartSignal::InControl)
        }
    }

    pub fn reset(&mut self) {
        self.m = 1.0;
        self.n = 0;
        if let EDetectorWindow::Fixed { width } = &self.window {
            self.ring = vec![1.0; *width];
            self.ring_pos = 0;
        }
    }

    #[must_use]
    pub fn e_process(&self) -> f64 {
        self.m
    }

    #[must_use]
    pub fn n_observations(&self) -> usize {
        self.n
    }
}
