# Contributing to Deskbrid

First off — thank you for considering a contribution. Deskbrid aims to be the universal HAL for Linux desktop automation agents, and every backend port, bug fix, and protocol improvement moves that needle.

## Quick Start

For build and run instructions, see the [README](../README.md). This guide covers development workflow, project conventions, and how to add a new backend.

## Project Structure

```
src/
├── main.rs             CLI entry: daemon (default), setup, install
├── lib.rs              DaemonState, ConnectionState (shared state structs)
├── daemon.rs           Unix socket listener, client handler, action dispatch
├── protocol.rs         Action enum, DeskbridEvent enum, parse/serialize
├── cli.rs              CLI argument parsing with clap
├── client.rs           Sync TCP client for manual testing
├── permissions.rs      Per-UID permission system (SO_PEERCRED + TOML)
├── setup.rs            `deskbrid setup` — auto-detect + install deps
├── capture.rs          Screenshot fallback (portal)
├── backend/
│   ├── mod.rs          DesktopBackend trait, DesktopEnv, desktop detection
│   ├── gnome.rs        GNOME backend (1,853 lines — reference backend)
│   ├── hyprland.rs     Hyprland backend (822 lines)
│   ├── kde.rs          KDE backend
│   └── x11.rs          X11 backend (xdotool + xclip + ImageMagick)
└── extensions/
    └── deskbrid@deskbrid/
        ├── extension.js    GNOME Shell extension
        └── metadata.json   Extension manifest

clients/
└── python/
    ├── setup.py
    └── deskbrid/
        ├── __init__.py     Re-exports
        ├── client.py       AsyncDeskbrid + sync Deskbrid wrapper
        ├── models.py       Pydantic response models
        └── events.py       Event subscription manager

deploy/
└── deskbrid.service    systemd user service unit
```

## Build from Source

```bash
git clone https://github.com/coe0718/deskbrid
cd deskbrid

# Debug build (fast iteration)
cargo build

# Release build (optimized)
cargo build --release

# Run the daemon with tracing
RUST_LOG=debug cargo run
```

The daemon must be run inside a desktop session so it can detect the compositor and connect to D-Bus.

### Python Client

```bash
cd clients/python
pip install -e .
```

## Running Tests

```bash
# All tests (unit tests only — no integration tests exist yet)
cargo test

# Test a specific module
cargo test -p deskbrid -- permissions::tests

# Verbose (useful when adding permission rules)
cargo test -- --nocapture
```

Tests are located in two places:

- **`src/permissions.rs`** (lines 238–441): 10 tests covering glob matching (`glob_match`), allow-all permissions, deny-list precedence, per-UID overrides, default-deny fallback, and action name mapping.
- **`src/protocol.rs`** (lines 1065–1083): 2 tests covering Action round-trip serialization and known-action count.

Write new tests as `#[cfg(test)] mod tests { ... }` inline in the source file — the project doesn't have a separate `tests/` directory yet.

### Test conventions

- Each test is a standalone `#[test]` function — no shared fixtures
- Use `assert!` / `assert_eq!` / `assert_ne!` (no `assert_matches!` yet)
- Permission tests use hand-constructed `PermissionsInner` structs (see `test_permissions_deny_screenshot` for the pattern)
- No async tests currently — all tests are synchronous

## Debugging

Enable tracing with `RUST_LOG`:

```bash
RUST_LOG=debug cargo run
RUST_LOG=trace cargo run    # Very verbose — includes every NDJSON line
```

The daemon logs:
- Connection accept/reject with UID from `SO_PEERCRED`
- Every dispatched action with its sequence number
- Permission check results
- Backend operation errors (full `anyhow` error chain)

For the Python client:

```python
import logging
logging.basicConfig(level=logging.DEBUG)
```

## Coding Conventions

### Rust

