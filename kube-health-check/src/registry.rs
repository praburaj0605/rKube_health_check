//! Check registration and bookkeeping.

use std::sync::Arc;

use crate::check::{CheckOptions, HealthCheck};

/// Registered check with resolved options.
pub struct RegisteredCheck {
    /// The check implementation.
    pub check: Arc<dyn HealthCheck>,
    /// Effective options for this check.
    pub options: CheckOptions,
    /// Consecutive failure counter.
    pub consecutive_failures: u32,
}

/// Registry of named health checks.
#[derive(Default)]
pub struct CheckRegistry {
    checks: Vec<RegisteredCheck>,
}

impl CheckRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Register a check.
    pub fn register(&mut self, check: Arc<dyn HealthCheck>, options: CheckOptions) {
        self.checks.push(RegisteredCheck {
            check,
            options,
            consecutive_failures: 0,
        });
    }

    /// Number of registered checks.
    pub fn len(&self) -> usize {
        self.checks.len()
    }

    /// Returns `true` if no checks are registered.
    pub fn is_empty(&self) -> bool {
        self.checks.is_empty()
    }

    /// Mutable access to registered checks.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut RegisteredCheck> {
        self.checks.iter_mut()
    }

    /// Immutable access.
    pub fn iter(&self) -> impl Iterator<Item = &RegisteredCheck> {
        self.checks.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::CustomCheck;

    #[tokio::test]
    async fn registers() {
        let mut reg = CheckRegistry::new();
        let c = Arc::new(CustomCheck::new("x", || async { Ok(()) }));
        reg.register(c, CheckOptions::default());
        assert_eq!(reg.len(), 1);
    }
}
