#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod types;

pub use types::{ChartSignal, ControlLimits, SpcError};
