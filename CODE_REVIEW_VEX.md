# Deskbrid — Deep Code Review

**Reviewer:** Vex  
**Date:** 2026-05-23  
**Version:** 0.8.0 (edition 2024)  
**Scope:** Full codebase — ~300+ Rust source files, ~45 docs, shell scripts, Python client

---

## Executive Summary

Deskbrid is a Rust daemon that bridges AI agents to Linux desktops via Unix socket, providing window management, input injection, file operations, browser CDP control, and more across 9 desktop environments (GNOME, KDE, Hyprland, X11, Sway, Niri, Wayfire, Labwc, COSMIC).

**The architecture is well-designed.** Backend abstraction via `DesktopBackend` trait, clean protocol layer, proper async throughout. The 9 backend implementations are impressively complete.

**The security posture needs hardening.** The daemon grants unrestricted desktop control (process execution, file I/O, keystroke injection, browser JS eval) but defaults to allow-all with no socket permission enforcement. The `deploy-site.sh` script contains a plaintext password. Several injection vectors exist in the browser CDP layer.

---

## Severity Breakdown

| Severity | Count | Summary |
|----------|-------|---------|
| 🔴 CRITICAL | 16 | Must fix before any production/external use |
| 🟡 WARNING | 24 | Should fix — security and correctness risks |
| 🔵 SUGGESTION | 20 | Consider — quality, maintainability, performance |

---

## 🔴 CRITICAL Findings

### C1. `src/permissions.rs:34-63` — Default allow-all on missing/invalid config

When no `permissions.toml` exists **or** the file fails to parse, the system falls back to `allow_all()` — permitting every action including `process.start`, `files.delete`, `browser.evaluate`, `input.*`. A corrupted or deleted config silently degrades to full access.

**Fix:** Default to deny-all when parsing fails. Only allow-all when explicitly configured.

### C2. `src/daemon/mod.rs:98-101` — Hardcoded UID 1000 in socket path fallback

```rust
.unwrap_or_else(|_| "/run/user/1000/deskbrid.sock".into())
```

Fails for any user not at UID 1000. More critically, if the parent directory doesn't exist, `create_dir_all()` at line 110 may create world-readable directories.

