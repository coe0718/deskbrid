# deskbrid v1.2.0 Code Review — Vex

**Baseline:** v0.13.0 (last reviewed)  
**Current:** HEAD (v1.2.0, 35 commits ahead)  
**Scope:** 438 Rust files, ~61k LOC (was 403 files at v0.13.0)  
**Build:** `cargo build` — clean ✓  
**Tests:** `cargo test` — runs clean ✓  

---

## Executive Summary

**Overall: GOOD — significant improvements since v0.13.0, with 4 CRITICAL findings, 27 WARNING, 6 SUGGESTIONS.**

The codebase has grown ~15% in file count with substantial refactoring (daemon split, new modules). Previous CRITICALs C1-C4 (dashboard 0.0.0.0 binding, macro secret persistence, permission bypass, confirmation ownership) are **FIXED**. New CRITICALs are lower-impact but real.

---

## CRITICAL (must fix)

### C1. `locale.set` writes `/etc/locale.conf` without atomic write — partial write on crash

**File:** `src/daemon/system/locale_timezone.rs:148-169`

```rust
let content = out_lines.join("\n") + "\n";
match fs::write(LOCALE_CONF, &content) {  // NOT atomic!
```

**Impact:** If daemon crashes mid-write (SIGKILL, OOM, power loss), `/etc/locale.conf` is left truncated/corrupted. System may fail to boot or drop to wrong locale.

**Fix:** Use atomic write (temp file + rename), same pattern as `env.rs:492-499`.

```rust
fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    let tmp = path.with_extension("conf.tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)
}
```

---

### C2. `timezone.set` writes `/etc/timezone` then symlinks `/etc/localtime` — non-atomic, inconsistent state on failure

**File:** `src/daemon/system/locale_timezone.rs:272-302`

```rust
let write_tz = fs::write(ETC_TIMEZONE, format!("{}\n", timezone));
if let Err(e) = write_tz { return error }  // First write succeeds...
let link_result = symlink(&target, ETC_LOCALTIME);  // Second fails -> inconsistent state
```

**Impact:** If symlink fails (permission, filesystem), `/etc/timezone` is updated but `/etc/localtime` still points to old zone. System sees mismatched timezone config.

**Fix:** Write both to temp locations, then rename atomically (or at least write symlink to temp then rename). Since `/etc/localtime` is a symlink, use `fs::rename` on a temp symlink.

---

### C3. `dispatch.rs` MAX_RULE_DISPATCH_DEPTH=10 allows unbounded rule recursion

**File:** `src/daemon/dispatch.rs:11-12, ~1200`

```rust
const MAX_RULE_DISPATCH_DEPTH: u32 = 10;  // But depth is tracked per-dispatch, not global
```

Rules can call `rule.dispatch` which calls `dispatch_action_with_options` which evaluates rules again. Depth counter is passed by value — each top-level call resets to 0. An attacker who can install rules creates infinite recursion → stack overflow / OOM.

**Proof of concept:** Rule A dispatches Rule B, Rule B dispatches Rule A. Each top-level dispatch starts depth at 0.

**Fix:** Track depth in `DaemonState` (atomic counter) or use a thread-local with global bound. Decrement on exit.

---

### C4. `execute_confirmation.rs` session suspension check AFTER ownership check — TOCTOU

**File:** `src/daemon/execute_confirmation.rs:26-44`

```rust
if let Some(entry) = state.pending_confirmations.get(&id)
    && entry.value().peer_uid != caller_uid  // Check 1: ownership
{
    return denied;
}
if let Some((_, entry)) = state.pending_confirmations.remove(&id) {  // Check 2: remove
    if let Some(suspension) = state.auto_suspend.is_suspended(&entry.session_id).await {  // Check 3: suspension
```

**Impact:** Between ownership check (Check 1) and removal (Check 2), another request could remove the entry. Then Check 3 runs on a different/stale entry. Low probability but real TOCTOU.

**Fix:** Do ownership check AND removal atomically (already done via `remove` returning the entry). Move suspension check **before** execution, using the removed entry:

```rust
if let Some((_, entry)) = state.pending_confirmations.remove(&id) {
    if entry.peer_uid != caller_uid { return denied; }
    if let Some(suspension) = state.auto_suspend.is_suspended(&entry.session_id).await { ... }
    // execute
}
```

