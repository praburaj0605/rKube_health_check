//! Production-grade security regression tests.
//!
//! Covers secret redaction, DetailLevel disclosure controls, metrics hygiene,
//! probe timeout DoS bounds, and JSON escaping of adversarial check names.

use std::time::{Duration, Instant};

use kube_health_check::{sanitize_error, CheckOptions, DetailLevel, HealthBuilder, HealthStatus};

const FIXTURE_SECRET: &str = "s3cret-FIXTURE-never-leak";
const FIXTURE_TOKEN: &str = "tok_LIVE_fixture_abc";
const FIXTURE_LEAK: &str = "leakme-fixture";

#[test]
fn redaction_table_never_leaks_fixtures() {
    let cases = [
        format!("postgres://u:{FIXTURE_SECRET}@db/app?password={FIXTURE_LEAK}"),
        format!("Authorization: Bearer {FIXTURE_TOKEN}"),
        format!("Authorization: Basic {FIXTURE_TOKEN}"),
        format!("Password={FIXTURE_SECRET};Server=sql"),
        format!(r#"{{"password":"{FIXTURE_LEAK}","api_key":"{FIXTURE_TOKEN}"}}"#),
        format!("pwd={FIXTURE_SECRET} token={FIXTURE_TOKEN} secret={FIXTURE_LEAK}"),
        format!("PASSWORD={FIXTURE_SECRET}&api_key={FIXTURE_TOKEN}"),
    ];

    for case in cases {
        let out = sanitize_error(case.clone());
        assert!(
            !out.contains(FIXTURE_SECRET),
            "leaked FIXTURE_SECRET in {case:?} -> {out:?}"
        );
        assert!(
            !out.contains(FIXTURE_TOKEN),
            "leaked FIXTURE_TOKEN in {case:?} -> {out:?}"
        );
        assert!(
            !out.contains(FIXTURE_LEAK),
            "leaked FIXTURE_LEAK in {case:?} -> {out:?}"
        );
    }
}

#[tokio::test]
async fn detail_level_ladder_disclosure() {
    let secret_err = format!("postgres://u:{FIXTURE_SECRET}@db/app?password={FIXTURE_LEAK}");

    // Minimal: no checks array
    let minimal = HealthBuilder::new("sec")
        .detail_level(DetailLevel::Minimal)
        .custom("db", {
            let e = secret_err.clone();
            move || {
                let e = e.clone();
                async move { Err(e) }
            }
        })
        .build();
    minimal.mark_ready().await;
    let report = minimal.ready().await;
    assert!(report.checks.is_none());
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains(FIXTURE_SECRET));
    assert!(!json.contains(FIXTURE_LEAK));

    // Standard: checks present, errors null
    let standard = HealthBuilder::new("sec")
        .detail_level(DetailLevel::Standard)
        .custom("db", {
            let e = secret_err.clone();
            move || {
                let e = e.clone();
                async move { Err(e) }
            }
        })
        .build();
    standard.mark_ready().await;
    let report = standard.ready().await;
    let checks = report.checks.as_ref().expect("checks");
    assert!(!checks.is_empty());
    assert!(checks.iter().all(|c| c.error.is_none()));
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains(FIXTURE_SECRET));
    assert!(!json.contains(FIXTURE_LEAK));

    // Full: errors present but redacted
    let full = HealthBuilder::new("sec")
        .detail_level(DetailLevel::Full)
        .custom("db", {
            let e = secret_err.clone();
            move || {
                let e = e.clone();
                async move { Err(e) }
            }
        })
        .build();
    full.mark_ready().await;
    let report = full.ready().await;
    assert_eq!(report.status, HealthStatus::Unhealthy);
    let checks = report.checks.as_ref().expect("checks");
    assert!(checks.iter().any(|c| c.error.is_some()));
    let json = serde_json::to_string(&report).unwrap();
    assert!(
        !json.contains(FIXTURE_SECRET),
        "full JSON leaked secret: {json}"
    );
    assert!(
        !json.contains(FIXTURE_LEAK),
        "full JSON leaked leak: {json}"
    );
    assert!(json.contains("***"));
}

