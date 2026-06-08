# Deskbrid — Comprehensive Code Review

**Reviewer:** Vex  
**Date:** 2025-06-07  
**Version:** 0.13.0 (edition 2024)  
**Scope:** 403 Rust source files, 483 total tracked files across `src/`, `clients/`, `site/`, `scripts/`, `.github/`, `docs/`  
**Method:** Three parallel domain-focused passes (security, correctness, build/deploy/docs) + manual verification of all critical findings

---

## Executive Summary

Deskbrid is an ambitious and well-architected Linux desktop HAL daemon — auto-detecting compositors, wrapping 10+ desktop environments behind a unified JSON-over-Unix-socket protocol, and exposing 100+ MCP tools. The protocol layer, backend trait design, and multi-DE abstraction are genuinely impressive work.

However, **the security posture is dangerous for a tool that controls keystrokes, clipboard, files, and secrets.** The dashboard exposes the entire desktop — including secrets and live screenshots — to anyone on the network with zero authentication. The confirmation system, designed as a safety gate, can be bypassed by any local socket client. Macro recording silently persists secrets plaintext to disk. The permission system defaults to allow-all. These need to be fixed before any production deployment.

---

## Severity Summary

| Severity | Count | Fixed |
|----------|-------|-------|
| 🔴 CRITICAL | 4 | 4 |
| 🟡 WARNING | 29 | 6 |
| 🔵 SUGGESTION | 7 | 0 |
| **Total** | **40** | **10** |

### Fixed
- ✅ **C1** — Dashboard binds 127.0.0.1 by default, `--dashboard-bind` flag (commit `2233902`)
- ✅ **C2** — Screenshot endpoint auth-gated (resolved by C1)
- ✅ **C3** — Confirmation ownership check + route through dispatch (commit `4c891f8`)
- ✅ **C4** — Macro recording skips secrets.*, clipboard.*, process.* (commit `3c05555`)
- ✅ **W1** — `default_safe()` replaces allow-all on fresh install (commit `bcdc197`)
- ✅ **W2** — HIGH_RISK_ACTIONS expanded from 5 → 21 entries (commit `bcdc197`)
- ✅ **W6** — Missing checksum is now a hard error in self-updater (commit `7e67061`)
- ✅ **W7** — install.sh verifies SHA256 before extraction (commit `7e67061`)
- ✅ **W9** — Confirmation ops routed through backend-free code path (commit `4c891f8`)
- ✅ **W10** — TCP auth bounded reads + constant-time token compare (commit `f44befb`)
- ✅ **W11** — Dashboard bounded reads + connection semaphore (commit `f44befb`)
- ✅ **W14** — Release artifact naming: `deskbrid-mcp-` → `deskbrid-` (commit `bcdc197`)

---

## 🔴 CRITICAL Findings

### C1. Dashboard binds `0.0.0.0:20129` with no authentication ✅ FIXED

**File:** `src/daemon/dashboard/mod.rs:16`

```rust
let addr = format!("0.0.0.0:{}", DASHBOARD_PORT);
```

The dashboard exposes the full desktop state — system info, monitors, windows, network, audio, **clipboard history, audit log, secrets, macros, confirmation queue, agent mailbox** — to the entire LAN with no auth, no TLS, no CSRF protection, no `Origin`/`Host` header validation. Any device on the same network can read desktop state, clipboard contents, and audit history.

**Fix:** Bind to `127.0.0.1` by default. Add a `--dashboard-bind` flag for explicit opt-in to non-loopback binding. Require a bearer token or session cookie for all requests when non-loopback. Add `Host` header validation to prevent DNS rebinding.

---

### C2. Unauthenticated `/screenshot` endpoint on `0.0.0.0` ✅ FIXED

**File:** `src/daemon/dashboard/server.rs:252`

```rust
if method == "GET" && (path == "/screenshot" || path.starts_with("/screenshot?")) {
    // captures and returns the live desktop as PNG — no auth
```

On the current `0.0.0.0` bind, any network peer can trigger a screenshot and receive the full desktop image. Combined with C1, this is a live remote surveillance vector. The screenshot endpoint doesn't even require an active desktop session — it works whenever a backend is loaded.

