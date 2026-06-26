# Codex Code Review

Date: 2026-06-26
Scope: Rust daemon, protocol/permission layer, MCP server wrappers, Python client bindings, and file-watch lifecycle. Documentation and generated build artifacts were not reviewed deeply except where they exposed protocol drift.

## Findings

### 1. High: MCP tools bypass the normal permission, confirmation, audit, and rate-limit path

Several MCP wrappers call lower-level executors directly instead of going through `dispatch_action_with_options`. The split is visible in `src/mcp/helpers.rs`: `do_execute` and `do_execute_with` check permissions/rate limits before calling `execute_action` (`src/mcp/helpers.rs:7-57`), but many other helpers call `execute_action` or specialized executors directly with no policy check at all (`src/mcp/helpers.rs:59-150`, `src/mcp/helpers.rs:153-215`, `src/mcp/helpers.rs:230-259`).

Concrete examples:

- `type_text`, `press_keys`, `mouse_click`, `click_coordinate`, and `drag` use direct helpers (`src/mcp/tools_input.rs:15-17`, `src/mcp/tools_input.rs:43-45`, `src/mcp/tools_input.rs:71-76`, `src/mcp/tools_input.rs:105-132`). These execute keyboard/mouse actions even if `permissions.toml` denies `input.keyboard` or `input.mouse`.
- Secret tools call `execute_secrets_action` directly (`src/mcp/tools_secrets.rs:14-20`, `src/mcp/tools_secrets.rs:33-45`, `src/mcp/tools_secrets.rs:58-81`; duplicated in generated `src/mcp/server.rs:1500-1567`). That bypasses the dispatcher confirmation gate that explicitly requires confirmation for secret reads/writes (`src/daemon/dispatch.rs:347-360`).
- Accessibility mutation helpers (`perform_action`, `set_element_value`, `click_element`) also bypass permission checks (`src/mcp/helpers.rs:178-206`).

Impact: MCP clients can perform sensitive desktop actions despite a denying permission policy. Secret reads/writes are especially severe because the tool descriptions promise confirmation, but the code returns or stores secrets immediately once `secret-tool` succeeds.

Recommendation: make MCP wrappers construct an `Action` and route every call through one central MCP dispatch helper that invokes `dispatch_action_with_options` with a synthetic MCP UID/session. Remove direct executor calls from tool wrappers. Add regression tests with deny-all permissions proving that MCP `type_text`, `click_coordinate`, a11y mutations, and `secrets_get_secret` are denied or confirmation-gated.

### 2. High: MCP `check_update` can perform a destructive self-update because `do_execute` ignores arguments

`do_execute` accepts `_args` but discards them when building the action JSON (`src/mcp/helpers.rs:7-14`). Some MCP tools pass important arguments through that discarded parameter. The worst case is `check_update`, which is marked read-only and calls:

```rust
do_execute(&self.state, "system.update", json!({"check": true}))
```

at `src/mcp/tools_system.rs:56-58`.

Because `{"check": true}` is ignored, the parsed action becomes `SystemUpdate { check: false, force: false }`. `run_json(false, false)` downloads and replaces the binary whenever a newer release exists (`src/cmd/update.rs:22-44`). So an MCP caller with explicit `system.update` permission can invoke the read-only `check_update` tool and trigger an actual update.

Impact: a tool advertised as a harmless update check can replace the running binary and restart the user service. This violates tool annotations, surprises users, and makes MCP permission review harder.

Recommendation: delete `do_execute` or make it merge args the same way `do_execute_with` does. Prefer a single dispatch wrapper. Add a unit/integration test asserting MCP `check_update` serializes `check: true` and never reaches the install branch.

### 3. High: Python TCP transport hangs after successful authentication

The daemon TCP path authenticates, then immediately hands the stream to the generic client handler (`src/daemon/tcp.rs:142-144`). The generic handler sends the normal `connected` frame (`src/daemon/client.rs:44-52`). It does not send a separate auth-success frame.

The Python client, however, treats the first post-auth line as an auth response (`clients/python/deskbrid/async_client.py:106-115`) and then `connect()` waits for another `connected` frame (`clients/python/deskbrid/async_client.py:72-75`). On valid auth, the client consumes the only `connected` frame inside `_connect_tcp`, returns, then hangs waiting for a second one.

Impact: `AsyncDeskbrid(tcp_port=..., tcp_token=...)` cannot connect reliably over TCP even with the correct token.

Recommendation: either have the server send an explicit auth-ok frame before entering the generic protocol, or change `_connect_tcp` to return the first non-error frame to `connect()` instead of consuming it. Add a Python async test with a fake TCP server that sends only the daemon's current post-auth `connected` frame.

### 4. Medium: TCP and MCP TCP auth readers can drop pipelined client messages