#[tokio::test]
async fn report_path_e2e_redacts_connection_string() {
    let health = HealthBuilder::new("e2e")
        .detail_level(DetailLevel::Full)
        .custom("postgres", || async {
            Err(format!(
                "connect failed postgres://u:{FIXTURE_SECRET}@db/app?password={FIXTURE_LEAK}"
            ))
        })
        .build();
    health.mark_ready().await;
    let report = health.health().await;
    let json = serde_json::to_string_pretty(&report).unwrap();
    assert!(!json.contains(FIXTURE_SECRET));
    assert!(!json.contains(FIXTURE_LEAK));
}

#[cfg(feature = "metrics")]
#[tokio::test]
async fn metrics_hygiene_no_secrets() {
    let health = HealthBuilder::new("metrics-sec")
        .with_metrics()
        .detail_level(DetailLevel::Full)
        .custom("db", || async {
            Err(format!(
                "postgres://u:{FIXTURE_SECRET}@db/app password={FIXTURE_LEAK}"
            ))
        })
        .build();
    health.mark_ready().await;
    let _ = health.ready().await;
    let text = health.metrics_text().expect("metrics enabled");
    assert!(text.contains("dependency_status") || text.contains("health_check_duration"));
    assert!(!text.contains(FIXTURE_SECRET));
    assert!(!text.contains(FIXTURE_LEAK));
    assert!(!text.contains("postgres://"));
}

#[tokio::test]
async fn dos_bound_concurrent_slow_checks_honor_timeout() {
    let timeout = Duration::from_millis(80);
    let mut builder = HealthBuilder::new("dos")
        .default_timeout(timeout)
        .default_retries(0, Duration::from_millis(1));

    // Five slow checks that would take ~1s each if run without timeout.
    for i in 0..5 {
        let name = format!("slow-{i}");
        builder = builder.custom(name, || async {
            tokio::time::sleep(Duration::from_secs(2)).await;
            Ok(())
        });
    }

    let health = builder.build();
    health.mark_ready().await;

    let start = Instant::now();
    let report = health.ready().await;
    let elapsed = start.elapsed();

    assert_eq!(report.status, HealthStatus::Unhealthy);
    // Concurrent: wall clock should be near timeout, not 5 * 2s.
    assert!(
        elapsed < timeout + Duration::from_millis(500),
        "wall clock {elapsed:?} exceeded timeout bound (timeout={timeout:?})"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "wall clock {elapsed:?} suggests checks were not timed out concurrently"
    );
}

#[tokio::test]
async fn dos_bound_retries_respect_budget() {
    let timeout = Duration::from_millis(50);
    let retries = 2u32;
    let interval = Duration::from_millis(20);
    let health = HealthBuilder::new("retry-dos")
        .default_timeout(timeout)
        .default_retries(retries, interval)
        .custom("slow", || async {
            tokio::time::sleep(Duration::from_secs(2)).await;
            Ok(())
        })
        .build();
    health.mark_ready().await;

    let start = Instant::now();
    let _ = health.ready().await;
    let elapsed = start.elapsed();

    // Upper bound: (retries+1)*timeout + retries*interval + slack
    let budget = timeout * (retries + 1) + interval * retries + Duration::from_millis(400);
    assert!(
        elapsed < budget,
        "elapsed {elapsed:?} exceeded retry budget {budget:?}"
    );
}

#[tokio::test]
async fn adversarial_check_name_produces_valid_json() {
    let nasty = "db\"}\n{\"injected\":true";
    let health = HealthBuilder::new("inject")
        .detail_level(DetailLevel::Full)
        .custom_with(nasty, CheckOptions::default(), || async {
            Err(format!("password={FIXTURE_SECRET}"))
        })
        .build();
    health.mark_ready().await;
    let report = health.ready().await;
    let json = serde_json::to_string(&report).unwrap();
    // Must parse as JSON (serde escapes the name).
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json");
    assert!(parsed.get("checks").is_some());
    assert!(!json.contains(FIXTURE_SECRET));
    // Injected object must not appear as a sibling key at top level.
    assert!(parsed.get("injected").is_none());
}
