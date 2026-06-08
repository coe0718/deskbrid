# Deskbrid v2 — Build Plan

## Constraint
**GNOME 46+ only.** No KDE, no wlroots, no version-guard garbage. If you're not on GNOME 46+, you don't get deskbrid. Keeps the codebase 3x smaller and the mental model clean.

---

## Phase 0: Scaffold

**Goal:** Binary boots, parses args, daemon sits on a socket.

- `cargo init deskbrid --bin`
- Dependencies: `tokio`, `clap`, `serde`, `serde_json`, `zbus`, `tracing`, `anyhow`, `uuid`
- CLI subcommand tree:
  ```
  deskbrid daemon          # start the server
  deskbrid status          # is it running?
  deskbrid <domain> <cmd>  # one-shot client commands
  ```
- `clap` subcommand enum with all domains (windows, input, clipboard, screenshot, etc.)
- Duplicate the v1 `lib.rs` module structure but strip every backend file
- Socket listener: `/run/user/$UID/deskbrid.sock`, tokio `UnixListener`, per-client read task + write channel
- NDJSON line reader: `BufReader` + `read_line` loop, max 1 MiB per line
- `Message` enum: all action types (serde deserialize), all response/event types (serde serialize)
- No backends yet — daemon accepts connections and responds with `NOT_SUPPORTED` to everything

**Files:** main.rs, cli.rs, protocol.rs, lib.rs

---

## Phase 1: Protocol Core

**Goal:** Full message type definitions, request routing, response dispatching.

- `Action` enum — all 37 action types with their params. One variant per action.
- `Response` enum — success + error variants with data shapes
- `Event` enum — all 29 event types with data shapes
- `ConnectionState` per client — subscription patterns, hotkey registrations, file watches, tracked processes
- Request router: `match action { Action::WindowsList => ..., Action::Screenshot => ... }`
- Each handler calls into the appropriate backend trait method
- NDJSON serializer/deserializer with `serde_json::from_reader` (streaming per-line)
- CLI client module: connects to socket, sends action JSON, reads response, prints to stdout
- `deskbrid windows list` → sends `{"type":"windows.list"}` → gets JSON back → prints formatted

**This is the most important phase.** Get the routing right and everything else snaps in cleanly.

**Files:** protocol.rs (message types), router.rs (action dispatch), client.rs (CLI → socket bridge)

---

## Phase 2: GNOME Backend (from scratch, GNOME 46)

**Goal:** Every action and event wired to a real GNOME 46 desktop.

### 2a — DesktopBackend Trait

```rust
#[async_trait]
pub trait DesktopBackend: Send + Sync {
    // Windows
    async fn windows_list(&self) -> Result<Vec<WindowInfo>>;
    async fn window_focus(&self, id: &str) -> Result<()>;
    async fn window_get(&self, id: &str) -> Result<WindowInfo>;

    // Workspaces
    async fn workspaces_list(&self) -> Result<Vec<WorkspaceInfo>>;
    async fn workspace_switch(&self, id: u32) -> Result<()>;

    // Input (Mutter RemoteDesktop)
    async fn keyboard_key(&self, key: &str, pressed: bool) -> Result<()>;
    async fn mouse_move(&self, x: f64, y: f64) -> Result<()>;
    async fn mouse_button(&self, button: &str, pressed: bool) -> Result<()>;
    async fn mouse_scroll(&self, dx: f64, dy: f64) -> Result<()>;

    // System
    async fn system_info(&self) -> Result<SystemInfo>;
    async fn idle_seconds(&self) -> Result<u64>;
    async fn power_action(&self, action: &str) -> Result<()>;
    async fn battery_status(&self) -> Result<Vec<BatteryInfo>>;

    // Events
    async fn subscribe(&self, patterns: &[String]) -> Result<()>;
    async fn unsubscribe(&self, patterns: &[String]) -> Result<()>;
}
```

Separate traits for the subsystems that have heavier APIs:

```rust
pub trait InputBackend: Send + Sync { /* keyboard + mouse */ }
pub trait CaptureBackend: Send + Sync { /* screenshots */ }
pub trait ClipboardBackend: Send + Sync { /* read + write */ }
pub trait NotificationBackend: Send + Sync { /* send + monitor */ }
```

### 2b — GNOME Backend Implementation

Everything through DBus (zbus 5.x). No shell scripts, no pipewire yet.

| Feature | DBus Interface | Notes |
|---|---|---|
| Window list | `org.deskbrid.WindowManager` (extension) | Keep the extension from v1, port to GNOME 46 |
| Window focus | `org.deskbrid.WindowManager` | Same extension |
| Input injection | `org.gnome.Mutter.RemoteDesktop` | Works cleanly on 46, absolute positioning works |
| Workspace control | `org.gnome.Shell.Extensions.deskbrid` | Extension provides workspace methods |
| Screenshots | `org.freedesktop.portal.Screenshot` | Portal API, no PipeWire needed for phase 2 |
| Notifications (send) | `org.freedesktop.Notifications` | Standard DBus |
| Notifications (monitor) | `org.freedesktop.Notifications` /watch | Subscribe to NotificationClosed + ActionInvoked |
| Clipboard | wl-paste/wl-copy | Same as v1 — works fine, skip rewrites |
| System info | logind + hostnamed + `org.gnome.Shell` | DBus introspection |
| Battery | UPower DBus | `org.freedesktop.UPower` |
| Idle detection | `org.gnome.Mutter.IdleMonitor` | GNOME 46 has this |
| Power actions | logind (`org.freedesktop.login1`) | Inhibit + suspend/reboot/shutdown |
| Global hotkeys | GNOME Shell extension | Add to the existing extension |

