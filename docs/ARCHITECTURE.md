# Architecture

Deskbrid is a Rust daemon that exposes a desktop automation API over a Unix socket. It auto-detects the running desktop environment, loads the matching backend, and translates JSON messages into desktop-specific operations.

## High-Level Design

```
┌─────────────┐   NDJSON   ┌──────────────────────────────────────┐
│   Client 1  │◄──────────►│                                      │
└─────────────┘  Unix Sock │           deskbrid daemon            │
                           │                                      │
┌─────────────┐            │  ┌────────────────────────────────┐  │
│   Client 2  │◄──────────►│  │          DaemonState            │  │
└─────────────┘            │  │  ┌──────────────────────────┐   │  │
                           │  │  │  Backend (trait object)   │   │  │
┌─────────────┐            │  │  │  ┌──────────┐            │   │  │
│ Python SDK  │◄──────────►│  │  │  │ Gnome ◄──┤            │   │  │
└─────────────┘            │  │  │  ├──────────┤            │   │  │
                           │  │  │  │Hyprland◄─┤            │   │  │
┌─────────────┐            │  │  │  ├──────────┤            │   │  │
│ GNOME Ext.  │◄─── DBus ─►│  │  │  │   KDE   ◄┤            │   │  │
└─────────────┘            │  │  │  ├──────────┤            │   │  │
                           │  │  │  │   X11   ◄┤            │   │  │
                           │  │  │  └──────────┘            │   │  │
                           │  │  │  Permissions (per-UID)   │   │  │
                           │  │  │  Event broadcast          │   │  │
                           │  │  └──────────────────────────┘   │  │
                           │  │                                  │  │
                           │  └──────────────────────────────────┘  │
                           └──────────────────────────────────────┘
```

## Transport: Unix Domain Socket

The daemon binds to:

```
$XDG_RUNTIME_DIR/deskbrid.sock      # typically /run/user/1000/deskbrid.sock
```

Socket lifecycle:
1. **Bind**: `UnixListener::bind()` on daemon start (`src/daemon.rs:24`)
2. **Cleanup**: stale socket removed before bind (`src/daemon.rs:18`)
3. **Accept**: each connection gets a `tokio::spawn` task (`src/daemon.rs:47-55`)
4. **Close**: implicit on client disconnect or `disconnect` action

The socket uses `SO_PEERCRED` (`src/permissions.rs:218-236`) to extract the connecting process's UID for permission evaluation. This is a Linux-specific security mechanism — the kernel provides the peer's UID/GID/PID on each connection, which is not forgeable by the client.

## Protocol: NDJSON

Messages are newline-delimited JSON (`\n` separator, 1 MiB max). The protocol is fully documented in [PROTOCOL.md](../PROTOCOL.md), but the key architectural patterns are:

### Request Flow

```
Client ──→ {"type": "windows.list", "id": "req-1"}
              ↓
          Action::from_json() parses the line
              ↓
          permissions.check(peer_uid, &action)
              ↓
          backend.windows_list().await
              ↓
Daemon  ──→ {"type": "response", "id": "req-1", "seq": 1, "status": "ok", "data": [...]}
```

### Action Dispatch (`src/daemon.rs:322-705`)

The `execute_action()` function is a large `match` on the `Action` enum that calls the corresponding backend trait method and wraps the result as `serde_json::Value`. Most actions are one-liners:

```rust
WindowsList => serde_json::json!(backend.windows_list().await?),
WindowsClose(ref id) => {
    backend.window_close(id).await?;
    serde_json::json!({"closed": id})
},
ClipboardRead => serde_json::json!({"text": backend.clipboard_read().await?}),
```

Actions that are unavailable on a specific compositor return backend errors and are reflected in `system.capabilities` / `capabilities.list` when the limitation is known.

### Layout Profiles

Layout profiles are daemon-managed JSON snapshots under `~/.config/deskbrid/layout_profiles/`. `layout_profiles.save` captures `system.info`, `workspaces.list`, and `windows.list`; `layout_profiles.restore` reloads the snapshot, matches current windows by ID/app/title, reapplies workspace placement and geometry through backend methods, minimizes windows that were saved minimized, and switches back to the saved active workspace. Monitor topology is saved for comparison and reported on restore, but monitor mode changes are not applied yet.

