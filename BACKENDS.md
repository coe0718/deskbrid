# Adding Desktop Environment Support to Deskbrid

**Scope:** Technical reference for implementing new `DesktopBackend` variants.
**Current backends:** GNOME, KDE Plasma, Hyprland, COSMIC, X11 (generic — covers Xfce/MATE/Cinnamon/i3/bspwm/etc.)
**Patterns used:** CLI subprocess, D-Bus (zbus), Wayland protocol helper binary, X11 XTest

---

## Architecture

Every backend lives in `src/backend/<name>/mod.rs` and implements the `DesktopBackend`
trait from `src/backend/mod.rs`. The trait has ~50 methods across 15 domains:

| Domain | Methods | Notes |
|--------|---------|-------|
| Windows | `list`, `focus`, `get`, `close`, `minimize`, `maximize`, `move_resize` | Core requirement |
| Workspaces | `list`, `switch`, `move_window` | Most compositors have this |
| Input | `keyboard_type`, `keyboard_key`, `keyboard_combo`, `mouse_move`, `click`, `scroll` | Usually via `ydotool` or `xdotool` |
| Clipboard | `read`, `write` | `wl-copy`/`wl-paste` on Wayland, `xclip` on X11 |
| Screenshot | `screenshot(monitor, region, window_id)` | `grim` on wlroots/Wayland, various others |
| Notifications | `send`, `close` | `notify-send` universal |
| System | `info`, `idle_seconds`, `power_action`, `battery_status` | OS-level, not DE-specific |
| Network | `status`, `interfaces`, `wifi_scan`, `wifi_connect` | `nmcli` universal |
| Bluetooth | `list`, `scan`, `stop_scan`, `connect`, `disconnect` | `bluetoothctl` universal |
| Files | `watch`, `unwatch`, `search` | `notify` crate + CLI |
| Audio | `list_sinks`, `set_sink_volume` | `pactl` universal |
| Monitor | `set_primary`, `set_resolution`, `set_scale`, `set_rotation`, `set_enabled` | Tool varies per DE |

### Three Implementation Patterns

**1. CLI Subprocess** — For compositors with a command-line IPC tool.
- Hyprland: `hyprctl clients -j`, `hyprctl dispatch focuswindow address:<id>`
- Sway: `swaymsg -t get_tree`, `swaymsg focus`
- niri: `niri msg --json windows`, `niri msg focus-window`
- Wayfire: `wf-ipc` or `wayfire-rs`

Pattern in code: `src/backend/hyprland/mod.rs` — `self.hyprctl_json(args)` /
`self.hyprctl_dispatch(dispatch)` / `self.sh(cmd, args)`

**2. D-Bus (zbus)** — For full desktop environments with D-Bus services.
- GNOME: `org.deskbrid.WindowManager` extension + Mutter `RemoteDesktop`
- KDE: KWin `Scripting` via `dbus-send` + `qdbus6`
- Budgie: Budgie WM / Mutter-based, could use GNOME extension approach
- Deepin DDE: `com.deepin.dde.daemon` / `com.deepin.wm`

Pattern in code: `src/backend/gnome/mod.rs` — `self.conn.call_method(...)`

**3. Wayland Protocol Helper Binary** — For compositors that only speak Wayland
protocols (no IPC, no D-Bus). A separate Rust binary opens a Wayland connection,
binds protocols like `ext_foreign_toplevel_list_v1`, and exposes the result as
JSON-over-stdout.

- COSMIC: `cosmic-helper` binary with `ZcosmicToplevelManagerV1`
- Labwc: wlr-foreign-toplevel-management only (no IPC)
- River: river-window-management-v1 protocol
- Sway/niri/Hyprland *could* also use this, but their IPC is faster

Pattern in code: `src/bin/cosmic_helper.rs`, `src/backend/cosmic/mod.rs` calling
`self.helper_json(args)`

---

## Priority Ranking

