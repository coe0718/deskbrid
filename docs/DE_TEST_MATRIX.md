# DE Test Matrix

Deskbrid protocol action support across 11 desktop environments.

> **Legend:** ✅ = Working &nbsp; ❌ = Broken &nbsp; ⚠️ = Partial &nbsp; 🔲 = Untested &nbsp; ⛔ = No Protocol Surface
>
> **KDE**, **COSMIC**, **GNOME**, **Hyprland**, and **Sway** tested on Turtle (EndeavourOS, real hardware). All other DEs have backend code but **zero runtime verification** — they're 🔲 until a daemon is started on a live session.

---

## Windows

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `windows.list` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 |
| `windows.focus` | ✅ | 🔲 | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 |
| `windows.get` | ✅ | 🔲 | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 |
| `windows.close` | ✅ | 🔲 | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 |
| `windows.minimize` | ✅ | 🔲 | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 |
| `windows.maximize` | ✅ | 🔲 | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 |
| `windows.move_resize` | ✅ | 🔲 | ✅ | ⛔ | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 |
| `windows.tile` | ✅ | 🔲 | ✅ | ⛔ | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 |
| `windows.activate_or_launch` | ✅ | 🔲 | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 |

## Workspaces

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `workspaces.list` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 |
| `workspaces.switch` | ✅ | 🔲 | ✅ | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 |
| `workspaces.move_window` | ✅ | 🔲 | ✅ | ✅ | ⚠️ | 🔲 | 🔲 | 🔲 | 🔲 |

## Input

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `input.keyboard` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 |
| `input.mouse` | ✅ | 🔲 | ✅ | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 |
| `input.mouse.drag` | ✅ | 🔲 | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 |
| `input.layouts.list` | ✅ | 🔲 | ✅ | ✅ | ❌ | 🔲 | 🔲 | 🔲 | 🔲 |
| `input.layout.get` | ✅ | 🔲 | ✅ | ✅ | ❌ | 🔲 | 🔲 | 🔲 | 🔲 |
| `input.layout.set` | ✅ | 🔲 | ✅ | ✅ | ❌ | 🔲 | 🔲 | 🔲 | 🔲 |
| `input.layout.add` | ✅ | 🔲 | ✅ | ✅ | ❌ | 🔲 | 🔲 | 🔲 | 🔲 |
| `input.layout.remove` | ✅ | 🔲 | ✅ | ✅ | ❌ | 🔲 | 🔲 | 🔲 | 🔲 |

## Monitor

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `monitor.list` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 |
| `monitor.set_primary` | ✅ | 🔲 | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 |
| `monitor.set_resolution` | ✅ | 🔲 | ✅ | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 |
| `monitor.set_scale` | ✅ | 🔲 | ✅ | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 |
| `monitor.set_rotation` | ✅ | 🔲 | ✅ | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 |
| `monitor.enable` | ✅ | 🔲 | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 |
| `monitor.disable` | ✅ | 🔲 | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 | 🔲 |

## System

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `system.info` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 |
| `system.idle` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 |
| `system.power` | ✅ | 🔲 | ✅ | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 |
| `system.battery` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | 🔲 | 🔲 |

## Notifications

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `notification.send` | ✅ | ❌ | ✅ | ✅ | ❌ | 🔲 | 🔲 | 🔲 | 🔲 |
| `notification.close` | ✅ | ❌ | ✅ | ✅ | ❌ | 🔲 | 🔲 | 🔲 | 🔲 |

---

## Daemon-Level (DE-Independent)

These actions don't touch the `DesktopBackend` trait. They should work on any DE where the daemon starts, but have been verified on KDE, COSMIC, GNOME, Hyprland, and Sway sessions (where noted).