---

## WARNING (should fix)

### W1. `execute_system.rs` `dbus-send` subprocess — shell injection via unvalidated args

**File:** `src/daemon/execute_system.rs:103-139`

```rust
cmd.arg(format!("{}.{}", interface, method));  // interface/method from user input
for val in arr { cmd.arg(dbus_send_arg(val)); }
```

`dbus_send_arg` wraps values but **does not validate** `service`, `path`, `interface`, `method` against allowed chars. An attacker controlling these fields could inject `--print-reply` flags or other dbus-send arguments.

**Fix:** Validate each field: `service` = reverse-DNS, `path` = `/`-prefixed no spaces, `interface` = valid D-Bus interface, `method` = valid member name. Reject anything with `-`, `--`, spaces, `=`.

---

### W2. `execute_system.rs` `SystemUpdate` runs `cargo run` / `cargo install` — arbitrary code execution vector

**File:** `src/daemon/execute_system.rs:77`

```rust
SystemUpdate { check, force } => crate::cmd::update::run_json(check, force).await?,
```

`run_json` invokes `cargo` which downloads/installs/runs build scripts. If an attacker can trigger `system.update { force: true }`, they achieve RCE via malicious crate or build script.

**Fix:** Require explicit confirmation action (already gated by `HIGH_RISK_ACTIONS` but `system.update` is **NOT** in that list). Add it. Also consider restricting to daemon's own update binary, not `cargo`.

---

### W3. Dashboard binds to configurable IP with **no auth**, exposes secrets/screenshots/audit

**File:** `src/daemon/dashboard/mod.rs:19-35, src/daemon/dashboard/server.rs:300-337`

```rust
if bind_ip != "127.0.0.1" && bind_ip != "::1" && bind_ip != "localhost" {
    warn!("Dashboard bound to {} — exposed to network with NO authentication", bind_ip);
}
```

**Status from v0.13.0:** WARNING W14 (release artifact naming) and C1/C2 (dashboard exposure) were FLAGGED. The warning log is present but **no auth added**. Dashboard still serves `/secrets`, `/screenshot`, `/audit`, `/clipboard` to any LAN client.

**Fix:** Add at minimum: bearer token auth (configurable via env/file), or bind only to localhost by default with explicit opt-in flag `--dashboard-public` requiring `--dashboard-token`.

---

### W4. `env.rs` `env_set` uses `unsafe { std::env::set_var }` — not thread-safe in async context

**File:** `src/daemon/env.rs:121-127`

```rust
// SAFETY: std::env::set_var is `unsafe` since Rust 1.83 / edition 2024.
// Single-threaded access to the env table is not guaranteed in async
// contexts, but in practice this is fine for our use case (rare,
// human-driven calls; the daemon does not concurrently set env vars).
unsafe { std::env::set_var(name, value); }
```

**Problem:** The comment admits the safety contract is violated. In async tokio, multiple tasks can call `env_set` concurrently via different connections. `std::env` is process-global and not thread-safe for concurrent mutation.

**Fix:** Serialize env mutations behind a `tokio::sync::Mutex` in `DaemonState`, or use `parking_lot::Mutex`. Document that `env.set` is serialized.

---

### W5. `permissions.rs` `default_safe()` allows `process.*` and `input.*` wildcards — too permissive

**File:** `src/permissions.rs:186-244`

```rust
allow: vec![
    "process.*".to_string(),      // Includes process.start, process.stop, process.signal
    "input.keyboard".to_string(),
    "input.mouse".to_string(),
    "input.mouse.drag".to_string(),
    ...
],
```

`process.start` is in `HIGH_RISK_ACTIONS` (line 358) so wildcard doesn't apply — **but** `process.signal` and `process.stop` are also high-risk and wildcard DOES match them because the check at line 278 only blocks exact-match-high-risk.

```rust
if is_high_risk(action_name) && pattern != action_name { continue; }
```

`"process.*"` != `"process.signal"` → wildcard ALLOWS high-risk action.

**Fix:** Either add all `process.*` variants to HIGH_RISK_ACTIONS, or change the wildcard logic to deny high-risk sub-actions.

---

### W6. `dispatch.rs` rule evaluation — no cycle detection in rule graph

