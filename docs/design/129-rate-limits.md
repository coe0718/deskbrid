# Deskbrid Per-Action Rate Limiting Specification (RATE_LIMITS.md)

## Design Rationale

The current rate limiter in deskbrid applies a uniform limit of 30 requests per second across all actions and all agents. This is insufficient for security-sensitive actions like credential access (`secrets.*`) which should be tightly throttled to prevent credential harvesting, while allowing higher bandwidth for less sensitive actions like system info queries.

A per-action, per-agent rate limiting system addresses these concerns by:

1. **Granular Control**: Different action namespaces (e.g., `secrets`, `files`, `process`) can have distinct limits based on their sensitivity and resource cost.
2. **Agent Isolation**: Limits are tracked per `(action_namespace, agent_uid)` pair, preventing a compromised or malicious agent from exhausting another agent's quota.
3. **Explicit Feedback**: When a limit is exceeded, the system returns a structured error with a `retry-after` field, enabling clients to back off intelligently rather than experiencing silent drops or ambiguous failures.
4. **Middleware Enforcement**: Implemented as a middleware layer in the dispatch pipeline, ensuring every action is checked before any expensive processing or permission checks occur.
5. **Secure Defaults**: Hardcoded default limits provide strong baseline security even when no `permissions.toml` is present or when the rate limiting section is omitted.

## Config Format

Rate limits are configured in `~/.config/deskbrid/permissions.toml` under a `[rate_limits]` table. Each key is an action namespace prefix (ending with `.`), and the value is a limit string in the format `<number>/<unit>` where `<unit>` is one of `s` (second), `m` (minute), or `h` (hour). An empty string (`""`) represents the wildcard namespace that matches any action not explicitly listed.

Example `permissions.toml`:

```toml
[rate_limits]
"secrets." = "5/m"          # 5 requests per minute for secret access
"files."   = "60/m"         # 60 requests per minute for file operations
"process." = "30/m"         # 30 requests per minute for process control
"terminal." = "10/m"        # 10 requests per minute for terminal spawning
"browser." = "20/m"         # 20 requests per minute for browser automation
"system."  = "60/m"         # 60 requests per minute for system queries
""         = "120/m"       # Wildcard: 120 requests per minute (2/sec) for unknown actions
```

If the `[rate_limits]` table is missing or a namespace is not found, the system falls back to hardcoded defaults (see below). To disable rate limiting for a namespace, set its value to `"0"` or disable via environment variable (not recommended for production).

## Rust Data Structures

We extend the existing rate limiting primitives in `src/daemon/rate_limit.rs` to support per-namespace tracking.

### RateLimitConfig (unchanged)
```rust
#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    pub per_second: f64,   // allowed requests per second (float for fractional rates)
    pub burst: f64,        // maximum burst size (tokens)
}
```

### RateBucket (unchanged)
Implements the token bucket algorithm:
```rust
#[derive(Debug, Clone)]
pub struct RateBucket {
    tokens: f64,
    last_refill: Instant,
}
```

### New: RateLimitStore
Holds the configuration and per-(uid, namespace) buckets.
```rust
#[derive(Debug)]
pub struct RateLimitStore {
    /// Mapping from action namespace prefix (e.g., "secrets.") to its RateLimitConfig.
    /// The empty string "" is the wildcard that matches any action.
    configs: HashMap<String, RateLimitConfig>,
    /// Per-(uid, namespace) token buckets.
    /// We use a nested HashMap: uid -> namespace -> RateBucket.
    /// This avoids allocating a bucket for every possible combination upfront.
    buckets: Mutex<HashMap<u32, HashMap<String, RateBucket>>>,
}
```

