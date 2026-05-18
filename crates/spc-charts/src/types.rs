use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChartSignal {
    InControl,
    Warning {
        statistic: f64,
    },
    OutOfControl {
        statistic: f64,
        observation_index: usize,
    },
}

impl ChartSignal {
    #[must_use]
    pub fn is_out_of_control(&self) -> bool {
        matches!(self, Self::OutOfControl { .. })
    }

    #[must_use]
    pub fn is_in_control(&self) -> bool {
        matches!(self, Self::InControl)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlLimits {
    pub mu_0: f64,
    pub sigma: f64,
}

impl ControlLimits {
    pub fn new(mu_0: f64, sigma: f64) -> Result<Self, SpcError> {
        if !sigma.is_finite() || sigma <= 0.0 {
            return Err(SpcError::NonPositiveSigma(sigma));
        }
        if !mu_0.is_finite() {
            return Err(SpcError::NonFiniteMu(mu_0));
        }
        Ok(Self { mu_0, sigma })
    }
}

#[derive(Debug, Clone, Error)]
pub enum SpcError {
    #[error("sigma must be positive and finite, got {0}")]
    NonPositiveSigma(f64),
    #[error("mu_0 must be finite, got {0}")]
    NonFiniteMu(f64),
    #[error("parameter {name} must be positive, got {value}")]
    NonPositiveParam { name: &'static str, value: f64 },
    #[error("lambda must be in (0, 1], got {0}")]
    InvalidLambda(f64),
    #[error("alpha must be in (0, 1), got {0}")]
    InvalidAlpha(f64),
    #[error("ARL matrix is singular at h={0}")]
    SingularArlMatrix(f64),
    #[error("window width must be >= 1, got {0}")]
    InvalidWindowWidth(usize),
    #[error("observation must be finite, got {0}")]
    NonFiniteInput(f64),
}
