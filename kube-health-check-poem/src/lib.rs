//! Poem routes for Kubernetes health probes.

use std::sync::Arc;

use kube_health_check::{HealthManager, HealthReport};
use poem::http::StatusCode;
use poem::web::Data;
use poem::{get, handler, Endpoint, EndpointExt, Response, Route};

/// Build a Poem endpoint with health probe routes.
pub fn health_route(health: Arc<HealthManager>) -> impl Endpoint {
    Route::new()
        .at("/health", get(health_handler))
        .at("/ready", get(ready_handler))
        .at("/live", get(live_handler))
        .at("/startup", get(startup_handler))
        .data(health)
}

#[handler]
async fn health_handler(Data(health): Data<&Arc<HealthManager>>) -> Response {
    let report = health.health().await;
    json_response(report, StatusCode::OK)
}

#[handler]
async fn ready_handler(Data(health): Data<&Arc<HealthManager>>) -> Response {
    let report = health.ready().await;
    let code = StatusCode::from_u16(report.http_status_ready(health.config().allow_degraded_ready))
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    json_response(report, code)
}

#[handler]
async fn live_handler(Data(health): Data<&Arc<HealthManager>>) -> Response {
    let report = health.live().await;
    let code = StatusCode::from_u16(report.http_status_live())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    json_response(report, code)
}

#[handler]
async fn startup_handler(Data(health): Data<&Arc<HealthManager>>) -> Response {
    let report = health.startup().await;
    let ok = report.startup.map(|s| s.is_ready()).unwrap_or(false) && report.is_success();
    let code = if ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    json_response(report, code)
}

fn json_response(report: HealthReport, status: StatusCode) -> Response {
    let body = serde_json::to_vec(&report).unwrap_or_default();
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(body)
}
