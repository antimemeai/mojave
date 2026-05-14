#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod arl;
pub mod combined;
pub mod cusum;
pub mod cusum_fir;
pub mod e_detector;
pub mod ewma;
pub mod shewhart;
pub mod types;

pub use arl::{cusum_arl, ewma_arl};
pub use combined::{CombinedChart, CombinedConfig};
pub use cusum::{CusumChart, CusumConfig};
pub use cusum_fir::{FirCusumChart, FirCusumConfig};
pub use e_detector::{EDetector, EDetectorConfig, EDetectorWindow, EValueSource, GaussianEValue};
pub use ewma::{EwmaChart, EwmaConfig};
pub use shewhart::{ShewhartChart, ShewhartConfig, ShewhartRule};
pub use types::{ChartSignal, ControlLimits, SpcError};

#[cfg(feature = "g-theory")]
pub mod g_theory;

#[cfg(feature = "g-theory")]
pub use g_theory::control_limits_from_g_theory;
