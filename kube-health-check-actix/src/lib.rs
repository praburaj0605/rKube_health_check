//! Actix-Web scope for Kubernetes health probes.

use std::sync::Arc;

use actix_web::web::{self, Data};
use actix_web::{HttpResponse, Responder, Scope};
use kube_health_check::HealthManager;

/// Build an Actix [`Scope`] with `/health`, `/ready`, `/live`, and `/startup`.
///
/// Mount under your `App` with `.service(health_scope(health))` — routes are at the
/// scope root (use `.service(web::scope("").service(...))` or mount at `/`).
pub fn health_scope(health: Arc<HealthManager>) -> Scope {
    web::scope("")
        .app_data(Data::new(health))
        .route("/health", web::get().to(health_handler))
        .route("/ready", web::get().to(ready_handler))
        .route("/live", web::get().to(live_handler))
        .route("/startup", web::get().to(startup_handler))
}

async fn health_handler(health: Data<Arc<HealthManager>>) -> impl Responder {
    let report = health.health().await;
    HttpResponse::Ok().json(report)
}

async fn ready_handler(health: Data<Arc<HealthManager>>) -> impl Responder {
    let report = health.ready().await;
    let code = report.http_status_ready(health.config().allow_degraded_ready);
    HttpResponse::build(
        actix_web::http::StatusCode::from_u16(code)
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
    )
    .json(report)
}

async fn live_handler(health: Data<Arc<HealthManager>>) -> impl Responder {
    let report = health.live().await;
    let code = report.http_status_live();
    HttpResponse::build(
        actix_web::http::StatusCode::from_u16(code)
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
    )
    .json(report)
}

async fn startup_handler(health: Data<Arc<HealthManager>>) -> impl Responder {
    let report = health.startup().await;
    let ok = report.startup.map(|s| s.is_ready()).unwrap_or(false) && report.is_success();
    if ok {
        HttpResponse::Ok().json(report)
    } else {
        HttpResponse::ServiceUnavailable().json(report)
    }
}
