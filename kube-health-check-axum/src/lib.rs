//! Axum routes for Kubernetes health probes.
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use kube_health_check::HealthBuilder;
//! use kube_health_check_axum::health_router;
//!
//! # async fn demo() {
//! let health = Arc::new(
//!     HealthBuilder::new("demo")
//!         .custom("ok", || async { Ok(()) })
//!         .build()
//! );
//! health.mark_ready().await;
//! let app = health_router(health);
//! # let _ = app;
//! # }
//! ```

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use kube_health_check::{HealthManager, HealthReport};

/// Build an Axum [`Router`] with `/health`, `/ready`, `/live`, and `/startup`.
///
/// Also mounts `/metrics` when this crate is built with the `metrics` feature and
/// the manager was created with [`kube_health_check::HealthBuilder::with_metrics`].
pub fn health_router(health: Arc<HealthManager>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .route("/live", get(live_handler))
        .route("/startup", get(startup_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(health)
}

async fn health_handler(State(health): State<Arc<HealthManager>>) -> Response {
    let report = health.health().await;
    json_response(report, StatusCode::OK)
}

async fn ready_handler(State(health): State<Arc<HealthManager>>) -> Response {
    let report = health.ready().await;
    let code = status_from_u16(report.http_status_ready(health.config().allow_degraded_ready));
    json_response(report, code)
}

async fn live_handler(State(health): State<Arc<HealthManager>>) -> Response {
    let report = health.live().await;
    let code = status_from_u16(report.http_status_live());
    json_response(report, code)
}

async fn startup_handler(State(health): State<Arc<HealthManager>>) -> Response {
    let report = health.startup().await;
    let ready = report.startup.map(|s| s.is_ready()).unwrap_or(false);
    let failed = report.startup.map(|s| s.is_failed()).unwrap_or(false);
    let code = if !failed && ready && report.is_success() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    json_response(report, code)
}

async fn metrics_handler(State(health): State<Arc<HealthManager>>) -> Response {
    #[cfg(feature = "metrics")]
    {
        if let Some(body) = health.metrics_text() {
            return (
                StatusCode::OK,
                [("content-type", "text/plain; version=0.0.4")],
                body,
            )
                .into_response();
        }
    }
    #[cfg(not(feature = "metrics"))]
    {
        let _ = health;
    }
    StatusCode::NOT_FOUND.into_response()
}

fn json_response(report: HealthReport, status: StatusCode) -> Response {
    (status, Json(report)).into_response()
}

fn status_from_u16(code: u16) -> StatusCode {
    StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use kube_health_check::HealthBuilder;
    use tower::ServiceExt;

    #[tokio::test]
    async fn endpoints_ok() {
        let health = Arc::new(
            HealthBuilder::new("svc")
                .custom("ok", || async { Ok(()) })
                .build(),
        );
        health.mark_ready().await;
        let app = health_router(health);

        for path in ["/health", "/ready", "/live", "/startup"] {
            let req = axum::http::Request::builder()
                .uri(path)
                .body(Body::empty())
                .unwrap();
            let res = app.clone().oneshot(req).await.unwrap();
            assert_eq!(res.status(), StatusCode::OK, "path {path}");
            let bytes = res.into_body().collect().await.unwrap().to_bytes();
            let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert!(v.get("service").is_some());
        }
    }

    #[tokio::test]
    async fn full_detail_never_leaks_fixture_secrets() {
        use kube_health_check::DetailLevel;

        const SECRET: &str = "axum-s3cret-FIXTURE";
        const LEAK: &str = "axum-leak-FIXTURE";

        let health = Arc::new(
            HealthBuilder::new("axum-sec")
                .detail_level(DetailLevel::Full)
                .custom("db", || async {
                    Err(format!(
                        "postgres://u:{SECRET}@db/app?password={LEAK} Authorization: Bearer {SECRET}"
                    ))
                })
                .build(),
        );
        health.mark_ready().await;
        let app = health_router(health);

        for path in ["/health", "/ready"] {
            let req = axum::http::Request::builder()
                .uri(path)
                .body(Body::empty())
                .unwrap();
            let res = app.clone().oneshot(req).await.unwrap();
            let bytes = res.into_body().collect().await.unwrap().to_bytes();
            let body = String::from_utf8_lossy(&bytes);
            assert!(!body.contains(SECRET), "{path} leaked SECRET: {body}");
            assert!(!body.contains(LEAK), "{path} leaked LEAK: {body}");
            let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert!(v.get("checks").is_some(), "{path} expected checks at Full");
        }
    }
}
