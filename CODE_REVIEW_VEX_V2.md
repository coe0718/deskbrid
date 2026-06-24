# Vex Code Review — deskbrid v1.0.0 Hardening Review

**Reviewer:** Vex
**Date:** 2026-06-24
**Commit range:** v0.13.0 → v1.0.0 (based on prior review of v0.13.0)
**Files reviewed:** `src/daemon/execute_secrets.rs`, `src/daemon/rate_limit.rs`, `src/daemon/rules/`, `src/daemon/persistence/`, `src/daemon/dashboard/`, `src/daemon/execute_capabilities.rs`

---

## 1. Secrets (`execute_secrets.rs`)

### ✅ Zeroize Called Correctly
**Status:** No issue found.

`SecretString` (lines 7-27) is a dedicated wrapper that:
- Derives `Zeroize` (line 7) — zeroizes on drop
- Implements `Drop` explicitly (lines 10-14) calling `self.0.zeroize()`
- Overrides `Debug` to print `<redacted>` (lines 16-20)
- In `get_secret()` (lines 137-140), the secret is zeroized immediately after `trim_end_matches()` before the function returns

```rust
let mut secret = SecretString(String::from_utf8(output.stdout)?);
let trimmed = secret.0.trim_end_matches('\n').to_string();
secret.0.zeroize();  // immediate zeroize
drop(secret);        // double-ensure drop
```

This pattern is sound.

### ⚠️ W1 — Error Messages Propagate secret-tool stderr to Clients
**Severity:** Warning (not critical — does not leak actual secret values)

In `list_collections()` (line 82), `get_secret()` (line 134), and `store_secret()` (line 181), the stderr output from `secret-tool` is embedded in error messages returned to clients:

```rust
anyhow::bail!("secret-tool search failed: {stderr}");
anyhow::bail!("secret-tool lookup failed: {stderr}");
anyhow::bail!("secret-tool store failed: {stderr}");
```

While this does **not** expose the actual secret value, it can leak:
- Attribute key names used in the secret lookup
- File paths from `secret-tool`'s internal state
- GNOME Keyring / libsecret implementation details
- Collection names the caller may not already know

**Recommendation:** Sanitize error messages before returning. Strip anything that looks like a secret value or path. Alternatively, return a generic `"secret-tool failed: internal error"` and log the full details server-side only.

### ⚠️ W2 — Rules-Dispatched Secrets Actions Bypass Confirmation Gate
**Severity:** Warning

In `dispatch.rs` lines 323-340, secrets actions require `options.require_confirmation == Some(true)`:

```rust
if is_secrets_action(&action) {
    if !matches!(&action, Action::SecretsListCollections)
        && options.require_confirmation != Some(true)
    {
        return response; // CONFIRMATION_REQUIRED error
    }
```

However, the rules engine (`daemon/rules/eval/engine.rs` lines 56-68) dispatches actions via:

```rust
crate::daemon::dispatch::dispatch_action(
    &request_id,
    parsed_action,
    &state,
    0, // peer_uid: rule actions run as daemon
    seq,
)
```

`dispatch_action` (dispatch.rs lines 22-38) uses `RequestOptions::default()` where `require_confirmation: None`. This means **rule-triggered secrets actions bypass the confirmation requirement entirely**.

A user who can create rules (via `rule.create`, allowed by default in `default_safe()`) can trigger `secrets.get_secret` or `secrets.store_secret` without any interactive confirmation.

**Mitigating factor:** The `default_safe()` permissions don't include `secrets.*` in the allow list, so the permission check would block it for peer_uid=0 (daemon) under default configuration. However, under `allow_all()` permissions (no config file), peer_uid=0 would pass all permission checks including HIGH_RISK actions, and then bypass confirmation via the rules engine path.

**Recommendation:** Either (a) add `require_confirmation: Some(true)` to the rules engine dispatch, or (b) skip the confirmation gate only for explicitly pre-approved rule actions, or (c) require `secrets.*` actions to be explicitly named in permissions for peer_uid=0 (daemon).

---

## 2. Rules Engine (`daemon/rules/`)

### ⚠️ W3 — Rules Engine Runs as UID 0 With No Privilege Boundary
**Severity:** Warning (privilege escalation path under `allow_all()`)

