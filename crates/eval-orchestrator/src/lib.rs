#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod config;
pub mod outcome_ext;
pub mod types;

pub use config::{
    AnalysisConfig, IrrConfig, IrrMetric, MonitorConfig, SequentialConfig, SequentialMethod,
    SpcChartType, SpcConfig, WindowSize,
};
pub use types::{
    AnalysisReport, Decision, IrrSummary, MeasurementIssue, MonitorSummary, OrchestratorError,
    SequentialSummary, SeriesKey, SpcSummary,
};
