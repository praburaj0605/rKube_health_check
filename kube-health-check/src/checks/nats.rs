//! NATS readiness check.

use async_trait::async_trait;

use crate::check::{CheckOptions, HealthCheck};

/// NATS dependency check via connect + flush.
pub struct NatsCheck {
    name: String,
    url: String,
    options: CheckOptions,
}

impl NatsCheck {
    /// Create with a NATS server URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            name: "nats".into(),
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
impl HealthCheck for NatsCheck {
    fn name(&self) -> &str {
        &self.name
    }

    fn options(&self) -> CheckOptions {
        self.options.clone()
    }

    async fn check(&self) -> Result<(), String> {
        let client = async_nats::connect(&self.url)
            .await
            .map_err(|e| crate::check::sanitize_error(format!("nats: {e}")))?;
        client
            .flush()
            .await
            .map_err(|e| format!("nats flush: {e}"))?;
        Ok(())
    }
}