When the rules engine dispatches an action, it uses `peer_uid: 0` (lines 56-68 of `engine.rs`):

```rust
tokio::spawn(async move {
    let result = crate::daemon::dispatch::dispatch_action(
        &request_id,
        parsed_action,
        &state,
        0, // peer_uid: rule actions run as daemon
        seq,
    )
```

This means rule actions run with the daemon's effective UID. If permissions are `allow_all()` (the backward-compatible mode when no `permissions.toml` exists), peer_uid=0 passes **all** permission checks including HIGH_RISK actions (`browser.evaluate`, `process.start`, `secrets.get_secret`, etc.).

**Scenario:** A malicious or compromised agent can:
1. Send `rule.create` to create a rule
2. The rule triggers on `WindowOpened` and dispatches `secrets.get_secret`
3. Under `allow_all()` permissions, this bypasses both confirmation AND permission checks

**Recommendation:** Rules engine should use a restricted permission context, not UID 0 with full daemon privileges. Consider a separate "rule execution permissions" that's a subset of daemon permissions, or explicitly deny HIGH_RISK actions for rule-dispatched actions regardless of permissions config.

### ⚠️ W4 — No Rule-Trigger-Rule Loop Detection
**Severity:** Warning

The rules engine (`engine.rs`) evaluates rules in response to `DeskbridEvent`s. There's no mechanism to prevent a rule's dispatched action from emitting an event that triggers another rule, creating a potential cascade/loop:

1. Rule A fires → dispatches action X → action X emits event E
2. Rule B's trigger matches E → Rule B fires → dispatches action Y
3. Action Y emits event F → Rule A's trigger matches F → ... loop

The engine has `cooldown_ms` and `max_fires` per-rule but these are tracked per-rule, not globally across rule chains. A cascading loop between two rules could consume infinite resources.

**Recommendation:** Add a dispatch depth counter or stack limit. Track the chain of rule→action→event→rule recursively. After N iterations (e.g., 3), stop dispatching and log a warning.

### ✅ Condition Evaluator Injection Safety
**Status:** No issue found.

`condition_matches()` in `matching.rs` (lines 186-217) evaluates `VarEquals` and `VarExists` by reading from session variables stored in a `DashMap`. The condition values are compared with simple `==` or `contains_key()` — no `eval()`, no `format!` with user data, no SQL concatenation. This is safe.

---

## 3. Rate Limiting (`rate_limit.rs`)

### ✅ C1 — Mutex Held for Minimal Duration
**Status:** No issue found.

In `RateLimitStore::check()` (lines 239-257):

```rust
let mut buckets = self.buckets.lock().unwrap();
let ns_buckets = buckets.entry(peer_uid).or_default();
let bucket = ns_buckets
    .entry(namespace.to_string())
    .or_insert_with(|| RateBucket::new(config));
bucket.take(config)  // immediately drop lock after operation
```

The lock is held only for the HashMap lookup and a single `f64` arithmetic operation — no I/O, no waiting. This is acceptable.

### ✅ C2 — Namespace Bypass via Crafted Strings: Not Reproducible
**Status:** No issue found (concern was based on a misunderstanding).

The reported concern was: "namespace bypass via crafted strings like `secrets.x.y`".

Looking at `action_namespace()` (lines 134-150):

```rust
pub(crate) fn action_namespace(action: &Action) -> &'static str {
    let at = action.action_type();  // static str from Action variant
    let prefix = at.split('.').next().unwrap_or("");
    if KNOWN_NAMESPACES.contains(&prefix) {
        match prefix {
            "secrets" => "secrets.",
            ...
        }
    } else {
        ""
    }
}
```

The `action_type()` is a **static string** derived from the `Action` enum variant being parsed, not from arbitrary user-controlled input. The Action enum is constructed via `Action::from_json()` which maps JSON `"type": "secrets.get_secret"` → `Action::SecretsGetSecret {...}`. There is no code path where a user-supplied arbitrary string like `"secrets.x.y"` can become an `action_type()` value that would bypass namespace rate limiting.

The concern does not apply to the current implementation.

### ✅ C3 — Rate Limiting Applied Based on Authenticated UID, Before Permissions
**Status:** No issue found.

