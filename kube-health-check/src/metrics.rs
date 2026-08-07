//! Prometheus-compatible metrics exporter (FR-008).

use std::time::Duration;

use prometheus::{
    Encoder, GaugeVec, HistogramOpts, HistogramVec, IntGaugeVec, Opts, Registry, TextEncoder,
};

use crate::status::HealthStatus;

/// Prometheus metrics for health checks.
#[derive(Clone)]
pub struct HealthMetrics {
    registry: Registry,
    duration: HistogramVec,
    dependency_status: IntGaugeVec,
    dependency_latency: GaugeVec,
    memory_usage: prometheus::Gauge,
    cpu_usage: prometheus::Gauge,
    overall_status: prometheus::IntGauge,
}

impl HealthMetrics {
    /// Create metrics bound to a private registry.
    pub fn new() -> prometheus::Result<Self> {
        let registry = Registry::new();
        let duration = HistogramVec::new(
            HistogramOpts::new(
                "health_check_duration_seconds",
                "Duration of health check evaluations",
            )
            .buckets(vec![0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 2.0]),
            &["check"],
        )?;
        let dependency_status = IntGaugeVec::new(
            Opts::new(
                "dependency_status",
                "Dependency health status (1=Healthy,2=Degraded,3=Unhealthy,4=Starting)",
            ),
            &["dependency"],
        )?;
        let dependency_latency = GaugeVec::new(
            Opts::new(
                "dependency_latency",
                "Last observed dependency check latency in seconds",
            ),
            &["dependency"],
        )?;
        let memory_usage =
            prometheus::Gauge::new("memory_usage", "Process/host memory usage bytes")?;
        let cpu_usage = prometheus::Gauge::new("cpu_usage", "CPU usage percent")?;
        let overall_status = prometheus::IntGauge::new(
            "health_overall_status",
            "Overall health status (1=Healthy,2=Degraded,3=Unhealthy,4=Starting)",
        )?;

        registry.register(Box::new(duration.clone()))?;
        registry.register(Box::new(dependency_status.clone()))?;
        registry.register(Box::new(dependency_latency.clone()))?;
        registry.register(Box::new(memory_usage.clone()))?;
        registry.register(Box::new(cpu_usage.clone()))?;
        registry.register(Box::new(overall_status.clone()))?;

        Ok(Self {
            registry,
            duration,
            dependency_status,
            dependency_latency,
            memory_usage,
            cpu_usage,
            overall_status,
        })
    }

    /// Record a single check observation.
    pub fn observe_check(&self, name: &str, status: HealthStatus, latency: Duration) {
        let secs = latency.as_secs_f64();
        self.duration.with_label_values(&[name]).observe(secs);
        self.dependency_status
            .with_label_values(&[name])
            .set(status.as_metric());
        self.dependency_latency.with_label_values(&[name]).set(secs);
    }

    /// Record overall aggregation.
    pub fn observe_overall(&self, status: HealthStatus, latency: Duration) {
        self.overall_status.set(status.as_metric());
        self.duration
            .with_label_values(&["__overall__"])
            .observe(latency.as_secs_f64());
    }

    /// Update memory gauge.
    pub fn set_memory_usage(&self, bytes: f64) {
        self.memory_usage.set(bytes);
    }

    /// Update CPU gauge.
    pub fn set_cpu_usage(&self, percent: f64) {
        self.cpu_usage.set(percent);
    }

    /// Encode metrics in Prometheus text format.
    pub fn encode(&self) -> String {
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        encoder.encode(&metric_families, &mut buffer).ok();
        String::from_utf8_lossy(&buffer).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes() {
        let m = HealthMetrics::new().unwrap();
        m.observe_check("db", HealthStatus::Healthy, Duration::from_millis(3));
        m.observe_overall(HealthStatus::Healthy, Duration::from_millis(3));
        let text = m.encode();
        assert!(text.contains("health_check_duration_seconds"));
        assert!(text.contains("dependency_status"));
    }
}
