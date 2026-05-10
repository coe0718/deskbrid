     1|# Deskbrid Architecture
     2|
     3|Deskbrid is a Unix socket daemon that provides programmatic control over the Linux desktop through a JSON-over-Unix-socket protocol. It acts as a bridge between client applications (AI agents, CLI tools, scripts) and desktop functionality — window management, input simulation, system queries, clipboard, audio, network, Bluetooth, and more.
     4|
     5|## Overview
     6|
     7|```
     8|┌─────────────────┐     NDJSON/Unix Socket     ┌──────────────────────┐
     9|│  Client Apps     │◄──────────────────────────►│  Deskbrid Daemon     │
    10|│  (Python, CLI,   │                            │                      │
    11|│   any lang)      │                            │  ┌────────────────┐  │
    12|│                  │                            │  │  GNOME Backend  │  │
    13|│                  │                            │  │  ┌────────────┐ │  │
    14|│                  │                            │  │  │  DBus Ext.  │ │  │
    15|│                  │                            │  │  │  (window mgmt)│  │
    16|│                  │                            │  │  ├────────────┤ │  │
    17|│                  │                            │  │  │  System DBus│ │  │
    18|│                  │                            │  │  │  (UPower, NM,│ │  │
    19|│                  │                            │  │  │   BlueZ)    │ │  │
    20|│                  │                            │  │  ├────────────┤ │  │
    21|│                  │                            │  │  │  CLI Tools  │ │  │
    22|│                  │                            │  │  │  (wtype,    │ │  │
    23|│                  │                            │  │  │  grim, wl-* │ │  │
    24|│                  │                            │  │  │  pactl...)  │ │  │
    25|│                  │                            │  │  └────────────┘ │  │
    26|│                  │                            │  └────────────────┘  │
    27|│                  │                            │                      │
    28|│                  │                            │  ┌────────────────┐  │
    29|│                  │                            │  │  Event         │  │
    30|│                  │  ◄─── events ──────────────│  │  Broadcast     │  │
    31|│                  │                            │  │  Channel       │  │
    32|│                  │                            │  └────────────────┘  │
    33|└─────────────────┘                            └──────────────────────┘
    34|```
    35|
    36|## Core Components
    37|
    38|### 1. Unix Socket Transport
    39|
    40|**Socket path:** `$XDG_RUNTIME_DIR/deskbrid.sock` (falls back to `/run/user/1000/deskbrid.sock`).
    41|
    42|The daemon starts by removing any leftover socket file, creating the parent directory, and binding a `tokio::net::UnixListener` on the path. On each incoming connection the listener spawns an asynchronous `handle_client` task. The socket path is predictable and scoped to the user's session, so only processes running under the same session user can connect.
    43|
    44|**Lifecycle:**
    45|1. Daemon starts → removes stale socket → binds listener
    46|2. Listener accepts connections in a loop, spawning one `tokio::spawn` task per client
    47|3. Each client task owns a split `(reader, writer)` pair on the socket
    48|4. On graceful disconnect, the client sends a `disconnect` action and the daemon responds with `disconnected`, then the read loop breaks
    49|5. On socket close (client drops), `reader.read_line()` returns `n == 0` and the task exits cleanly
    50|6. On daemon shutdown, the socket file remains (cleaned up next start)
    51|
    52|### 2. NDJSON Protocol
    53|
    54|All communication uses **NDJSON (Newline-Delimited JSON)** — one complete JSON object per line, terminated by `\n`. No framing, no length prefixes, no binary encoding. Every line is a self-contained message.
    55|
    56|**Message types:**
    57|
    58|| `type` field   | Direction       | Purpose                                |
    59||---------------|----------------|----------------------------------------|
    60|| `action`      | Client → Daemon | Execute a desktop action               |
    61|| `ping`        | Client → Daemon | Health check (no backend needed)       |
    62|| `subscribe`   | Client → Daemon | Subscribe to event patterns            |
    63|| `unsubscribe` | Client → Daemon | Unsubscribe from event patterns        |
    64|| `disconnect`  | Client → Daemon | Gracefully close the connection        |
    65|| `response`    | Daemon → Client | Action result (ok or error)            |
    66|| `connected`   | Daemon → Client | Sent immediately after socket connect  |
    67|| `disconnected`| Daemon → Client | Confirms graceful disconnect           |
    68|| `pong`        | Daemon → Client | Response to `ping`                     |
    69|| `event`       | Daemon → Client | Push event matching subscription       |
    70|
    71|**Request structure — flat key-value at the top level (not nested JSON-RPC):**
    72|
    73|```json
    74|{"type": "action", "id": "windows.list", "seq": 1, "action": {"windows.list": {}}}
    75|```
    76|
    77|The `action` field in the original message is optional — the daemon's `Action::from_json` parser reads `type` as the action discriminator. What matters is that each action type has a corresponding string value in the `type` field (e.g. `"windows.list"`, `"input.keyboard"`).
    78|
    79|**Response structure:**
    80|
    81|```json
    82|{
    83|  "type": "response",
    84|  "id": "action",
    85|  "seq": 1,
    86|  "status": "ok",
    87|  "data": [ ... ]
    88|}
    89|```
    90|
    91|On failure, `status` is `"error"` and an `error` object carries `code` and `message`:
    92|
    93|```json
    94|{
    95|  "type": "response",
    96|  "id": "action",
    97|  "seq": 1,
    98|  "status": "error",
    99|  "error": { "code": "INTERNAL_ERROR", "message": "no backend loaded" }
   100|}
   101|```
   102|
   103|**Error codes used:**
   104|- `INVALID_PARAMS` — malformed JSON or unknown action type
   105|- `INTERNAL_ERROR` — backend operation failed
   106|- `NOT_SUPPORTED` — no backend loaded
   107|
   108|**`connected` message** (sent immediately after socket connect, before any client message):
   109|
   110|```json
   111|{"type": "connected", "id": "server", "seq": 0, "data": {"version": "0.4.1", "protocol": "deskbrid-v2"}}
   112|```
   113|
   114|Clients should wait for this message before sending commands.
   115|
   116|### 3. Message Dispatch Flow
   117|
   118|The daemon's `handle_client` function runs a `tokio::select!` loop with two branches:
   119|
   120|1. **Event forwarding** — reads from the per-client MPSC channel attached to the broadcast receiver, checks the event type against the client's subscription set, and writes matching events to the socket
   121|2. **Client input** — reads one line from the socket, parses it into an `Action`, then dispatches:
   122|
   123|```
   124|Client line → Action::from_json() → match action {
   125|    Action::Ping          → respond with "pong"
   126|    Action::Subscribe     → insert patterns into conn.subscriptions
   127|    Action::Unsubscribe   → remove patterns from conn.subscriptions
   128|    Action::Disconnect    → respond with "disconnected", break
   129|    Action::FilesWatch    → track path in conn.watched_paths + dispatch
   130|    Action::FilesUnwatch  → remove path from conn.watched_paths + dispatch
   131|    _                     → dispatch_action(action, state, seq)
   132|}
   133|```
   134|
   135|`dispatch_action` locks the backend (read lock) and calls `execute_action` which pattern-matches on the `Action` variant and calls the corresponding `DesktopBackend` trait method. Results are serialised to a JSON response envelope. If no backend is loaded, it returns a `NOT_SUPPORTED` error.
   136|
   137|### 4. Daemon State
   138|
   139|Defined in `src/lib.rs` and shared across all connections via `Arc`:
   140|
   141|```rust
   142|pub struct DaemonState {
   143|    pub backend: Arc<RwLock<Option<Box<dyn backend::DesktopBackend>>>>,
   144|    pub event_tx: broadcast::Sender<DeskbridEvent>,
   145|}
   146|```
   147|
   148|- **backend** — wrapped in `RwLock` so multiple connections can dispatch concurrently. Only the daemon startup writes to it (inserting the loaded backend). Clients read it.
   149|- **event_tx** — a `tokio::sync::broadcast::channel(256)`. The GNOME backend holds a clone of the sender and pushes events into it. Each client connection subscribes to the broadcast and forwards matching events through an intermediate MPSC channel.
   150|
   151|**Per-connection state** (`ConnectionState`):
   152|
   153|```rust
   154|pub struct ConnectionState {
   155|    pub subscriptions: HashSet<String>,  // glob patterns like "file.*"
   156|    pub hotkeys: HashSet<String>,        // registered hotkey IDs
   157|    pub watched_paths: HashSet<String>,  // file watch paths
   158|}
   159|```
   160|
   161|### 5. Event Broadcast System
   162|
   163|Events flow through a three-stage pipeline:
   164|
   165|1. **Backend produces events** — the GNOME backend's `files_watch` method sets up a `notify` watcher on a directory. When `notify` fires, the callback constructs a `DeskbridEvent` and sends it through the `event_tx` broadcast sender.
   166|
   167|2. **Broadcast fan-out** — `event_tx.send()` distributes the event to all subscribed receivers. Each client connection holds a `broadcast::Receiver` that it obtained by calling `state.event_tx.subscribe()`.
   168|
   169|3. **Subscription matching** — each client task runs a forwarder that reads from the broadcast, serialises the event to JSON, and writes it to the per-client MPSC channel. The main `select!` loop reads from the MPSC receiver and checks the event type against `conn.subscriptions` using glob-style matching:
   170|
   171|```
   172|event_matches_any(subscriptions, event_type):
   173|
   174|- exact match: sub == event_type
   175|- prefix glob: sub = "file.*" matches "file.created", "file.modified", etc.
   176|- wildcard:    sub = "*" matches everything
   177|```
   178|
   179|**Supported events** (from `DeskbridEvent` enum):
   180|
   181|| Event type        | Fields                                    |
   182||-------------------|-------------------------------------------|
   183|| `file.created`    | `path: String`, `timestamp: u64`          |
   184|| `file.modified`   | `path: String`, `timestamp: u64`          |
   185|| `file.deleted`    | `path: String`, `timestamp: u64`          |
   186|| `file.renamed`    | `old_path: String`, `new_path: String`, `timestamp: u64` |
   187|
   188|The GNOME Shell extension also emits a `WindowStateChanged` DBus signal (debounced at 150ms), but this is not yet forwarded through the broadcast channel — it's available for future use.
   189|
   190|**Event envelope** (sent to client):
   191|
   192|```json
   193|{"type": "event", "id": "file.created", "data": {"event": "file.created", "path": "/tmp/test.txt", "timestamp": 1715000000}}
   194|```
   195|
   196|### 6. The GNOME Backend
   197|
   198|The GNOME backend (`src/backend/gnome.rs`) implements the `DesktopBackend` trait and uses four distinct integration strategies:
   199|
   200|#### 6a. GNOME Shell DBus Extension
   201|
   202|A custom GNOME Shell extension (`extensions/deskbrid@deskbrid/extension.js`) exposes a DBus interface at:
   203|
   204|- **Service:** `org.deskbrid.WindowManager`
   205|- **Object path:** `/org/deskbrid/WindowManager`
   206|- **Interface:** `org.deskbrid.WindowManager`
   207|
   208|**Methods:**
   209|
   210|| Method        | Input args                 | Output      | Purpose                        |
   211||---------------|---------------------------|-------------|--------------------------------|
   212|| `ListWindows` | none                      | `s` (JSON)  | Returns serialised window list |
   213|| `FocusedWindow` | none                   | `s` (JSON)  | Returns focused window info    |
   214|| `FocusWindow` | `app_id`, `title`, `exact` | `b` (bool) | Focus a window by app_id/title |
   215|
   216|**Signals:**
   217|
   218|| Signal               | Payload                              | Debounce |
   219||----------------------|--------------------------------------|----------|
   220|| `WindowStateChanged` | JSON string of focused window info   | 150ms    |
   221|
   222|The backend accesses this extension via `gdbus call` (CLI) rather than a direct `zbus` call — the `gdbus` CLI handles the session bus and GNOME-specific marshalling:
   223|
   224|```rust
   225|self.sh("gdbus", &[
   226|    "call", "--session",
   227|    "--dest", "org.deskbrid.WindowManager",
   228|    "--object-path", "/org/deskbrid/WindowManager",
   229|    "--method", "org.deskbrid.WindowManager.ListWindows"
   230|]).await?
   231|```
   232|
   233|The JSON returned by the extension is wrapped in gdbus's tuple format `('[json]',)` and parsed by `parse_extension_json_windows()`.
   234|
   235|#### 6b. System DBus (zbus)
   236|
   237|The backend uses the `zbus` crate to call system DBus services directly:
   238|
   239|- **UPower** (`org.freedesktop.UPower`) — battery status via `org.freedesktop.UPower.Device` properties (`Percentage`, `State`, `TimeToEmpty`)
   240|- **NetworkManager** (`org.freedesktop.NetworkManager`) — connectivity state, interface list, Wi-Fi access point scanning via `GetAllDevices`, device properties, and `org.freedesktop.NetworkManager.Device.Wireless` for AP lists
   241|- **BlueZ** (`org.bluez`) — Bluetooth device discovery via `ObjectManager.GetManagedObjects`, adapter management, and device connection/disconnection
   242|- **Mutter IdleMonitor** (`org.gnome.Mutter.IdleMonitor`) — idle time via `GetIdletime` on `/org/gnome/Mutter/IdleMonitor/Core`
   243|
   244|All zbus calls go through a shared `zbus::Connection` instance stored in the backend struct:
   245|
   246|```rust
   247|pub struct GnomeBackend {
   248|    conn: zbus::Connection,
   249|    event_tx: broadcast::Sender<DeskbridEvent>,
   250|    watchers: Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>,
   251|}
   252|```
   253|
   254|#### 6c. CLI Wrappers
   255|
   256|For desktop operations that lack a stable DBus API or are better handled by ecosystem CLIs:
   257|
   258|| Operation          | CLI tool(s)                        | Notes                                   |
   259||--------------------|------------------------------------|-----------------------------------------|
   260|| Keyboard input     | `wtype` (primary), `ydotool` (fallback) | Text typing and key combos          |
   261|| Mouse control      | `ydotool` mouse subcommands        | Move, click, scroll                     |
   262|| Screenshots        | `grim` + `slurp` (region/window)  | Outputs PNG to temp dir                 |
   263|| Clipboard read     | `wl-paste`                        | Requires wl-clipboard                   |
   264|| Clipboard write    | `wl-copy`                         | Requires wl-clipboard                   |
   265|| Notifications      | `notify-send`                     | Standard libnotify interface            |
   266|| Audio              | `pactl`                           | List sinks, set volume (PipeWire compat)|
   267|| Wi-Fi connect      | `nmcli`                           | Reliable connection setup               |
   268|| Idle time (fallback)| `loginctl` + `xssstate`          | Used when Mutter idle monitor is unavailable |
   269|
   270|The backend's `sh()` method runs CLI commands asynchronously via `tokio::process::Command` and returns stdout. The companion `sh_ok()` method checks if a command is available without error output.
   271|
   272|#### 6d. File Watching
   273|
   274|File system monitoring uses the `notify` crate, creating a `notify::RecommendedWatcher` per watched path. Watchers are stored in an `Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>` so they live for the duration of the backend. When a file event fires, the `notify` callback sends a `DeskbridEvent` through the broadcast channel.
   275|
   276|### 7. Backend Plugin System
   277|
   278|Backends implement the `DesktopBackend` trait defined in `src/backend/mod.rs`. The factory function `create_backend()` attempts to initialise an available backend:
   279|
   280|```rust
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
   288|```
   289|
   290|When the daemon starts, it calls `create_backend()`:
   291|
   292|- On success: stores the backend in `DaemonState.backend`, logs which backend loaded (e.g. "GNOME backend loaded", "Hyprland backend loaded", "KDE backend loaded")
   293|- On failure: logs a warning, continues without desktop features. All desktop actions return `NOT_SUPPORTED` errors
   294|
   295|The trait is designed to be implemented for other desktop environments (Sway, Xfce, Cinnamon, etc.) by adding a new backend module and updating `create_backend()`.
   296|
   297|### 8. Connection Lifecycle (Detailed)
   298|
   299|```
   300|1. Client connects to Unix socket
   301|2. Daemon receives connection in accept loop
   302|3. Daemon sends "connected" message with version info
   303|4. Client receives "connected" and knows it can send commands
   304|5. Loop:
   305|   a. Client sends JSON action line
   306|   b. Daemon increments seq counter
   307|   c. Daemon parses action → dispatches → serialises response
   308|   d. Daemon writes response line back to socket
   309|   e. Concurrently: daemon reads broadcast events and forwards
   310|      matching ones to client
   311|6. Optional: client sends "subscribe" to register event patterns
   312|7. Client sends "disconnect" → daemon responds "disconnected" → break
   313|   OR client closes socket → read returns 0 → break
   314|8. Daemon task exits, connection state dropped
   315|```
   316|
   317|### 9. Module Map
   318|
   319|```
   320|src/
   321|├── main.rs         — Entry point: parses args, dispatches to daemon or client mode
   322|├── lib.rs          — DaemonState, ConnectionState, module declarations
   323|├── daemon.rs       — Unix socket listener, client handler, message dispatch loop
   324|├── protocol.rs     — Action enum, Envelope, response/event types, JSON (de)serialisation
   325|├── cli.rs          — CLI argument parsing (subcommands: daemon, status, stop, restart, install, setup)
   326|├── client.rs       — Embedded client mode (reads NDJSON from stdin, sends to daemon)
├── capture.rs      — Screenshot helpers (grim on GNOME/Hyprland, spectacle+convert on KDE)
└── backend/
    ├── mod.rs      — DesktopBackend trait definition, create_backend() factory, desktop detection
    ├── gnome.rs    — GNOME backend implementation (DBus, CLI wrappers, file watching)
    ├── hyprland.rs — Hyprland backend implementation (hyprctl JSON, ydotool, grim)
    └── kde.rs      — KDE backend implementation (KWin D-Bus scripting, ydotool, spectacle)
   331|```
   332|
   333|### 10. Key Design Decisions
   334|
   335|- **Async throughout** — built on `tokio` with async trait methods, async CLI execution, and async socket I/O. No blocking calls in the hot path.
   336|- **No auth** — the Unix socket's filesystem permissions are the security boundary. Only the owning user can connect. There is no API key, token, or authentication layer.
   337|- **Backend-optional startup** — the daemon starts even without a backend, so it can respond to `ping` and manage connections. Desktop features are absent but the daemon doesn't crash.
   338|- **CLI-first for certain operations** — `nmcli` for Wi-Fi connect, `pactl` for audio, `notify-send` for notifications — these tools are well-tested, handle edge cases the daemon shouldn't replicate, and are forward-compatible across desktop environments.
   339|- **gdbus for extension, zbus for system services** — the GNOME Shell extension is called through the `gdbus` CLI because it runs on the session bus and GNOME Shell's GIO DBus implementation; system services (UPower, NetworkManager, BlueZ) use the `zbus` Rust crate for type-safe, async DBus calls.
   340|