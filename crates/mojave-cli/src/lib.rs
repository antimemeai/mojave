#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod commands;
pub mod config;
pub mod detect;
pub mod error;
pub mod hint;
pub mod output;
