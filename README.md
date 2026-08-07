# kube-health-check

Production-ready Kubernetes health endpoints for Rust — liveness, readiness, and startup probes with pluggable dependency checks, custom async checks, status aggregation, and Prometheus metrics.

## Quick start (Axum)

```rust
use std::sync::Arc;
use kube_health_check::HealthBuilder;
use kube_health_check_axum::health_router;

#[tokio::main]
async fn main() {
    let health = Arc::new(
        HealthBuilder::new("my-service")
            .version(env!("CARGO_PKG_VERSION"))
            .custom("ready", || async { Ok(()) })
            .build(),
    );
    health.mark_ready().await;

    let app = health_router(health);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

Endpoints:

| Path | Purpose |
|------|---------|
| `/live` | Process liveness (no external I/O) |
| `/ready` | Dependency + custom readiness |
| `/startup` | Startup probe state machine |
| `/health` | Aggregated operator overview |
| `/metrics` | Prometheus text (optional feature) |

## Crates

| Crate | Role |
|-------|------|
| `kube-health-check` | Core library |
| `kube-health-check-axum` | Axum adapter |
| `kube-health-check-actix` | Actix-Web adapter |
| `kube-health-check-warp` | Warp adapter |
| `kube-health-check-rocket` | Rocket adapter |
| `kube-health-check-hyper` | Hyper adapter |
| `kube-health-check-poem` | Poem adapter |

## Feature flags (core)

`metrics`, `liveness-ext`, `postgres`, `mysql`, `sqlite`, `mssql`, `redis`, `mongodb`, `kafka`, `rabbitmq`, `nats`, `elasticsearch`, `full`

Built-in checkers:

| Feature | Mechanism |
|---------|-----------|
| `postgres` / `mysql` / `sqlite` | `sqlx` `SELECT 1` |
| `redis` | Redis `PING` |
| `rabbitmq` | AMQP connect via `lapin` |
| `nats` | Connect + flush via `async-nats` |
| `mssql` / `mongodb` / `kafka` / `elasticsearch` | TCP / lightweight HTTP reachability (use `CustomCheck` for deeper driver-level probes) |

## Kubernetes probes

```yaml
livenessProbe:
  httpGet:
    path: /live
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10
readinessProbe:
  httpGet:
    path: /ready
    port: 8080
  periodSeconds: 5
startupProbe:
  httpGet:
    path: /startup
    port: 8080
  failureThreshold: 30
  periodSeconds: 2
```

See [docs/kubernetes.md](docs/kubernetes.md) for detail levels, aggregation rules, and security notes.

## Security

See [docs/security.md](docs/security.md) for the threat model and how to run audit, deny, fuzz, and security regression tests.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
