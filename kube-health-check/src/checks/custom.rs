//! Unlimited custom async health checks (FR-005).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;

use crate::check::{CheckOptions, HealthCheck};

type CheckFut = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;
type CheckFn = dyn Fn() -> CheckFut + Send + Sync;

/// Closure-backed custom health check.
pub struct CustomCheck {
    name: String,
    func: Arc<CheckFn>,
    options: CheckOptions,
}

impl CustomCheck {
    /// Create a custom check from an async closure.
    pub fn new<F, Fut>(name: impl Into<String>, f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        Self {
            name: name.into(),
            func: Arc::new(move || Box::pin(f())),
            options: CheckOptions::default(),
        }
    }

    /// Attach options.
    pub fn with_options(mut self, options: CheckOptions) -> Self {
        self.options = options;
        self
    }
}

#[async_trait]
impl HealthCheck for CustomCheck {
    fn name(&self) -> &str {
        &self.name
    }

    fn options(&self) -> CheckOptions {
        self.options.clone()
    }

    async fn check(&self) -> Result<(), String> {
        (self.func)().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ok_and_err() {
        let ok = CustomCheck::new("a", || async { Ok(()) });
        assert!(ok.check().await.is_ok());
        let bad = CustomCheck::new("b", || async { Err("x".into()) });
        assert!(bad.check().await.is_err());
    }
}