| Priority | DE | Pattern | Users | Effort | Key Reason |
|----------|----|---------|-------|--------|------------|
| **P0** | Sway | CLI (swaymsg) | Very high (wlroots flagship) | DONE v0.7.0 | `swaymsg` CLI — full DesktopBackend, 790 lines, 3 tests |
| **P0** | **Cinnamon** (X11 Fix) | **X11 backend + wmctrl** | **Very high (Linux Mint #1 distro)** | **1-2 days** | **Just need `windows_list` via wmctrl. Fixes ALL X11 DEs.** |
| **P1** | Niri | CLI (niri msg) + Rust crate | Growing fast (Rust ecosystem) | 1-2 days | `niri-ipc` crate, JSON IPC. Low effort. |
| **P1** | Wayfire | CLI (wf-ipc) + Rust crate | Moderate (wlroots 3D) | DONE v0.7.0 | `wf-ipc` CLI — full DesktopBackend, 645 lines |
| **P1** | **Cinnamon** (Full) | **Cinnamon JS extension + D-Bus** | **Very high** | **3-4 days** | **Muffin = Mutter fork. Same pattern as GNOME extension.** |
| **P2** | Labwc | Helper binary (wlr-ftm) | Moderate (stacking WM) | DONE v0.7.0 | Helper binary + DesktopBackend, 915 lines, stub commands |
| **P3** | Budgie 10.10 | D-Bus / GNOME extension adj. | Significant | 3-5 days | Uses Mutter — might share GNOME code |
| **P3** | Deepin DDE | D-Bus (dde-daemon) | Significant (China market) | 4-6 days | Two compositors (KWin fork → Treeland) |
| **P3** | **MATE** | **X11 backend + wmctrl (shared)** | **Significant (Linux Mint)** | **—** | **Already covered by X11 backend. Needs wmctrl windows_list.** |
| **P4** | River | Helper binary (river-wm-v1) | Niche (Zig compositor) | 3-5 days | No IPC, Wayland protocols only |
| **P4** | LXQt 2.x | X11 backend / Labwc helper | Moderate | — | X11 version covered by X11 backend |
| **P4** | Enlightenment | Helper binary (EFL IPC) | Niche | 5-7 days | Unique EFL architecture |

---

## Cinnamon Backend (P0/P1)

**Status:** Currently routes to X11 backend. `windows_list` returns empty. Muffin = Mutter fork with JS extension system.
**Effort:** 1-2 days for X11 fix (P0), 3-4 days for full extension (P1).

### Detection

```rust
// src/backend/mod.rs — already routes to X11:
if lower.contains("x11") || lower.contains("xfce")
    || lower.contains("mate") || lower.contains("cinnamon")
{
    return DesktopEnv::X11;
}
```

Currently correct for X11 session. When Cinnamon Wayland stabilizes (targeting 2026),
add a dedicated `DesktopEnv::Cinnamon` variant.

### Architecture

Cinnamon uses **Muffin**, a hard fork of Mutter (GNOME's compositor library).
Cinnamon itself is a monolithic JavaScript process (`cjs`, SpiderMonkey-based)
that embeds libmuffin — same architecture as GNOME Shell embedding libmutter.

Two implementation tracks:

#### Track A: X11 Backend Improvement (P0 — 1-2 days)

Cinnamon's X11 session is a standard EWMH/NetWM-compliant X11 window manager.
The X11 backend already handles input (`xdotool`), clipboard (`xclip`),
screenshots (`import`), and notifications (`notify-send`). The missing piece is
`windows_list`.

`wmctrl -lGpx` output format:
```
0x03e00003  0  1234  0    0    1920  1080  Navigator.firefox  host  Mozilla Firefox
│           │  │     │    │    │     │     │                  │     └── Window title
│           │  │     │    │    │     │     │                  └── Client machine / host
│           │  │     │    │    │     │     └── WM_CLASS (instance.class; app_id from class)
│           │  │     │    │    │     └── Height
│           │  │     │    │     └── Width
│           │  │     │    └── Y position
│           │  │     └── X position
│           │  └── PID (`-p`)
│           └── Desktop number (-1 = sticky)
└── Window ID (hex)
```

Add to `src/backend/x11/helpers.rs`:
```rust
pub(super) async fn list_windows_wmctrl() -> anyhow::Result<Vec<protocol::WindowInfo>> {
    let out = Command::new("wmctrl")
        .args(["-lGpx"])
        .output().await?;
    // Parse id, desktop, pid, geometry, WM_CLASS, host, title
}
```

**This benefits ALL X11 DEs at once:** Xfce, MATE, Cinnamon, i3, bspwm, openbox,
fluxbox, etc. The only gap is `is_minimized` and `is_focused` flags (wmctrl
doesn't expose them), but `workspace_id`, `geometry`, `pid`, `app_id`, and
`title` are all available.

#### Track B: Cinnamon Extension (P1 — 3-4 days)

Cinnamon extensions are JavaScript modules loaded into the cinnamon process via
`cjs` (SpiderMonkey). They have access to Muffin's internal API through
GObject introspection — exactly the same pattern as GNOME Shell extensions.

Create `extensions/cinnamon/deskbrid@deskbrid/`:

```javascript
// extension.js — same pattern as the GNOME extension
const { GLib, Gio, Meta, Shell } = imports.gi;

// Muffin window API (identical to Mutter's MetaWindow):
// global.display.get_tab_list(Meta.TabList.NORMAL_ALL, null)
// → array of MetaWindows with .title, .get_pid(), .get_wm_class(),
//   .get_geometry(), .is_minimized(), .has_focus(), .get_workspace()

// Export via D-Bus:
const DBusService = Gio.DBusExportedObject.wrapJSObject(
    `<node>
        <interface name="org.deskbrid.WindowManager">
            <method name="ListWindows">
                <arg type="s" direction="out"/>
            </method>
            <method name="FocusWindow">
                <arg type="u" direction="in"/>
            </method>
            <method name="CloseWindow">
                <arg type="u" direction="in"/>
            </method>
            <method name="MinimizeWindow">
                <arg type="u" direction="in"/>
            </method>
            <method name="MaximizeWindow">
                <arg type="u" direction="in"/>
            </method>
            <method name="MoveResizeWindow">
                <arg type="u" direction="in"/>
                <arg type="i" direction="in"/>
                <arg type="i" direction="in"/>
                <arg type="i" direction="in"/>
                <arg type="i" direction="in"/>
            </method>
            <method name="SwitchWorkspace">
                <arg type="u" direction="in"/>
            </method>
        </interface>
    </node>`,
    { /* method implementations */ }
);
DBusService.export(Gio.DBus.session, '/org/deskbrid/WindowManager');
```

Cinnamon extensions are installed to:
```
~/.local/share/cinnamon/extensions/deskbrid@deskbrid/
```

Enable via:
```
gsettings set org.cinnamon enabled-extensions "['deskbrid@deskbrid']"
```

Or via Cinnamon Settings → Extensions.

### What the extension unlocks

| Domain | Method | Muffin/Meta API |
|--------|--------|-----------------|
| Window list | `ListWindows` → JSON | `global.display.get_tab_list()` |
| Focus | `FocusWindow(id)` | `meta_window.activate(global.get_current_time())` |
| Close | `CloseWindow(id)` | `meta_window.delete(global.get_current_time())` |
| Minimize | `MinimizeWindow(id)` | `meta_window.minimize()` |
| Maximize | `MaximizeWindow(id)` | `meta_window.maximize()` |
| Move/Resize | `MoveResizeWindow(id,x,y,w,h)` | `meta_window.move_resize_frame()` |
| Workspace list | `ListWorkspaces` | `global.screen.get_workspaces()` |
| Switch workspace | `SwitchWorkspace(id)` | `workspace.activate(global.get_current_time())` |

### Setup (`src/setup.rs`)

```rust
DesktopEnv::Cinnamon => setup_cinnamon().await,
```

```rust
async fn setup_cinnamon() -> anyhow::Result<()> {
    // Install extension to ~/.local/share/cinnamon/extensions/
    // Enable via gsettings
}
```

### Dependencies

| What | Tool | Notes |
|------|------|-------|
| Window control (X11) | `xdotool` + `wmctrl` | Until extension is installed |
| Window control (full) | Cinnamon extension | Via D-Bus |
| Input | `xdotool` | X11 — no ydotoold needed |
| Clipboard | `xclip` | X11 |
| Screenshots | `import` (ImageMagick) | X11 |
| Notifications | `notify-send` | All DEs |

---

## MATE Backend (P3)

**Status:** Currently routes to X11 backend. No separate backend needed.
**Effort:** 0 days (already covered) + 1-2 days for shared `wmctrl` windows_list.

### Detection

Already handled:
```rust
if lower.contains("mate") { return DesktopEnv::X11; }
```

### Architecture

MATE uses **Marco**, a fork of Metacity (GNOME 2's window manager). Marco is a
pure X11 window manager with no JS extension system, no D-Bus window management
API, and no Wayland support.

**There is nothing to implement.** MATE is already fully served by the X11
backend. The only missing piece is `windows_list`, which is the same `wmctrl`
fix needed for Cinnamon and every other X11 DE.

### What already works

| Domain | Tool | Status |
|--------|------|--------|
| Window focus/close | `xdotool` | ✅ |
| Window minimize/maximize | `xdotool` + `wmctrl` | ✅ (via X11 backend) |
| Window move/resize | `xdotool` | ✅ |
| Keyboard input | `xdotool type` | ✅ |
| Mouse input | `xdotool mousemove/click` | ✅ |
| Clipboard | `xclip` | ✅ |
| Screenshots | `import` (ImageMagick) | ✅ |
| Workspace switch | `xdotool set_desktop` | ✅ |
| Notifications | `notify-send` | ✅ |
| Monitor control | `xrandr` | ✅ |
| **Window listing** | **wmctrl** | ✅ |

### The wmctrl Fix (Shared X11 Improvement)

Add `wmctrl`-based window listing to `src/backend/x11/helpers.rs`. This
single change makes `windows_list` work for:

- **Cinnamon** (Linux Mint flagship)
- **MATE** (Linux Mint alternative)
- **Xfce** (Xubuntu)
- **i3 / bspwm / herbstluftwm** (tiling WMs)
- **Openbox / Fluxbox / JWM** (stacking WMs)
- **Any other EWMH-compliant X11 window manager**

Parsing `wmctrl -lGpx`:
```rust
// Window ID    Desktop  PID   X   Y   W     H     WM_CLASS           Host         Title
// 0x03e00003   0        1234  0   0   1920  1080  Navigator.firefox workstation  Mozilla Firefox
pub(super) fn parse_wmctrl_line(line: &str) -> Option<protocol::WindowInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 9 { return None; }
    let id = parts[0].to_string();
    let ws = parts[1].parse::<i32>().ok().map(|d| if d < 0 { 0 } else { d as u32 });
    let pid = parts[2].parse::<u32>().ok();
    let x = parts[3].parse::<i32>().ok()?;
    let y = parts[4].parse::<i32>().ok()?;
    let w = parts[5].parse::<u32>().ok()?;
    let h = parts[6].parse::<u32>().ok()?;
    // parts[7] is WM_CLASS, parts[8] is the client machine, parts[9..] is the title.
    let app_id = parts[7].rsplit('.').next().unwrap_or(parts[7]).to_ascii_lowercase();
    // ...
}
```

### Detection detail

Since MATE doesn't need its own backend, the current detection logic is correct.
If MATE ever adds Wayland support (no plans announced), revisit with a dedicated
backend then.

**Status:** Implemented in the shared X11 backend. **Effort:** Done.

### Detection

```rust
// Add to detect_desktop() in src/backend/mod.rs
if lower.contains("sway") {
    return DesktopEnv::Sway;
}
// Process fallback
if pgrep("sway") { return DesktopEnv::Sway; }
```

### API surface

Sway exposes an IPC protocol over `$SWAYSOCK`. Two access methods:

**A) `swaymsg` CLI** (simplest, matches Hyprland pattern):
```
swaymsg -t get_tree              # JSON tree: windows, workspaces, outputs
swaymsg -t get_workspaces         # workspace list
swaymsg -t get_outputs            # monitor list
swaymsg -t get_inputs             # input devices
swaymsg focus                     # focus window by criteria
swaymsg [con_id=<id>] focus       # focus specific window
swaymsg [con_id=<id>] kill        # close
swaymsg [con_id=<id>] move scratchpad  # minimize
swaymsg fullscreen               # toggle maximize
swaymsg move position <x> <y>    # move
swaymsg resize set <w> <h>       # resize
swaymsg workspace <n>            # switch workspace
swaymsg move container to workspace <n>  # move window
swaymsg output <name> resolution <w>x<h> # monitor
```

**B) `swayipc` crate** (Rust, no subprocess overhead):
```toml
[dependencies]
swayipc = "3"
```

```rust
use swayipc::Connection;

let mut conn = Connection::new()?;
let tree = conn.get_tree()?;       // returns serde_json::Value
let workspaces = conn.get_workspaces()?;
```

### Implementation plan

Create `src/backend/sway/` with:
- `mod.rs` — `SwayBackend` struct + impl
- `helpers.rs` — swaymsg JSON parsing, swayipc wrapper

Share with Hyprland backend for: input (`ydotool`), clipboard (`wl-copy`/`wl-paste`),
screenshot (`grim`), notifications (`notify-send`), audio (`pactl`), network (`nmcli`),
bluetooth (`bluetoothctl`).

The `get_tree` JSON has this structure:
```json
{
  "id": 1,
  "name": "root",
  "type": "root",
  "nodes": [
    {
      "id": 2,
      "name": "__i3",
      "type": "output",
      "nodes": [
        {
          "id": 3,
          "name": "1",
          "type": "workspace",
          "nodes": [
            {
              "id": 42,
              "name": "Firefox",
              "type": "con",
              "app_id": "firefox",
              "pid": 1234,
              "focused": true,
              "rect": { "x": 0, "y": 0, "width": 1920, "height": 1080 }
            }
          ]
        }
      ]
    }
  ]
}
```

---

## Niri Backend (P1)

**Status:** Not implemented. **Effort:** 1-2 days.

### Detection

```rust
if lower.contains("niri") { return DesktopEnv::Niri; }
if pgrep("niri") { return DesktopEnv::Niri; }
```

### API surface

Niri has a JSON IPC socket at `$NIRI_SOCKET`. Two access methods:

**A) `niri msg --json` CLI**:
```
niri msg --json windows
niri msg --json workspaces
niri msg --json outputs
niri msg --json focused-window
niri msg focus-window <id>
niri msg close-window <id>
niri msg set-window-column-width <id> <width>
niri msg switch-workspace <id>
niri msg focus-window-left/right/up/down
niri msg move-window-to-workspace <id>
niri msg event-stream                       # continuous events
```

**B) `niri-ipc` crate** (Rust, native):
```toml
niri-ipc = "=26"  # pin exact — not semver-stable
```