**Fix:** Use `/run/user/{uid}` with `std::process::id()` → `/proc/self/status` UID lookup, or fail if `XDG_RUNTIME_DIR` is unset (it's always set on modern Linux).

### C3. `src/daemon/mod.rs:107-113` — TOCTOU race + no socket permission enforcement

```rust
let _ = tokio::fs::remove_file(&sock).await;  // line 107
let listener = UnixListener::bind(&sock)?;     // line 113
```

Two issues:
1. Classic TOCTOU — attacker can symlink between remove and bind
2. No `chmod 0600` after bind — socket uses process umask (often 0644), allowing any local user to connect

**Fix:** Use `bind()` then `std::fs::set_permissions(sock, 0o600)`. Or use `UnixListener::bind` with socket addr that has `sun_path` set correctly. Consider `abstract` socket namespace (no filesystem race).

### C4. `src/daemon/helpers.rs:43-68` — Arbitrary command execution with zero validation

`spawn_detached_process()` passes `command`, `workdir`, and `env` directly to `Command::new()` with no sanitization, no allowlist, no path restriction. Combined with allow-all default permissions, any local user who can reach the socket can execute anything.

**Fix:** Add configurable command allowlist. At minimum, restrict working directories to `$HOME` and below. Validate `env` keys against dangerous ones (`LD_PRELOAD`, `PATH`).

### C5. `src/daemon/execute_files.rs:46-153` — No path traversal protection

`expand_path()` (`helpers.rs:73-81`) only does tilde expansion — no `canonicalize()`, no symlink resolution, no `../` rejection. Combined with `FilesWrite` (creates parent dirs + writes anywhere) and `FilesDelete` with `recursive: true` (calls `remove_dir_all`), this allows reading/writing/deleting any file on the system.

**Fix:** Canonicalize paths after expansion. Add configurable path sandbox (`allowed_dirs`). Reject paths containing `../` after canonicalization.

### C6. `src/browser.rs:109-111` — CSS selector injection → arbitrary JS execution

```rust
format!("document.querySelector('{}')", selector.replace('\'', "\\'"))
```

The single-quote escaping is insufficient. A crafted selector can break out of the string context and execute arbitrary JavaScript.

**Fix:** Use JSON serialization for the selector — pass it as a proper CDP `Runtime.evaluate` parameter, not string-interpolated.

### C7. `src/browser.rs:186-203` — Incomplete JS string escaping in `set_text()`

Manual escaping of `\`, `'`, `\n`, `\r` misses `</script>`, backticks, unicode escapes, and template literals. Crafted input can break out and execute arbitrary JS.

**Fix:** Use `serde_json::to_string()` for the text value, then inject the JSON string literal into the JS expression.

### C8. `src/browser.rs:42-72` — Unrestricted JS execution via `evaluate()`

`browser.evaluate` runs arbitrary JavaScript in any browser tab with no sandboxing. Cookies, localStorage, auth tokens, and credentials are all accessible. Combined with allow-all defaults, this is a credential theft vector.

**Fix:** This is by design, but it should be behind a strict permission gate. Require explicit per-tab authorization. Consider an allowlist of JS patterns or a sandboxed execution mode.

### C9. `src/daemon/terminal_create.rs:26-29` — Arbitrary binary execution via `shell` parameter

The `shell` parameter from the protocol is passed directly to `Command::new(&shell)` with no validation. An attacker can execute any binary, including setuid binaries.

**Fix:** Validate `shell` against a known list (`/bin/bash`, `/bin/zsh`, `/bin/sh`, `/bin/fish`). Or resolve and check it's in `/bin/` or `/usr/bin/`.

### C10. `scripts/deploy-site.sh:8` — Plaintext password in git-tracked file

```
NUC_PASS="[REDACTED]"
```

A literal SSH/sudo password committed to the repository. Used with `sshpass -p` on line 14 and piped to `sudo -S` on line 16. This is an immediate credential leak.

**Fix:** Remove from git immediately. Rotate the password. Use SSH keys + `sudo` with NOPASSWD for the specific command, or use environment variables / secret manager.

### C11. `scripts/deploy-site.sh:14` — `StrictHostKeyChecking=no`

Disables SSH host key verification, enabling MITM attacks on the deployment.

**Fix:** Pre-populate `known_hosts` or use `StrictHostKeyChecking=accept-new`.

### C12. `src/permissions.rs:164-177` — `getsockopt` doesn't validate returned `len`

After `getsockopt` returns, `len` is not verified to still equal `size_of::<libc::ucred>()`. If the kernel returns fewer bytes, `cred` is partially uninitialized. This is a potential info leak via `cred.uid`.

**Fix:** After `getsockopt`, assert `len == std::mem::size_of::<libc::ucred>()`.

### C13. `src/main.rs:24-27` — `unsafe { std::env::set_var() }` is UB in Rust 2024 edition

The SAFETY comment says "called at startup before threads are spawned" but `#[tokio::main]` has already spawned the runtime at this point. Other threads exist.

**Fix:** Set the env var before the `#[tokio::main]` attribute, or use a runtime builder with `on_enter`, or reinitialize the tracing subscriber instead.

### C14. `src/mcp/server.rs:44` — `rt.block_on()` inside async context risks deadlock

`block()` and `execute()` call `rt.block_on()` from within what may already be a tokio runtime context (rmcp calls tool handlers from its own async runtime). Can panic or deadlock.

**Fix:** Use `tokio::runtime::Handle::current()` or restructure to avoid nested `block_on`.

### C15. `src/mcp/mod.rs:137-138` — MCP `tools/call` without `initialized` check

The `tools/call` handler checks `if initialized` but the fallback `_` handler does NOT. A client can bypass initialization by sending an unknown method.

**Fix:** Always check initialization in the fallback handler.

### C16. `src/daemon/client.rs:70` — No line length limit on `read_line`

```rust
result = reader.read_line(&mut line) => {
```

Reads until `\n` with no size limit. A malicious client could send a multi-gigabyte line to exhaust memory.

**Fix:** Add a byte limit (e.g., 10MB). Use `reader.take(MAX_BYTES).read_line()` or manual buffered reading.

---

## 🟡 WARNING Findings

### W1. `src/daemon/client.rs:12` — Peer UID fallback to `u32::MAX`

When `socket_peer_uid()` fails, `peer_uid` becomes `u32::MAX`. With allow-all default, the connection proceeds unrestricted. Even with per-UID rules, `u32::MAX` likely has no specific deny rules.

### W2. `src/daemon/rate_limit.rs:26-32` — Rate limit disable via environment variable

`DESKBRID_RATE_LIMIT_PER_SEC=0` disables rate limiting entirely. An attacker who can influence the daemon's environment bypasses all rate protection.

### W3. `src/daemon/rate_limit.rs:68-75` — Per-UID rate limit bypassed by multiple connections

Rate limiting is per peer UID. A user can open multiple simultaneous connections, each with their own bucket.

### W4. `src/daemon/helpers.rs:73-81` — `expand_path` doesn't canonicalize

Tilde expansion only. Symlinks and `../` not resolved. See C5 for exploitation details.

### W5. `src/daemon/execute_process.rs:48-56` — `ensure_safe_pid` only blocks PID ≤ 1

Doesn't protect against killing critical system processes (PID 2 kthreadd, etc.). Should exclude well-known kernel PIDs.

### W6. `src/daemon/execute_process.rs:88-114` — Busy-poll with 100ms sleep

`ProcessWait` calls `libc::kill(pid, 0)` every 100ms. Burns CPU for long-lived waits. Use `waitpid` with `WNOHANG` or inotify on `/proc/{pid}`.

### W7. `src/daemon/execute_stubs.rs:181-185` — External HTTP for geolocation

`ip_geo_lookup()` makes unconditional HTTP GET to `https://ipapi.co/json/` — leaks host IP and network info to third-party.

### W8. `src/daemon/execute_network.rs:16-21` — WiFi password in plaintext

WiFi credentials flow through the Unix socket as JSON, visible in audit logs and process memory.

### W9. `src/daemon/execute_screenshot.rs:13-23` — Screenshots return base64 to any client

With allow-all default, any local user can capture any monitor. Passwords, private messages, etc.

### W10. `src/daemon/terminal_create.rs:73-75` — Arbitrary environment variable injection

`LD_PRELOAD`, `PATH`, and other dangerous env vars can be set on spawned terminals.

### W11. `src/browser/cdp.rs:19-57` — CDP endpoint on predictable ports

Tries ports 9222 and 9229 first. An attacker who binds first could intercept CDP connections.

### W12. `src/backend/hyprland/input.rs:36-46` / `src/backend/gnome/input.rs:71-84` — Double `std::sync::Mutex::lock().unwrap()` in async

Lock → read → lock → write in same scope. A panic between the two locks causes deadlock on `std::sync::Mutex` (not poisoning, deadlock because the first lock is still held).

### W13. `src/abs_pointer.rs:90,104,113,131` — Blocking `std::thread::sleep` in async context

`click_at`, `drag` use blocking sleep in async MCP tool handlers. Should use `tokio::time::sleep` in `spawn_blocking`.

### W14. `src/abs_pointer.rs:178-184` — Unknown button defaults to BTN_LEFT

Silent fallback means invalid input causes unexpected left clicks.

### W15. `src/backend/gnome/screenshot.rs:14` — Temp file path collision

`/tmp/deskbrid_screenshot_{timestamp}.png` uses second-resolution timestamps. Rapid calls overwrite each other.

### W16. `src/backend/gnome/inner.rs:161-166` — Hardcoded Bluetooth adapter `hci0`

Multi-adapter systems can't find devices on `hci1+`.

### W17. `src/backend/hyprland/core.rs:13` — Hardcoded `/run/user/1000` fallback

Same pattern as C2 but in backend code.

### W18. `src/backend/gnome/screenshot.rs:62-70` — `get_png_dimensions` reads entire file

`std::fs::read(path)?` loads entire PNG just for 24 bytes of header. Use `File::open` + `Read::read_exact`.

### W19. `src/daemon/terminal.rs:97-116` — Unbounded terminal write

`write_terminal()` writes entire input string to PTY with no size limit.

### W20. `src/protocol/parse/files.rs:7-51` — Empty path allowed

`unwrap_or("")` for path fields. Empty path on `files.delete` attempts to delete cwd.

### W21. `src/protocol/parse/process.rs:9-17` — Empty command array allowed

`unwrap_or_default()` produces empty Vec. Should be caught at parse time.

### W22. `src/mcp/tools.rs:94-98` — Unvalidated tool name passed to executor

Fallback handler passes arbitrary tool names as action types. Could expose unintended functionality.

### W23. `src/a11y.rs:20-46` — No hard cap on total tree nodes

BFS queue grows without bound. With 50 children × 10 depth = potentially billions of nodes.

### W24. `Cargo.toml:17` — `tokio` with `features = ["full"]`

Unnecessary attack surface and compilation bloat. Only a subset is needed.

---

## 🔵 SUGGESTION Findings

### S1. `Cargo.toml` — No `[profile.release]` section

Missing `strip = true`, `lto = true`, `panic = "abort"`. Release binary ships with debug symbols and unnecessary unwinding code.

### S2. `Cargo.toml:50` — `notify` enables `macos_kqueue` on Linux-only daemon

Unnecessary feature flag.

### S3. `src/permissions.rs:104` — Default deny is correct when file exists but no pattern matches

Good design. The `allow_all()` default is the issue (see C1).

### S4. `src/daemon/helpers.rs:69` — `child.id().unwrap_or(0)` returns PID 0 on failure

PID 0 is misleading. Return an error instead.

### S5. `src/lib.rs:74-76` — `AtomicU32::fetch_add` with `Ordering::Relaxed` for IDs

Should be `SeqCst` for identifiers that may be persisted.

### S6. `src/mcp/types.rs` — No range validation on `SetVolume.volume`

Negative volumes or volumes > 1.0 passed directly to backend.

### S7. `src/mcp/tool_list.rs` — Schema drift risk with `tools.rs`

Manually maintained tool definitions in two places. Consider single source of truth.

### S8. `src/mcp/mod.rs` — Two independent MCP server implementations

One hand-written JSON-RPC, one rmcp-based. Duplication risks divergence.

### S9. `src/backend/mod.rs:21-53` — GNOME fallback for unknown desktops

Unknown DE (i3wm, etc.) falls back to GNOME backend which will fail. Return error instead.

### S10. `src/backend/gnome/input.rs:99-108` — Float-to-int truncation without rounding

`dy as i32` truncates. A scroll of 0.7 becomes 0 (no scroll). Should round first.

### S11. `src/backend/gnome/system.rs:128-132` — Battery time_remaining overflow

`((energy / energy_rate) * 60.0) as u32` can overflow if `energy_rate` is tiny. Add `.min()`.

### S12. `src/backend/kde/io.rs:56` — `ev.take().unwrap()` can panic

Defensive coding would use `if let Some(ev)`.

### S13. `src/backend/cosmic/workspaces.rs:62` — `std::sync::Mutex` in async context

Consider `tokio::sync::Mutex` for async safety.

### S14. All backends — Inconsistent `_follow` parameter handling

GNOME ignores `follow` param in `workspace_move_window`. Document whether it's best-effort.

### S15. `src/a11y/bus.rs:116-135` — `child_path` swallows D-Bus errors silently

Returns `None` on permission errors or bus failures. Tree traversal skips children without logging.

### S16. `src/client.rs:30-45` — Event forwarder task never cleaned up

Spawned task leaks when client disconnects. Add cancellation via `CancellationToken` or drop guard.

### S17. `clients/python/deskbrid/async_client.py:15` — Hardcoded UID 1000 fallback

Should use `os.getuid()` for `/run/user/{uid}`.

### S18. `clients/python/deskbrid/async_client.py:197-198` — Size limit checked post-read

1 MiB limit checked after full `readline()` completes. Already in memory by then.

### S19. `site/install.sh:237` — Binary downloaded without integrity check

No SHA256 or GPG signature verification on the tarball.

### S20. `site/install.sh:213` — `chmod 666 /dev/uinput` suggestion

World-writable uinput allows any user to inject input events.

---

## Architecture Observations

### What's Good

1. **Backend abstraction** — `DesktopBackend` trait is clean. All 9 backends implement the full interface.
2. **Protocol layer** — Parse/serialize split is well-organized. JSON protocol is simple and debuggable.
3. **Async throughout** — Proper use of tokio. No blocking syscalls in the hot path (except `abs_pointer.rs` sleep calls).
4. **Permissions system** — When a config file exists, the deny-first + glob matching design is solid.
5. **Unsafe blocks** — Only 19 instances, all for legitimate libc interop. No unchecked array access or transmute abuse.
6. **Error handling** — Consistent `anyhow` usage. No `unwrap()` in production paths (except `Mutex::lock()`).

### What Needs Attention

1. **Two MCP servers** — `mcp/mod.rs` (hand-rolled) and `mcp/server.rs` (rmcp). These will diverge.
2. **Backend code duplication** — Sway, Niri, Wayfire, Labwc share nearly identical wlroots patterns. Extract shared module.
3. **`std::sync::Mutex` vs `tokio::sync::Mutex`** — Mixed usage. Backends use `std::sync::Mutex`, daemon state uses `tokio::sync::Mutex`. The `.unwrap()` on all lock acquisitions is risky (poisoning panic on panic).
4. **No test coverage visible** — `#[cfg(test)]` modules exist but coverage is unknown. Process execution, file ops, browser CDP, and permissions all need integration tests.

---

## Priority Fix Order

1. **Remove `NUC_PASS` from `deploy-site.sh`** — rotate credential immediately
2. **Permissions: deny-all default** — change `allow_all()` to deny-all on parse failure
3. **Socket: `chmod 0600`** — restrict to owner only after bind
4. **Path sandboxing** — canonicalize + restrict to allowed dirs in `expand_path()`
5. **Command allowlist** — restrict `process.start` executables
6. **Browser: fix JS injection** — use JSON serialization, not string formatting
7. **Terminal: validate `shell` parameter** — allowlist known shells
8. **Line length limit** — cap `read_line()` at 10MB
9. **Add `[profile.release]`** — `strip = true`, `lto = true`
10. **Add checksum verification** to `install.sh`

---

*End of review. — Vex*