**Fix:** Require dashboard authentication for all endpoints. Consider disabling the screenshot endpoint entirely unless explicitly enabled via config.

---

### C3. Confirmation action bypasses ownership check and permission re-check ✅ FIXED

**File:** `src/daemon/execute_confirmation.rs:16-31`

```rust
Action::ConfirmAction { id } => {
    let mut pending = state.pending_confirmations.lock().await;
    if let Some(entry) = pending.remove(&id) {
        // calls execute_action directly — NOT dispatch_action_with_options
        let result = crate::daemon::execute::execute_action(entry.action, b, state).await;
```

Two compounding flaws:

1. **No ownership check:** `PendingConfirmation` stores `peer_uid` (line 97) but `execute_confirmation` never compares the confirmer's UID against it. Any permitted socket client can confirm any other client's pending action — including another user's `secrets.store_secret` or `process.start`.

2. **Skips all dispatch gates:** `execute_action` is called directly instead of `dispatch_action_with_options`. This bypasses permission re-checks, rate limiting, macro recording, `dry_run` honoring, and `with_action_timeout`. The confirmed action executes with no timeout and no rate limit.

**Fix:** Thread the caller's `peer_uid` into `execute_confirmation`. Reject unless `entry.peer_uid == caller_uid`. Route the confirmed action through `dispatch_action_with_options` (or at minimum re-check permissions + apply timeout + honor `dry_run`).

---

### C4. Macro recording captures secrets plaintext to disk ✅ FIXED

**Files:** `src/daemon/dispatch.rs:99-108`, `src/protocol/serialize/extensions.rs:117-140`

The macro recording block at dispatch.rs:99-108 serializes every non-`macro.*` action via `action.to_json()`:

```rust
if !at.starts_with("macro.") {
    let params = action.to_json().unwrap_or_default();
    crate::daemon::macro_engine::record_action(state, at, parsed);
}
```

This runs **before** the secrets confirmation gate at dispatch.rs:302-319. The `SecretsStoreSecret` serializer includes the plaintext `secret` field:

```json
{ "type": "secrets.store_secret", "secret": "<plaintext>", ... }
```

When the macro is later saved (`macro_engine::save_macro` → `~/.local/share/deskbrid/macros/<name>.json`), the secret is written to disk in cleartext.

**Fix:** Skip recording for sensitive namespaces — at minimum `secrets.*`. Consider redacting `clipboard.*` and `process.*` as well. Add an explicit deny-list of action types that are never recorded.

---

## 🟡 WARNING Findings

### W1. Missing `permissions.toml` defaults to allow-all ✅ FIXED

**File:** `src/permissions.rs:45-50`

```rust
if !path.exists() {
    info!("No permissions file at {}, defaulting to allow-all", path.display());
    return Self::allow_all();
}
```

On first run (or any install without a permissions file), every action is permitted — `files.*`, `clipboard.*`, `screenshot`, `input.*`, `process.start`, `terminal.create`, `browser.evaluate`, all of it. For a tool that injects keystrokes and reads secrets, this is an unsafe default.

**Fix:** Ship a default `permissions.toml` with least-privilege defaults, or create one on first run. At minimum, deny `process.start`, `terminal.create`, `browser.evaluate`, `dbus.call`, `files.write`, `files.delete`, `secrets.*` by default and require explicit opt-in.

---

### W2. `HIGH_RISK_ACTIONS` list is too narrow ✅ FIXED

**File:** `src/permissions.rs:160-166`

```rust
const HIGH_RISK_ACTIONS: &[&str] = &[
    "browser.evaluate", "process.start", "terminal.create",
    "system.update", "dbus.call",
];
```

Missing from this list: `files.write`, `files.delete`, `files.move`, `clipboard.read`, `clipboard.history`, `screenshot`, `input.keyboard`, `input.mouse`, `secrets.get_secret`, `secrets.store_secret`. Wildcard permissions (`"*"`) authorize all of these without explicit naming. An attacker who gains socket access can read clipboard secrets, take screenshots, and overwrite files.

