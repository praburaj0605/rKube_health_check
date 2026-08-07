//! Production-ready Kubernetes health check library for Rust.
//!
//! Exposes liveness, readiness, and startup probes with pluggable dependency
//! checks, custom async checks, status aggregation, and optional Prometheus metrics.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use kube_health_check::HealthBuilder;
//!
//! # async fn demo() {
//! let health = HealthBuilder::new("my-service")
//!     .version(env!("CARGO_PKG_VERSION"))
//!     .custom("ready", || async { Ok(()) })
//!     .build();
//!
//! let report = health.ready().await;
//! assert!(report.is_success());
//! # }
//! ```

#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

mod aggregation;
mod builder;
mod check;
mod config;
mod manager;
mod registry;
mod response;
mod startup;
mod status;

pub mod checks;
pub mod liveness;

#[cfg(feature = "metrics")]
pub mod metrics;

pub use aggregation::aggregate;
pub use builder::HealthBuilder;
pub use check::{sanitize_error, CheckOptions, CheckResult, HealthCheck};
pub use config::{DetailLevel, HealthConfig};
pub use manager::HealthManager;
pub use registry::CheckRegistry;
pub use response::{CheckReport, HealthReport};
pub use startup::StartupState;
pub use status::HealthStatus;
