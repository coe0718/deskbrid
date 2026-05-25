# DE Test Matrix

Deskbrid protocol action support across 11 desktop environments.

> **Legend:** ✅ = Working &nbsp; ❌ = Broken &nbsp; ⚠️ = Partial &nbsp; 🔲 = Untested &nbsp; ⛔ = No Protocol Surface
>
> **KDE**, **COSMIC**, **GNOME**, **Hyprland**, **Sway**, and **Labwc** tested on Turtle (EndeavourOS, real hardware). All other DEs have backend code but **zero runtime verification** — they're 🔲 until a daemon is started on a live session.

---

## Windows

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `windows.list` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `windows.focus` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `windows.get` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `windows.close` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `windows.minimize` | ✅ | ❌ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ⛔ | 🔲 |
| `windows.maximize` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `windows.move_resize` | ✅ | ✅ | ✅ | ⛔ | ✅ | 🔲 | 🔲 | ⛔ | 🔲 |
| `windows.tile` | ✅ | ✅ | ✅ | ⛔ | ✅ | 🔲 | 🔲 | ⛔ | 🔲 |
| `windows.activate_or_launch` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ⚠️ | 🔲 |

## Workspaces

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `workspaces.list` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `workspaces.switch` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `workspaces.move_window` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ⛔ | 🔲 |

## Input

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `input.keyboard` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `input.mouse` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `input.mouse.drag` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `input.list_layouts` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `input.get_layout` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `input.set_layout` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `input.add_layout` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `input.remove_layout` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |

## Monitor

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `monitor.list` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `monitor.set_primary` | ✅ | ❌ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ⛔ | 🔲 |
| `monitor.set_resolution` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `monitor.set_scale` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `monitor.set_rotation` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `monitor.enable` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `monitor.disable` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |

## System

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `system.info` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `system.idle` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `system.power` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |
| `system.battery` | ✅ | ✅ | ✅ | ✅ | ✅ | 🔲 | 🔲 | ✅ | 🔲 |

## Notifications

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `notification.send` | ✅ | ❌ | ✅ | ✅ | ❌ | 🔲 | 🔲 | ✅ | 🔲 |
| `notification.close` | ✅ | ❌ | ✅ | ✅ | ❌ | 🔲 | 🔲 | ✅ | 🔲 |

---

## Daemon-Level (DE-Independent)

These actions don't touch the `DesktopBackend` trait. They should work on any DE where the daemon starts, but have been verified on KDE, COSMIC, GNOME, Hyprland, Sway, and Labwc sessions (where noted).

| Category | Actions | Tested On |
|---|---|---|
| Clipboard | `read`, `write`, `history`, `history.clear` | KDE, COSMIC, GNOME, Hyprland, Labwc |
| Apps | `list`, `search`, `get` | KDE, COSMIC, GNOME, Hyprland |
| MPRIS Media | `list`, `get`, `control` | KDE, COSMIC, GNOME |
| Color & Screenshot | `color.pick`, `screenshot`, `screenshot.ocr`, `screenshot.diff` | KDE, COSMIC, GNOME, Hyprland, Labwc |
| Audit | `audit.log`, `audit.clear` | KDE, COSMIC, GNOME |
| Services & Journal | `service.*`, `timer.list`, `journal.query` | KDE, COSMIC, GNOME, Hyprland |
| Network | `status`, `interfaces`, `wifi.scan`, `wifi.connect` | KDE, COSMIC, GNOME, Hyprland, Labwc |
| Bluetooth | `list`, `scan`, `scan_stop`, `connect`, `disconnect`, `pair`, `forget` | KDE, COSMIC, GNOME, Hyprland ⚠️ |
| Files | `watch`, `unwatch`, `search`, `read`, `write`, `copy`, `move`, `delete`, `mkdir`, `list` | KDE, COSMIC, GNOME, Hyprland |
| Browser (CDP) | `list_tabs`, `navigate`, `evaluate`, `screenshot_tab`, `click` | KDE, COSMIC, GNOME |
| A11y (AT-SPI2) | `tree`, `get_element`, `click_element`, `get_text`, `snapshot_tree`, `perform_action`, `set_value`, `list_apps`, `doctor` | KDE, COSMIC, GNOME |
| Process | `list`, `start`, `stop`, `signal`, `exists`, `wait` | KDE, COSMIC, GNOME |
| Terminal / PTY | `create`, `write`, `read`, `resize`, `list`, `kill` | KDE, COSMIC, GNOME, Hyprland |
| Hotkeys | `register`, `unregister` | KDE, COSMIC, GNOME |
| Audio | `list_sinks`, `set_sink_volume` | KDE, COSMIC, GNOME, Hyprland, Labwc |
| Layout Profiles | `list`, `get`, `save`, `delete`, `restore` | KDE, COSMIC, GNOME |
| Location & UI | `location.get`, `ui.tree.get`, `ui.element.click`, `ui.element.set_text` | KDE, COSMIC, GNOME |
| Meta | `capabilities.list` | KDE, COSMIC, GNOME |