```rust
use niri_ipc::socket::Socket;
use niri_ipc::{Request, Action};

let mut sock = Socket::new()?;
let windows = sock.send(Request::Windows)??;
let output = sock.send(Request::Outputs)??;
sock.send(Request::Action(Action::FocusWindow { id: Some(window_id.into()) }))?;
```

Key types:
- `niri_ipc::Window` — `id`, `title`, `app_id`, `pid`, `is_focused`, `is_maximized`,
  `workspace_id`, `geometry: Rect { x, y, w, h }`
- `niri_ipc::Output` — `id`, `name`, `current_mode: Option<Mode { width, height, refresh_rate }>`
- `niri_ipc::Workspace` — `id`, `name`, `is_active`, `output: Option<String>`

### Notes

- Niri is scrollable-tiling. No minimize concept (windows are columns). Maximize
  means full-width column.
- `niri-ipc` follows niri version (not semver) — pin exact version or use `swaymsg`-style CLI wrapper.
- Input/shared deps same as Sway/Hyprland.

---

## Wayfire Backend (P1)

**Status:** Not implemented. **Effort:** 2-3 days.

### Detection

```rust
if lower.contains("wayfire") { return DesktopEnv::Wayfire; }
if pgrep("wayfire") { return DesktopEnv::Wayfire; }
```

