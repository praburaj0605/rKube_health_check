//! Redis readiness check.

use async_trait::async_trait;
use redis::aio::ConnectionManager;

use crate::check::{CheckOptions, HealthCheck};

/// Redis dependency check using PING.
pub struct RedisCheck {
    name: String,
    conn: ConnectionManager,
    options: CheckOptions,
}

impl RedisCheck {
    /// Create from a connection manager.
    pub fn new(conn: ConnectionManager) -> Self {
        Self {
            name: "redis".into(),
            conn,
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
impl HealthCheck for RedisCheck {
    fn name(&self) -> &str {
        &self.name
    }

    fn options(&self) -> CheckOptions {
        self.options.clone()
    }

    async fn check(&self) -> Result<(), String> {
        let mut conn = self.conn.clone();
        redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .map(|_| ())
            .map_err(|e| crate::check::sanitize_error(e.to_string()))
    }
}