| Category | Actions | Tested On |
|---|---|---|
| Clipboard | `read`, `write`, `history`, `history.clear` | KDE, COSMIC, GNOME, Hyprland |
| Apps | `list`, `search`, `get` | KDE, COSMIC, GNOME, Hyprland |
| MPRIS Media | `list`, `get`, `control` | KDE, COSMIC, GNOME |
| Color & Screenshot | `color.pick`, `screenshot`, `screenshot.ocr`, `screenshot.diff` | KDE, COSMIC, GNOME, Hyprland |
| Audit | `audit.log`, `audit.clear` | KDE, COSMIC, GNOME |
| Services & Journal | `service.*`, `timer.list`, `journal.query` | KDE, COSMIC, GNOME, Hyprland |
| Network | `status`, `interfaces`, `wifi.scan`, `wifi.connect` | KDE, COSMIC, GNOME, Hyprland |
| Bluetooth | `list`, `scan`, `scan_stop`, `connect`, `disconnect`, `pair`, `forget` | KDE, COSMIC, GNOME, Hyprland ⚠️ |
| Files | `watch`, `unwatch`, `search`, `read`, `write`, `copy`, `move`, `delete`, `mkdir`, `list` | KDE, COSMIC, GNOME, Hyprland |
| Browser (CDP) | `list_tabs`, `navigate`, `evaluate`, `screenshot_tab`, `click` | KDE, COSMIC, GNOME |
| A11y (AT-SPI2) | `tree`, `get_element`, `click_element`, `get_text`, `snapshot_tree`, `perform_action`, `set_value`, `list_apps`, `doctor` | KDE, COSMIC, GNOME |
| Process | `list`, `start`, `stop`, `signal`, `exists`, `wait` | KDE, COSMIC, GNOME |
| Terminal / PTY | `create`, `write`, `read`, `resize`, `list`, `kill` | KDE, COSMIC, GNOME, Hyprland |
| Hotkeys | `register`, `unregister` | KDE, COSMIC, GNOME |
| Audio | `list_sinks`, `set_sink_volume` | KDE, COSMIC, GNOME, Hyprland |
| Layout Profiles | `list`, `get`, `save`, `delete`, `restore` | KDE, COSMIC, GNOME |
| Location & UI | `location.get`, `ui.tree.get`, `ui.element.click`, `ui.element.set_text` | KDE, COSMIC, GNOME |
| Meta | `capabilities.list` | KDE, COSMIC, GNOME |

---

## Known Gaps

| DE | Gaps | Notes |
|---|---|---|
| **COSMIC** | `windows.move_resize` ⛔, `windows.tile` ⛔ | `zcosmic_toplevel_manager_v1` (v4) has no geometry control. `set_rectangle` is a visual hint only, not a move/resize command. Capabilities enum: close/activate/maximize/minimize/fullscreen/workspace/sticky — no move, no resize. Super+Click drag works at the compositor level but there is no programmatic API. |
| **KDE** | No known gaps | All 7 bugs from initial test matrix fixed. |
| **GNOME** | No known gaps | Mutter 50.1, Wayland. Full test passed: windows, workspaces, input, monitor, system, notifications, keyboard layouts. |
| **Hyprland** | `notification.send` ❌, `notification.close` ❌, `screenshot.ocr` ❌, Bluetooth ⚠️, 16/33 actions 🔲 untested | **Tested May 2026** on Hyprland 0.54.3. 7/33 DE-dependent actions verified. Notifications hung (30s timeout) — no notification daemon on bare Hyprland install. OCR requires tesseract (not installed). Bluetooth adapter dead on test hardware (Turtle laptop). Daemon-level: clipboard, apps, screenshots, services, network, wifi, files, terminal, audio all ✅. color.pick fixed via hyprpicker backend (commit 2641b1b). screenshot-diff sandbox fixed (/tmp allowed). terminal.create permission message fixed. 16 DE-dependent actions remain untested — need a fuller test pass. |
| **Sway** | `input.layouts.*` ❌ (5 actions), `notification.send/close` ❌, `workspaces.move_window` ⚠️, 12/33 actions 🔲 untested | **Tested May 2026** on Sway 1.11 (headless). 13/33 DE-dependent actions verified passing. Keyboard layouts: backend returns "not supported" for all 5 layout actions — needs implementation. Notifications hung (30s timeout) — no notification daemon on bare install (same as Hyprland). `workspaces.move_window` returns "swaymsg failed" — likely due to no real windows in headless mode, needs verification with real windows. `system.power` routes correctly but systemd auth blocks non-interactive calls. Daemon-level: same capabilities as other DEs. SWAYSOCK handling confirmed working (daemon reads at startup, passes to swaymsg subprocesses). |
| **Niri** | 🔲 Untested | Backend exists — scroll-based tiling WM, no minimize concept. |
| **Wayfire** | 🔲 Untested | Backend exists with workspace/window stubs. |
| **Labwc** | 🔲 Untested | Backend exists with workspace/window stubs. |
| **X11** | 🔲 Untested | Full backend in `src/backend/x11/` — needs live session verification. |

## Architecture

- **DE-dependent actions** (Windows, Workspaces, Input, Monitor, Notifications, System) route through the `DesktopBackend` trait — 9 backends, each with 44+ mandatory methods
- **DE-independent actions** (Files, Process, Terminal, etc.) use D-Bus, sysfs, systemd, AT-SPI2, CDP, or direct OS calls — should work anywhere the daemon runs
- `windows.tile` composites `system_info()` + `window_move_resize()` — move_resize gaps cascade to tile
- `windows.activate_or_launch` composites `windows_list()` + `window_focus()` + daemon spawn
- `layout_profiles.save/restore` are daemon-level orchestrations
