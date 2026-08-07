//! Warp filters for Kubernetes health probes.

use std::sync::Arc;

use kube_health_check::{HealthManager, HealthReport};
use warp::http::StatusCode;
use warp::{Filter, Rejection, Reply};

/// Combined Warp filter for `/health`, `/ready`, `/live`, and `/startup`.
pub fn health_filters(
    health: Arc<HealthManager>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let health_route = {
        let health = health.clone();
        warp::path!("health").and(warp::get()).and_then(move || {
            let health = health.clone();
            async move {
                let report = health.health().await;
                ok_json(report, StatusCode::OK)
            }
        })
    };

    let ready_route = {
        let health = health.clone();
        warp::path!("ready").and(warp::get()).and_then(move || {
            let health = health.clone();
            async move {
                let report = health.ready().await;
                let code = StatusCode::from_u16(
                    report.http_status_ready(health.config().allow_degraded_ready),
                )
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                ok_json(report, code)
            }
        })
    };

    let live_route = {
        let health = health.clone();
        warp::path!("live").and(warp::get()).and_then(move || {
            let health = health.clone();
            async move {
                let report = health.live().await;
                let code = StatusCode::from_u16(report.http_status_live())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                ok_json(report, code)
            }
        })
    };

    let startup_route = {
        let health = health;
        warp::path!("startup").and(warp::get()).and_then(move || {
            let health = health.clone();
            async move {
                let report = health.startup().await;
                let ok =
                    report.startup.map(|s| s.is_ready()).unwrap_or(false) && report.is_success();
                let code = if ok {
                    StatusCode::OK
                } else {
                    StatusCode::SERVICE_UNAVAILABLE
                };
                ok_json(report, code)
            }
        })
    };

    health_route
        .or(ready_route)
        .or(live_route)
        .or(startup_route)
}

fn ok_json(report: HealthReport, status: StatusCode) -> Result<impl Reply, Rejection> {
    Ok(warp::reply::with_status(warp::reply::json(&report), status))
}
