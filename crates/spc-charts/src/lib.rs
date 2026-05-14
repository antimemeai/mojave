#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod combined;
pub mod cusum;
pub mod cusum_fir;
pub mod ewma;
pub mod shewhart;
pub mod types;

pub use combined::{CombinedChart, CombinedConfig};
pub use cusum::{CusumChart, CusumConfig};
pub use cusum_fir::{FirCusumChart, FirCusumConfig};
pub use ewma::{EwmaChart, EwmaConfig};
pub use shewhart::{ShewhartChart, ShewhartConfig, ShewhartRule};
pub use types::{ChartSignal, ControlLimits, SpcError};
