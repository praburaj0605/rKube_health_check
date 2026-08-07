//! Health check trait and result types (FR-002 / FR-005 / FR-006).

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::status::HealthStatus;

/// Per-check timeout / retry / threshold options (FR-006).
#[derive(Debug, Clone)]
pub struct CheckOptions {
    /// Soft timeout for a single attempt.
    pub timeout: Option<Duration>,
    /// Extra attempts after the first failure.
    pub retry_count: Option<u32>,
    /// Delay between retries.
    pub retry_interval: Option<Duration>,
    /// Grace period before this check can contribute Unhealthy.
    pub grace_period: Option<Duration>,
    /// Consecutive failures required to report Unhealthy.
    pub failure_threshold: Option<u32>,
    /// When `false`, failures contribute Degraded instead of Unhealthy.
    pub critical: bool,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self {
            timeout: None,
            retry_count: None,
            retry_interval: None,
            grace_period: None,
            failure_threshold: None,
            critical: true,
        }
    }
}

impl CheckOptions {
    /// Builder-style: mark check as non-critical.
    pub fn non_critical(mut self) -> Self {
        self.critical = false;
        self
    }

    /// Builder-style: set timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Builder-style: set retries.
    pub fn with_retries(mut self, count: u32, interval: Duration) -> Self {
        self.retry_count = Some(count);
        self.retry_interval = Some(interval);
        self
    }

    /// Builder-style: set failure threshold.
    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = Some(threshold.max(1));
        self
    }
}

/// Outcome of a single health check execution.
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Check name.
    pub name: String,
    /// Status for this check.
    pub status: HealthStatus,
    /// Wall-clock duration of the check.
    pub latency: Duration,
    /// Optional sanitized error message (never include secrets).
    pub error: Option<String>,
    /// When the check completed.
    pub timestamp: DateTime<Utc>,
    /// Whether this check is critical for readiness.
    pub critical: bool,
}

impl CheckResult {
    /// Successful check helper.
    pub fn healthy(name: impl Into<String>, latency: Duration) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Healthy,
            latency,
            error: None,
            timestamp: Utc::now(),
            critical: true,
        }
    }

    /// Failed check helper.
    pub fn unhealthy(name: impl Into<String>, latency: Duration, error: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Unhealthy,
            latency,
            error: Some(sanitize_error(error.into())),
            timestamp: Utc::now(),
            critical: true,
        }
    }

    /// Returns `true` if the check passed.
    pub fn is_ok(&self) -> bool {
        matches!(self.status, HealthStatus::Healthy)
    }
}

/// Redact common secret patterns from error strings.
///
/// Covers URL userinfo, query/header key=value secrets, JDBC/ADO `Password=`,
/// `Authorization: Bearer|Basic`, and JSON `"password":"..."` shapes. Hostnames
/// and ports are preserved for operations.
pub fn sanitize_error(msg: String) -> String {
    let mut out = redact_url_userinfo(msg);
    out = redact_authorization(out);
    out = redact_json_secrets(out);
    out = redact_keyed_secrets(out);
    out
}

fn redact_url_userinfo(input: String) -> String {
    // scheme://user:password@host → scheme://***:***@host
    let bytes = input.as_bytes();
    let lower = input.to_ascii_lowercase();
    let mut result = String::with_capacity(input.len());
    let mut i = 0;
    while i < bytes.len() {
        if let Some(rel) = lower[i..].find("://") {
            let scheme_end = i + rel + 3;
            result.push_str(&input[i..scheme_end]);
            if let Some(at_rel) = input[scheme_end..].find('@') {
                let userinfo = &input[scheme_end..scheme_end + at_rel];
                // Only treat as userinfo if it looks like credentials (contains ':')
                // and does not contain '/' (path) before '@'.
                if userinfo.contains(':') && !userinfo.contains('/') && !userinfo.contains(' ') {
                    result.push_str("***:***");
                    result.push('@');
                    i = scheme_end + at_rel + 1;
                    continue;
                }
            }
            i = scheme_end;
        } else {
            result.push_str(&input[i..]);
            break;
        }
    }
    if result.is_empty() {
        input
    } else {
        result
    }
}

fn redact_authorization(input: String) -> String {
    redact_authorization_from(input, 0)
}

