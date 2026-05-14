#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod cusum;
pub mod cusum_fir;
pub mod shewhart;
pub mod types;

pub use cusum::{CusumChart, CusumConfig};
pub use cusum_fir::{FirCusumChart, FirCusumConfig};
pub use shewhart::{ShewhartChart, ShewhartConfig, ShewhartRule};
pub use types::{ChartSignal, ControlLimits, SpcError};
