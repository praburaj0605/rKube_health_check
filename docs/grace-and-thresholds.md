# Failure threshold and grace period matrix

| Scenario | Grace active? | Failures < threshold? | Reported check status | Aggregated (critical) |
|----------|---------------|------------------------|-----------------------|------------------------|
| First fail after start | yes | n/a | Healthy (softened) | Healthy / Starting |
| Failures below threshold | no | yes | Healthy (softened) | Healthy |
| Failures ≥ threshold | no | no | Unhealthy | Unhealthy |
| Non-critical fail ≥ threshold | no | no | Unhealthy (check) | Degraded |

Grace period and failure threshold delay pod kill/restart storms during boot and brief blips.