### Helper: parse_limit_string
Converts strings like `"10/m"` into a `RateLimitConfig`.
```rust
fn parse_limit_string(s: &str) -> Option<RateLimitConfig> {
    if s == "0" {
        return Some(RateLimitConfig { per_second: 0.0, burst: 0.0 });
    }
    let (num_str, unit) = s.split_at(s.len() - 1);
    let per_sec: f64 = num_str.parse().ok()?;
    let per_second = match unit {
        "s" => per_sec,
        "m" => per_sec / 60.0,
        "h" => per_sec / 3600.0,
        _ => return None,
    };
    if per_second <= 0.0 {
        return None;
    }
    // Default burst to 4x the per-second rate, minimum 1.
    let burst = (per_second * 4.0).max(1.0);
    Some(RateLimitConfig { per_second, burst })
}
```

## Hooking into the Dispatch Pipeline

The rate check is inserted as the very first step in `dispatch_action_with_options` (in `src/daemon/dispatch.rs`), before permission checks and any action-specific logic. This ensures cheap rejection of excessive traffic.

### Modified Dispatch Flow
1. **Extract Action Namespace**: From the `Action` enum, derive a string prefix (e.g., `Action::SecretsGetSecret` → `"secrets."`). We’ll add a helper function `action_namespace(&Action) -> String` that returns the dot-separated prefix (including the trailing dot). For actions without a clear namespace (e.g., `system.info`), we use the part before the first dot, or the whole string if no dot exists, then append a dot. The wildcard `""` matches any action.
2. **Lookup Config**: Check `state.rate_limit_store.configs` for the longest matching prefix (explicit match preferred over wildcard). If none found, use the hardcoded default for that namespace (see Default Limits below).
3. **Get or Create Bucket**: For the given `peer_uid` and resolved namespace, retrieve or initialize a `RateBucket` from `state.rate_limit_store.buckets`.
4. **Consume Token**: Call `bucket.take(config)`. If it returns `Some(RateLimitHit)`, construct a rate-limited error response and return early. If `None`, the action proceeds.

### Error Response
On limit exceed, return a JSON response matching deskbrid’s existing error format:
```json
{
  "type": "response",
  "id": "<request_id>",
  "seq": <sequence_number>,
  "status": "error",
  "error": {
    "code": "RATE_LIMITED",
    "message": "rate limit exceeded for action '<namespace>'; retry after <ms> ms",
    "retry_after_ms": <retry_after_ms>
  }
}
```
This is consistent with other error responses (e.g., permission denied) and includes a `retry_after_ms` field for clients to honor.

### State Initialization
In `src/daemon/lib.rs` (or `src/daemon/mod.rs`), initialize `RateLimitStore` in `DaemonState`:
```rust
pub struct DaemonState {
    // ... existing fields ...
    pub rate_limit_store: RateLimitStore,
}
```
The store is populated from `permissions.toml` during `Permissions::load()` (or a dedicated load function). If the file is missing or the `[rate_limits]` section is absent, the store is initialized with hardcoded defaults.

## Default Limits per Namespace

Hardcoded defaults ensure security even without configuration. Values are chosen to be restrictive for high-risk actions and generous for low-risk, observational actions.

| Namespace | Example Actions               | Default Limit | Rationale |
|-----------|-------------------------------|---------------|-----------|
| `secrets.` | `secrets.get_secret`, `secrets.store_secret` | `5/m` | Credential access is extremely sensitive; low limit prevents brute-force or harvesting. |
| `files.`   | `files.read`, `files.write`, `files.list` | `60/m` | File I/O can be costly; moderate limit prevents excessive disk churn while allowing reasonable workflows. |
| `process.` | `process.start`, `process.kill` | `30/m` | Process control is powerful but not as critical as secrets; moderate limit. |
| `terminal.`| `terminal.create`, `terminal.send_keys` | `10/m` | Spawning terminals can lead to resource exhaustion; low limit. |
| `browser.` | `browser.navigate`, `browser.screenshot` | `20/m` | Browser automation is moderately risky; limit to prevent abuse. |
| `system.`  | `system.info`, `system.battery`, `system.idle` | `60/m` | System queries are frequent and relatively safe; higher limit to support monitoring dashboards. |
| `""` (wildcard) | All other actions | `120/m` (2/sec) | Security-first: unknown actions treated with suspicion, not given the benefit of the doubt. |

### Deriving the Namespace from an Action

