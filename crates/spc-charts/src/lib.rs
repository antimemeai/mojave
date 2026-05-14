#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod shewhart;
pub mod types;

pub use shewhart::{ShewhartChart, ShewhartConfig, ShewhartRule};
pub use types::{ChartSignal, ControlLimits, SpcError};