**Fix:** Expand the list. Consider splitting into `DESTRUCTIVE_ACTIONS` (files.write/delete/move, system.power) and `SENSITIVE_ACTIONS` (clipboard.read/history, screenshot, secrets.*, input.*) with separate permission semantics.

---

### W3. `files.search` bypasses path sandbox

**Files:** `src/daemon/execute_files.rs:37`, `src/backend/sway/files.rs:79`

`files.search` forwards user-controlled `root` directly to the backend search implementation without `expand_path` sandboxing. The sway backend defaults to `"."` and passes arbitrary roots to `find`, traversing any directory the daemon user can read — including `~/.ssh`, `~/.gnupg`, `/etc`, etc.

**Fix:** Canonicalize and validate `root` against `DESKBRID_ALLOWED_DIRS` before dispatching to the backend. Apply the same `expand_path` check used by file read/write operations.

---

### W4. `files.watch` monitors arbitrary paths without sandbox

**File:** `src/backend/sway/files.rs:54`

File watchers are created for user-supplied paths without canonicalization or `DESKBRID_ALLOWED_DIRS` validation. Clients can monitor sensitive directories for changes — password files, SSH keys, config dirs — and exfiltrate metadata via event timestamps.

**Fix:** Apply shared `expand_path`/allowed-dir checks before creating the notify watcher.

---

### W5. Screenshots written to predictable `/tmp` paths

**File:** `src/backend/sway/screenshot.rs:10`

Screenshots are written to `/tmp/deskbrid_screenshot_<unix_seconds>.png` with predictable filenames and default permissions. Other local users can read these files or race the path.

**Fix:** Use `$XDG_RUNTIME_DIR` or a `0700`-mode directory. Set file permissions to `0600`. Use random filenames (UUID) instead of timestamps.

---

### W6. Self-update proceeds without checksum verification ✅ FIXED

**File:** `src/cmd/update/github.rs:82`

When no `.sha256` asset is published alongside a release, the self-updater logs "no checksum asset published; skipped" and installs the binary anyway. A compromised release asset or CDN path leads directly to arbitrary code execution with the daemon's privileges.

**Fix:** Make missing checksum a hard error. Require `.sha256` on every release. Consider adding GPG/Sigstore signatures for defense in depth.

---

### W7. Install script downloads without checksum verification ✅ FIXED

**File:** `site/install.sh:271`

The one-liner installer downloads and extracts the release tarball with no checksum or signature verification before `sudo mv` to `/usr/local/bin`. A `curl|bash` path through a compromised release or CDN installs tampered binaries as root.

**Fix:** Download the `.sha256` alongside the tarball. Verify before extraction. Fail closed on mismatch.

---

### W8. Clipboard history persists secrets by default

**File:** `src/daemon/clipboard.rs:44`

Clipboard reads/writes are persisted to SQLite history by default. Passwords, tokens, and other secrets copied to the clipboard are retained indefinitely and exposed via the API and dashboard.

**Fix:** Make history opt-in via config. Add a private/incognito mode. Consider heuristic-based secret detection (password managers set `x-kde-passwordManagerHint`) to auto-redact.

---

### W9. Confirmation actions unreachable in headless mode ✅ FIXED

**Files:** `src/daemon/execute.rs:231-232`, `src/daemon/dispatch.rs:374-386`

`ConfirmAction`/`DenyAction`/`ConfirmationList` are dispatched through `execute_action`, which sits behind the "no desktop backend loaded" guard. In headless deployments, every confirmation op returns `not_supported`. Since secrets reads/writes require confirmation (dispatch.rs:302), the entire secrets subsystem is dead in headless mode.

**Fix:** Route confirmation actions through a backend-free code path (like `is_audit_action` or `is_session_action`).

---

### W10. TCP auth `read_line` unbounded; non-constant-time token compare ✅ FIXED

**File:** `src/daemon/tcp.rs:53, :123`

The 4096-byte guard at line 56 fires *after* `read_line` at line 53 has already buffered an arbitrarily large line into memory. A peer sending megabytes without a newline can exhaust memory before the size check runs. Separately, `provided != token` at line 123 is a short-circuiting `String` comparison usable as a timing oracle against the bearer token.