### API surface

Wayfire 0.9+ has an IPC socket at `$WAYFIRE_SOCKET` or `~/.wayfire/ipc-socket-<id>`.

**A) `wf-ipc` CLI (ships with Wayfire)**:
```
wf-ipc -j                                 # list views as JSON
wf-ipc -j view-info <id>                  # single view
wf-ipc set-view-options <id> minimized    # minimize
wf-ipc close-view <id>                    # close
wf-ipc set-view-workspace <id> <ws>       # move to workspace
wf-ipc focus-view <id>                    # focus
wf-ipc set-workspace <n>                  # switch workspace
wf-ipc list-outputs -j                    # monitors
```

**B) `wayfire-rs` crate** (Rust):
```toml
wayfire-rs = "0.2"
```

```rust
use wayfire_rs::WayfireConnection;

let wf = WayfireConnection::new()?;
let views = wf.list_views()?;
wf.focus_view(&view_id)?;
wf.close_view(&view_id)?;
```

### Notes

- Wayfire is wlroots-based. Shares all shared deps with Sway (grim, ydotool, wl-clipboard).
- Wayfire has no native maximize/minimize — it's a stacking compositor with plugins.
  Use the `window-rules` plugin or `wf-ipc set-view-options`.
- Monitor control via `wlr-randr` or Wayfire IPC output commands.