In `dispatch_action_with_options()` (dispatch.rs lines 54-71):

```rust
// Per-namespace rate limit check (#129) — runs before global check
if let Some(hit) = state.rate_limit_store.check(peer_uid, &action) {
    return namespace_rate_limited_response(...);
}
if let Some(hit) = check_rate_limit(state, peer_uid).await {
    return rate_limited_response(seq, hit);
}
// Check permissions first  ← after rate limiting
if !state.permissions.check(peer_uid, &action) {
```

Rate limiting is applied before the permission check. The `peer_uid` is extracted from the Unix socket's `SO_PEERCRED` (permissions.rs lines 280-306) — the kernel-verified UID of the connected client. An unauthenticated client is still rate-limited by their socket UID.

---

## 4. SQLite Persistence (`daemon/persistence/`)

### ✅ C4 — SQL Injection: Not Found
**Status:** No issue found.

All user-supplied fields are bound via parameterized queries:

- `audit.rs` lines 14-24: `INSERT OR REPLACE INTO audit_log ... VALUES (?1, ?2, ...)` with `params![...]`
- `audit.rs` lines 37-53: Dynamic SQL construction appends `AND action = ?` with `Box::new(at.to_string())` as a bound parameter
- `clipboard.rs` lines 27-32: `WHERE text LIKE ?1` with `params![like, limit as i64]`
- `blackboard.rs` lines 15-25: `INSERT INTO blackboard ... VALUES (?1, ?2, ?3, ?4, ?5, ?6)` with `rusqlite::params![...]`
- `sessions.rs` lines 17-22: All fields parameterized

No string concatenation of user input into SQL.

### ⚠️ W5 — Migration Non-Atomicity on Crash
**Severity:** Warning

In `run_migrations()` (persistence/mod.rs lines 111-139):

```rust
for v in stored..CURRENT_SCHEMA_VERSION {
    match v {
        0 => { /* no DDL needed */ }
    }
    self.conn
        .pragma_update(None, "user_version", v + 1)  // Version updated
        .context(...);                                // But DDL may have failed
}
```

Each migration step:
1. Applies the migration (e.g., future `ALTER TABLE` DDL)
2. Updates `user_version` pragma to `v + 1`

If the process crashes **between** step 1 and step 2, the `user_version` is already incremented but the DDL was not committed. On restart, `run_migrations()` would skip past this version and never replay it.

Additionally, DDL in SQLite (like `ALTER TABLE`) is not automatically rolled back on error within a transaction unless the transaction is explicitly started with `BEGIN EXCLUSIVE`.

**Recommendation:** Wrap the entire migration loop in a transaction, or perform `user_version` update atomically with the DDL using `SAVEPOINT`.

### ⚠️ W6 — Audit Log params Field Can Store Malformed JSON
**Severity:** Warning (audit log poisoning / DoS, not direct code execution)

In `audit.rs`, `insert_audit()` stores a `params_json` value derived from `AuditEntry.error` and `AuditEntry.dry_run`. The field is `TEXT` in SQLite and stores raw JSON (or `None`).

If an attacker with write access to the DB (or a compromised audit path) stores a `params` value that is not valid JSON (e.g., `"error": "valid JSON but then { broken"`), then `parse_audit_params()` (lines 128-141) silently returns `(None, None)`:

```rust
let Ok(val) = serde_json::from_str::<serde_json::Value>(json) else {
    return (None, None);  // Silently fails, losing the error context
};
```

This means audit log entries can silently lose their error information, poisoning the audit trail.

**Recommendation:** `parse_audit_params()` should log a warning when JSON parsing fails, so malformed entries are detectable.

---

## 5. Dashboard (`daemon/dashboard/`)

### ✅ C7 — Localhost Binding Confirmed, Warning on Non-Localhost
**Status:** No issue found.

Default bind address is `127.0.0.1` (`cli/mod.rs` line 101):

```rust
#[arg(long, default_value = "127.0.0.1")]
dashboard_bind: String,
```

In `dashboard/mod.rs` lines 19-35, there's an explicit runtime warning when binding to non-localhost:

```rust
if bind_ip != "127.0.0.1" && bind_ip != "::1" && bind_ip != "localhost" {
    warn!(
        "Dashboard bound to {} — exposed to network with NO authentication. \
         Anyone on the network can see clipboard, screenshots, audit log, and window titles.",
        bind_ip
    );
}
```

