//! Status aggregation rules (FR-007).

use crate::check::CheckResult;
use crate::startup::StartupState;
use crate::status::HealthStatus;

/// Aggregate individual check results into an overall status.
///
/// Rules:
/// - If startup is incomplete (and not Failed), status is `Starting`.
/// - Any critical failure → `Unhealthy`.
/// - Any non-critical failure → `Degraded` (unless already Unhealthy).
/// - Otherwise → `Healthy`.
pub fn aggregate(results: &[CheckResult], startup: StartupState) -> HealthStatus {
    if !startup.is_ready() && !startup.is_failed() {
        return HealthStatus::Starting;
    }
    if startup.is_failed() {
        return HealthStatus::Unhealthy;
    }

    let mut degraded = false;
    for r in results {
        if r.is_ok() {
            continue;
        }
        if r.critical {
            return HealthStatus::Unhealthy;
        }
        degraded = true;
    }

    if degraded {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn starting_overrides() {
        let results = vec![CheckResult::healthy("a", Duration::ZERO)];
        assert_eq!(
            aggregate(&results, StartupState::Initializing),
            HealthStatus::Starting
        );
    }

    #[test]
    fn critical_failure() {
        let mut bad = CheckResult::unhealthy("db", Duration::ZERO, "err");
        bad.critical = true;
        assert_eq!(
            aggregate(&[bad], StartupState::Ready),
            HealthStatus::Unhealthy
        );
    }

    #[test]
    fn non_critical_degraded() {
        let mut bad = CheckResult::unhealthy("cache", Duration::ZERO, "err");
        bad.critical = false;
        assert_eq!(
            aggregate(&[bad], StartupState::Ready),
            HealthStatus::Degraded
        );
    }

    #[test]
    fn all_healthy() {
        let ok = CheckResult::healthy("a", Duration::ZERO);
        assert_eq!(aggregate(&[ok], StartupState::Ready), HealthStatus::Healthy);
    }
}
