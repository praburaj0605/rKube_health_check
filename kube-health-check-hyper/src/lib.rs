//! Hyper service helpers for Kubernetes health probes.

use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use kube_health_check::{HealthManager, HealthReport};

type BoxBody = Full<Bytes>;
type BoxFut = Pin<Box<dyn Future<Output = Result<Response<BoxBody>, Infallible>> + Send>>;

/// Create a Hyper service function that handles health probe paths.
pub fn health_service(health: Arc<HealthManager>) -> impl Fn(Request<Incoming>) -> BoxFut + Clone {
    move |req: Request<Incoming>| {
        let health = health.clone();
        Box::pin(async move {
            let path = req.uri().path();
            let (report, status) = match (req.method().as_str(), path) {
                ("GET", "/health") => {
                    let report = health.health().await;
                    (report, StatusCode::OK)
                }
                ("GET", "/ready") => {
                    let report = health.ready().await;
                    let code = report.http_status_ready(health.config().allow_degraded_ready);
                    (
                        report,
                        StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                    )
                }
                ("GET", "/live") => {
                    let report = health.live().await;
                    let code = report.http_status_live();
                    (
                        report,
                        StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                    )
                }
                ("GET", "/startup") => {
                    let report = health.startup().await;
                    let ok = report.startup.map(|s| s.is_ready()).unwrap_or(false)
                        && report.is_success();
                    let status = if ok {
                        StatusCode::OK
                    } else {
                        StatusCode::SERVICE_UNAVAILABLE
                    };
                    (report, status)
                }
                _ => {
                    return Ok(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Full::new(Bytes::from_static(b"not found")))
                        .unwrap());
                }
            };
            Ok(json_response(report, status))
        })
    }
}

fn json_response(report: HealthReport, status: StatusCode) -> Response<BoxBody> {
    let body = serde_json::to_vec(&report).unwrap_or_default();
    Response::builder()
        .status(status)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}