Deskbrid already has `Action::action_type()` returning strings like `"secrets.get-secret"`,
`"files.read"`, `"system.info"`. We split on the first `.` and append `.` to get the
namespace. No need for `std::any::type_name` or a separate `namespace()` method —
the action type string IS the namespace source.

```rust
fn action_namespace(action: &Action) -> &'static str {
    let at = action.action_type();
    match at.split('.').next() {
        Some(prefix) if KNOWN_NAMESPACES.contains(prefix) => {
            // Return a pointer into a static string to avoid allocation
            // Known namespaces: "secrets", "files", "process", "terminal", "browser", "system"
            ...
        }
        _ => "", // wildcard — unknown actions fall through
    }
}
```

In the dispatch layer (`src/daemon/dispatch.rs`), the namespace is derived inline:

```rust
let namespace = action.action_type()
    .split('.')
    .next()
    .filter(|ns| KNOWN_NAMESPACES.contains(ns))
    .unwrap_or(""); // "" = wildcard
```

This avoids bloating the `Action` enum with a dedicated method.

## Codebase-Specific Adjustments

### 1. `configs` is read-only — no Mutex needed

The `configs: HashMap<String, RateLimitConfig>` is populated at startup from
`permissions.toml` and never modified. Only `buckets` needs locking. Revised structure:

```rust
pub struct RateLimitStore {
    /// Read-only after init — populated from permissions.toml + hardcoded defaults.
    configs: HashMap<String, RateLimitConfig>,
    /// Per-(uid, namespace) token buckets. Only this needs a Mutex.
    buckets: Mutex<HashMap<u32, HashMap<String, RateBucket>>>,
}
```

### 2. Preserve existing rate limit code

The old `DaemonState.rate_limit` field and `check_rate_limit()` in `src/daemon/rate_limit.rs`
are kept for backward compatibility. The new per-namespace system is a parallel check
inserted before the existing global check in `dispatch_action_with_options`. The old
path can be deprecated later.

### 3. Draft defaults are final

Per Jeremy's sign-off (2026-06-07):

| Namespace | Limit |
|-----------|-------|
| `secrets.` | 5/m |
| `terminal.` | 10/m |
| `process.` | 30/m |
| `browser.` | 20/m |
| `files.` | 60/m |
| `system.` | 60/m |
| wildcard | 120/m (2/sec) |

### 4. Test: Per-UID bucket isolation

A critical test: agent UID 1000 exhausting `secrets.` quota must NOT affect agent
UID 1001. Each `(uid, namespace)` pair gets its own token bucket. Verified with:

```rust
#[test]
fn uid_isolation_secrets_buckets_independent() {
    // UID 1000 exhausts secrets quota
    for _ in 0..5 { store.take(1000, "secrets.", config); }
    assert!(store.take(1000, "secrets.", config).is_some()); // rate limited

    // UID 1001 still has full quota
    assert!(store.take(1001, "secrets.", config).is_none()); // OK
}
```

## Failure in Production Without This System

Without per-action, per-agent rate limiting:

1. **Credential Harvesting**: A malicious or compromised agent could call `secrets.get_secret` in a tight loop, rapidly exfiltrating all secrets stored in deskbrid’s vault before detection.
2. **Denial-of-Service via Resource Exhaustion**: An agent could spam expensive actions like `files.read` on large files or `process.start` loops, consuming CPU, disk I/O, or process table slots, degrading or crashing the desktop session for all users on the machine.
3. **Fairness Issues**: A poorly behaving agent could monopolize the deskbrid socket, causing high-latency or timeouts for well-behaved agents sharing the same UID (if UID-based) or even different UIDs (if the limiter were truly global).
4. **Stealth Abuse**: Because the current limiter returns generic errors (or none) when exceeded, an attacker might not realize they are being throttled and could continue attempting to bypass limits through distributed timing attacks.
5. **Lack of Observability**: Without per-action metrics, administrators cannot identify which actions are being abused or adjust limits based on real-world usage.

Implementing the proposed system closes these gaps, providing a defense-in-depth layer that complements existing permission checks and audit logging.