---

## Labwc Backend (P2)

**Status:** Not implemented. **Effort:** 3-5 days.

**Key constraint:** Labwc has NO external IPC protocol. Zero. No swaymsg, no D-Bus,
no custom socket. The sole control path is Wayland protocols — specifically
`wlr-foreign-toplevel-management-v1`.

This means Labwc **requires** a helper binary approach, exactly like COSMIC.

### Detection

```rust
if lower.contains("labwc") { return DesktopEnv::Labwc; }
if pgrep("labwc") { return DesktopEnv::Labwc; }
```

### Implementation plan

Create `src/bin/labwc_helper.rs` (similar to `cosmic_helper.rs`):

```rust
// Bind wlr-foreign-toplevel-management-v1 protocols
// Expose CLI: labwc-helper list-windows, labwc-helper focus <id>, etc.
```

Protocols needed:
- `zwlr_foreign_toplevel_manager_v1` — list/monitor windows
- `zwlr_foreign_toplevel_handle_v1` — `set_maximized()`, `set_minimized()`,
  `set_fullscreen()`, `activate()`, `close()`
- `wl_output` — monitor info
- `wl_seat` — keyboard state (for input)

Screenshots via `grim` (wlroots). Input via `ydotool`. Clipboard via `wl-clipboard`.