- **Edition**: 2024 (see `Cargo.toml`)
- **Formatting**: `cargo fmt` — use `rustfmt` with default settings
- **Linting**: `cargo clippy` — run before every PR
- **Error handling**: `anyhow::Result` for backend trait methods; `anyhow::bail!()` for early returns
- **Async**: `tokio` runtime, `async fn` everywhere. The `async-trait` crate is used because Rust doesn't yet support `async fn` in traits natively
- **Naming**: snake_case for functions/variables, CamelCase for types, SCREAMING_SNAKE for constants
- **Match arms**: exhaustive matches on `Action` — the compiler enforces completeness via `#[deny(unreachable_patterns)]` on `execute_action()`
- **Avoid**: unsafe blocks beyond the two controlled uses (`libc::kill` for PID operations, `libc::getsockopt` for `SO_PEERCRED`)
- **Tracing**: `tracing::debug!` for per-action logging, `tracing::info!` for lifecycle events, `tracing::warn!` for recoverable errors

### Protocol

- New action types must be dot-separated lowercased names (e.g., `media.play_pause`)
- Add the action name to `Action::public_action_types()` return list
- Add the parse path in `Action::from_json()`
- Add the `ActionName` trait mapping in `permissions.rs::action_name()` 
- Wire the dispatch in `daemon.rs::execute_action()`

### Python

- **Formatting**: `ruff format`
- **Linting**: `ruff check`
- **Type annotations**: full type hints — the SDK uses `from __future__ import annotations` and Python 3.10+ union syntax (`str | None`)
- **Async-first**: `AsyncDeskbrid` is the primary implementation; `Deskbrid` (sync) is a thin wrapper over `_LoopThread`
- **DeskbridError**: custom exception with `code` and `message` fields matching protocol error codes

## How to Add a New Backend

Deskbrid is designed to make this straightforward. Here's the process:

### 1. Add the enum variant

In `src/backend/mod.rs`, add your compositor to `DesktopEnv`:

```rust
pub enum DesktopEnv {
    Gnome,
    Hyprland,
    Kde,
    X11,
    Sway,          // ← new
}
```

And add its detection in `detect_desktop()`:

```rust
if lower.contains("sway") { return DesktopEnv::Sway; }
```

### 2. Create the backend file

Create `src/backend/sway.rs`. It must:

```rust
use async_trait::async_trait;
use crate::protocol::{self, ScreenshotResult, Region, WindowInfo};
// ...

pub struct SwayBackend {
    event_tx: broadcast::Sender<DeskbridEvent>,
}

impl SwayBackend {
    pub async fn new(event_tx: broadcast::Sender<DeskbridEvent>) -> anyhow::Result<Self> {
        Ok(Self { event_tx })
    }
}

#[async_trait]
impl DesktopBackend for SwayBackend {
    // Implement all trait methods
    async fn windows_list(&self) -> anyhow::Result<Vec<WindowInfo>> { ... }
    async fn window_focus(&self, id: &str) -> anyhow::Result<()> { ... }
    // ... 30+ methods
}
```

### 3. Wire detection → backend

In `src/backend/mod.rs`, add the match arm:

```rust
DesktopEnv::Sway => {
    let backend = crate::backend::sway::SwayBackend::new(state.event_tx.clone()).await?;
    Box::new(backend)
}
```

### 4. Add system health checks

Add your backend's dependency checks in `build_system_health()`:

```rust
} else if desktop.contains("sway") {
    deps.insert("swaymsg".to_string(), check_in_path("swaymsg"));
    deps.insert("grim".to_string(), check_in_path("grim"));
    deps.insert("wl_clipboard".to_string(), check_clipboard_tools());
}
```

### 5. Add system capabilities

Add capabilities annotations in `build_system_capabilities()` if needed:

```rust
if desktop.contains("sway") {
    set_requires(&mut actions, "windows.list", &["swaymsg"]);
    set_requires(&mut actions, "windows.focus", &["swaymsg"]);
}
```

### 6. Add backend notes

```rust
"sway": "window control via swaymsg IPC"
```

### 7. Update setup

Add a `setup_sway()` variant in `src/setup.rs` if automated dependency installation makes sense.

### 8. Regenerate capabilities

That's it — the protocol layer, dispatcher, permissions, and health checks all work automatically. Run `cargo build` to verify compilation. The `capabilities.list` action will immediately reflect the new backend.

