//! SQLite readiness check via sqlx.

use async_trait::async_trait;
use sqlx::sqlite::SqlitePool;

use crate::check::{CheckOptions, HealthCheck};

/// SQLite dependency check.
pub struct SqliteCheck {
    name: String,
    pool: SqlitePool,
    options: CheckOptions,
}

impl SqliteCheck {
    /// Create from an existing pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            name: "sqlite".into(),
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
impl HealthCheck for SqliteCheck {
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