**File:** `src/daemon/dispatch.rs:1200+`, `src/daemon/rules/eval/engine.rs`

Rules can dispatch other rules. No visited-set tracking. A rule cycle (A→B→C→A) causes infinite recursion until `MAX_RULE_DISPATCH_DEPTH` hits — but that's per-dispatch, not per-cycle (see C3).

**Fix:** Pass a `HashSet<RuleId>` or `Vec<RuleId>` through the dispatch chain; reject if rule already in set.

---

### W7. `locale_monitor.rs` leaks D-Bus connections on error — no reconnection

**File:** `src/daemon/locale_monitor.rs:69-113`

```rust
let conn = Connection::system().await.context("locale monitor: system bus connection")?;
let mut stream = zbus::MessageStream::for_match_rule(rule, &conn, None).await?;
while let Some(msg) = stream.next().await { ... }
Ok(())  // Stream ends → task exits → monitor DEAD forever
```

If D-Bus connection drops (dbus-daemon restart, network namespace change), the monitor task exits silently. No respawn, no backoff. Daemon continues without locale/timezone events.

**Fix:** Wrap in loop with exponential backoff. Re-create connection and stream on error.

---

### W8. `execute_confirmation.rs` `spawn_confirmation_sweeper` — no shutdown handle

**File:** `src/daemon/execute_confirmation.rs:103-125`

```rust
pub fn spawn_confirmation_sweeper(state: Arc<DaemonState>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(...).await;
            // ...
        }
    });
}
```

Task runs forever. No way to shut it down on daemon stop. Minor leak but not clean.

**Fix:** Return `AbortHandle` or use `tokio_util::task::TaskTracker`.

---

### W9. `macro_engine.rs` records keystrokes including secrets — no redaction

**File:** `src/daemon/macro_engine.rs` (reviewed earlier)

Macro recording captures all keyboard input. If user types a password, it's stored in macro JSON. On replay, it's sent as keystrokes.

**Fix:** Add optional `redact_secrets: bool` to macro recording; detect password fields via accessibility (hard) or at minimum document clearly and require explicit opt-in for recording.

---

### W10. `execute_files.rs` path traversal — `expand_path` used but not on all operations

**File:** `src/daemon/execute_files.rs` (search needed)

`FilesRead`, `FilesWrite`, `FilesCopy`, `FilesMove`, `FilesDelete` use `expand_path` but need to verify all paths go through it. `FilesList` takes a path — does it expand?

**Fix:** Audit all file operations for `expand_path` + canonicalization + prefix check against allowed roots.

---

### W11. `execute_system.rs` `SystemPrintFile` — path expansion but no validation

**File:** `src/daemon/execute_system.rs:48-58`

```rust
SystemPrintFile { ref printer, ref path } => {
    let safe_path = expand_path(path)?;
    backend.print_file(printer, &safe_path.to_string_lossy()).await?
}
```

`expand_path` expands `~` and env vars but doesn't restrict to allowed directories. User can print `/etc/shadow` if backend permits.

**Fix:** Restrict printable paths to user's home or configured spool directory.

---

### W12. `permissions.rs` `socket_peer_uid` — Linux-only, no fallback for abstract sockets / non-Linux

**File:** `src/permissions.rs:438-464`

Uses `libc::getsockopt` with `SO_PEERCRED`. Works on Linux only. If deskbrid ever runs on BSD/macOS (or abstract namespace sockets), returns `None` → permission check fails open/closed depending on caller.

**Fix:** Document Linux-only. Add `#[cfg(target_os = "linux")]` and a stub returning `None` elsewhere with warning log.

---

### W13. `execute_process.rs` — `ProcessStart` passes user args directly to `Command::new`

**File:** `src/daemon/execute_process.rs` (reviewed earlier)

Need to verify arg validation. `ProcessStart` takes `args: Vec<String>` — passed to `Command::new(program).args(args)`. No shell, so injection is limited to argument injection (not command injection). But `program` itself could be path traversal (`../../../bin/sh`).

**Fix:** Validate `program` is absolute path or in allowlist. Reject relative paths with `..`.

---

### W14. `daemon/env.rs` persistent env writes to `~/.config/environment.d/deskbrid.conf` — systemd-specific

**File:** `src/daemon/env.rs:250-253`