### Notes

- Labwc has virtual desktop (workspace) support via the `ws` config, but no
  protocol-level workspace IPC. Workspace detection would need the helper binary
  to track `zwlr_foreign_toplevel_handle_v1.output_enter/leave` events.
- Labwc 0.8+ has a `labwc-reconfigure` command that accepts SIGHUP — no IPC though.

---

## Budgie Backend (P3)

**Status:** Not implemented. **Effort:** 3-5 days.

### Detection

```rust
if lower.contains("budgie") { return DesktopEnv::Budgie; }
if pgrep("budgie-wm") { return DesktopEnv::Budgie; }
```

### Architecture

Budgie 10.10 runs on Wayland. The window manager is `budgie-wm` which is built on
Mutter (GNOME's compositor library). The compositor uses `budgie-desktop-services`
for D-Bus IPC.

**Two implementation paths:**

**A) GNOME Extension approach (reuse GNOME backend):**
Since `budgie-wm` is Mutter-based, the existing GNOME Shell extension's D-Bus
interface (`org.deskbrid.WindowManager`) might work if Budgie exposes it.
Test: does `gdbus introspect --session --dest org.deskbrid.WindowManager` work?

**B) Budgie-specific D-Bus approach:**
Budgie exposes its own D-Bus API for window management:
```
com.solus-project.budgie.wm
```
Or via `org.buddiesofbudgie.Budgie` (newer versions).

