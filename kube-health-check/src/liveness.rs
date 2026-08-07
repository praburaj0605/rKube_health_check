//! Process-local liveness monitoring (FR-003).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::check::CheckResult;
use crate::config::HealthConfig;
use crate::status::HealthStatus;

/// Tracks runtime liveness signals without external I/O.
pub struct LivenessMonitor {
    last_heartbeat_ms: AtomicU64,
    panicked: AtomicBool,
    started: Instant,
    /// Max age of heartbeat before considering event loop stuck (0 = disabled).
    pub heartbeat_timeout: Duration,
}

impl Default for LivenessMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl LivenessMonitor {
    /// Create a monitor with heartbeat disabled by default.
    pub fn new() -> Self {
        let now = now_ms();
        Self {
            last_heartbeat_ms: AtomicU64::new(now),
            panicked: AtomicBool::new(false),
            started: Instant::now(),
            heartbeat_timeout: Duration::ZERO,
        }
    }

    /// Record event-loop / worker heartbeat.
    pub fn heartbeat(&self) {
        self.last_heartbeat_ms.store(now_ms(), Ordering::Relaxed);
    }

    /// Record that a panic was caught.
    pub fn note_panic(&self) {
        self.panicked.store(true, Ordering::Relaxed);
    }

    /// Clear panic flag after recovery.
    pub fn clear_panic(&self) {
        self.panicked.store(false, Ordering::Relaxed);
    }

    /// Evaluate liveness against config thresholds.
    pub fn check(&self, config: &HealthConfig) -> CheckResult {
        let start = Instant::now();

        if self.panicked.load(Ordering::Relaxed) {
            return CheckResult::unhealthy(
                "liveness",
                start.elapsed(),
                "panic recovery state active",
            );
        }

        if !self.heartbeat_timeout.is_zero() {
            let last = self.last_heartbeat_ms.load(Ordering::Relaxed);
            let age = now_ms().saturating_sub(last);
            if age > self.heartbeat_timeout.as_millis() as u64 {
                return CheckResult::unhealthy(
                    "liveness",
                    start.elapsed(),
                    format!("heartbeat stale ({age}ms)"),
                );
            }
        }

        #[cfg(feature = "liveness-ext")]
        {
            if let Some(limit) = config.memory_threshold_bytes {
                if let Some(used) = memory_used_bytes() {
                    if used > limit {
                        return CheckResult::unhealthy(
                            "liveness",
                            start.elapsed(),
                            format!("memory {used} exceeds threshold {limit}"),
                        );
                    }
                }
            }
            if let Some(limit) = config.cpu_threshold_percent {
                if let Some(cpu) = cpu_usage_percent() {
                    if cpu > limit {
                        return CheckResult::unhealthy(
                            "liveness",
                            start.elapsed(),
                            format!("cpu {cpu:.1}% exceeds threshold {limit:.1}%"),
                        );
                    }
                }
            }
        }

        #[cfg(not(feature = "liveness-ext"))]
        {
            let _ = config;
        }

        // Runtime alive + worker threads: process is executing this code.
        let mut r = CheckResult::healthy("liveness", start.elapsed());
        r.status = HealthStatus::Healthy;
        // Uptime available for debugging via latency alone; keep message-free.
        let _ = self.started;
        r
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(feature = "liveness-ext")]
fn memory_used_bytes() -> Option<u64> {
    use sysinfo::System;
    let mut sys = System::new();
    sys.refresh_memory();
    Some(sys.used_memory())
}

#[cfg(feature = "liveness-ext")]
fn cpu_usage_percent() -> Option<f32> {
    use sysinfo::System;
    let mut sys = System::new();
    sys.refresh_cpu_usage();
    // First refresh is often 0; do a short sample.
    std::thread::sleep(Duration::from_millis(50));
    sys.refresh_cpu_usage();
    Some(sys.global_cpu_usage())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_by_default() {
        let mon = LivenessMonitor::new();
        let cfg = HealthConfig::new("svc");
        assert!(mon.check(&cfg).is_ok());
    }

    #[test]
    fn panic_flag() {
        let mon = LivenessMonitor::new();
        mon.note_panic();
        let cfg = HealthConfig::new("svc");
        assert!(!mon.check(&cfg).is_ok());
        mon.clear_panic();
        assert!(mon.check(&cfg).is_ok());
    }

    #[test]
    fn stale_heartbeat() {
        let mut mon = LivenessMonitor::new();
        mon.heartbeat_timeout = Duration::from_millis(10);
        mon.last_heartbeat_ms.store(0, Ordering::Relaxed);
        let cfg = HealthConfig::new("svc");
        assert!(!mon.check(&cfg).is_ok());
    }
}
