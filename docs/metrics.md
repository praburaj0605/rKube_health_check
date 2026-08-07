# Metrics cardinality guidelines

Prometheus metrics exposed by `kube-health-check` (feature `metrics`):

| Metric | Labels | Notes |
|--------|--------|-------|
| `health_check_duration_seconds` | `check` | Histogram; keep check names stable and low-cardinality |
| `dependency_status` | `dependency` | Gauge 1–4 mapping Healthy→Starting |
| `dependency_latency` | `dependency` | Last latency in seconds |
| `health_overall_status` | — | Overall aggregation |
| `memory_usage` | — | Set via `HealthMetrics::set_memory_usage` |
| `cpu_usage` | — | Set via `HealthMetrics::set_cpu_usage` |

Do **not** put pod IDs, request IDs, or connection strings in label values.
