//! Rocket routes for Kubernetes health probes.

use std::sync::Arc;

use kube_health_check::{HealthManager, HealthReport};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, routes, Route, State};

/// Rocket managed state wrapper.
pub struct HealthState(pub Arc<HealthManager>);

/// All health probe routes.
pub fn health_routes() -> Vec<Route> {
    routes![health, ready, live, startup]
}

#[get("/health")]
async fn health(state: &State<HealthState>) -> (Status, Json<HealthReport>) {
    let report = state.0.health().await;
    (Status::Ok, Json(report))
}

#[get("/ready")]
async fn ready(state: &State<HealthState>) -> (Status, Json<HealthReport>) {
    let report = state.0.ready().await;
    let code = report.http_status_ready(state.0.config().allow_degraded_ready);
    (
        Status::from_code(code).unwrap_or(Status::InternalServerError),
        Json(report),
    )
}

#[get("/live")]
async fn live(state: &State<HealthState>) -> (Status, Json<HealthReport>) {
    let report = state.0.live().await;
    let code = report.http_status_live();
    (
        Status::from_code(code).unwrap_or(Status::InternalServerError),
        Json(report),
    )
}

#[get("/startup")]
async fn startup(state: &State<HealthState>) -> (Status, Json<HealthReport>) {
    let report = state.0.startup().await;
    let ok = report.startup.map(|s| s.is_ready()).unwrap_or(false) && report.is_success();
    let status = if ok {
        Status::Ok
    } else {
        Status::ServiceUnavailable
    };
    (status, Json(report))
}
