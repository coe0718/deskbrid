use std::time::Instant;

use crate::DaemonState;

const DEFAULT_RATE_LIMIT_PER_SECOND: f64 = 30.0;
const DEFAULT_RATE_LIMIT_BURST: f64 = 120.0;

#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    pub per_second: f64,
    pub burst: f64,
}

#[derive(Debug, Clone)]
pub struct RateBucket {
    tokens: f64,
    last_refill: Instant,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RateLimitHit {
    pub retry_after_ms: u64,
}

pub(crate) fn rate_limit_from_env() -> Option<RateLimitConfig> {
    let per_second = std::env::var("DESKBRID_RATE_LIMIT_PER_SEC")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(DEFAULT_RATE_LIMIT_PER_SECOND);
    if per_second <= 0.0 {
        return None;
    }

    let burst = std::env::var("DESKBRID_RATE_LIMIT_BURST")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(DEFAULT_RATE_LIMIT_BURST)
        .max(1.0);

    Some(RateLimitConfig { per_second, burst })
}

impl RateBucket {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            tokens: config.burst,
            last_refill: Instant::now(),
        }
    }

    fn take(&mut self, config: RateLimitConfig) -> Option<RateLimitHit> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.last_refill = now;
        self.tokens = (self.tokens + elapsed * config.per_second).min(config.burst);

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            return None;
        }

        let missing = 1.0 - self.tokens;
        let retry_after_ms = ((missing / config.per_second) * 1000.0).ceil() as u64;
        Some(RateLimitHit { retry_after_ms })
    }
}

pub(crate) async fn check_rate_limit(state: &DaemonState, peer_uid: u32) -> Option<RateLimitHit> {
    let config = state.rate_limit?;
    let mut buckets = state.rate_limits.lock().await;
    let bucket = buckets
        .entry(peer_uid)
        .or_insert_with(|| RateBucket::new(config));
    bucket.take(config)
}

pub(crate) fn rate_limited_response(seq: u64, hit: RateLimitHit) -> serde_json::Value {
    serde_json::json!({
        "type": "response",
        "id": "action",
        "seq": seq,
        "status": "error",
        "error": {
            "code": "RATE_LIMITED",
            "message": format!("rate limit exceeded; retry after {} ms", hit.retry_after_ms),
            "retry_after_ms": hit.retry_after_ms
        }
    })
}

// ── Per-namespace rate limiting (#129) ─────────────────────────────────────

use crate::protocol::Action;
use std::collections::HashMap;
use std::sync::Mutex;

/// Known action namespaces that get per-action rate limits.
const KNOWN_NAMESPACES: &[&str] = &[
    "secrets", "files", "process", "terminal", "browser", "system",
];

/// Parse a rate limit string like "5/m", "60/m", "120/m" into a RateLimitConfig.
/// Returns None for "0" (disabled) or unparseable strings.
pub(crate) fn parse_limit_string(s: &str) -> Option<RateLimitConfig> {
    if s.is_empty() {
        return None;
    }
    if s == "0" {
        return Some(RateLimitConfig {
            per_second: 0.0,
            burst: 0.0,
        });
    }
    let slash_pos = s.find('/')?;
    let num_str = &s[..slash_pos];
    let unit = &s[slash_pos + 1..];
    let quantity: f64 = num_str.parse().ok()?;
    if quantity <= 0.0 || unit.is_empty() {
        return None;
    }
    let per_second = match unit {
        "s" => quantity,
        "m" => quantity / 60.0,
        "h" => quantity / 3600.0,
        _ => return None,
    };
    let burst = (per_second * 4.0).max(1.0);
    Some(RateLimitConfig { per_second, burst })
}

/// Derive the action namespace prefix from an Action.
/// Returns "secrets.", "files.", etc. for known namespaces,
/// or "" (wildcard) for unknown actions.
pub(crate) fn action_namespace(action: &Action) -> &'static str {
    let at = action.action_type();
    let prefix = at.split('.').next().unwrap_or("");
    if KNOWN_NAMESPACES.contains(&prefix) {
        match prefix {
            "secrets" => "secrets.",
            "files" => "files.",
            "process" => "process.",
            "terminal" => "terminal.",
            "browser" => "browser.",
            "system" => "system.",
            _ => "",
        }
    } else {
        ""
    }
}

/// Default rate limits per namespace. Secure defaults — apply when no
/// permissions.toml override is present.
fn default_namespace_limits() -> HashMap<String, RateLimitConfig> {
    let mut m = HashMap::new();
    m.insert(
        "secrets.".into(),
        RateLimitConfig {
            per_second: 5.0 / 60.0, // 5/min
            burst: 1.0,
        },
    );
    m.insert(
        "terminal.".into(),
        RateLimitConfig {
            per_second: 10.0 / 60.0, // 10/min
            burst: 1.0,
        },
    );
    m.insert(
        "process.".into(),
        RateLimitConfig {
            per_second: 30.0 / 60.0, // 30/min
            burst: 2.0,
        },
    );
    m.insert(
        "browser.".into(),
        RateLimitConfig {
            per_second: 20.0 / 60.0, // 20/min
            burst: 1.0,
        },
    );
    m.insert(
        "files.".into(),
        RateLimitConfig {
            per_second: 1.0, // 60/min
            burst: 4.0,
        },
    );
    m.insert(
        "system.".into(),
        RateLimitConfig {
            per_second: 1.0, // 60/min
            burst: 4.0,
        },
    );
    m.insert(
        "".into(),
        RateLimitConfig {
            per_second: 2.0, // 120/min = 2/sec
            burst: 8.0,
        },
    );
    m
}