This is good defense-in-depth.

### ✅ Connection Limit
**Status:** No issue found.

`MAX_DASHBOARD_CONNECTIONS = 32` (line 9) with a semaphore-based limit (lines 36-49). Rejected connections log a warning and continue. This prevents fd/memory exhaustion from connection flooding.

---

## 6. Bonus: `execute_capabilities.rs` Lines 17-21 Self-Reporting Bug

### ⚠️ B1 — `ui.tree.get` Falsely Reported as Unsupported
**Severity:** Warning (incorrect capability reporting, not a security bug)

The `execute_capabilities()` function (lines 17-21) marks these as unsupported:

```rust
let mut unsupported = vec![
    serde_json::json!({"action":"ui.tree.get","reason":"AT-SPI not integrated yet"}),
    serde_json::json!({"action":"ui.element.click","reason":"AT-SPI not integrated yet"}),
    serde_json::json!({"action":"ui.element.set_text","reason":"AT-SPI not integrated yet"}),
];
```

However, in `execute_stubs.rs` (lines 33-45), these are **actually implemented**:

```rust
UiTreeGet => {
    // AT-SPI tree via a11y module (for desktop UI, not browser DOM)
    crate::a11y::tree(Some(5)).await?  // ← WORKING AT-SPI
}
UiElementClick { ref selector, tab_index } => crate::browser::click(tab_index, selector).await?,  // CDP, not AT-SPI, but WORKING
UiElementSetText { ref selector, ref text, tab_index } => crate::browser::set_text(tab_index, selector, text).await?,  // CDP, not AT-SPI, but WORKING
```

| Action | Reported Reason | Actual Status |
|--------|-----------------|---------------|
| `ui.tree.get` | "AT-SPI not integrated yet" | **Working** — AT-SPI via `crate::a11y::tree()` |
| `ui.element.click` | "AT-SPI not integrated yet" | **Working** — CDP via `crate::browser::click()` |
| `ui.element.set_text` | "AT-SPI not integrated yet" | **Working** — CDP via `crate::browser::set_text()` |

**`ui.tree.get`** is the clear bug: the comment in `execute_stubs.rs` even says "AT-SPI tree via a11y module", confirming it's wired to working AT-SPI. The capability report incorrectly tells clients it's unsupported.

**Recommendation:** Remove `ui.tree.get` from the `unsupported` list. `ui.element.click` and `ui.element.set_text` are correctly labeled as "not AT-SPI" (they use CDP, not AT-SPI) — but they're functional, so the capability report's framing is misleading. Consider changing the reason to something like "browser CDP-based, not AT-SPI" for clarity.

---

## Summary

| ID | Severity | Area | Issue |
|----|----------|------|-------|
| W1 | Warning | Secrets | Error messages propagate secret-tool stderr to clients |
| W2 | Warning | Secrets | Rules-dispatched secrets actions bypass confirmation gate |
| W3 | Warning | Rules | Rules engine runs as UID 0 with no privilege boundary under `allow_all()` |
| W4 | Warning | Rules | No rule-trigger-rule loop detection |
| W5 | Warning | Persistence | Migration non-atomic on crash — `user_version` updated before DDL commits |
| W6 | Warning | Persistence | Audit log `params` field silently loses data on malformed JSON |
| B1 | Bonus | Capabilities | `ui.tree.get` incorrectly reported as unsupported despite working AT-SPI |
| C1–C4, C7 | — | Rate Limit, Dashboard | No issues found |

**Previous review (v0.13.0):** 4 CRITICAL, 29 WARNING, 7 SUGGESTION
**This review (v1.0.0):** 0 CRITICAL, 6 WARNING, 1 Bonus

The dashboard binding (previously CRITICAL C1 — 0.0.0.0 exposure) has been properly fixed with localhost default and runtime warning. The binary corruption found in `context.rs` is resolved. The codebase shows significant hardening since v0.13.0.

**Recommended fixes:** W2 (confirmation bypass) and W3 (rules UID 0 privilege) should be addressed together — the confirmation requirement for secrets was specifically designed to protect against the scenario W3 enables (automated secrets access without user interaction).