Window listing/control would use Mutter's `MetaWindow` D-Bus API directly:
```
org.gnome.Mutter.DisplayCore
org.gnome.Mutter.Window
```

### Notes

- Budgie 11 is being rewritten (different architecture). Focus effort on Budgie 10.10
  where the codebase is stable.
- For X11 Budgie (< 10.10), the X11 backend already covers it.
- Screenshot via `grim`. Input via `ydotool`. Clipboard via `wl-clipboard`.

---

## Deepin DDE Backend (P3)

**Status:** Not implemented. **Effort:** 4-6 days.

### Detection

```rust
if lower.contains("deepin") { return DesktopEnv::Deepin; }
if pgrep("dde-desktop") { return DesktopEnv::Deepin; }
// Also check: pgrep("treeland") for Deepin 25+
```

### Architecture

Deepin 23 uses `dde-kwin` (KWin fork) as compositor. Deepin 25+ uses **Treeland**
(wlroots + Qt Quick compositor). Both expose D-Bus services via `dde-daemon`.

**D-Bus interfaces (both versions):**
```
com.deepin.wm                  # Window management (list, focus, close, etc.)
com.deepin.daemon.Display     # Monitor management
com.deepin.daemon.Network     # NetworkManager wrapper
com.deepin.dde.daemon         # System settings (power, audio, input)
com.deepin.dde.osd            # OSD notifications
```

**For Treeland (Deepin 25+):**
- Treeland has Wayland protocol extensions: `ztreeland_toplevel_manager_v1`
- Also exposes `org.deepin.dde.Treeland1` on D-Bus for some operations
- Fallback: helper binary using wlr-foreign-toplevel-management

### Implementation plan

Priority: D-Bus via `zbus` since `dde-daemon` handles most operations:

```rust
// D-Bus: com.deepin.wm
// Methods: GetWindows(), GetActiveWindow(), FocusWindow(window_id),
//          MinimizeWindow(window_id), CloseWindow(window_id), SwitchWorkspace(id)
// Properties: WindowList, WorkspaceCount, CurrentWorkspace
```

Shared deps: `ydotool` (input), `grim` (screenshots), `wl-clipboard` (clipboard),
`notify-send` (notifications), `pactl` (audio), `nmcli` (network), `bluetoothctl` (BT).

---

## River Backend (P4)

**Status:** Not implemented. **Effort:** 3-5 days.

### Detection

```rust
if lower.contains("river") { return DesktopEnv::River; }
if pgrep("river") { return DesktopEnv::River; }
```

### API surface

River has NO external IPC. It uses Wayland protocols exclusively:

- `river_window_manager_v1` — the official window management protocol
  - `push_view()`, `pop_view()`, `modify_view()`, `focus_view()`, `close_view()`
  - `set_geometry_hint()`
- `river_layout_v1` — layout management
- `river_status_v1` — status (window list, tags, etc.)
- `ext_foreign_toplevel_list_v1` — alternative window listing

**Access:** Only via a Wayland client connection. Must use a helper binary
(same pattern as `cosmic_helper.rs`).

### Implementation plan

Create `src/bin/river_helper.rs`:
```rust
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::river::*;
```

The helper binary exposes CLI commands similar to the COSMIC helper:
```
river-helper list-windows     → JSON: [{id, title, app_id, focused, tags, ...}]
river-helper focus <id>       → success/fail
river-helper close <id>
river-helper set-tags <id> <tags>
```

River uses tags (like workspaces). Workspace IDs are tag bitmasks.

---

## wlroots Common Backend (Long-term)

**What:** A generic wlroots backend using `wr_foreign_toplevel_management_v1` protocol
that works on any wlroots compositor (Labwc, River, dwl, Wayfire without IPC, etc.)
**Effort:** 5-7 days for the helper binary.

