//! Health status enumeration and helpers.

use serde::{Deserialize, Serialize};

/// Aggregated application health status (FR-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum HealthStatus {
    /// All critical and non-critical checks passed.
    Healthy,
    /// Non-critical check(s) failed; service may still serve traffic.
    Degraded,
    /// Critical check(s) failed.
    Unhealthy,
    /// Application is still starting up.
    Starting,
}

impl HealthStatus {
    /// Returns `true` when Kubernetes readiness should succeed.
    pub fn is_ready(self, allow_degraded: bool) -> bool {
        matches!(self, HealthStatus::Healthy)
            || (allow_degraded && matches!(self, HealthStatus::Degraded))
    }

    /// Numeric rank used for Prometheus gauges (`1` healthy … `4` starting).
    pub fn as_metric(self) -> i64 {
        match self {
            HealthStatus::Healthy => 1,
            HealthStatus::Degraded => 2,
            HealthStatus::Unhealthy => 3,
            HealthStatus::Starting => 4,
        }
    }
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Degraded => write!(f, "Degraded"),
            HealthStatus::Unhealthy => write!(f, "Unhealthy"),
            HealthStatus::Starting => write!(f, "Starting"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_semantics() {
        assert!(HealthStatus::Healthy.is_ready(false));
        assert!(!HealthStatus::Degraded.is_ready(false));
        assert!(HealthStatus::Degraded.is_ready(true));
        assert!(!HealthStatus::Unhealthy.is_ready(true));
        assert!(!HealthStatus::Starting.is_ready(true));
    }
}
