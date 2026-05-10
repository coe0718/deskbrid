# Contributing to Deskbrid

Deskbrid is a desktop bridge daemon that connects AI agents and scripts to the Linux desktop. It's built in Rust with async I/O and a plugin-style backend architecture.

## Project Structure

```
deskbrid/
├── src/
│   ├── main.rs           — Entry point. Parses CLI args, starts daemon or client mode
│   ├── lib.rs            — DaemonState, ConnectionState, module re-exports
│   ├── daemon.rs         — Unix socket listener, client handler, message dispatch
│   ├── protocol.rs       — Action enum, DeskbridEvent, response types, JSON serialisation
│   ├── cli.rs            — CLI argument parsing (daemon, status, stop, restart, install, setup)
│   ├── client.rs         — Built-in client mode (reads NDJSON from stdin, sends to socket)
│   ├── capture.rs        — Screenshot helpers (grim on GNOME/Hyprland, spectacle+convert on KDE)
│   └── backend/
│       ├── mod.rs        — DesktopBackend trait, create_backend() factory
│       ├── gnome.rs      — GNOME backend (zbus DBus, gdbus CLI, wtype, grim, pactl, etc.)
│       ├── hyprland.rs   — Hyprland backend (hyprctl JSON CLI, ydotool, grim)
│       └── kde.rs        — KDE backend (KWin D-Bus scripting, ydotool, spectacle, ImageMagick)
├── clients/
│   └── python/
│       ├── deskbrid/
│       │   ├── __init__.py
│       │   ├── client.py  — Python client library (high-level API)
│       │   ├── models.py  — Pydantic models for all data types
│       │   └── events.py  — Event subscription helpers
│       └── README.md
├── extensions/
│   └── deskbrid@deskbrid/
│       ├── extension.js   — GNOME Shell extension (DBus window manager)
│       └── metadata.json
├── deploy/
│   └── deskbrid.service   — systemd user service unit
├── PROTOCOL.md            — Wire protocol specification
├── DEPENDENCIES.md        — Required system packages by domain
├── demo.sh                — End-to-end demo script
└── Cargo.toml             — Rust dependencies and build config
```

## Building from Source

```bash
cargo build --release
```

The binary is placed at `target/release/deskbrid`. Install it:

```bash
sudo cp target/release/deskbrid /usr/local/bin/
```

For quick iteration during development:

```bash
cargo run -- daemon
```

The daemon detaches after logging "listening on ...". It can be stopped with `deskbrid stop`, or by killing the process and removing the socket manually:

```bash
rm $XDG_RUNTIME_DIR/deskbrid.sock
```

See the [README quick start](/README.md) for full setup instructions including dependencies.

## Running Tests

```bash
# Run the full test suite
cargo test

# Run with output (useful for debugging)
cargo test -- --nocapture

# Run a specific test
cargo test test_name -- --nocapture
```

✅ **No special test environment is needed** — the crate's unit tests (protocol serialisation, parsing, data models) don't require a running daemon or a Wayland session.

There is also a Python client test suite — see `clients/python/` for running those.

> **Integration testing note:** Integration tests that need a running daemon require a compatible Wayland session (GNOME 46+, Hyprland, or KDE Plasma) and are not part of the unit test suite. See `demo.sh` for manual end-to-end testing patterns.

## Coding Conventions

### Rust

- **Async everywhere.** All backend trait methods are `async`. Socket I/O uses `tokio` primitives. Use `tokio::process::Command` for spawning external tools, never `std::process::Command`.
- **Error handling.** Use `anyhow::Result` for fallible operations. Return context-rich errors with `anyhow::Context`. The daemon wraps errors into the JSON `INTERNAL_ERROR` response — don't panic.
- **Naming.** Match Rust conventions: `snake_case` for functions/variables, `CamelCase` for types, `SCREAMING_SNAKE` for constants. Action enum variants are `CamelCase` in code (`WindowsList`, `InputKeyboardType`), `snake_case` in the JSON wire format (`windows.list`, `input.keyboard`).
- **Trait methods.** All `DesktopBackend` trait method signatures use `&self`. State that needs mutation should use interior mutability (`Mutex`, `RwLock`, `Arc`).
- **Formatting.** `cargo fmt` before committing. The project uses standard Rust formatting.
- **Linting.** `cargo clippy` should pass without warnings.