Reuses the `cosmic-helper` pattern but with standard wlr protocols instead of
COSMIC-specific ones. The helper binary would be compiled as `wlroots-helper` and
feature-detect protocols at runtime.

This is a **medium-term investment** that replaces the need for individual helper
binaries per wlroots compositor. See `src/bin/cosmic_helper.rs` for the pattern.

---

## Changes Required Per New Backend

### 1. Detection (`src/backend/mod.rs`)
```rust
enum DesktopEnv {
    // Add variant
    Sway,
}

async fn detect_desktop() -> DesktopEnv {
    // Add to XDG_CURRENT_DESKTOP check
    if lower.contains("sway") { return DesktopEnv::Sway; }
    // Add to pgrep fallback
    if pgrep("sway") { return DesktopEnv::Sway; }
}

async fn create_backend(...) -> ... {
    match desktop {
        DesktopEnv::Sway => sway::SwayBackend::new(event_tx)
            .await.map(|b| Box::new(b) as Box<dyn DesktopBackend>),
        // ...
    }
}
```

### 2. Backend module (`src/backend/<name>/`)
```
src/backend/<name>/
├── mod.rs        # Backend struct + DesktopBackend impl + new()
├── helpers.rs    # JSON parsing, IPC helpers
```

### 3. Dependencies (`Cargo.toml`)
Optional features for crate-backed DEs:
```toml
[features]
sway = ["dep:swayipc"]
niri = ["dep:niri-ipc"]
wayfire = ["dep:wayfire-rs"]
```

For helper-binary DEs: no new crate deps, CLI-based.

### 4. Setup (`src/setup.rs`)
Add detection + setup messages for each new DE.

### 5. Capabilities (`src/daemon/capabilities/`)
- `mod.rs`: Add `apply_<name>_capability_overrides()` + insert checks
- `overrides.rs`: Add per-action requires/unsupported/session overrides

### 6. Health checks (`src/daemon/capabilities/mod.rs`)
Add `insert_<name>_deps()`:
```rust
async fn insert_sway_deps(deps: &mut ...) {
    deps.insert("swaymsg", check_in_path("swaymsg").await);
    deps.insert("grim", check_in_path("grim").await);
    deps.insert("ydotoold", check_process("ydotoold").await);
    // ...
}
```

---

## Shared Infrastructure (No DE-Specific Code Needed)

These domains are OS-level and don't vary per DE. New backends should call them
identically:

| Domain | Tool/API | Reusable From |
|--------|----------|---------------|
| Keyboard input | `ydotool` (Wayland) / `xdotool` (X11) | All wayland backends |
| Mouse input | `ydotool` (Wayland) / `xdotool` (X11) | All wayland backends |
| Clipboard | `wl-copy`/`wl-paste` (Wayland) / `xclip` (X11) | All backends |
| Screenshot | `grim` (wlroots/Wayland) / `gnome-screenshot` | All wlroots backends |
| Notifications | `notify-send` | All backends |
| Audio | `pactl` | All backends |
| Network/WiFi | `nmcli` | All backends |
| Bluetooth | `bluetoothctl` | All backends |
| File watching | `notify` crate (inotify) | All backends |
| Systemd/logind | `systemctl`, `loginctl`, `journalctl` | All backends |
| Polkit | `pkcheck` | All backends |

---

## Reference: Existing Backend LOC

| Backend | mod.rs | helpers.rs | Other | Total |
|---------|--------|------------|-------|-------|
| GNOME | 808 | 13 submodules (~200 ea.) | ~3400 | ~3400 |
| KDE | 1221 | 215 + tests 187 | ~1623 | ~1623 |
| Hyprland | 873 | free_functions 148 + helpers 269 | ~1290 | ~1290 |
| COSMIC | 700 | helpers 182 | ~882 + 439 helper | ~1300 |
| X11 | 413 | helpers 117 | ~530 | ~530 |

**Expected LOC for new backends:**
- CLI-based (Sway, niri): 400-600 LOC
- D-Bus-based (Budgie, Deepin): 600-800 LOC
- Helper binary (Labwc, River): 800-1200 LOC (including helper binary)
