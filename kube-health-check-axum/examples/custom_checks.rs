use std::sync::Arc;
use std::time::Duration;

use kube_health_check::{CheckOptions, DetailLevel, HealthBuilder};
use kube_health_check_axum::health_router;

#[tokio::main]
async fn main() {
    let health = Arc::new(
        HealthBuilder::new("custom-demo")
            .detail_level(DetailLevel::Full)
            .default_timeout(Duration::from_secs(1))
            .custom("feature_flags", || async { Ok(()) })
            .custom_with(
                "third_party_api",
                CheckOptions::default()
                    .non_critical()
                    .with_timeout(Duration::from_millis(200)),
                || async { Err("upstream timeout".into()) },
            )
            .build(),
    );
    health.mark_ready().await;

    let report = health.ready().await;
    println!("{}", serde_json::to_string_pretty(&report).unwrap());

    let app = health_router(health);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8081")
        .await
        .unwrap();
    println!("listening on http://127.0.0.1:8081");
    axum::serve(listener, app).await.unwrap();
}