### Python Client

- Type annotations everywhere.
- Use `pydantic` models for all data structures.
- `deskbrid.py` (the single-file client) stays as a standalone importable module.
- Maintain backward compatibility with the one-liner import: `from deskbrid import Deskbrid`.

### GNOME Shell Extension

- Use `let` declarations (GNOME 45+ `var` compatibility is handled where needed).
- Minimal dependencies on GNOME Shell internals.
- All new extension features must support the current `enable()`/`disable()` lifecycle.

## How to Add a New Backend

The `DesktopBackend` trait in `src/backend/mod.rs` defines all methods a backend must implement:

```rust
#[async_trait]
pub trait DesktopBackend: Send + Sync {
    // Windows
    async fn windows_list(&self) -> anyhow::Result<Vec<WindowInfo>>;
    async fn window_focus(&self, id: &str) -> anyhow::Result<()>;
    async fn window_get(&self, id: &str) -> anyhow::Result<WindowInfo>;

    // Workspaces
    async fn workspaces_list(&self) -> anyhow::Result<Vec<WorkspaceInfo>>;
    async fn workspace_switch(&self, id: u32) -> anyhow::Result<()>;
    async fn workspace_move_window(&self, window_id: &str, workspace_id: u32, follow: bool) -> anyhow::Result<()>;

    // Input — keyboard
    async fn keyboard_type(&self, text: &str) -> anyhow::Result<()>;
    async fn keyboard_key(&self, key: &str) -> anyhow::Result<()>;
    async fn keyboard_combo(&self, keys: &[String]) -> anyhow::Result<()>;

    // Input — mouse
    async fn mouse_move(&self, x: f64, y: f64) -> anyhow::Result<()>;
    async fn mouse_click(&self, button: &str) -> anyhow::Result<()>;
    async fn mouse_scroll(&self, dx: f64, dy: f64) -> anyhow::Result<()>;

    // Clipboard
    async fn clipboard_read(&self) -> anyhow::Result<String>;
    async fn clipboard_write(&self, text: &str) -> anyhow::Result<()>;

    // Screenshot
    async fn screenshot(&self, monitor: Option<u32>, region: Option<Rect>, window_id: Option<String>) -> anyhow::Result<ScreenshotResult>;

    // Notifications
    async fn notification_send(&self, app_name: &str, title: &str, body: &str, urgency: &str) -> anyhow::Result<u32>;
    async fn notification_close(&self, id: u32) -> anyhow::Result<()>;

    // System
    async fn system_info(&self) -> anyhow::Result<SystemInfo>;
    async fn idle_seconds(&self) -> anyhow::Result<u64>;
    async fn power_action(&self, action: &str) -> anyhow::Result<()>;
    async fn battery_status(&self) -> anyhow::Result<Vec<BatteryInfo>>;

    // Network
    async fn network_status(&self) -> anyhow::Result<NetworkStatusInfo>;
    async fn network_interfaces(&self) -> anyhow::Result<Vec<NetworkInterfaceInfo>>;
    async fn wifi_scan(&self) -> anyhow::Result<Vec<WifiNetworkInfo>>;
    async fn wifi_connect(&self, ssid: &str, password: Option<&str>) -> anyhow::Result<()>;

    // Bluetooth
    async fn bluetooth_list(&self) -> anyhow::Result<Vec<BluetoothDeviceInfo>>;
    async fn bluetooth_scan(&self, duration: Option<u32>) -> anyhow::Result<()>;
    async fn bluetooth_stop_scan(&self) -> anyhow::Result<()>;
    async fn bluetooth_connect(&self, address: &str) -> anyhow::Result<()>;
    async fn bluetooth_disconnect(&self, address: &str) -> anyhow::Result<()>;

    // Audio
    async fn audio_list_sinks(&self) -> anyhow::Result<Vec<AudioSinkInfo>>;
    async fn audio_set_sink_volume(&self, sink_id: u32, volume: f64) -> anyhow::Result<()>;

    // Files
    async fn files_watch(&self, path: &str, recursive: bool, patterns: Option<&[String]>) -> anyhow::Result<()>;
    async fn files_unwatch(&self, path: &str) -> anyhow::Result<()>;
    async fn files_search(&self, pattern: &str, root: Option<&str>, max_results: Option<u32>) -> anyhow::Result<Vec<String>>;
}
```

