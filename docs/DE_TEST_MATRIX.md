# DE Test Matrix

Deskbrid protocol action support across 11 desktop environments.

> **Legend:** ✅ = Working &nbsp; ❌ = Broken &nbsp; ⚠️ = Partial &nbsp; 🔲 = Untested

---

## Windows

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `windows.list` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `windows.focus` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `windows.get` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `windows.close` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `windows.minimize` | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ | ✅ |
| `windows.maximize` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `windows.move_resize` | ✅ | ✅ | ✅ | ❌ | ✅ | ⚠️ | ❌ | ❌ | ✅ |
| `windows.tile` | ✅ | ✅ | ✅ | ❌ | ✅ | ⚠️ | ❌ | ❌ | ✅ |
| `windows.activate_or_launch` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Workspaces

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `workspaces.list` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `workspaces.switch` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `workspaces.move_window` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Layout Profiles

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `layout_profiles.list` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `layout_profiles.get` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `layout_profiles.save` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `layout_profiles.delete` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `layout_profiles.restore` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Input

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `input.keyboard` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `input.mouse` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `input.mouse.drag` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `input.layouts.list` | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| `input.layout.get` | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| `input.layout.set` | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| `input.layout.add` | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| `input.layout.remove` | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |

## Clipboard

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `clipboard.read` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `clipboard.write` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `clipboard.history` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `clipboard.history.clear` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Apps

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `apps.list` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `apps.search` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `apps.get` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## MPRIS Media

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `mpris.list` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `mpris.get` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `mpris.control` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Color & Screenshot

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `color.pick` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `screenshot` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `screenshot.ocr` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `screenshot.diff` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Audit

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `audit.log` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `audit.clear` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Notifications

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `notification.send` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `notification.close` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## System

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `system.info` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `system.capabilities` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `system.health` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `system.idle` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `system.power` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `system.battery` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `system.inhibit` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `system.sessions` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `system.lock_session` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `system.check_auth` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `system.elevate` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Services & Journal

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `service.status` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `service.start` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `service.stop` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `service.restart` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `timer.list` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `journal.query` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Network

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `network.status` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `network.interfaces` | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `network.wifi.scan` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `network.wifi.connect` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Bluetooth

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `bluetooth.list` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `bluetooth.scan` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `bluetooth.scan_stop` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `bluetooth.connect` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `bluetooth.disconnect` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `bluetooth.pair` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `bluetooth.forget` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Files

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `files.watch` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `files.unwatch` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `files.search` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `files.read` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `files.write` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `files.copy` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `files.move` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `files.delete` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `files.mkdir` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `files.list` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Browser (CDP)

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `browser.list_tabs` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `browser.navigate` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `browser.evaluate` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `browser.screenshot_tab` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `browser.click` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Accessibility (AT-SPI2)

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `a11y.tree` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `a11y.get_element` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `a11y.click_element` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `a11y.get_text` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `a11y.snapshot_tree` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `a11y.perform_action` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `a11y.set_value` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `a11y.list_apps` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `a11y.doctor` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Process

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `process.list` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `process.start` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `process.stop` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `process.signal` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `process.exists` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `process.wait` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Terminal / PTY

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `terminal.create` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `terminal.write` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `terminal.read` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `terminal.resize` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `terminal.list` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `terminal.kill` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Hotkeys

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `hotkeys.register` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `hotkeys.unregister` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Audio

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `audio.list_sinks` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `audio.set_sink_volume` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Monitor

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `monitor.list` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `monitor.set_primary` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `monitor.set_resolution` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `monitor.set_scale` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `monitor.set_rotation` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `monitor.enable` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `monitor.disable` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Location & UI

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `location.get` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `ui.tree.get` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `ui.element.click` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `ui.element.set_text` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Meta

| Action | GNOME | Hyprland | KDE | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| `capabilities.list` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

---

## Known Gaps

| DE | Gaps |
|---|---|
| **Niri** | `windows.minimize` ❌ (no minimize concept). `windows.move_resize` ⚠️ (column-width only). Keyboard layout ❌ |
| **COSMIC** | `windows.move_resize` ❌. Keyboard layout ❌ |
| **Wayfire** | `windows.move_resize` ❌. Keyboard layout ❌ |
| **Labwc** | `windows.move_resize` ❌. Keyboard layout ❌ |
| **KDE** | Process/A11y/Browser: daemon/MCP only (no CLI). Keyboard layouts: backend ok, no CLI |
| **Sway** | Keyboard layout ❌ |
| **GNOME** | No known gaps |
| **Hyprland** | No known gaps |
| **X11** | No known gaps |

## Architecture

- ✅ actions on all DEs are **backend-independent** — handled via D-Bus, sysfs, systemd, AT-SPI2, CDP, or OS processes
- Core DE-specific actions use the `DesktopBackend` trait — all 9 backends override 44 mandatory methods
- `windows.tile` composites `system_info()` + `window_move_resize()` — move_resize gaps cascade to tile
- `windows.activate_or_launch` composites `windows_list()` + `window_focus()` + daemon spawn
- `layout_profiles.save/restore` are daemon-level orchestrations