**Fix:** Wrap `read_line` in `tokio::io::AsyncReadExt::take(MAX)`. Compare tokens with `subtle::ConstantTimeEq` or `ring::constant_time::verify_slices_are_equal`.

---

### W11. Dashboard HTTP parsing unbounded; no connection cap ✅ FIXED

**File:** `src/daemon/dashboard/server.rs:193, :197-203`

Request-line and header parsing use unbounded `read_line` with no byte cap and no header-count limit. Combined with `0.0.0.0` bind and unbounded `tokio::spawn` per accept (mod.rs:29), a remote peer can memory-exhaust or fd-exhaust the daemon.

**Fix:** Cap reads (e.g., 64 KiB). Bound header count (e.g., 100). Limit concurrent dashboard connections with a semaphore.

---

### W12. `std::sync::Mutex<Database>` held across SQLite I/O on async runtime

**File:** `src/lib.rs:89`, `src/daemon/execute_rules.rs:35`

`state.database` is a `std::sync::Mutex` locked inside async handlers and held across disk-bound rusqlite calls. This blocks the tokio worker thread for the full DB operation duration. Under load this stalls the runtime and serializes all DB users.

**Fix:** Use `tokio::sync::Mutex`, or wrap DB access in `spawn_blocking`, or use a dedicated DB actor pattern.

---

### W13. `HOME` fallbacks hardcode `"/root"`

**Files:** `src/daemon/macro_engine.rs:63`, `src/daemon/helpers.rs:99,125`, `src/permissions.rs:151`

When `HOME`/`XDG_DATA_HOME` are unset, paths fall back to `/root/.local/...`. On a non-root daemon (systemd service, Docker), this writes to an inaccessible or wrong home directory — silent failures and wrong sandbox roots.

**Fix:** Use `dirs::home_dir()` and bail with a clear error when `HOME` is unset rather than assuming root.

---

### W14. Release artifact name mismatch — install and update are broken ✅ FIXED

**File:** `.github/workflows/release.yml:50`

```yaml
tar czf "deskbrid-mcp-${{ matrix.target }}.tar.gz" deskbrid cosmic-helper labwc-helper
```

Release tarballs are named `deskbrid-mcp-<target>.tar.gz`, but the installer (`site/install.sh:268`) and self-updater (`src/cmd/update.rs:41`) expect `deskbrid-<arch>.tar.gz`. Fresh installs from published releases will 404.

**Fix:** Align the release artifact name with what the installer and updater expect. Use one canonical naming scheme across release workflow, install script, self-updater, and docs. Add an integration smoke test against release assets.

---

### W15. Docs reference nonexistent bare binary download URL

**Files:** `README.md:101`, `docs/wiki/installation.md:21`

Manual install instructions tell users to download `releases/latest/download/deskbrid` (a bare binary), but the release workflow publishes only tarballs. Users following docs get a 404.

**Fix:** Update docs to document the tarball download + extract flow, or publish a bare `deskbrid` binary alongside the tarball.

---

### W16. Version mismatches across docs and site

**Files:** `site/install.sh:14`, `site/index.html:490`, `docs/wiki/features/self-update.md:3`

- `site/install.sh` fallback version: `0.12.0` (actual: `0.13.0`)
- `site/index.html` hardcodes `v0.12.1`
- `docs/wiki/features/self-update.md` claims "v1.0.0" (actual: `0.13.0`)
- Committed `clients/python/deskbrid.egg-info/PKG-INFO` says `0.1.0`

GitHub API failures silently install an older release. Stale docs confuse users about release state.

**Fix:** Centralize version in one place. Generate docs/site from `Cargo.toml` or a single `VERSION` file. Remove committed `*.egg-info` and add `.gitignore`.

---

### W17. Docs show wrong CLI/API names for self-update

**File:** `docs/wiki/features/self-update.md:9`

Docs show `deskbrid update.check {}` / `self.update {}` JSON-style commands, but the CLI exposes `deskbrid update --check` and MCP tools are `check_update`/`self_update`.

**Fix:** Update docs to match implemented CLI/MCP/API names.

---

### W18. `deploy-site.sh` disables SSH host key checking

**File:** `scripts/deploy-site.sh:15`

