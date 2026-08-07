//! PostgreSQL readiness check via sqlx.

use async_trait::async_trait;
use sqlx::postgres::PgPool;

use crate::check::{CheckOptions, HealthCheck};

/// PostgreSQL dependency check.
pub struct PostgresCheck {
    name: String,
    pool: PgPool,
    options: CheckOptions,
}

impl PostgresCheck {
    /// Create from an existing pool.
    pub fn new(pool: PgPool) -> Self {
        Self {
            name: "postgres".into(),
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
impl HealthCheck for PostgresCheck {
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
