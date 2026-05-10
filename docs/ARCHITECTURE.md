# Deskbrid Architecture

Deskbrid is a Unix socket daemon that provides programmatic control over the Linux desktop through a JSON-over-Unix-socket protocol. It acts as a bridge between client applications (AI agents, CLI tools, scripts) and desktop functionality — window management, input simulation, system queries, clipboard, audio, network, Bluetooth, and more.

## Overview

```
┌─────────────────┐     NDJSON/Unix Socket     ┌──────────────────────┐
│  Client Apps     │◄──────────────────────────►│  Deskbrid Daemon     │
│  (Python, CLI,   │                            │                      │
│   any lang)      │                            │  ┌────────────────┐  │
│                  │                            │  │  GNOME Backend  │  │
│                  │                            │  │  ┌────────────┐ │  │
│                  │                            │  │  │  DBus Ext.  │ │  │
│                  │                            │  │  │  (window mgmt)│  │
│                  │                            │  │  ├────────────┤ │  │
│                  │                            │  │  │  System DBus│ │  │
│                  │                            │  │  │  (UPower, NM,│ │  │
│                  │                            │  │  │   BlueZ)    │ │  │
│                  │                            │  │  ├────────────┤ │  │
│                  │                            │  │  │  CLI Tools  │ │  │
│                  │                            │  │  │  (wtype,    │ │  │
│                  │                            │  │  │  grim, wl-* │ │  │
│                  │                            │  │  │  pactl...)  │ │  │
│                  │                            │  │  └────────────┘ │  │
│                  │                            │  └────────────────┘  │
│                  │                            │                      │
│                  │                            │  ┌────────────────┐  │
│                  │                            │  │  Event         │  │
│                  │  ◄─── events ──────────────│  │  Broadcast     │  │
│                  │                            │  │  Channel       │  │
│                  │                            │  └────────────────┘  │
└─────────────────┘                            └──────────────────────┘
```

## Core Components

### 1. Unix Socket Transport

**Socket path:** `$XDG_RUNTIME_DIR/deskbrid.sock` (falls back to `/run/user/1000/deskbrid.sock`).

The daemon starts by removing any leftover socket file, creating the parent directory, and binding a `tokio::net::UnixListener` on the path. On each incoming connection the listener spawns an asynchronous `handle_client` task. The socket path is predictable and scoped to the user's session, so only processes running under the same session user can connect.

**Lifecycle:**
1. Daemon starts → removes stale socket → binds listener
2. Listener accepts connections in a loop, spawning one `tokio::spawn` task per client
3. Each client task owns a split `(reader, writer)` pair on the socket
4. On graceful disconnect, the client sends a `disconnect` action and the daemon responds with `disconnected`, then the read loop breaks
5. On socket close (client drops), `reader.read_line()` returns `n == 0` and the task exits cleanly
6. On daemon shutdown, the socket file remains (cleaned up next start)

### 2. NDJSON Protocol

All communication uses **NDJSON (Newline-Delimited JSON)** — one complete JSON object per line, terminated by `\n`. No framing, no length prefixes, no binary encoding. Every line is a self-contained message.

**Message types:**

`type` field   | Direction       | Purpose                                |
---------------|----------------|----------------------------------------|
`action`      | Client → Daemon | Execute a desktop action               |
`ping`        | Client → Daemon | Health check (no backend needed)       |
`subscribe`   | Client → Daemon | Subscribe to event patterns            |
`unsubscribe` | Client → Daemon | Unsubscribe from event patterns        |
`disconnect`  | Client → Daemon | Gracefully close the connection        |
`response`    | Daemon → Client | Action result (ok or error)            |
`connected`   | Daemon → Client | Sent immediately after socket connect  |
`disconnected`| Daemon → Client | Confirms graceful disconnect           |
`pong`        | Daemon → Client | Response to `ping`                     |
`event`       | Daemon → Client | Push event matching subscription       |

**Request structure — flat key-value at the top level (not nested JSON-RPC):**

```json
{"type": "action", "id": "windows.list", "seq": 1, "action": {"windows.list": {}}}
```

The `action` field in the original message is optional — the daemon's `Action::from_json` parser reads `type` as the action discriminator. What matters is that each action type has a corresponding string value in the `type` field (e.g. `"windows.list"`, `"input.keyboard"`).

**Response structure:**

