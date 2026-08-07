//! MySQL readiness check via sqlx.

use async_trait::async_trait;
use sqlx::mysql::MySqlPool;

use crate::check::{CheckOptions, HealthCheck};

/// MySQL dependency check.
pub struct MysqlCheck {
    name: String,
    pool: MySqlPool,
    options: CheckOptions,
}

impl MysqlCheck {
    /// Create from an existing pool.
    pub fn new(pool: MySqlPool) -> Self {
        Self {
            name: "mysql".into(),
            pool,
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
impl HealthCheck for MysqlCheck {
    fn name(&self) -> &str {
        &self.name
    }

    fn options(&self) -> CheckOptions {
        self.options.clone()
    }

    async fn check(&self) -> Result<(), String> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| crate::check::sanitize_error(e.to_string()))
    }
}
