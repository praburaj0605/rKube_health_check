//! RabbitMQ readiness check via lapin.

use async_trait::async_trait;
use lapin::Connection;

use crate::check::{CheckOptions, HealthCheck};

/// RabbitMQ dependency check.
pub struct RabbitMqCheck {
    name: String,
    url: String,
    options: CheckOptions,
}

impl RabbitMqCheck {
    /// Create with an AMQP URL (secrets are never returned in errors).
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            name: "rabbitmq".into(),
            url: url.into(),
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
impl HealthCheck for RabbitMqCheck {
    fn name(&self) -> &str {
        &self.name
    }

    fn options(&self) -> CheckOptions {
        self.options.clone()
    }

    async fn check(&self) -> Result<(), String> {
        let conn = Connection::connect(&self.url, lapin::ConnectionProperties::default())
            .await
            .map_err(|e| crate::check::sanitize_error(format!("rabbitmq: {e}")))?;
        let _ = conn.close(200, "health-check").await;
        Ok(())
    }
}