## The `DesktopBackend` Trait

Full trait defined at `src/backend/mod.rs:91-174`:

```rust
#[async_trait]
pub trait DesktopBackend: Send + Sync {
    // Windows
    async fn windows_list(&self) -> anyhow::Result<Vec<WindowInfo>>;
    async fn window_focus(&self, id: &str) -> anyhow::Result<()>;
    async fn window_get(&self, id: &str) -> anyhow::Result<WindowInfo>;
    async fn window_close(&self, id: &str) -> anyhow::Result<()>;
    async fn window_minimize(&self, id: &str) -> anyhow::Result<()>;
    async fn window_maximize(&self, id: &str) -> anyhow::Result<()>;
    async fn window_move_resize(&self, id: &str, x: i32, y: i32, w: u32, h: u32) -> anyhow::Result<()>;

    // Workspaces
    async fn workspaces_list(&self) -> anyhow::Result<Vec<WorkspaceInfo>>;
    async fn workspace_switch(&self, id: u32) -> anyhow::Result<()>;
    async fn workspace_move_window(&self, window_id: &str, workspace_id: u32, follow: bool) -> anyhow::Result<()>;

    // Input
    async fn keyboard_type(&self, text: &str) -> anyhow::Result<()>;
    async fn keyboard_key(&self, key: &str) -> anyhow::Result<()>;
    async fn keyboard_combo(&self, keys: &[String]) -> anyhow::Result<()>;
    async fn mouse_move(&self, x: f64, y: f64) -> anyhow::Result<()>;
    async fn mouse_click(&self, button: &str) -> anyhow::Result<()>;
    async fn mouse_scroll(&self, dx: f64, dy: f64) -> anyhow::Result<()>;

    // Clipboard
    async fn clipboard_read(&self) -> anyhow::Result<String>;
    async fn clipboard_write(&self, text: &str) -> anyhow::Result<()>;

    // Screenshot
    async fn screenshot(&self, monitor: Option<u32>, region: Option<Region>,
                        window_id: Option<String>) -> anyhow::Result<ScreenshotResult>;

    // Notifications
    async fn notification_send(&self, title: &str, body: &str, urgency: &str) -> anyhow::Result<u32>;
    async fn notification_close(&self, id: u32) -> anyhow::Result<()>;

    // System
    async fn system_info(&self) -> anyhow::Result<SystemInfo>;
    async fn system_idle(&self) -> anyhow::Result<u64>;
    async fn system_battery(&self) -> anyhow::Result<Vec<BatteryInfo>>;
    async fn system_power(&self, action: &str) -> anyhow::Result<()>;

    // Network
    async fn network_status(&self) -> anyhow::Result<NetworkStatus>;
    async fn network_interfaces(&self) -> anyhow::Result<Vec<InterfaceInfo>>;
    async fn network_wifi_scan(&self) -> anyhow::Result<Vec<WifiNetwork>>;
    async fn network_wifi_connect(&self, ssid: &str, password: &str) -> anyhow::Result<()>;

    // Bluetooth
    async fn bluetooth_list(&self) -> anyhow::Result<Vec<BtDevice>>;
    async fn bluetooth_scan(&self, duration: Option<u64>) -> anyhow::Result<()>;
    async fn bluetooth_stop_scan(&self) -> anyhow::Result<()>;
    async fn bluetooth_connect(&self, addr: &str) -> anyhow::Result<()>;
    async fn bluetooth_disconnect(&self, addr: &str) -> anyhow::Result<()>;

    // Files
    async fn files_watch(&self, path: &str, recursive: bool, patterns: Vec<String>) -> anyhow::Result<()>;
    async fn files_unwatch(&self, path: &str) -> anyhow::Result<()>;
    async fn files_search(&self, pattern: &str, root: &str, max_results: usize) -> anyhow::Result<Vec<String>>;

    // Process
    async fn process_list(&self) -> anyhow::Result<Vec<ProcessInfo>>;
    async fn process_start(&self, command: &[String], workdir: Option<&str>,
                           env: Option<&HashMap<String,String>>) -> anyhow::Result<u32>;

    // Audio
    async fn audio_list_sinks(&self) -> anyhow::Result<Vec<SinkInfo>>;
    async fn audio_set_sink_volume(&self, sink_id: u32, volume: f64) -> anyhow::Result<()>;
}
```