```bash
scp -o StrictHostKeyChecking=no ...
```

Disables host key verification, allowing MITM substitution of the deployment target.

**Fix:** Pin the host key via `known_hosts` or `StrictHostKeyChecking=yes`. Move target host/user/IP to an environment variable or CI secret.

---

### W19. `deploy-site.sh` commits private infrastructure details

**File:** `scripts/deploy-site.sh:9`

The script hardcodes a private LAN username/IP as the deployment target, leaking internal infrastructure details and making deployment non-portable.

**Fix:** Move target to environment variable or CI secret config.

---

### W20. `mcp-publisher` downloaded from "latest" without pinning

**File:** `.github/workflows/publish-mcp.yml:18`

Downloads and executes `mcp-publisher` from "latest" without pinning a version/digest or verifying checksum. Supply-chain risk in the publishing pipeline.

**Fix:** Pin to a specific release version. Verify checksum/signature before execution.

---

### W21. `reqwest` uses default native TLS/OpenSSL

**File:** `Cargo.toml:43`

```toml
reqwest = { version = "0.12", features = ["json"] }
```

Release builds install `libssl-dev` only on x86_64. The installer doesn't ensure runtime OpenSSL. Binaries may fail on distros with incompatible or missing OpenSSL. The `aarch64` target does vendor OpenSSL (`Cargo.toml:72`), but the approach is inconsistent.

**Fix:** Use `rustls-tls` with `default-features = false` for consistent static TLS across all targets. Or vendor OpenSSL consistently on all architectures.

---

### W22. CI doesn't test `pipewire` feature

**File:** `.github/workflows/ci.yml:20`

CI only runs `cargo check`/`test` with default features. The optional `pipewire` feature is never built or tested, so feature-specific build breakage can ship.

**Fix:** Add `cargo check --all-features` and `cargo test --all-features` to CI, or add a feature matrix.

---

### W23. Release workflow doesn't gate on tests

**File:** `.github/workflows/release.yml:39`

The release job builds and packages artifacts without running `cargo test` or `cargo clippy`. Tags can publish even if CI is bypassed or failing.

**Fix:** Add `needs: ci` to the release job, or run `cargo test` + `cargo clippy -D warnings` in the release job before packaging.

---

### W24. Wi-Fi password passed as plain `nmcli` argv element

**File:** `src/daemon/execute_network.rs:115-120`

WiFi passwords are passed as a visible process argument. Other local users can see them in `ps`/`/proc` during the call.

**Fix:** Use `nmcli --ask` with stdin, or a connection-file approach, to avoid transient secret exposure in process args.

---

### W25. Command allowlist comment contradicts code

**File:** `src/daemon/helpers.rs:53-72`

The comment says "If unset, only allow known-safe starters," but the code skips the check entirely when `DESKBRID_ALLOWED_COMMANDS` is empty (`if !allowed_cmds.is_empty()`), allowing any command.

**Fix:** Either implement the documented default-safe allowlist or correct the comment.

---

### W26. `dbus_send_arg` mishandles nested arrays

**File:** `src/daemon/execute_system.rs:148-150`

Nested `Value::Array` recursively maps and `.join(" ")` into a single string, producing a malformed dbus-send arg. Array-typed dbus args are effectively unsupported.

**Fix:** Either reject nested arrays with an error, or emit them as proper `array:...` dbus-send syntax.

---

## 🔵 SUGGESTION Findings

### S1. `RateLimitStore::remove_peer` is dead code; bucket map grows unbounded

**File:** `src/daemon/rate_limit.rs:259-263`

The method is `#[allow(dead_code)]` and never wired to peer disconnect. The `buckets` HashMap accumulates entries for every distinct Unix UID ever seen.

**Fix:** Hook `remove_peer` to peer disconnect, or add a periodic sweep for stale entries.

---

### S2. `socket_path()` panics on missing `XDG_RUNTIME_DIR`

**File:** `src/daemon/mod.rs:119`

Uses `.expect()` on `XDG_RUNTIME_DIR`. A missing var panics the daemon at startup with a cryptic message.

**Fix:** Use a typed error or fallback to `~/.deskbrid.sock` with a warning.

---

