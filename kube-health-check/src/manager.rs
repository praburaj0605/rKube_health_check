//! Health manager: concurrent check execution and probe reports.

use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;

use crate::aggregation::aggregate;
use crate::check::{CheckOptions, CheckResult, HealthCheck};
use crate::config::HealthConfig;
use crate::liveness::LivenessMonitor;
use crate::registry::CheckRegistry;
use crate::response::HealthReport;
use crate::startup::StartupState;
use crate::status::HealthStatus;

#[cfg(feature = "metrics")]
use crate::metrics::HealthMetrics;

/// Orchestrates health checks and exposes probe reports.
pub struct HealthManager {
    config: HealthConfig,
    registry: RwLock<CheckRegistry>,
    startup: RwLock<StartupState>,
    started_at: Instant,
    liveness: LivenessMonitor,
    #[cfg(feature = "metrics")]
    metrics: Option<HealthMetrics>,
}

impl HealthManager {
    pub(crate) fn new(
        config: HealthConfig,
        registry: CheckRegistry,
        #[cfg(feature = "metrics")] metrics: Option<HealthMetrics>,
    ) -> Self {
        Self {
            config,
            registry: RwLock::new(registry),
            startup: RwLock::new(StartupState::Initializing),
            started_at: Instant::now(),
            liveness: LivenessMonitor::new(),
            #[cfg(feature = "metrics")]
            metrics,
        }
    }

    /// Shared reference helper.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Borrow configuration.
    pub fn config(&self) -> &HealthConfig {
        &self.config
    }

    /// Current startup state.
    pub async fn startup_state(&self) -> StartupState {
        *self.startup.read().await
    }

    /// Set startup state explicitly.
    pub async fn set_startup_state(&self, state: StartupState) {
        *self.startup.write().await = state;
    }

    /// Advance startup state machine by one step.
    pub async fn advance_startup(&self) -> StartupState {
        let mut guard = self.startup.write().await;
        *guard = guard.advance();
        *guard
    }

    /// Mark startup as Ready (typical after deps connected).
    pub async fn mark_ready(&self) {
        *self.startup.write().await = StartupState::Ready;
    }

    /// Mark startup as Failed.
    pub async fn mark_failed(&self) {
        *self.startup.write().await = StartupState::Failed;
    }

    /// Register an additional check at runtime.
    pub async fn register(&self, check: Arc<dyn HealthCheck>, options: CheckOptions) {
        self.registry.write().await.register(check, options);
    }

    /// Record a liveness heartbeat (event-loop / worker responsiveness).
    pub fn heartbeat(&self) {
        self.liveness.heartbeat();
    }

    /// Mark that a panic was observed (optional panic-hook integration).
    pub fn note_panic(&self) {
        self.liveness.note_panic();
    }

    /// Aggregated `/health` report (runs readiness checks).
    pub async fn health(&self) -> HealthReport {
        self.run_readiness_report().await
    }

    /// `/ready` report.
    pub async fn ready(&self) -> HealthReport {
        self.run_readiness_report().await
    }

    /// `/live` report — process-local only (no external I/O).
    pub async fn live(&self) -> HealthReport {
        let start = Instant::now();
        let result = self.liveness.check(&self.config);
        let results = vec![result];
        let status = if results[0].is_ok() {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        };
        let latency_ms = start.elapsed().as_millis() as u64;

        #[cfg(feature = "metrics")]
        if let Some(m) = &self.metrics {
            m.observe_check(
                "liveness",
                status,
                std::time::Duration::from_millis(latency_ms),
            );
        }

        HealthReport::from_results(&self.config, status, latency_ms, &results, None)
    }

    /// `/startup` report.
    pub async fn startup(&self) -> HealthReport {
        let state = self.startup_state().await;
        let status = if state.is_ready() {
            // Still verify deps once ready so startup doesn't lie after Ready.
            let report = self.run_readiness_report().await;
            return HealthReport {
                startup: Some(state),
                ..report
            };
        } else if state.is_failed() {
            HealthStatus::Unhealthy
        } else {
            HealthStatus::Starting
        };

        HealthReport::from_results(&self.config, status, 0, &[], Some(state))
    }

    /// Gather Prometheus metrics text (feature = metrics).
    #[cfg(feature = "metrics")]
    pub fn metrics_text(&self) -> Option<String> {
        self.metrics.as_ref().map(|m| m.encode())
    }

    async fn run_readiness_report(&self) -> HealthReport {
        let start = Instant::now();
        let startup = self.startup_state().await;
        let results = self.execute_all().await;
        let status = aggregate(&results, startup);
        let latency_ms = start.elapsed().as_millis() as u64;

        #[cfg(feature = "metrics")]
        if let Some(m) = &self.metrics {
            m.observe_overall(status, std::time::Duration::from_millis(latency_ms));
            for r in &results {
                m.observe_check(&r.name, r.status, r.latency);
            }
        }

        HealthReport::from_results(&self.config, status, latency_ms, &results, Some(startup))
    }