### 2c — Event Monitoring

Background tokio tasks that watch DBus signals and push events to subscribed clients:

- `WindowMonitorTask` — watches EWMH/extension signals → `window.focus_changed`, `window.opened`, etc.
- `ClipboardMonitorTask` — watches wl-paste → `clipboard.changed`
- `NotificationMonitorTask` — watches `org.freedesktop.Notifications` → `notification.received`
- `IdleMonitorTask` — watches Mutter idle → `system.idle_changed`
- `PowerMonitorTask` — watches UPower → `power.battery_changed`, `power.cable_changed`
- `NetworkMonitorTask` — watches NetworkManager → `network.connectivity_changed`
- `FileWatchTask` — not DBus, uses `inotify` via `notify` crate → `files.changed`

Each task checks: "does any connected client have a subscription matching this event?" If yes, push.

---

## Phase 3: PipeWire Screencast

**Goal:** Real-time screen capture via PipeWire instead of the portal fallback.

- `--features pipewire` feature flag (same as v1)
- This time: PipeWire 1.0.5 with working 0.8 Rust crate bindings
- Screencast session: `org.freedesktop.portal.ScreenCast` → PipeWire stream → DMA buffer → PNG
- On success: replace the portal fallback with PipeWire
- On failure: fall back to portal (same as v1)

**We do this last** because the portal fallback already works. PipeWire is the upgrade, not the dependency.

---

## Phase 4: Files + Processes + Network + BT + Location

**Goal:** The extras that make deskbrid more than just a window manager.

| Feature | Mechanism | Priority |
|---|---|---|
| File watching | `notify` crate (inotify) | High — agents need this |
| File search | `fd` or `find` subprocess | High — script-friendly |
| Process start | tokio `Command` | High — "agent runs build" |
| Network status | NetworkManager DBus | Medium |
| Wi-Fi scan/connect | NetworkManager DBus + nmcli | Medium |
| BT list/scan/connect | BlueZ DBus (`org.bluez`) | Low — nice to have |
| Location | geoclue DBus or geoip fallback | Low — novelty |

**Start with file watching + process start.** Those are the ones agents actually need. Network and BT can wait.

---

## Phase 5: Polish

- systemd user service unit
- Graceful shutdown (SIGTERM → close all sockets → save state)
- `deskbrid status` — PID file check, uptime, client count
- Logging: structured tracing, `DESKBRID_LOG=debug` env var
- Error messages that don't suck
- CI: clippy + fmt + build

---

## What We Keep From v1

| Component | Keep? | Why |
|---|---|---|
| protocol design (JSON/Unix socket) | ✅ | Solid, battle-tested |
| GNOME Shell extension | ✅ | Works on GNOME 46, just needs workspace + hotkey additions |
| `capture.rs` (portal fallback) | ✅ | Works, keep as fallback even after PipeWire |
| `screenshot_portal.py` | ✅ | Works, keep |
| `clipboard.rs` (wl-paste polling) | ✅ | Works, replace with --watch if it's fixed in 24.04 |
| `gnome.rs` (947 lines of version-guard hell) | ❌ | Rewrite from scratch for GNOME 46 |
| `screencast.rs` (broken pipewire) | ❌ | Rewrite for PipeWire 1.0.5 |
| `input.rs` (relative pointer fallback) | ❌ | Rewrite — GNOME 46 supports absolute positioning natively |
| `backend/detect.rs` | ❌ | Not needed — GNOME 46+ only |
| `backend/kde.rs`, `backend/wlroots.rs` | ❌ | Not in scope |
| `backend/types.rs` | ❌ | Rewrite with cleaner types |
| `backend/audio.rs` | ❌ | Rewrite for PipeWire 1.0.5 |
| `config.rs` | ❌ | Simplify — no config needed in v1 |

---

## Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
zbus = "5"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1"
uuid = { version = "1", features = ["v4"] }
notify = "7"          # file watching (phase 4)
chrono = "0.4"        # timestamps

[features]
pipewire = ["dep:pipewire", "dep:spa", "dep:image"]
```

---

## Milestones

| # | Phase | What Works | Est. |
|---|---|---|---|
| 0 | Scaffold | Binary boots, accepts connections, responds NOT_SUPPORTED | 1 session |
| 1 | Protocol Core | CLI sends actions, daemon routes, returns responses | 1 session |
| 2a | GNOME backend | Window list/focus, workspaces, screenshots, clipboard, system info | 2 sessions |
| 2b | Event monitoring | Window focus/clipboard/notification/network events pushed to subscribers | 1 session |
| 2c | Input injection | Keyboard type/key/combo, mouse move/click/scroll | 1 session |
| 2d | Power + battery | Suspend/lock/shutdown, battery monitoring, idle detection | 1 session |
| 3 | PipeWire | Screencast via PipeWire with portal fallback | 1 session |
| 4 | Files + processes | File watching, file search, process launch | 1 session |
| 4b | Network + BT | Status, wifi scan/connect, BT list/scan/connect | 1 session |
| 5 | Polish | systemd unit, logging, error handling, CI | 1 session |

**Total: ~10 sessions** to a fully functional v2.
