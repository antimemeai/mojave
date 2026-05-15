#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod config;
pub mod instrument;
pub mod instruments;
pub mod outcome_ext;
pub mod router;
pub mod types;

pub use config::{
    AnalysisConfig, IrrConfig, IrrMetric, MonitorConfig, SequentialConfig, SequentialMethod,
    SpcChartType, SpcConfig, WindowSize,
};
pub use instrument::InstrumentId;
pub use types::{
    AnalysisReport, Decision, IrrSummary, MeasurementIssue, MonitorSummary, OrchestratorError,
    SequentialSummary, SeriesKey, SpcSummary,
};
