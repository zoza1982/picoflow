//! PicoFlow - Lightweight DAG workflow orchestrator for edge devices

pub mod cli;
pub mod cron_scheduler;
pub mod daemon;
pub mod dag;
pub mod error;
pub mod executors;
pub mod logging;
/// Prometheus metrics endpoint. Behind the optional `metrics` feature (off by default)
/// so the edge binary doesn't pay for it unless explicitly enabled.
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod models;
pub mod parser;
pub mod retry;
pub mod scheduler;
pub mod state;
pub mod templates;