**Guidelines for implementations:**

- Return `anyhow::bail!("not implemented for <your-backend>")` for methods you can't implement yet. The daemon converts these into graceful protocol errors.
- Use shell commands via `tokio::process::Command` for CLI-dependent operations (the GNOME backend's `sh()` helper is a good pattern to copy).
- Use the `event_tx: broadcast::Sender<DeskbridEvent>` for push events (file watching, potentially device events).
- Prefer compositor IPC (D-Bus, socket) over CLI where available — it's faster and avoids shell injection surface.

## Adding a New Action

1. **Protocol** (`src/protocol.rs`): Add variant to `Action` enum, add to `public_action_types()`, add parse path in `from_json()`
2. **Permissions** (`src/permissions.rs`): Add action name mapping in `action_name()` 
3. **Backend trait** (`src/backend/mod.rs`): Add method to `DesktopBackend`
4. **Backends**: Implement in each backend file (or return `bail!("not implemented")` for now)
5. **Dispatch** (`src/daemon.rs`): Add match arm in `execute_action()`
6. **API doc** (`docs/API.md`): Document request/response format with real examples
7. **Protocol doc** (`PROTOCOL.md`): Add to the action reference table

## PR Process

1. **Branch**: `feature/<short-description>` or `fix/<short-description>`
2. **Pre-commit**:
   ```bash
   cargo fmt
   cargo clippy
   cargo test
   ```
3. **Commit messages**: Conventional Commits format
   ```
   feat(gnome): add window close via Extension D-Bus
   fix(protocol): correct bluetooth.scan_stop action name in dispatch
   docs(api): document process.wait timeout behavior
   ```
4. **PR scope**: Single concern per PR. A backend addition is one PR; a protocol change is another.
5. **Review**: At minimum, the PR author should verify on their own machine. The project doesn't have CI yet.
6. **Merge**: Squash merge to `main` with a clean commit message.

## Known Issues & Stubs

These areas are explicitly marked as incomplete and are good targets for contributions:

| Area | Status | What's Needed |
|------|--------|--------------|
| `windows.close/minimize/maximize/move_resize` | Stub | Backend trait methods exist but every backend returns "not implemented" |
| `ui.tree.get`, `ui.element.click/set_text` | Stub | AT-SPI D-Bus integration for accessibility tree traversal |
| `bluetooth.pair/forget` | Stub | Trait methods exist, no backend implements them |
| `location.get` | Placeholder | Returns `"not yet implemented"` |
| `screencast.start/stop` | Stub | PipeWire-based screen capture (behind `pipewire` feature flag, not wired) |
| `hotkeys.register/unregister` | Placeholder | Accepts requests but does nothing — needs backend wiring |
|| X11 backend | Functional | Implemented: window focus/get, keyboard/mouse input, clipboard, screenshot, notifications, workspace switch. Missing: window close/minimize/maximize/move_resize, audio, bluetooth, WiFi, file watching |
| Integration tests | Missing | No `tests/` directory yet. End-to-end tests against a running daemon |
|| CI pipeline | Release only | GitHub Actions builds on `v*` tag push. No per-PR CI (tests not run automatically) |

## Architecture Reference

For a deep dive into the daemon's internals, see [ARCHITECTURE.md](ARCHITECTURE.md).

For the full API reference with every action's request/response format, see [API.md](API.md).

For protocol-level details (message format, connection lifecycle, error codes), see [PROTOCOL.md](../PROTOCOL.md).

For dependency setup per desktop environment, see [DEPENDENCIES.md](../DEPENDENCIES.md).

## Environment

- **Rust**: Latest stable (the project uses edition 2024, available since Rust 1.85+)
- **Python**: 3.10+ (for the client SDK)
- **OS**: Linux with a Wayland or X11 desktop session
- **D-Bus**: Session bus running (required for GNOME and KDE backends)