### Connection Lifecycle

Each client connection follows this lifecycle inside `handle_client()` (`src/daemon.rs:64-217`):

1. **Peer identity**: `socket_peer_uid()` extracts the connecting UID
2. **Stream split**: `stream.into_split()` gives separate reader/writer halves
3. **Connected handshake**: daemon immediately sends the `connected` message with version, protocol name, and UID
4. **Event forwarder**: a background task subscribes to the broadcast channel and forwards matching events to this client's writer
5. **Command loop**: `tokio::select!` between event forwarding and reading lines — the daemon reads one NDJSON line, parses it with `Action::from_json()`, checks permissions, dispatches, and writes a response
6. **Cleanup**: on EOF or `disconnect`, the loop exits and the writer half is dropped

```rust
// src/daemon.rs:64-80 — handshake
let peer_uid = socket_peer_uid(&stream).unwrap_or(u32::MAX);
let (reader, mut writer) = stream.into_split();
let mut conn = ConnectionState::default();

let connected = serde_json::json!({
    "type": "connected", "id": "server", "seq": 0,
    "data": { "version": "0.6.0", "protocol": "deskbrid-v2", "uid": peer_uid }
});
writer.write_all(...).await?;
```

### Event Subscription Model

Events are pushed via a tokio `broadcast::channel(256)` (`src/lib.rs:27`). The daemon's event flow:

```
Backend    ──→ event_tx.send(DeskbridEvent::FileCreated { ... })
                    │
                    ▼
             broadcast::channel(256)
                    │
          ┌─────────┼──────────┐
          ▼         ▼          ▼
      Client 1  Client 2   Client 3
      (writer)  (writer)   (writer)
```

Each client spawns a forwarder task that:
1. Subscribes to the broadcast channel
2. Receives every event
3. Checks if it matches the client's subscription patterns (`conn.subscriptions`)
4. If matched, wraps it in an event envelope and writes to the client socket

Subscription matching supports glob patterns (`src/daemon.rs:221-239`):

```rust
fn event_matches_any(subscriptions: &HashSet<String>, event_type: &str) -> bool {
    // Exact match: "file.created" == "file.created"
    // Category match: "file.*" matches "file.created", "file.modified"
    // Wildcard: "*" matches everything
}
```

This means a subscription of `"file.*"` will receive `file.created`, `file.modified`, `file.deleted`, and `file.renamed` events — without needing to enumerate them.

The daemon also emits synthetic events from certain actions via `emit_action_event()` (`src/daemon.rs:282-320`):

| Action | Event Type Emitted |
|--------|-------------------|
| `windows.focus` | `WindowFocused { window_id, timestamp }` |
| `workspaces.switch` | `WorkspaceChanged { workspace_id, timestamp }` |
| `workspaces.move_window` | `WorkspaceWindowMoved { window_id, workspace_id, timestamp }` |

These let subscribers react to state changes that result from the daemon's own actions.

### Event Types

Defined in the `DeskbridEvent` enum (`src/protocol.rs`):

- `FileCreated`, `FileModified`, `FileDeleted`, `FileRenamed` — from `notify::Watcher`
- `WindowFocused` — from focus action
- `WorkspaceChanged` — from workspace switch
- `WorkspaceWindowMoved` — from move-window action

Each carries a Unix timestamp.

## Backend Architecture

All backends implement the `DesktopBackend` trait (`src/backend/mod.rs:91-174`):

```rust
#[async_trait]
pub trait DesktopBackend: Send + Sync {
    async fn windows_list(&self) -> anyhow::Result<Vec<WindowInfo>>;
    async fn window_focus(&self, id: &str) -> anyhow::Result<()>;
    async fn keyboard_type(&self, text: &str) -> anyhow::Result<()>;
    async fn clipboard_read(&self) -> anyhow::Result<String>;
    async fn screenshot(&self, monitor: Option<u32>, region: Option<Region>,
                        window_id: Option<String>) -> anyhow::Result<ScreenshotResult>;
    // ... 30+ more methods
}
```

The trait covers 14 domains: windows, workspaces, input, clipboard, screenshot, notifications, system, network, bluetooth, files, process, audio, hotkeys, UI accessibility. New domains start from the trait, then each backend implements them.

### Desktop Detection (`src/backend/mod.rs:33-79`)