```json
{
 "type": "response",
 "id": "action",
 "seq": 1,
 "status": "ok",
 "data": [ ... ]
}
```

On failure, `status` is `"error"` and an `error` object carries `code` and `message`:

```json
{
 "type": "response",
 "id": "action",
 "seq": 1,
 "status": "error",
 "error": { "code": "INTERNAL_ERROR", "message": "no backend loaded" }
}
```

**Error codes used:**
- `INVALID_PARAMS` — malformed JSON or unknown action type
- `INTERNAL_ERROR` — backend operation failed
- `NOT_SUPPORTED` — no backend loaded

**`connected` message** (sent immediately after socket connect, before any client message):

```json
{"type": "connected", "id": "server", "seq": 0, "data": {"version": "0.4.1", "protocol": "deskbrid-v2"}}
```

Clients should wait for this message before sending commands.

### 3. Message Dispatch Flow

The daemon's `handle_client` function runs a `tokio::select!` loop with two branches:

1. **Event forwarding** — reads from the per-client MPSC channel attached to the broadcast receiver, checks the event type against the client's subscription set, and writes matching events to the socket
2. **Client input** — reads one line from the socket, parses it into an `Action`, then dispatches:

```
Client line → Action::from_json() → match action {
   Action::Ping          → respond with "pong"
   Action::Subscribe     → insert patterns into conn.subscriptions
   Action::Unsubscribe   → remove patterns from conn.subscriptions
   Action::Disconnect    → respond with "disconnected", break
   Action::FilesWatch    → track path in conn.watched_paths + dispatch
   Action::FilesUnwatch  → remove path from conn.watched_paths + dispatch
   _                     → dispatch_action(action, state, seq)
}
```

`dispatch_action` locks the backend (read lock) and calls `execute_action` which pattern-matches on the `Action` variant and calls the corresponding `DesktopBackend` trait method. Results are serialised to a JSON response envelope. If no backend is loaded, it returns a `NOT_SUPPORTED` error.

### 4. Daemon State

Defined in `src/lib.rs` and shared across all connections via `Arc`:

```rust
pub struct DaemonState {
   pub backend: Arc<RwLock<Option<Box<dyn backend::DesktopBackend>>>>,
   pub event_tx: broadcast::Sender<DeskbridEvent>,
}
```

- **backend** — wrapped in `RwLock` so multiple connections can dispatch concurrently. Only the daemon startup writes to it (inserting the loaded backend). Clients read it.
- **event_tx** — a `tokio::sync::broadcast::channel(256)`. The GNOME backend holds a clone of the sender and pushes events into it. Each client connection subscribes to the broadcast and forwards matching events through an intermediate MPSC channel.

**Per-connection state** (`ConnectionState`):

```rust
pub struct ConnectionState {
   pub subscriptions: HashSet<String>,  // glob patterns like "file.*"
   pub hotkeys: HashSet<String>,        // registered hotkey IDs
   pub watched_paths: HashSet<String>,  // file watch paths
}
```

### 5. Event Broadcast System

Events flow through a three-stage pipeline:

1. **Backend produces events** — the GNOME backend's `files_watch` method sets up a `notify` watcher on a directory. When `notify` fires, the callback constructs a `DeskbridEvent` and sends it through the `event_tx` broadcast sender.

2. **Broadcast fan-out** — `event_tx.send()` distributes the event to all subscribed receivers. Each client connection holds a `broadcast::Receiver` that it obtained by calling `state.event_tx.subscribe()`.

3. **Subscription matching** — each client task runs a forwarder that reads from the broadcast, serialises the event to JSON, and writes it to the per-client MPSC channel. The main `select!` loop reads from the MPSC receiver and checks the event type against `conn.subscriptions` using glob-style matching:

```
event_matches_any(subscriptions, event_type):

- exact match: sub == event_type
- prefix glob: sub = "file.*" matches "file.created", "file.modified", etc.
- wildcard:    sub = "*" matches everything
```

**Supported events** (from `DeskbridEvent` enum):

Event type        | Fields                                    |
-------------------|-------------------------------------------|
`file.created`    | `path: String`, `timestamp: u64`          |
`file.modified`   | `path: String`, `timestamp: u64`          |
`file.deleted`    | `path: String`, `timestamp: u64`          |
`file.renamed`    | `old_path: String`, `new_path: String`, `timestamp: u64` |

