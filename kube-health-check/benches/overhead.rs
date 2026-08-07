use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};
use kube_health_check::HealthBuilder;
use tokio::runtime::Runtime;

fn ready_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let health = HealthBuilder::new("bench")
        .custom("ok", || async { Ok(()) })
        .build();
    rt.block_on(health.mark_ready());

    c.bench_function("ready_one_custom_check", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = health.ready().await;
        });
    });

    c.bench_function("live_probe", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = health.live().await;
        });
    });

    // Warm sanity: single call should be well under 1ms on a quiet machine for live.
    let start = std::time::Instant::now();
    rt.block_on(health.live());
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(5),
        "live probe took {elapsed:?}, expected << 5ms"
    );
}

criterion_group!(benches, ready_overhead);
criterion_main!(benches);