**To add a new backend (e.g. Sway, Xfce, Cinnamon):**

1. **Create the module:** `src/backend/backend_name.rs` with a struct implementing `DesktopBackend`
2. **Register it in the factory:** Update `create_backend()` in `src/backend/mod.rs` to attempt your backend (it should return `Err` gracefully if the environment isn't detected)
3. **Handle environment detection:** Check for session type, available DBus services, or `$XDG_CURRENT_DESKTOP` in the constructor

```rust
// src/backend/mod.rs — factory example
pub async fn create_backend(
    event_tx: broadcast::Sender<DeskbridEvent>,
) -> anyhow::Result<Box<dyn DesktopBackend>> {
    let session = std::env::var("XDG_SESSION_DESKTOP")
        .unwrap_or_default();

    match session.as_str() {
        "gnome" | "GNOME" => {
            let backend = GnomeBackend::new(event_tx).await?;
            Ok(Box::new(backend))
        }
        "kde" | "KDE" | "plasma" => {
            let backend = KdeBackend::new(event_tx).await?;
            Ok(Box::new(backend))
        }
        "sway" | "Hyprland" => {
            let backend = WlrootsBackend::new(event_tx).await?;
            Ok(Box::new(backend))
        }
        _ => anyhow::bail!("unsupported desktop environment: {}", session),
    }
}
```

4. **Use the same tools where possible:** `ydotool` for keyboard/mouse input, `wl-*` for clipboard, `pactl` for audio and `notify-send` for notifications all work across Wayland compositors. Screenshots vary per backend — use `grim` on GNOME/Hyprland, `spectacle` + ImageMagick `convert` on KDE. DBus-based features (Power, NetworkManager, BlueZ) are desktop-agnostic and can be reused.

### Backend Implementation Tips

- Make the constructor fallible (`async fn new(...) -> anyhow::Result<Self>`) — return an error with a clear message if a required tool or DBus service isn't available
- Use `tokio::process::Command` for all external tool invocations, never `std::process::Command`
- If the backend doesn't support a method, return an `anyhow::bail!` with a descriptive message — the daemon turns it into the `INTERNAL_ERROR` response
- The `event_tx` broadcast sender is passed to the constructor so the backend can emit events (file changes, etc.)
- Use `notify` crate for file watching just like the GNOME backend does — helpers for that are desktop-agnostic

## How to Add a New Action

1. **Add the variant to `Action` enum** in `src/protocol.rs`:
   ```rust
   Action::MyNewAction { param1: String, param2: Option<u32> }
   ```

2. **Define response data types** in the same file if needed (add to the response structs area)

3. **Update `execute_action`** in `src/daemon.rs` to handle the new variant, calling the corresponding `DesktopBackend` method:
   ```rust
   Action::MyNewAction { param1, param2 } => {
       serde_json::json!(backend.my_new_action(&param1, param2).await?)
   }
   ```

4. **Add the trait method** to `DesktopBackend` in `src/backend/mod.rs`

5. **Implement it** in every backend (`src/backend/gnome.rs` and any others)

6. **Update JSON parsing** if the action doesn't follow the standard pattern — check `Action::from_json()` in `protocol.rs`

7. **Document** the new action in `docs/API.md` and the wire format in `PROTOCOL.md`

## How to Add a New Event

1. **Add variant to `DeskbridEvent`** in `src/protocol.rs`:
   ```rust
   #[derive(Clone, Serialize, Deserialize, Debug, ...)]
   pub enum DeskbridEvent {
       FileCreated { path: String, timestamp: u64 },
       FileModified { path: String, timestamp: u64 },
       FileDeleted { path: String, timestamp: u64 },
       FileRenamed { old_path: String, new_path: String, timestamp: u64 },
       // Add your new event:
       ClipboardChanged { content: String, timestamp: u64 },
   }
   ```

2. **Emit from the backend** — the backend holds a `broadcast::Sender<DeskbridEvent>`. Call `event_tx.send(event)` when the event occurs:
   ```rust
   let _ = self.event_tx.send(DeskbridEvent::ClipboardChanged {
       content: text,
       timestamp: std::time::SystemTime::now()
           .duration_since(std::time::UNIX_EPOCH)
           .unwrap_or_default()
           .as_secs(),
   });
   ```

3. **No changes needed in the daemon** — the event loop already handles any `DeskbridEvent` variant generically. The event's variant name (via `serde`) becomes the `event` field in the JSON sent to clients.

4. **Document** the event type in both `docs/API.md` (Event Subscriptions section) and `PROTOCOL.md`.

## Python Client

The Python client at `clients/python/` is a first-class consumer of the daemon protocol. When making protocol changes:

1. Update `models.py` with any new data types (using pydantic models)
2. Update `client.py` with high-level methods for any new actions
3. Keep `deskbrid.py` (the single-file convenience module) in sync — it re-exports `Deskbrid` from the package
4. Run the client tests if present

The Python client is installed as a regular package:

```bash
cd clients/python
pip install -e .
```

## GNOME Shell Extension

The extension at `extensions/deskbrid@deskbrid/` provides window management via DBus. When extending:

1. Add new methods to the DBus interface XML in `extension.js`
2. Re-register them in the `enable()` function's `Gio.DBusExportedObject.wrapJSObject` call
3. Update the zbus/gdbus calls in `src/backend/gnome.rs` to match
4. Install/update the extension in GNOME Shell:

```bash
mkdir -p ~/.local/share/gnome-shell/extensions/deskbrid@deskbrid
cp -r extensions/deskbrid@deskbrid/* ~/.local/share/gnome-shell/extensions/deskbrid@deskbrid/
```

Then restart GNOME Shell (`Alt+F2` → `r` → Enter) or log out and back in.

## PR Process

1. **Fork the repo** and create a feature branch from `main`
2. **One logical change per PR** — if you're adding Bluetooth pairing and fixing a bug in the protocol parser, split them
3. **Update docs** — any new action, event, or protocol change must be reflected in `docs/API.md` and `PROTOCOL.md`
4. **Add backend implementations** — if you add a trait method, implement it in all existing backends. If a backend can't support it (e.g. no Bluetooth on a headless system), return a descriptive error
5. **Keep the Python client in sync** — if you change the protocol, update `clients/python/`
6. **Pass CI:** `cargo fmt`, `cargo clippy`, `cargo test` should all pass
7. **Submit** — open the PR with a clear title and description. Reference any related issues

### Checklist Before Submitting

- [ ] `cargo build --release` succeeds
- [ ] `cargo test` passes
- [ ] `cargo clippy` is clean
- [ ] `cargo fmt` has been run
- [ ] `docs/API.md` updated if protocol changed
- [ ] `PROTOCOL.md` updated if wire format changed
- [ ] Python client updated if action/event types changed
- [ ] GNOME Shell extension updated if window management changed
- [ ] `demo.sh` still works for manual verification

## Getting Help

- Read the [architecture overview](ARCHITECTURE.md) for system design
- Check the [API reference](API.md) for all available actions
- [PROTOCOL.md](/PROTOCOL.md) for the wire format specification
- [DEPENDENCIES.md](/DEPENDENCIES.md) for required system packages
- Open an issue on GitHub for questions not covered by docs