/// Per-namespace, per-UID rate limiting store.
///
/// `configs` is populated at startup and is read-only thereafter.
/// `buckets` is the only mutable state.
pub struct RateLimitStore {
    pub configs: HashMap<String, RateLimitConfig>,
    buckets: Mutex<HashMap<u32, HashMap<String, RateBucket>>>,
}

impl RateLimitStore {
    pub fn new() -> Self {
        Self {
            configs: default_namespace_limits(),
            buckets: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_config(&mut self, namespace: String, config: RateLimitConfig) {
        self.configs.insert(namespace, config);
    }

    pub fn load_overrides(&mut self, overrides: &HashMap<String, String>) {
        for (ns, limit_str) in overrides {
            if let Some(config) = parse_limit_string(limit_str) {
                self.configs.insert(ns.clone(), config);
            }
        }
    }

    /// Check if an action is rate-limited for the given peer UID.
    /// Returns None if allowed, Some(RateLimitHit) if throttled.
    pub(crate) fn check(&self, peer_uid: u32, action: &Action) -> Option<RateLimitHit> {
        let namespace = action_namespace(action);
        let config = self
            .configs
            .get(namespace)
            .or_else(|| self.configs.get(""))
            .copied()?;

        if config.per_second <= 0.0 {
            return None;
        }

        let mut buckets = self.buckets.lock().unwrap();
        let ns_buckets = buckets.entry(peer_uid).or_default();
        let bucket = ns_buckets
            .entry(namespace.to_string())
            .or_insert_with(|| RateBucket::new(config));
        bucket.take(config)
    }

    /// Remove all buckets for a disconnected peer. Call from the client
    /// disconnect handler to prevent unbounded growth of the bucket map.
    pub fn remove_peer(&self, peer_uid: u32) {
        let mut buckets = self.buckets.lock().unwrap();
        buckets.remove(&peer_uid);
    }

    /// Sweep peers that haven't been seen in `max_age` seconds.
    /// Call periodically (e.g., every 10 minutes) to keep the bucket map bounded.
    pub fn sweep_stale(&self, max_age: std::time::Duration) {
        let now = std::time::Instant::now();
        let mut buckets = self.buckets.lock().unwrap();
        buckets.retain(|_, ns_map| {
            ns_map
                .values()
                .any(|b| now.duration_since(b.last_refill) < max_age)
        });
    }
}

impl Default for RateLimitStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Action;

    #[test]
    fn bucket_allows_burst_then_reports_retry() {
        let config = RateLimitConfig {
            per_second: 1.0,
            burst: 1.0,
        };
        let mut bucket = RateBucket::new(config);

        assert!(bucket.take(config).is_none());
        let hit = bucket.take(config).expect("limited");
        assert!(hit.retry_after_ms > 0);
    }

    #[test]
    fn parse_limit_string_valid() {
        let c = parse_limit_string("5/m").unwrap();
        assert!((c.per_second - 5.0 / 60.0).abs() < 0.001);
        assert_eq!(c.burst, 1.0);

        let c = parse_limit_string("60/m").unwrap();
        assert!((c.per_second - 1.0).abs() < 0.001);
        assert_eq!(c.burst, 4.0);

        let c = parse_limit_string("120/m").unwrap();
        assert!((c.per_second - 2.0).abs() < 0.001);
        assert_eq!(c.burst, 8.0);
    }

    #[test]
    fn parse_limit_string_zero_disables() {
        let c = parse_limit_string("0").unwrap();
        assert_eq!(c.per_second, 0.0);
        assert_eq!(c.burst, 0.0);
    }

    #[test]
    fn parse_limit_string_invalid() {
        assert!(parse_limit_string("").is_none());
        assert!(parse_limit_string("not-a-number/m").is_none());
        assert!(parse_limit_string("5/x").is_none());
    }

    #[test]
    fn action_namespace_known() {
        assert_eq!(
            action_namespace(&Action::SecretsGetSecret {
                attributes: HashMap::new()
            }),
            "secrets."
        );
        assert_eq!(action_namespace(&Action::SystemInfo), "system.");
        assert_eq!(
            action_namespace(&Action::FilesRead {
                path: "x".into(),
                limit: None,
                offset: None
            }),
            "files."
        );
    }

    #[test]
    fn action_namespace_unknown_goes_to_wildcard() {
        assert_eq!(action_namespace(&Action::Ping), "");
        assert_eq!(action_namespace(&Action::WindowsList), "");
    }

    #[test]
    fn uid_isolation_secrets_buckets_independent() {
        let store = RateLimitStore::new();
        let get_secret = Action::SecretsGetSecret {
            attributes: {
                let mut m = HashMap::new();
                m.insert("service".into(), "test".into());
                m
            },
        };

        // UID 1000 exhausts secrets quota (5/min burst=1 → 1 token)
        assert!(store.check(1000, &get_secret).is_none());
        assert!(store.check(1000, &get_secret).is_some());

        // UID 1001 still has full quota
        assert!(store.check(1001, &get_secret).is_none());
    }

    #[test]
    fn wildcard_namespace_catches_unknown_actions() {
        let store = RateLimitStore::new();
        let ping = Action::Ping;

        // Wildcard: 2/sec, burst=8 → 8 tokens available
        for _ in 0..8 {
            assert!(store.check(1000, &ping).is_none());
        }
        assert!(store.check(1000, &ping).is_some());
    }

    #[test]
    fn disabled_namespace_passes_all() {
        let mut store = RateLimitStore::new();
        store.set_config(
            "secrets.".into(),
            RateLimitConfig {
                per_second: 0.0,
                burst: 0.0,
            },
        );
        let action = Action::SecretsGetSecret {
            attributes: HashMap::new(),
        };
        for _ in 0..100 {
            assert!(store.check(2000, &action).is_none());
        }
    }
}
