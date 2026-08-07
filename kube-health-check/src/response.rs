//! Standardized JSON health response types (FR-010).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::check::CheckResult;
use crate::config::{DetailLevel, HealthConfig};
use crate::startup::StartupState;
use crate::status::HealthStatus;

/// Individual check entry in a health report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckReport {
    /// Check name.
    pub name: String,
    /// Check status.
    pub status: HealthStatus,
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Optional error (omitted/null at lower detail levels).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Completion timestamp.
    pub timestamp: DateTime<Utc>,
}

/// Top-level health response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Service name.
    pub service: String,
    /// Version.
    pub version: String,
    /// Build id.
    pub build: String,
    /// Report timestamp.
    pub timestamp: DateTime<Utc>,
    /// Hostname.
    pub hostname: String,
    /// Aggregated status.
    pub status: HealthStatus,
    /// Overall latency in milliseconds.
    pub latency_ms: u64,
    /// Startup state (useful for `/startup`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup: Option<StartupState>,
    /// Individual checks (omitted for Minimal detail).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checks: Option<Vec<CheckReport>>,
}

impl HealthReport {
    /// Build a report from config + aggregated results.
    pub fn from_results(
        config: &HealthConfig,
        status: HealthStatus,
        latency_ms: u64,
        results: &[CheckResult],
        startup: Option<StartupState>,
    ) -> Self {
        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".into());

        let checks = match config.detail_level {
            DetailLevel::Minimal => None,
            DetailLevel::Standard => Some(
                results
                    .iter()
                    .map(|r| CheckReport {
                        name: r.name.clone(),
                        status: r.status,
                        latency_ms: duration_ms(r.latency),
                        error: None,
                        timestamp: r.timestamp,
                    })
                    .collect(),
            ),
            DetailLevel::Full => Some(
                results
                    .iter()
                    .map(|r| CheckReport {
                        name: r.name.clone(),
                        status: r.status,
                        latency_ms: duration_ms(r.latency),
                        error: r.error.clone(),
                        timestamp: r.timestamp,
                    })
                    .collect(),
            ),
        };

        Self {
            service: config.service_name.clone(),
            version: config.version.clone(),
            build: config.build.clone(),
            timestamp: Utc::now(),
            hostname,
            status,
            latency_ms,
            startup,
            checks,
        }
    }

    /// Returns `true` when aggregated status is Healthy (or Degraded if allowed).
    pub fn is_success(&self) -> bool {
        matches!(self.status, HealthStatus::Healthy)
    }

    /// Suggested HTTP status code for readiness/startup style probes.
    pub fn http_status_ready(&self, allow_degraded: bool) -> u16 {
        if self.status.is_ready(allow_degraded) {
            200
        } else {
            503
        }
    }

    /// Suggested HTTP status for liveness (`/live`).
    pub fn http_status_live(&self) -> u16 {
        if matches!(self.status, HealthStatus::Unhealthy) {
            503
        } else {
            200
        }
    }
}

fn duration_ms(d: std::time::Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn minimal_omits_checks() {
        let mut cfg = HealthConfig::new("svc");
        cfg.detail_level = DetailLevel::Minimal;
        let results = vec![CheckResult::healthy("db", Duration::from_millis(1))];
        let report = HealthReport::from_results(&cfg, HealthStatus::Healthy, 1, &results, None);
        assert!(report.checks.is_none());
    }

    #[test]
    fn full_includes_errors() {
        let mut cfg = HealthConfig::new("svc");
        cfg.detail_level = DetailLevel::Full;
        let results = vec![CheckResult::unhealthy(
            "db",
            Duration::from_millis(2),
            "boom",
        )];
        let report = HealthReport::from_results(&cfg, HealthStatus::Unhealthy, 2, &results, None);
        assert_eq!(
            report.checks.as_ref().unwrap()[0].error.as_deref(),
            Some("boom")
        );
    }
}