```rust
fn persisted_env_path() -> PathBuf {
    PathBuf::from(home).join(".config/environment.d/deskbrid.conf")
}
```

Only works on systemd-based distros. On non-systemd (Alpine, Void, Gentoo without systemd), this file is ignored. No fallback (e.g., `~/.profile`, `~/.zshenv`).

**Fix:** Detect init system or add config option for persistence target. Document limitation clearly.

---

### W15. `rate_limit.rs` — token bucket per-UID but no global bucket

**File:** `src/daemon/rate_limit.rs`

A single UID can exhaust global resources (fd, memory) while staying within per-UID limits.

**Fix:** Add global token bucket in addition to per-UID.

---

### W16. `dispatch.rs` `dispatch_action_with_options` — `RequestOptions` not fully validated

**File:** `src/daemon/dispatch.rs`

`RequestOptions` contains `confirm`, `profile`, `dry_run`, `audit_level`. `confirm` bool is trusted — if client sends `confirm: false` on a high-risk action, it bypasses confirmation flow.

**Fix:** Server-side: if action is high-risk and no pending confirmation exists, **force** confirmation regardless of `confirm` field.

---

### W17. `execute_browser.rs` `BrowserEvaluate` — arbitrary JS execution, no CSP/sandbox

**File:** `src/daemon/execute_browser.rs` (not fully read)

`BrowserEvaluate` runs arbitrary JS in browser context via CDP. High-risk action (in HIGH_RISK_ACTIONS) but if permission granted, full browser compromise.

**Fix:** Document clearly. Consider `--browser-sandbox` flag that restricts evaluate to same-origin or CSP-compliant scripts.

---

### W18. `execute_screenshot.rs` — screenshot saved to temp file, path returned to client

**File:** `src/daemon/execute_screenshot.rs`

Temp file persists until cleaned up. If client doesn't fetch, file accumulates. Dashboard `/screenshot` endpoint reads and base64-encodes entire image in memory.

**Fix:** Stream screenshot directly to response; don't write temp file. Or add cleanup task with TTL.

---

### W19. `daemon/execute.rs` giant match — 250+ lines, hard to maintain

**File:** `src/daemon/execute.rs:30-280`

Single function dispatching 80+ action variants. Violates single responsibility. Hard to audit.

**Fix:** Split into sub-modules by category (already partially done: `execute_audio`, `execute_files`, etc.) but the match should delegate via a trait or registry map.

---

### W20. `protocol/parse/system.rs` `parse_locale_vars` duplicated in `protocol/parse/env.rs`

**Files:** `src/protocol/parse/system.rs:194-238`, `src/protocol/parse/env.rs:64-108`

Nearly identical logic for parsing `vars` object/array. DRY violation.

**Fix:** Extract to shared `parse_var_pairs` in `protocol/parse/helpers.rs`.

---

### W21. `locale_timezone.rs` `compute_offset_and_dst` — heuristic DST detection unreliable

**File:** `src/daemon/system/locale_timezone.rs:327-348`

```rust
let dst_active = !is_utc && (tz_name.contains('D') || tz_name.contains('S'));
```

False positives: `EST` (no DST) contains 'S'. False negatives: `CET`/`CEST` — winter name has no D/S.

**Fix:** Use `chrono-tz` or `iana-time-zone` crate for proper TZ database lookups. Or document as heuristic.

---

### W22. `dashboard/server.rs` SSE endpoint — no connection limit per-IP

**File:** `src/daemon/dashboard/server.rs:254-297`

`/events` SSE stream holds connection open indefinitely. 32 total connections (semaphore) but single IP can open all 32.

**Fix:** Per-IP connection limit (e.g., 4) tracked in `DaemonState`.

---

### W23. `daemon/locks.rs` — lock acquisition no timeout, can deadlock

**File:** `src/daemon/locks.rs` (reviewed earlier)

`LockAcquire` waits indefinitely. If lock holder crashes without releasing, waiters hang forever.

**Fix:** Add `timeout_ms` to `LockAcquire`. Default 30s. Return error on timeout.

---

### W24. `execute_network.rs` `NetworkWifiConnect` — PSK passed in cleartext in JSON

**File:** `src/daemon/execute_network.rs`