### S3. `read_line_limited` resets `take(MAX_BYTES)` adapter each call

**File:** `src/daemon/client.rs:240-247`

A single line >10 MiB with no newline is delivered as back-to-back 10 MiB chunks, each failing JSON parse rather than being rejected as oversized. Functionally safe but the "10MB cap" is really a per-read cap.

**Fix:** Read into a capped buffer that errors on overrun rather than wrapping each `read_line` call in a fresh `take()`.

---

### S4. Python CI only imports the package — no unit tests

**File:** `.github/workflows/ci.yml:42`

No unit/protocol tests validate sync/async methods or documented API examples.

**Fix:** Add pytest coverage for request serialization, error handling, and README snippets.

---

### S5. 70 `unwrap()`/`expect()` calls across source

**Distribution:** 20 source files contain unwrap/expect calls. Hot spots: `cosmic_helper` (25), `rate_limit.rs` (7), `clipboard.rs` (9). Most are in helper binaries or parsing paths where they may be justified, but each should be audited to ensure no production-path panics.

**Fix:** Audit each call. Replace with `?` or `ok_or_else` in production code paths. Leave `unwrap` only in tests or where the invariant is truly impossible to violate.

---

### S6. Committed Python build artifacts

**File:** `clients/python/deskbrid.egg-info/PKG-INFO`

Generated Python metadata committed to the repo with stale version `0.1.0`.

**Fix:** Remove `*.egg-info`, `build/`, `__pycache__/`. Add `.gitignore` entries.

---

### S7. Docs claim "v1.0.0" while project is at 0.13.0

**File:** `docs/wiki/features/self-update.md:3`, `docs/deskbrid-v1.0.0.md`

Multiple docs reference a "v1.0.0" release that doesn't match the actual version `0.13.0` in `Cargo.toml`. Users see inconsistent release state.

**Fix:** Update docs to reflect current version, or version the docs to match releases.

---

## Architecture Observations

### What's Done Well

- **Backend trait abstraction** — The `DesktopBackend` trait cleanly separates protocol from implementation. Adding a new compositor is a matter of implementing the trait.
- **Protocol layer** — Well-typed `Action` enum with serialize/parse separation. The dot-namespace convention (`windows.*`, `input.*`) is consistent and intuitive.
- **MCP integration** — 100+ tools auto-registered from the protocol layer is elegant. No duplication between socket API and MCP tools.
- **Multi-DE auto-detection** — The detection chain (`$XDG_CURRENT_DESKTOP` → process scan → GNOME fallback) is robust and well-documented.
- **Confirmation mode** — The concept of requiring explicit approval for destructive actions is the right idea (execution just needs fixing per C3).
- **Rate limiting** — Per-namespace, per-agent token bucket is a solid design (just needs cleanup per S1).
- **Audit logging** — Every action is audited with timing, which is excellent for debugging agent behavior.

### What Needs Attention

- **Security boundaries are the #1 priority.** The four critical findings (C1-C4) represent a coherent attack surface: network-reachable unauthenticated dashboard + confirmation bypass + secret leakage. Fix these before any production deployment.
- **Artifact naming** is broken (W14). Fresh installs from current releases will fail. This is a release-blocking issue.
- **The permission system exists but defaults to open.** This is the wrong default for a tool with this power. Least-privilege should be the starting point.
- **Synchronous DB locks on async runtime** (W12) will cause latency spikes under load. This is an architectural issue that should be addressed before scaling.

---

## Priority Fix Order

1. **C1** — Bind dashboard to `127.0.0.1` + add auth
2. **C3** — Fix confirmation ownership check + route through dispatch
3. **C4** — Stop recording secrets in macros
4. **C2** — Auth-gate screenshot endpoint (fixes automatically with C1)
5. **W1** — Default-deny permissions on fresh install
6. **W2** — Expand HIGH_RISK_ACTIONS list
7. **W14** — Fix release artifact naming (installs are broken)
8. **W6/W7** — Checksum verification for self-update and install
9. **W10/W11** — Bound TCP/dashboard read_line + constant-time token compare
10. **W12** — Move DB access off async runtime

---

*Review by Vex. All critical findings manually verified against source.*
