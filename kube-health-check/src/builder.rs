//! Fluent builder for [`HealthManager`] (SRS Builder module).

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use crate::check::{CheckOptions, HealthCheck};
use crate::checks::CustomCheck;
use crate::config::{DetailLevel, HealthConfig};
use crate::manager::HealthManager;
use crate::registry::CheckRegistry;

#[cfg(feature = "metrics")]
use crate::metrics::HealthMetrics;

/// Fluent builder for health check configuration.
pub struct HealthBuilder {
    config: HealthConfig,
    registry: CheckRegistry,
    #[cfg(feature = "metrics")]
    enable_metrics: bool,
}

impl HealthBuilder {
    /// Start building a health manager for `service_name`.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            config: HealthConfig::new(service_name),
            registry: CheckRegistry::new(),
            #[cfg(feature = "metrics")]
            enable_metrics: false,
        }
    }

    /// Set semantic version.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.config.version = version.into();
        self
    }

    /// Set build identifier.
    pub fn build_id(mut self, build: impl Into<String>) -> Self {
        self.config.build = build.into();
        self
    }

    /// Set JSON response detail level.
    pub fn detail_level(mut self, level: DetailLevel) -> Self {
        self.config.detail_level = level;
        self
    }

    /// Default per-check timeout.
    pub fn default_timeout(mut self, timeout: Duration) -> Self {
        self.config.default_timeout = timeout;
        self
    }

    /// Default retry count and interval.
    pub fn default_retries(mut self, count: u32, interval: Duration) -> Self {
        self.config.default_retry_count = count;
        self.config.default_retry_interval = interval;
        self
    }

    /// Grace period after start before failures count.
    pub fn grace_period(mut self, period: Duration) -> Self {
        self.config.grace_period = period;
        self
    }

    /// Consecutive failures required for Unhealthy.
    pub fn failure_threshold(mut self, threshold: u32) -> Self {
        self.config.failure_threshold = threshold.max(1);
        self
    }

    /// Allow Degraded status to pass readiness probes.
    pub fn allow_degraded_ready(mut self, allow: bool) -> Self {
        self.config.allow_degraded_ready = allow;
        self
    }

    /// Memory threshold for extended liveness (bytes).
    pub fn memory_threshold_bytes(mut self, bytes: u64) -> Self {
        self.config.memory_threshold_bytes = Some(bytes);
        self
    }

    /// CPU threshold for extended liveness (percent).
    pub fn cpu_threshold_percent(mut self, pct: f32) -> Self {
        self.config.cpu_threshold_percent = Some(pct);
        self
    }

    /// Apply `HEALTH_*` environment overrides.
    pub fn from_env(mut self) -> Self {
        self.config = self.config.from_env();
        self
    }

    /// Register a typed health check.
    pub fn check(mut self, check: impl HealthCheck + 'static) -> Self {
        let options = check.options();
        self.registry.register(Arc::new(check), options);
        self
    }

    /// Register a check with explicit options.
    pub fn check_with(mut self, check: impl HealthCheck + 'static, options: CheckOptions) -> Self {
        self.registry.register(Arc::new(check), options);
        self
    }

    /// Register an async custom check (FR-005).
    pub fn custom<F, Fut>(self, name: impl Into<String>, f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        self.check(CustomCheck::new(name, f))
    }

    /// Register a custom check with options.
    pub fn custom_with<F, Fut>(self, name: impl Into<String>, options: CheckOptions, f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        self.check_with(CustomCheck::new(name, f), options)
    }

    /// Enable Prometheus metrics collection.
    #[cfg(feature = "metrics")]
    pub fn with_metrics(mut self) -> Self {
        self.enable_metrics = true;
        self
    }

    /// Finalize and create a [`HealthManager`].
    pub fn build(self) -> HealthManager {
        #[cfg(feature = "metrics")]
        let metrics = if self.enable_metrics {
            Some(HealthMetrics::new().expect("prometheus metrics registry"))
        } else {
            None
        };

        HealthManager::new(
            self.config,
            self.registry,
            #[cfg(feature = "metrics")]
            metrics,
        )
    }

    /// Finalize as `Arc<HealthManager>`.
    pub fn build_shared(self) -> Arc<HealthManager> {
        self.build().shared()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn one_liner() {
        let h = HealthBuilder::new("demo")
            .version("1.0.0")
            .custom("ping", || async { Ok(()) })
            .build();
        h.mark_ready().await;
        assert!(h.health().await.is_success());
    }
}