    async fn execute_all(&self) -> Vec<CheckResult> {
        let mut registry = self.registry.write().await;
        let elapsed_since_start = self.started_at.elapsed();

        let mut futs = Vec::with_capacity(registry.len());
        // Snapshot names/options then run concurrently without holding complex borrows.
        let specs: Vec<(Arc<dyn HealthCheck>, CheckOptions, u32)> = registry
            .iter()
            .map(|r| {
                (
                    Arc::clone(&r.check),
                    r.options.clone(),
                    r.consecutive_failures,
                )
            })
            .collect();

        for (check, options, consecutive) in specs {
            let timeout = options.timeout.unwrap_or(self.config.default_timeout);
            let retries = options
                .retry_count
                .unwrap_or(self.config.default_retry_count);
            let retry_interval = options
                .retry_interval
                .unwrap_or(self.config.default_retry_interval);
            let grace = options.grace_period.unwrap_or(self.config.grace_period);
            let threshold = options
                .failure_threshold
                .unwrap_or(self.config.failure_threshold)
                .max(1);
            let critical = options.critical;
            let name = check.name().to_string();

            futs.push(async move {
                let attempt_start = Instant::now();
                let mut last_err = None;
                let mut success = false;

                for attempt in 0..=retries {
                    if attempt > 0 {
                        tokio::time::sleep(retry_interval).await;
                    }
                    let outcome = tokio::time::timeout(timeout, check.check()).await;
                    match outcome {
                        Ok(Ok(())) => {
                            success = true;
                            break;
                        }
                        Ok(Err(e)) => last_err = Some(e),
                        Err(_) => last_err = Some(format!("timeout after {timeout:?}")),
                    }
                }

                let latency = attempt_start.elapsed();
                if success {
                    let mut r = CheckResult::healthy(name, latency);
                    r.critical = critical;
                    (r, true)
                } else {
                    let in_grace = elapsed_since_start < grace;
                    let new_failures = consecutive.saturating_add(1);
                    let below_threshold = new_failures < threshold;

                    let mut r = CheckResult::unhealthy(
                        name,
                        latency,
                        last_err.unwrap_or_else(|| "unknown error".into()),
                    );
                    r.critical = critical;

                    if in_grace || below_threshold {
                        // Soften during grace / below threshold so brief blips don't fail probes.
                        r.status = HealthStatus::Healthy;
                        r.error = None;
                    }
                    (r, false)
                }
            });
        }

        let outcomes = futures::future::join_all(futs).await;

        // Update consecutive failure counters
        for (registered, (result, ok)) in registry.iter_mut().zip(outcomes.iter()) {
            if *ok {
                registered.consecutive_failures = 0;
            } else {
                registered.consecutive_failures = registered.consecutive_failures.saturating_add(1);
            }
            let _ = result;
        }

        outcomes.into_iter().map(|(r, _)| r).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::HealthBuilder;
    use crate::checks::CustomCheck;
    use std::time::Duration;

    #[tokio::test]
    async fn ready_with_custom_ok() {
        let health = HealthBuilder::new("svc")
            .custom("ok", || async { Ok(()) })
            .build();
        health.mark_ready().await;
        let report = health.ready().await;
        assert!(report.is_success());
        assert_eq!(report.http_status_ready(false), 200);
    }

    #[tokio::test]
    async fn ready_fails_critical() {
        let health = HealthBuilder::new("svc")
            .custom("bad", || async { Err("nope".into()) })
            .build();
        health.mark_ready().await;
        let report = health.ready().await;
        assert_eq!(report.status, HealthStatus::Unhealthy);
        assert_eq!(report.http_status_ready(false), 503);
    }

    #[tokio::test]
    async fn live_always_ok_by_default() {
        let health = HealthBuilder::new("svc").build();
        let report = health.live().await;
        assert_eq!(report.http_status_live(), 200);
    }

    #[tokio::test]
    async fn startup_starting_then_ready() {
        let health = HealthBuilder::new("svc")
            .check(CustomCheck::new("x", || async { Ok(()) }))
            .build();
        let s = health.startup().await;
        assert_eq!(s.status, HealthStatus::Starting);
        assert_eq!(s.http_status_ready(false), 503);
        health.mark_ready().await;
        let s2 = health.startup().await;
        assert!(s2.is_success());
    }

    #[tokio::test]
    async fn timeout_marks_unhealthy() {
        let health = HealthBuilder::new("svc")
            .default_timeout(Duration::from_millis(20))
            .custom("slow", || async {
                tokio::time::sleep(Duration::from_millis(200)).await;
                Ok(())
            })
            .build();
        health.mark_ready().await;
        let report = health.ready().await;
        assert_eq!(report.status, HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn non_critical_degraded() {
        let health = HealthBuilder::new("svc")
            .custom_with("soft", CheckOptions::default().non_critical(), || async {
                Err("x".into())
            })
            .build();
        health.mark_ready().await;
        let report = health.ready().await;
        assert_eq!(report.status, HealthStatus::Degraded);
    }
}
