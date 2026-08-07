//! Integration tests for dependency checkers that can run without external services.

#![cfg(feature = "sqlite")]

use std::time::Duration;

use kube_health_check::checks::SqliteCheck;
use kube_health_check::{CheckOptions, HealthBuilder, HealthStatus};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn sqlite_check_passes() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("sqlite memory");

    let health = HealthBuilder::new("sqlite-demo")
        .default_timeout(Duration::from_secs(2))
        .check(SqliteCheck::new(pool))
        .build();
    health.mark_ready().await;

    let report = health.ready().await;
    assert_eq!(report.status, HealthStatus::Healthy);
    assert!(report
        .checks
        .as_ref()
        .unwrap()
        .iter()
        .any(|c| c.name == "sqlite"));
}

#[tokio::test]
async fn tcp_check_localhost_refused() {
    use kube_health_check::checks::TcpCheck;

    let health = HealthBuilder::new("tcp-demo")
        .default_timeout(Duration::from_millis(200))
        .check_with(
            TcpCheck::new("closed", "127.0.0.1:1"),
            CheckOptions::default(),
        )
        .build();
    health.mark_ready().await;
    let report = health.ready().await;
    assert_eq!(report.status, HealthStatus::Unhealthy);
}
