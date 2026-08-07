# Kubernetes probe configuration

## Recommended probe mapping

| Probe | Path | Notes |
|-------|------|-------|
| livenessProbe | `/live` | Process-local only. Do **not** depend on databases. |
| readinessProbe | `/ready` | Runs registered dependency and custom checks. |
| startupProbe | `/startup` | Returns 503 until `HealthManager::mark_ready()`. |

## Example Deployment snippet

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: demo
spec:
  template:
    spec:
      containers:
        - name: app
          ports:
            - containerPort: 8080
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

## Aggregation rules

- Startup incomplete → `Starting` (HTTP 503 on `/startup` and `/ready`)
- Any **critical** check failure → `Unhealthy`
- Only **non-critical** failures → `Degraded`
- All pass → `Healthy`

Set `allow_degraded_ready(true)` (or `HEALTH_ALLOW_DEGRADED_READY=true`) if Degraded should still pass readiness.

## Detail levels

| Level | Includes |
|-------|----------|
| `minimal` | Service metadata + overall status |
| `standard` (default) | Check names + statuses (no errors) |
| `full` | Errors (sanitized; passwords redacted) |

Configure via `HealthBuilder::detail_level` or `HEALTH_DETAIL_LEVEL`.

## Security

- Never put credentials in check names or custom error strings.
- Connection-string secrets in driver errors are redacted (`password=***`).
- Prefer `standard` or `minimal` in public/shared clusters.