The GNOME Shell extension also emits a `WindowStateChanged` DBus signal (debounced at 150ms), but this is not yet forwarded through the broadcast channel — it's available for future use.

**Event envelope** (sent to client):

```json
{"type": "event", "id": "file.created", "data": {"event": "file.created", "path": "/tmp/test.txt", "timestamp": 1715000000}}
```

### 6. The GNOME Backend

The GNOME backend (`src/backend/gnome.rs`) implements the `DesktopBackend` trait and uses four distinct integration strategies:

#### 6a. GNOME Shell DBus Extension

A custom GNOME Shell extension (`extensions/deskbrid@deskbrid/extension.js`) exposes a DBus interface at:

- **Service:** `org.deskbrid.WindowManager`
- **Object path:** `/org/deskbrid/WindowManager`
- **Interface:** `org.deskbrid.WindowManager`

**Methods:**

Method        | Input args                 | Output      | Purpose                        |
---------------|---------------------------|-------------|--------------------------------|
`ListWindows` | none                      | `s` (JSON)  | Returns serialised window list |
`FocusedWindow` | none                   | `s` (JSON)  | Returns focused window info    |
`FocusWindow` | `app_id`, `title`, `exact` | `b` (bool) | Focus a window by app_id/title |

**Signals:**

Signal               | Payload                              | Debounce |
----------------------|--------------------------------------|----------|
`WindowStateChanged` | JSON string of focused window info   | 150ms    |

The backend accesses this extension via `gdbus call` (CLI) rather than a direct `zbus` call — the `gdbus` CLI handles the session bus and GNOME-specific marshalling:

```rust
self.sh("gdbus", &[
   "call", "--session",
   "--dest", "org.deskbrid.WindowManager",
   "--object-path", "/org/deskbrid/WindowManager",
   "--method", "org.deskbrid.WindowManager.ListWindows"
]).await?
```

The JSON returned by the extension is wrapped in gdbus's tuple format `('[json]',)` and parsed by `parse_extension_json_windows()`.

#### 6b. System DBus (zbus)

The backend uses the `zbus` crate to call system DBus services directly:

- **UPower** (`org.freedesktop.UPower`) — battery status via `org.freedesktop.UPower.Device` properties (`Percentage`, `State`, `TimeToEmpty`)
- **NetworkManager** (`org.freedesktop.NetworkManager`) — connectivity state, interface list, Wi-Fi access point scanning via `GetAllDevices`, device properties, and `org.freedesktop.NetworkManager.Device.Wireless` for AP lists
- **BlueZ** (`org.bluez`) — Bluetooth device discovery via `ObjectManager.GetManagedObjects`, adapter management, and device connection/disconnection
- **Mutter IdleMonitor** (`org.gnome.Mutter.IdleMonitor`) — idle time via `GetIdletime` on `/org/gnome/Mutter/IdleMonitor/Core`

All zbus calls go through a shared `zbus::Connection` instance stored in the backend struct:

```rust
pub struct GnomeBackend {
   conn: zbus::Connection,
   event_tx: broadcast::Sender<DeskbridEvent>,
   watchers: Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>,
}
```

#### 6c. CLI Wrappers

For desktop operations that lack a stable DBus API or are better handled by ecosystem CLIs:

Operation          | CLI tool(s)                        | Notes                                   |
--------------------|------------------------------------|-----------------------------------------|
Keyboard input     | `wtype` (primary), `ydotool` (fallback) | Text typing and key combos          |
Mouse control      | `ydotool` mouse subcommands        | Move, click, scroll                     |
Screenshots        | `grim` + `slurp` (region/window)  | Outputs PNG to temp dir                 |
Clipboard read     | `wl-paste`                        | Requires wl-clipboard                   |
Clipboard write    | `wl-copy`                         | Requires wl-clipboard                   |
Notifications      | `notify-send`                     | Standard libnotify interface            |
Audio              | `pactl`                           | List sinks, set volume (PipeWire compat)|
Wi-Fi connect      | `nmcli`                           | Reliable connection setup               |
Idle time (fallback)| `loginctl` + `xssstate`          | Used when Mutter idle monitor is unavailable |

The backend's `sh()` method runs CLI commands asynchronously via `tokio::process::Command` and returns stdout. The companion `sh_ok()` method checks if a command is available without error output.

#### 6d. File Watching