fn redact_authorization_from(mut out: String, mut search_from: usize) -> String {
    loop {
        let lower = out.to_ascii_lowercase();
        if search_from >= lower.len() {
            break;
        }
        let Some(rel) = lower[search_from..].find("authorization:") else {
            break;
        };
        let abs = search_from + rel;
        let after_colon = abs + "authorization:".len();
        let rest = &out[after_colon..];
        let trimmed_start = rest.len() - rest.trim_start().len();
        let value_start = after_colon + trimmed_start;
        if value_start >= out.len() {
            break;
        }
        let value_lower = out[value_start..].to_ascii_lowercase();
        let scheme_len = if value_lower.starts_with("bearer ") {
            7usize
        } else if value_lower.starts_with("basic ") {
            6usize
        } else {
            search_from = after_colon;
            continue;
        };
        let token_start = value_start + scheme_len;
        let token_rest = &out[token_start..];
        let end_rel = token_rest
            .find([' ', '\r', '\n', ',', ';', '"', '\''])
            .unwrap_or(token_rest.len());
        out.replace_range(token_start..token_start + end_rel, "***");
        search_from = token_start + 3;
    }
    out
}

fn redact_json_secrets(input: String) -> String {
    let keys = [
        "password", "passwd", "pwd", "secret", "token", "api_key", "apikey",
    ];
    let mut out = input;
    for key in keys {
        // "key":"value" or "key": "value"
        let patterns = [
            format!("\"{key}\":\""),
            format!("\"{key}\": \""),
            format!("\"{key}\" : \""),
        ];
        for pat in patterns {
            out = redact_all_after_pattern(&out, &pat, |rest| rest.find('"').unwrap_or(rest.len()));
        }
    }
    out
}

fn redact_keyed_secrets(input: String) -> String {
    let keys = [
        "password=",
        "pwd=",
        "passwd=",
        "secret=",
        "token=",
        "api_key=",
        "apikey=",
    ];
    let mut out = input;
    for key in keys {
        out = redact_all_after_pattern_ci(&out, key, |rest| {
            rest.find(['&', ' ', ';', '"', '\'', '\r', '\n', ','])
                .unwrap_or(rest.len())
        });
    }
    out
}

fn redact_all_after_pattern(input: &str, pattern: &str, end_at: impl Fn(&str) -> usize) -> String {
    let lower_pat = pattern.to_ascii_lowercase();
    let mut out = input.to_string();
    let mut search_from = 0;
    loop {
        let lower = out.to_ascii_lowercase();
        if search_from >= lower.len() {
            break;
        }
        let Some(rel) = lower[search_from..].find(&lower_pat) else {
            break;
        };
        let start = search_from + rel + pattern.len();
        if start > out.len() {
            break;
        }
        let end_rel = end_at(&out[start..]);
        out.replace_range(start..start + end_rel, "***");
        search_from = start + 3;
    }
    out
}

fn redact_all_after_pattern_ci(
    input: &str,
    pattern: &str,
    end_at: impl Fn(&str) -> usize,
) -> String {
    redact_all_after_pattern(input, pattern, end_at)
}

/// Async health check contract.
#[async_trait]
pub trait HealthCheck: Send + Sync {
    /// Stable check name used in JSON reports and metrics.
    fn name(&self) -> &str;

    /// Per-check options; defaults inherit from [`crate::HealthConfig`].
    fn options(&self) -> CheckOptions {
        CheckOptions::default()
    }

    /// Execute the check. Implementations must not panic and must not return secrets.
    async fn check(&self) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_password_query() {
        let s = sanitize_error("connect failed password=supersecret&ssl=true".into());
        assert!(s.contains("password=***"));
        assert!(!s.contains("supersecret"));
    }

    #[test]
    fn sanitizes_url_userinfo() {
        let s = sanitize_error("db error postgres://alice:s3cret@db.internal:5432/app".into());
        assert!(!s.contains("s3cret"));
        assert!(s.contains("postgres://***:***@db.internal:5432/app"));
    }

    #[test]
    fn sanitizes_bearer_and_password_ado() {
        let s =
            sanitize_error("Authorization: Bearer tok_LIVE_abc Password=Hunter2;Server=sql".into());
        assert!(!s.contains("tok_LIVE_abc"));
        assert!(!s.contains("Hunter2"));
        assert!(s.contains("Bearer ***"));
        assert!(s.contains("Password=***"));
    }

    #[test]
    fn sanitizes_json_password() {
        let s = sanitize_error(r#"upstream {"password":"leakme","host":"db"}"#.into());
        assert!(!s.contains("leakme"));
        assert!(s.contains(r#""password":"***""#));
    }

    #[test]
    fn sanitizes_multiple_keyed_secrets() {
        let s = sanitize_error("a password=one&token=two secret=three".into());
        assert!(!s.contains("one"));
        assert!(!s.contains("two"));
        assert!(!s.contains("three"));
    }
}