Detection runs in this priority order:
1. **`XDG_CURRENT_DESKTOP`** env var — fastest, covers all major DEs
2. **Process scan** (`pgrep -x Hyprland`, `pgrep -x kwin_wayland`) — catches compositors without the env var
3. **`$DISPLAY` without `$WAYLAND_DISPLAY`** — signals X11
4. **Fallback to GNOME** — safe default

```rust
async fn detect_desktop() -> DesktopEnv {
    // 1. Check XDG_CURRENT_DESKTOP
    if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
        let lower = desktop.to_lowercase();
        if lower.contains("hyprland") { return DesktopEnv::Hyprland; }
        if lower.contains("kde") || lower.contains("plasma") { return DesktopEnv::Kde; }
        if lower.contains("gnome") { return DesktopEnv::Gnome; }
        if lower.contains("x11") || lower.contains("xfce")
            || lower.contains("mate") || lower.contains("cinnamon") { return DesktopEnv::X11; }
    }
    // 2. Process scan
    if pgrep("Hyprland") { return DesktopEnv::Hyprland; }
    if pgrep("kwin_wayland") { return DesktopEnv::Kde; }
    // 3. X11 detection
    if std::env::var("DISPLAY").is_ok() && std::env::var("WAYLAND_DISPLAY").is_err() { return DesktopEnv::X11; }
    // 4. Default
    DesktopEnv::Gnome
}
```

### GNOME Backend (`src/backend/gnome.rs`)

The primary backend — 1,853 lines, the most complete implementation. It uses:

**Mutter RemoteDesktop** — for input injection (keyboard/mouse) via the `zbus` crate. The backend opens a RemoteDesktop session on startup (`init_remote_desktop()`) and a ScreenCast session for absolute mouse positioning (`init_screen_cast()` — best-effort, relative motion works without it).

**GNOME Shell Extension** — for window/workspace management via D-Bus at `org.deskbrid.WindowManager`. The extension communicates through the standard GNOME Shell D-Bus service and the `imports.ui` APIs.

**CLI tools** — for operations outside the extension's scope:

| Tool | Purpose | Used For |
|------|---------|----------|
| `ydotool` | Keyboard + mouse input | `keyboard_type`, `keyboard_key`, `keyboard_combo`, `mouse_move`, `mouse_click`, `mouse_scroll` |
| `wl-copy` / `wl-paste` | Clipboard | `clipboard_write`, `clipboard_read` |
| `grim` | Screenshot | `screenshot` |
| `notify-send` | Notifications | `notification_send`, `notification_close` |
| `nmcli` | WiFi | `wifi_scan`, `wifi_connect` |
| `bluetoothctl` | Bluetooth | `bluetooth_list`, `bluetooth_scan`, `bluetooth_connect` |
| `pactl` | Audio | `audio_list_sinks`, `audio_set_sink_volume` |

**File watching** — uses the `notify` crate (inotify on Linux) with `std::sync::Mutex<HashMap<String, RecommendedWatcher>>` to manage per-path watchers. File events are forwarded through the broadcast channel as `DeskbridEvent` variants.

### Hyprland Backend (`src/backend/hyprland.rs`)

Uses `hyprctl` CLI for window/workspace management (no D-Bus, no extension). Input injection via `ydotool`, screenshots via `grim`. 822 lines.

```
hyprctl clients -j                                  → windows list
hyprctl dispatch focuswindow address:<id>            → window focus
hyprctl dispatch workspace <id>                      → workspace switch
grim -g "<x>,<y> <w>x<h>" /tmp/deskbrid/screenshot  → screenshot
```

### KDE Backend (`src/backend/kde.rs`)

Uses `qdbus6` for KWin control, `ydotool` for input, `spectacle` for screenshots (cropped by `imagemagick convert`).

```
qdbus6 org.kde.KWin /KWin supportInformation  → windows
spectacle --background --nonotify --fullscreen  → screenshot
```

### X11 Backend (`src/backend/x11.rs`)

A functional X11 backend using xdotool, xclip, ImageMagick, and notify-send. Clocking in at 284 lines, it covers the most commonly used operations with real CLI-based implementations:

| Domain | Tool | Operations Implemented |
|--------|------|----------------------|
| Window focus/control | `xdotool` + `wmctrl` | `window_focus`, `window_close`, `window_minimize`, `window_maximize`, `window_move_resize`, `windows.activate_or_launch` launch fallback |
| Window info | `xdotool getwindowname <id>` | `window_get` |
| Workspace switch | `xdotool set_desktop <id>` | `workspace_switch`, `workspaces_list` |
| Keyboard input | `xdotool type/key/key+` | `keyboard_type`, `keyboard_key`, `keyboard_combo` |
| Mouse input | `xdotool mousemove/click` | `mouse_move`, `mouse_click`, `mouse_scroll` |
| Clipboard | `xclip -o/-i -selection clipboard` | `clipboard_read`, `clipboard_write` |
| Screenshot | `import -window root` (ImageMagick) | `screenshot` (fullscreen + region crop) |
| Notifications | `notify-send` | `notification_send` (close is a no-op) |
| System info | Hardcoded defaults | `system_info`, `idle_seconds` |

Operations **not** implemented (return `"not implemented on x11 backend"`): `workspace_move_window`, `power_action`, `wifi_connect`, `bluetooth_scan/stop_scan/connect/disconnect`, `files_watch/unwatch`, `audio_set_sink_volume`. The `windows_list` method returns an empty vector — X11 window enumeration is an open contribution target.

The `system_info()` method returns a monitor with `id: 0, name: "X11", width: 1920, height: 1080, scale: 1.0` as a reasonable default for coordinate normalization.

## Daemon State

The `DaemonState` struct (`src/lib.rs:17-23`) is the shared state for the entire daemon:

```rust
pub struct DaemonState {
    /// The loaded desktop backend (writable — hot-reloadable)
    pub backend: Arc<RwLock<Option<Box<dyn backend::DesktopBackend>>>>,
    /// Broadcast channel for push events (file changes, workspace switches, etc.)
    pub event_tx: broadcast::Sender<DeskbridEvent>,
    /// Scoped permissions per UID, loaded from ~/.config/deskbrid/permissions.toml
    pub permissions: Permissions,
}
```

Wrapped in `Arc<DaemonState>` so all connection tasks share the same state.

The `ConnectionState` struct (`src/lib.rs:43-51`) holds per-connection data:

```rust
pub struct ConnectionState {
    /// Glob-pattern event subscriptions (e.g. "file.*", "window.focused")
    pub subscriptions: HashSet<String>,
    /// Registered hotkey IDs (for cleanup on disconnect)
    pub hotkeys: HashSet<String>,
    /// Watched file paths (for cleanup on disconnect)
    pub watched_paths: HashSet<String>,
}
```

## Permissions System (`src/permissions.rs`)

The permissions file lives at `~/.config/deskbrid/permissions.toml`. Format:

```toml
[permissions.uid:1000]
allow = ["*"]

[permissions.uid:1001]
allow = ["windows.*", "clipboard.read"]
deny = ["screenshot"]
```

Key design decisions:
- **No file** → allow-all (backward compatible with existing installs)
- **Invalid file** → logged warning + allow-all fallback
- **Glob matching**: `"*"` matches everything, `"windows.*"` matches `windows.list`, `windows.focus`, etc.
- **Deny wins**: explicit deny always overrides allow
- **Default deny**: if no rule matches a UID's action, it's denied
- **Per-connection**: `DaemonState.permissions` is shared across all connections; loaded once at startup

Permission checking happens in `dispatch_action()` (`src/daemon.rs:248-250`), before the backend is even consulted:

```rust
if !state.permissions.check(peer_uid, &action) {
    return permission_denied_response(seq);
}
```

## Screen Capture (`src/capture.rs`)

The `fallback_screenshot()` function tries two methods in order:

1. **`gnome-screenshot -f <path>`** — works on GNOME X11 and some Wayland sessions
2. **XDG Desktop Portal** — Python script at `scripts/screenshot_portal.py` using `dbus` portal API

Both save to `/tmp/deskbrid/screenshot_<timestamp>.png`. Returns the file path on success.

The pipewire screencast method (via Mutter ScreenCast) is noted as a future improvement.

## System Health & Remediation (`src/daemon.rs:843-998`)

The `system.health` action checks per-backend dependencies:

- **GNOME**: extension D-Bus reachability, `grim`, `wl-clipboard`
- **KDE**: `qdbus6`, `spectacle`, `imagemagick convert`, `ydotoold`, `/dev/uinput`
- **Hyprland**: `hyprctl`, `ydotoold`, `/dev/uinput`, `grim`

The `system.remediate` action can auto-fix:
- **ydotoold**: start it as a background process (`nohup ydotoold &`)
- **KDE ydotoold autostart**: create the KDE autostart `.desktop` entry

```rust
match check {
    "ydotoold" => {
        // nohup ydotoold >/tmp/deskbrid-ydotoold.log 2>&1 &
        // Then verify with pgrep
    }
    "kde_ydotoold_autostart" => {
        // Write ~/.config/autostart/ydotoold.desktop
    }
}
```

## Safety Boundaries

The daemon protects against dangerous operations:

**PID guards** (`src/daemon.rs:707-722`):
```rust
fn ensure_safe_pid(pid: u32) -> anyhow::Result<()> {
    if pid <= 1 { bail!("refusing to target reserved pid {}", pid); }
    if pid > i32::MAX as u32 { bail!("refusing out-of-range pid"); }
    if pid == std::process::id() { bail!("refusing to target deskbrid daemon"); }
    Ok(())
}
```

Blocked PIDs: 0, 1, and the daemon's own PID.

## Client SDK

The Python client at `clients/python/` provides both async and sync interfaces:

```python
from deskbrid import Client

# Async
async with Client() as client:
    windows = await client.windows_list()
    await client.window_focus("0x3a0000b")

# Sync
client = Client()
client.connect()
windows = client.windows_list()
```

Key features:
- Automatic socket path discovery (`$XDG_RUNTIME_DIR/deskbrid.sock`)
- Async event subscription via callback
- Pydantic models for all response types
- Token bucket rate limiting (10 req/s default)

## systemd Integration

The `deploy/deskbrid.service` systemd user service manages the daemon lifecycle:

```ini
[Unit]
Description=Deskbrid desktop automation daemon
After=graphical-session.target

[Service]
ExecStart=/usr/local/bin/deskbrid
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
```

Install with:
```bash
cp deploy/deskbrid.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now deskbrid
```

## Module Map

```
src/
├── main.rs              # CLI entry: daemon (default), setup, install
├── lib.rs               # DaemonState, ConnectionState
├── daemon.rs            # Unix socket listener, client handler, action dispatch
├── protocol.rs          # Action enum, DeskbridEvent enum, parse/serialize
├── cli.rs               # CLI argument parsing with clap
├── client.rs            # Sync TCP client for testing
├── backend/
│   ├── mod.rs           # DesktopBackend trait, DesktopEnv, desktop detection
│   ├── gnome.rs         # GNOME backend (Mutter DBus + tools)
│   ├── hyprland.rs      # Hyprland backend (hyprctl + tools)
│   ├── kde.rs           # KDE backend (qdbus + tools)
│   └── x11.rs           # X11 backend (xdotool + xclip + ImageMagick)
├── permissions.rs       # Per-UID permission system, SO_PEERCRED
├── capture.rs           # Screenshot fallback (gnome-screenshot → portal)
├── setup.rs             # `deskbrid setup` — auto-detect + install deps
└── extensions/          # GNOME Shell extension source
    └── deskbrid@deskbrid/
        ├── extension.js
        └── metadata.json

clients/
└── python/
    └── deskbrid/
        ├── __init__.py  # Re-exports: Client, models, events
        ├── client.py    # Async/sync client, event subscription
        ├── models.py    # Pydantic models for API responses
        └── events.py    # Event types

scripts/
├── screenshot_portal.py # XDG Desktop Portal screenshot helper

deploy/
├── deskbrid.service     # systemd user service
```

## Dependency Graph

```
deskbrid
├── tokio                # Async runtime (net, fs, process, sync)
├── serde / serde_json   # JSON serialization
├── clap                 # CLI argument parsing
├── tracing              # Structured logging
├── async-trait          # Async trait methods
├── zbus                 # D-Bus client (GNOME backend only)
├── notify               # File watching (inotify)
├── libc                 # SO_PEERCRED, kill syscall, PID protection
├── toml                 # Permissions file parsing (serde-based)
└── futures-util         # Stream combinators
```