Both TCP auth paths create a temporary `BufReader`, call `read_line`, drop the buffer, then recover the inner reader (`src/daemon/tcp.rs:54-59`, `src/mcp/server.rs:2415-2423`). `BufReader` is allowed to read past the newline into its internal buffer. If a client sends the auth line and its first protocol request in the same packet, bytes after the auth newline can be buffered and then discarded when `BufReader` is dropped.

Impact: first client requests can disappear nondeterministically depending on packet coalescing and buffering. This is difficult to debug because waiting between auth and the first request hides the bug.

Recommendation: keep the `BufReader` alive and pass it into the protocol handler, or preserve buffered bytes with a framed transport. Add a test that writes `auth\nrequest\n` in a single write and asserts the first request is processed.

### 5. Medium: File watchers leak after client disconnect and unwatch uses a different path key than watch

The per-client state tracks watched paths (`src/lib.rs:235-254`), and `FilesWatch`/`FilesUnwatch` update that set (`src/daemon/client.rs:205-214`). On disconnect, cleanup only removes rate-limit buckets (`src/daemon/client.rs:231-237`); no watched paths are unwatched. Backends store actual `notify` watcher objects, so orphaned watches can survive until daemon exit.

There is also a key mismatch: `FilesWatch` expands/canonicalizes the path before registering the backend watcher (`src/daemon/execute_files.rs:19-28`), while `FilesUnwatch` passes the original caller path directly (`src/daemon/execute_files.rs:30-32`). A watch created with `~/dir`, a relative path, or a symlinked path may not be removed by an unwatch using the same user-facing input, because backends key the watcher by the registered path (`src/backend/hyprland/files.rs:42-44`).

Impact: disconnected clients can leave live filesystem watchers behind, consuming resources and continuing to emit file events. Explicit unwatch can silently fail for non-canonical path forms.

Recommendation: store canonical watch keys in `ConnectionState`, canonicalize `FilesUnwatch`, and on disconnect iterate the connection's watched paths and call backend `files_unwatch` for each. Add tests for `~/path` watch/unwatch and disconnect cleanup.

### 6. Medium: Protocol names drift across daemon, Python client, docs, and capabilities

The Python backlight bindings send `system.backlight.get` and `system.backlight.set` with a `percent` field (`clients/python/deskbrid/actions_async.py:383-397`). The daemon parser accepts `system.backlight_get` and `system.backlight_set`, and set expects `value` (`src/protocol/parse/system.rs:38-44`). The capabilities code also annotates the dotted names (`src/daemon/capabilities/mod.rs:123-128`), while the serializer/action type uses underscored names.

Impact: Python backlight APIs fail with `unknown action type`, and capability reports can describe actions that the daemon does not parse. Users following docs that mention the dotted names will hit the same failure.

Recommendation: choose one canonical spelling, support aliases during a deprecation window, and add protocol round-trip tests that compare parser, serializer, public action list, client bindings, and docs examples for each public action.

### 7. Low: Public action lists and default permissions are missing newer or renamed actions

`Action::public_action_types()` lists only the original network actions through `network.wifi.connect` (`src/protocol/action_impl.rs:106-109`), but the action serializer exposes many more network actions such as `network.connections.list`, hotspot, DNS, WWAN, and VPN (`src/protocol/serialize/action_type.rs:112-121`). Capability/tool discovery built from `public_action_types()` will under-report supported network functionality.

Default safe permissions also include `input.layouts.*` (`src/permissions.rs:120-129`), but actual layout actions are split between `input.layouts.list` and `input.layout.get/set/add/remove` (`src/protocol/action_impl.rs:26-30`, `src/protocol/serialize/action_type.rs:28-32`). Fresh installs therefore allow listing layouts but deny get/set/add/remove unless users add more explicit permissions.

Impact: discovery and default policy behavior are inconsistent with implemented functionality.

Recommendation: generate `public_action_types()` from a single source of truth or add a test comparing it with all serialized action variants. Add `input.layout.*` or explicit layout action names to safe defaults if layout mutation is intended to be allowed.

## Verification

- The findings above were addressed in the follow-up fix set.
- `cargo fmt --check` passed.
- `cargo test` passed: 144 library tests, 1 integration test, and doctests completed successfully.
- `cargo clippy --all-targets --all-features -- -D warnings` passed.
- `python3 -m compileall deskbrid` passed for the Python client package.
- Python pytest tests were not run because `python3 -m pytest tests -q` failed with `No module named pytest`.

## Suggested Regression Tests

- MCP permission tests under deny-all/default-deny policies for every tool wrapper that currently uses `self.call(do_...)`.
- MCP secret tests proving `secrets_get_secret` and `secrets_store_secret` return `CONFIRMATION_REQUIRED` unless the normal confirmation flow is used.
- MCP `check_update` test proving `check: true` reaches `Action::SystemUpdate`.
- TCP client handshake test for the Python client.
- TCP pipelining test that sends auth and first request in one write.
- File watch lifecycle test for canonicalized paths and disconnect cleanup.
- Protocol consistency test comparing parser names, serializer names, public action list, Python client action strings, and docs examples.
