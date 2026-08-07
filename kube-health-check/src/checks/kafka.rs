//! Kafka readiness check via TCP connectivity to bootstrap brokers.
//!
//! Avoids linking `librdkafka` so the crate builds cleanly on all platforms.
//! For richer metadata checks, register a [`crate::checks::CustomCheck`].

use async_trait::async_trait;
use tokio::net::TcpStream;

use crate::check::{CheckOptions, HealthCheck};

/// Kafka dependency check (TCP to at least one bootstrap broker).
pub struct KafkaCheck {
    name: String,
    brokers: Vec<String>,
    options: CheckOptions,
}

impl KafkaCheck {
    /// Create with bootstrap brokers list (comma-separated `host:port`).
    pub fn new(brokers: impl AsRef<str>) -> Self {
        let brokers = brokers
            .as_ref()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Self {
            name: "kafka".into(),
            brokers,
            options: CheckOptions::default(),
        }
    }

    /// Override check name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Attach options.
    pub fn with_options(mut self, options: CheckOptions) -> Self {
        self.options = options;
        self
    }
}

#[async_trait]
impl HealthCheck for KafkaCheck {
    fn name(&self) -> &str {
        &self.name
    }

    fn options(&self) -> CheckOptions {
        self.options.clone()
    }

    async fn check(&self) -> Result<(), String> {
        if self.brokers.is_empty() {
            return Err("kafka: no brokers configured".into());
        }
        let mut errors = Vec::new();
        for addr in &self.brokers {
            match TcpStream::connect(addr).await {
                Ok(_) => return Ok(()),
                Err(e) => errors.push(format!("{addr}: {e}")),
            }
        }
        Err(format!(
            "kafka: all brokers unreachable ({})",
            errors.join("; ")
        ))
    }
}
