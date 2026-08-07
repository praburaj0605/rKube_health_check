# Security testing guide

This crate exposes unauthenticated Kubernetes probe JSON. Security tests guard
against secret leakage, excessive disclosure, and probe DoS amplification.

## Local commands

```bash
# Security regression suite (redaction, DetailLevel, DoS bounds)
cargo test -p kube-health-check --features metrics --test security

# Axum disclosure contract
cargo test -p kube-health-check-axum

# Supply chain
cargo install cargo-audit cargo-deny
cargo audit
cargo deny check

# Clippy (treat warnings as errors)
cargo clippy -p kube-health-check --features metrics -- -D warnings

# Fuzz sanitizer (requires nightly + cargo-fuzz)
cargo install cargo-fuzz
cd kube-health-check
cargo +nightly fuzz run fuzz_sanitize_error -- -max_total_time=30
```

## Threat model (library scope)

| Risk | Mitigation |
|------|------------|
| Secrets in check errors | `sanitize_error` + DetailLevel gating |
| Topology / metadata disclosure | Prefer `minimal`/`standard` in shared clusters |
| Slow-check DoS on `/ready` | Per-check timeouts; concurrent execution bounded by timeout |
| Vulnerable dependencies | `cargo audit` / `cargo deny` in CI |

## Supply-chain notes

- `prometheus` was bumped to `0.14` (fixes protobuf recursion advisory).
- `async-nats` was bumped to `0.50` (clears rustls-webpki advisories).
- `RUSTSEC-2023-0071` (rsa / Marvin) is ignored in [deny.toml](../deny.toml) and [.cargo/audit.toml](../.cargo/audit.toml): transitive via optional `sqlx-mysql`, no fixed upstream release; this crate does not implement RSA itself.