---

## Known Gaps

| DE | Gaps | Notes |
|---|---|---|
| **COSMIC** | `windows.move_resize` ⛔, `windows.tile` ⛔ | `zcosmic_toplevel_manager_v1` (v4) has no geometry control. `set_rectangle` is a visual hint only, not a move/resize command. |
| **KDE** | No known gaps | All 7 bugs from initial test matrix fixed. |
| **GNOME** | No known gaps | Mutter 50.1, Wayland. Full test passed. |
| **Hyprland** | `windows.minimize` ❌, `monitor.set_primary` ❌, `notification.send/close` ❌ | **Tested May 2026** on Hyprland 0.54.3 (Turtle). 30/33 ✅. `windows.minimize`: compositor limitation. `monitor.set_primary`: compositor limitation. Notifications: no daemon. Keyboard layouts all ✅ (parser fixed for 0.54+ `rules:` format). |
| **Sway** | `notification.send/close` ❌ | **Tested May 2026** on Sway 1.11 (Turtle). 31/33 ✅. Notifications: no daemon. Keyboard layouts implemented via swaymsg. |
| **Labwc** | `windows.move_resize` ⛔, `windows.minimize` ⛔, `windows.tile` ⛔, `monitor.set_primary` ⛔, `workspaces.move_window` ⛔ | **Tested May 2026** on Labwc 0.9.7 (Turtle). 26/33 ✅. `move_resize`/`minimize`/`tile`: NO_PROTOCOL_SURFACE — wlrctl doesn't expose these, and labwc has no IPC for window geometry or minimize. `set_primary`: NO_PROTOCOL_SURFACE — wlr-randr has no --primary flag, no protocol. `workspace.move_window`: NO_PROTOCOL_SURFACE — no Wayland protocol exists, no Labwc IPC. Notifications: mako daemon installed, working. Workspaces list/switch via ext_workspace_v1 protocol client. Keyboard layouts via XKB_DEFAULT_LAYOUT env file. `set_resolution`: fixed with wlr-randr fallback. `activate_or_launch`: works via daemon-level spawn. |
| **Niri** | 🔲 Untested | Backend exists — scroll-based tiling WM. |
| **Wayfire** | 🔲 Untested | Backend exists with workspace/window stubs. |
| **X11** | 🔲 Untested | Full backend in `src/backend/x11/` — needs live session verification. |

## Architecture

- **DE-dependent actions** (Windows, Workspaces, Input, Monitor, Notifications, System) route through the `DesktopBackend` trait — 9 backends, each with 44+ mandatory methods
- **DE-independent actions** (Files, Process, Terminal, etc.) use D-Bus, sysfs, systemd, AT-SPI2, CDP, or direct OS calls — should work anywhere the daemon runs
- `windows.tile` composites `system_info()` + `window_move_resize()` — move_resize gaps cascade to tile
- `windows.activate_or_launch` composites `windows_list()` + `window_focus()` + daemon spawn
- `layout_profiles.save/restore` are daemon-level orchestrations
