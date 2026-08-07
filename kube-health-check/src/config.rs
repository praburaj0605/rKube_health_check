//! Configuration types for health reporting.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// How much detail to include in JSON health responses (security NFR).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DetailLevel {
    /// Status and service metadata only; no check list or errors.
    Minimal,
    /// Include check names and statuses without error messages.
    #[default]
    Standard,
    /// Include full check details including sanitized error messages.
    Full,
}

/// Runtime configuration for the health manager.
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Service / application name.
    pub service_name: String,
    /// Semantic version string.
    pub version: String,
    /// Build identifier (commit SHA, CI build number, etc.).
    pub build: String,
    /// Response detail level.
    pub detail_level: DetailLevel,
    /// Default per-check timeout.
    pub default_timeout: Duration,
    /// Default retry count after the first attempt.
    pub default_retry_count: u32,
    /// Delay between retries.
    pub default_retry_interval: Duration,
    /// Grace period after process start before failures count as Unhealthy.
    pub grace_period: Duration,
    /// Consecutive failures required before marking Unhealthy.
    pub failure_threshold: u32,
    /// When `true`, `/ready` returns 200 for Degraded status.
    pub allow_degraded_ready: bool,
    /// Optional memory usage threshold in bytes (liveness-ext).
    pub memory_threshold_bytes: Option<u64>,
    /// Optional CPU usage threshold as percentage 0–100 (liveness-ext).
    pub cpu_threshold_percent: Option<f32>,
}

impl HealthConfig {
    /// Create config with sensible production defaults.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            version: String::from("0.0.0"),
            build: String::from("unknown"),
            detail_level: DetailLevel::Standard,
            default_timeout: Duration::from_secs(2),
            default_retry_count: 0,
            default_retry_interval: Duration::from_millis(100),
            grace_period: Duration::from_secs(0),
            failure_threshold: 1,
            allow_degraded_ready: false,
            memory_threshold_bytes: None,
            cpu_threshold_percent: None,
        }
    }

    /// Load overrides from environment variables prefixed with `HEALTH_`.
    ///
    /// Supported keys:
    /// - `HEALTH_DETAIL_LEVEL` = `minimal` | `standard` | `full`
    /// - `HEALTH_DEFAULT_TIMEOUT_MS`
    /// - `HEALTH_RETRY_COUNT`
    /// - `HEALTH_RETRY_INTERVAL_MS`
    /// - `HEALTH_GRACE_PERIOD_MS`
    /// - `HEALTH_FAILURE_THRESHOLD`
    /// - `HEALTH_ALLOW_DEGRADED_READY` = `true` | `false`
    /// - `HEALTH_MEMORY_THRESHOLD_BYTES`
    /// - `HEALTH_CPU_THRESHOLD_PERCENT`
    pub fn from_env(mut self) -> Self {
        if let Ok(v) = std::env::var("HEALTH_DETAIL_LEVEL") {
            self.detail_level = match v.to_ascii_lowercase().as_str() {
                "minimal" => DetailLevel::Minimal,
                "full" => DetailLevel::Full,
                _ => DetailLevel::Standard,
            };
        }
        if let Ok(v) = std::env::var("HEALTH_DEFAULT_TIMEOUT_MS") {
            if let Ok(ms) = v.parse::<u64>() {
                self.default_timeout = Duration::from_millis(ms);
            }
        }
        if let Ok(v) = std::env::var("HEALTH_RETRY_COUNT") {
            if let Ok(n) = v.parse::<u32>() {
                self.default_retry_count = n;
            }
        }
        if let Ok(v) = std::env::var("HEALTH_RETRY_INTERVAL_MS") {
            if let Ok(ms) = v.parse::<u64>() {
                self.default_retry_interval = Duration::from_millis(ms);
            }
        }
        if let Ok(v) = std::env::var("HEALTH_GRACE_PERIOD_MS") {
            if let Ok(ms) = v.parse::<u64>() {
                self.grace_period = Duration::from_millis(ms);
            }
        }
        if let Ok(v) = std::env::var("HEALTH_FAILURE_THRESHOLD") {
            if let Ok(n) = v.parse::<u32>() {
                self.failure_threshold = n.max(1);
            }
        }
        if let Ok(v) = std::env::var("HEALTH_ALLOW_DEGRADED_READY") {
            self.allow_degraded_ready =
                matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes");
        }
        if let Ok(v) = std::env::var("HEALTH_MEMORY_THRESHOLD_BYTES") {
            if let Ok(n) = v.parse::<u64>() {
                self.memory_threshold_bytes = Some(n);
            }
        }
        if let Ok(v) = std::env::var("HEALTH_CPU_THRESHOLD_PERCENT") {
            if let Ok(n) = v.parse::<f32>() {
                self.cpu_threshold_percent = Some(n);
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let c = HealthConfig::new("svc");
        assert_eq!(c.service_name, "svc");
        assert_eq!(c.detail_level, DetailLevel::Standard);
        assert_eq!(c.failure_threshold, 1);
    }
}