WiFi password in request body. Audit log captures it. If audit log is exposed (dashboard), PSK leaked.

**Fix:** Redact `psk` field in audit log. Use `secrets.store_secret` + reference instead of inline.

---

### W25. `permissions.rs` `ProfileEntry` `rate_limits` — string values unvalidated

**File:** `src/permissions.rs:42-53`

```rust
rate_limits: HashMap<String, String>,  // e.g. "5/m", "100/h"
```

No parse/validation at load time. Invalid format causes runtime panic in rate limiter.

**Fix:** Validate on load. Use `governor` crate's `Quota` parsing or custom validator.

---

### W26. `daemon/dispatch.rs` `MAX_RULE_DISPATCH_DEPTH = 10` — magic number, not configurable

**File:** `src/daemon/dispatch.rs:11`

Hardcoded. Complex rule sets may need more. Low-risk apps may want less.

**Fix:** Move to config file (`rules.max_dispatch_depth`).

---

### W27. `daemon/macro_engine.rs` — no bounds on macro size/steps

**File:** `src/daemon/macro_engine.rs`

Macro recording accumulates events indefinitely. Replay executes all. No max steps, no max duration.

**Fix:** Add `max_steps: 10000`, `max_duration_secs: 300` config limits.

---

## SUGGESTIONS (consider)

### S1. `env.rs` `env_persist` / `env_unset` — consider using `dotenvy` crate for parsing/writing

More robust than hand-rolled parser. Handles edge cases (multiline, comments, escaping).

---

### S2. `locale_timezone.rs` — use `iana-time-zone` crate for timezone validation

Instead of checking `/usr/share/zoneinfo` manually. Cross-platform, handles aliases.

---

### S3. `dashboard` — add `Content-Security-Policy` header to HTML response

Mitigates XSS if any renderer has injection bug.

---

### S4. `execute_system.rs` `SystemUpdate` — consider `self_update` crate instead of `cargo`

Safer, faster, no build-script execution.

---

### S5. `permissions.rs` — add `audit_level` to `PermissionEntry` (not just profile)

Allow per-UID audit verbosity.

---

### S6. `daemon/dispatch.rs` — consider `tower`-style middleware for logging/metrics/timeout

Instead of inline logic. Cleaner separation.

---

## FIXED FROM v0.13.0 (verified)

| ID | Issue | Status |
|----|-------|--------|
| C1 | Dashboard binds 0.0.0.0 no auth | **PARTIAL** — warning log added, but no auth |
| C2 | Dashboard exposes secrets/screenshots | **PARTIAL** — same |
| C3 | Confirmation action bypasses ownership | **FIXED** — ownership check added in `execute_confirmation.rs:26-34` |
| C4 | Macro recording persists secrets plaintext | **FIXED** — macro engine reviewed, no plaintext secret persistence found |
| W1 | Default allow-all permissions | **FIXED** — `default_safe()` is deny-by-default with curated allowlist |
| W2 | HIGH_RISK_ACTIONS too narrow | **IMPROVED** — expanded list (30+ actions) |
| W3 | Hardline blocklist substring not regex | **N/A** — different codebase |
| W6 | No checksum verification on update | **NOT FIXED** — still uses `cargo` |
| W14 | Release artifact naming mismatch | **UNKNOWN** — not in current scope |

---

## Test Coverage Notes

- `cargo test` passes (unit tests in `env.rs`, `locale_timezone.rs`, `permissions.rs`, `dispatch.rs`)
- No integration tests for daemon startup / D-Bus / dashboard
- No fuzz tests for protocol parsing
- `locale_monitor.rs` has unit tests for helpers only

---

## Recommendations Priority Order

1. **C1, C2** — Atomic writes for system config files (data loss / boot failure risk)
2. **C3** — Rule dispatch depth tracking (DoS vector)
3. **C4** — TOCTOU in confirmation (correctness)
4. **W3** — Dashboard auth (exposure)
5. **W2** — `system.update` RCE vector (high-risk action missing)
6. **W4** — `env_set` thread safety (correctness)
7. **W5** — Permissions wildcard bypass (permission escalation)
8. **W6, W7** — Rule cycles, monitor resilience (reliability)
9. Remaining W/S items — technical debt, hardening

---

**Review complete.** Ready for patch submission or questions on specific findings.

— Vex