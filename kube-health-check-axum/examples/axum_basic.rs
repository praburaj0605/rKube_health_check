use std::sync::Arc;

use kube_health_check::HealthBuilder;
use kube_health_check_axum::health_router;

#[tokio::main]
async fn main() {
    let health = Arc::new(
        HealthBuilder::new("axum-demo")
            .version(env!("CARGO_PKG_VERSION"))
            .build_id("local")
            .custom("license", || async { Ok(()) })
            .custom("filesystem", || async { Ok(()) })
            .build(),
    );

    // Simulate startup progression
    health.advance_startup().await; // LoadingConfiguration
    health.advance_startup().await; // ConnectingDependencies
    health.mark_ready().await;

    let app = health_router(health);
    let addr = "127.0.0.1:8080";
    println!("listening on http://{addr}  (/health /ready /live /startup)");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