File system monitoring uses the `notify` crate, creating a `notify::RecommendedWatcher` per watched path. Watchers are stored in an `Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>` so they live for the duration of the backend. When a file event fires, the `notify` callback sends a `DeskbridEvent` through the broadcast channel.

### 7. Backend Plugin System

Backends implement the `DesktopBackend` trait defined in `src/backend/mod.rs`. The factory function `create_backend()` attempts to initialise an available backend:

```rust
pub async fn create_backend(
    event_tx: broadcast::Sender<DeskbridEvent>,
) -> anyhow::Result<Box<dyn DesktopBackend>> {
    // Auto-detect desktop and load matching backend
    match detect_desktop().await? {
        Desktop::Gnome => GnomeBackend::new(event_tx).await.map(|b| Box::new(b) as Box<dyn DesktopBackend>),
        Desktop::Hyprland => HyprlandBackend::new(event_tx).await.map(|b| Box::new(b) as Box<dyn DesktopBackend>),
        Desktop::Kde => KdeBackend::new(event_tx).await.map(|b| Box::new(b) as Box<dyn DesktopBackend>),
    }
}
```

When the daemon starts, it calls `create_backend()`:

- On success: stores the backend in `DaemonState.backend`, logs which backend loaded (e.g. "GNOME backend loaded", "Hyprland backend loaded", "KDE backend loaded")
- On failure: logs a warning, continues without desktop features. All desktop actions return `NOT_SUPPORTED` errors

The trait is designed to be implemented for other desktop environments (Sway, Xfce, Cinnamon, etc.) by adding a new backend module and updating `create_backend()`.

### 8. Connection Lifecycle (Detailed)

```
1. Client connects to Unix socket
2. Daemon receives connection in accept loop
3. Daemon sends "connected" message with version info
4. Client receives "connected" and knows it can send commands
5. Loop:
  a. Client sends JSON action line
  b. Daemon increments seq counter
  c. Daemon parses action → dispatches → serialises response
  d. Daemon writes response line back to socket
  e. Concurrently: daemon reads broadcast events and forwards
     matching ones to client
6. Optional: client sends "subscribe" to register event patterns
7. Client sends "disconnect" → daemon responds "disconnected" → break
  OR client closes socket → read returns 0 → break
8. Daemon task exits, connection state dropped
```

### 9. Module Map

```
src/
├── main.rs         — Entry point: parses args, dispatches to daemon or client mode
├── lib.rs          — DaemonState, ConnectionState, module declarations
├── daemon.rs       — Unix socket listener, client handler, message dispatch loop
├── protocol.rs     — Action enum, Envelope, response/event types, JSON (de)serialisation
├── cli.rs          — CLI argument parsing (subcommands: daemon, status, stop, restart, install, setup)
├── client.rs       — Embedded client mode (reads NDJSON from stdin, sends to daemon)
├── capture.rs      — Screenshot helpers (grim on GNOME/Hyprland, spectacle+convert on KDE)
└── backend/
    ├── mod.rs      — DesktopBackend trait definition, create_backend() factory, desktop detection
    ├── gnome.rs    — GNOME backend implementation (DBus, CLI wrappers, file watching)
    ├── hyprland.rs — Hyprland backend implementation (hyprctl JSON, ydotool, grim)
    └── kde.rs      — KDE backend implementation (KWin D-Bus scripting, ydotool, spectacle)
```

### 10. Key Design Decisions

- **Async throughout** — built on `tokio` with async trait methods, async CLI execution, and async socket I/O. No blocking calls in the hot path.
- **No auth** — the Unix socket's filesystem permissions are the security boundary. Only the owning user can connect. There is no API key, token, or authentication layer.
- **Backend-optional startup** — the daemon starts even without a backend, so it can respond to `ping` and manage connections. Desktop features are absent but the daemon doesn't crash.
- **CLI-first for certain operations** — `nmcli` for Wi-Fi connect, `pactl` for audio, `notify-send` for notifications — these tools are well-tested, handle edge cases the daemon shouldn't replicate, and are forward-compatible across desktop environments.
- **gdbus for extension, zbus for system services** — the GNOME Shell extension is called through the `gdbus` CLI because it runs on the session bus and GNOME Shell's GIO DBus implementation; system services (UPower, NetworkManager, BlueZ) use the `zbus` Rust crate for type-safe, async DBus calls.
