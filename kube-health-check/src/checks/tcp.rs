//! Generic TCP connectivity check (useful for MSSQL and unknown services).

use async_trait::async_trait;
use tokio::net::TcpStream;

use crate::check::{CheckOptions, HealthCheck};

/// Verifies that a TCP endpoint accepts connections.
pub struct TcpCheck {
    name: String,
    addr: String,
    options: CheckOptions,
}

impl TcpCheck {
    /// Create a TCP check for `host:port`.
    pub fn new(name: impl Into<String>, addr: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            addr: addr.into(),
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
impl HealthCheck for TcpCheck {
    fn name(&self) -> &str {
        &self.name
    }

    fn options(&self) -> CheckOptions {
        self.options.clone()
    }

    async fn check(&self) -> Result<(), String> {
        TcpStream::connect(&self.addr)
            .await
            .map(|_| ())
            .map_err(|e| format!("tcp connect to {}: {e}", self.addr))
    }
}
