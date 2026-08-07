//! Elasticsearch readiness check via HTTP ping (no official client crate).

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::check::{CheckOptions, HealthCheck};

/// Elasticsearch dependency check (HTTP GET `/` against host:port).
pub struct ElasticsearchCheck {
    name: String,
    /// `host:port` (default port 9200).
    addr: String,
    options: CheckOptions,
}

impl ElasticsearchCheck {
    /// Create with `host:port` (e.g. `127.0.0.1:9200`).
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            name: "elasticsearch".into(),
            addr: addr.into(),
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
impl HealthCheck for ElasticsearchCheck {
    fn name(&self) -> &str {
        &self.name
    }

    fn options(&self) -> CheckOptions {
        self.options.clone()
    }

    async fn check(&self) -> Result<(), String> {
        let mut stream = TcpStream::connect(&self.addr)
            .await
            .map_err(|e| format!("elasticsearch connect {}: {e}", self.addr))?;
        let req = format!(
            "GET / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            self.addr
        );
        stream
            .write_all(req.as_bytes())
            .await
            .map_err(|e| format!("elasticsearch write: {e}"))?;
        let mut buf = [0u8; 128];
        let n = stream
            .read(&mut buf)
            .await
            .map_err(|e| format!("elasticsearch read: {e}"))?;
        let head = String::from_utf8_lossy(&buf[..n]);
        if head.contains("200") || head.contains("401") || head.contains("403") {
            // Reachable (auth errors still mean the cluster is up).
            Ok(())
        } else {
            Err(format!(
                "elasticsearch unexpected response: {}",
                head.lines().next().unwrap_or("")
            ))
        }
    }
}
