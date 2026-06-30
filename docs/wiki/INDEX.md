# Deskbrid Documentation

**The HAL your Linux desktop agents are missing.**

Deskbrid is a single Rust binary that auto-detects your desktop environment and provides a unified JSON-over-Unix-socket protocol for desktop automation on Linux. One daemon, one protocol, one binary — works across GNOME, Hyprland, KDE, wlroots compositors, and X11.

## Quick Links

- **[Installation](installation.md)** — install Deskbrid and configure desktop dependencies
- **[Quick Start](quick-start.md)** — get running in a few minutes
- **[Protocol Overview](protocol/overview.md)** — JSON protocol fundamentals
- **[v1.0.0 Release Notes](../deskbrid-v1.0.0.md)** — stable release notes, breaking changes, migration

## Features

Full feature documentation by domain — every action, parameter, and example:

| Domain | Actions | Doc |
|--------|---------|-----|
| Apps | `apps.list`, `apps.search`, `apps.get` | [Apps](features/apps.md) |
| Audio | `audio.volume`, `audio.mute`, `audio.sinks` | [Audio](features/audio.md) |
| Audit | `audit.query`, `audit.clear` | [Audit](features/audit.md) |
| Backlight | `backlight.list`, `backlight.get`, `backlight.set` | [Backlight](features/backlight.md) |
| Battery | `battery.get` | [Battery](features/battery.md) |
| Bluetooth | `bluetooth.list`, `bluetooth.pair`, `bluetooth.remove` | [Bluetooth](features/bluetooth.md) |
| Browser | `browser.cdp` | [Browser CDP](features/browser_chrome_devtools_protocol.md) |
| Clipboard | `clipboard.read`, `clipboard.write`, `clipboard.history` | [Clipboard](features/clipboard.md) |
| Color Picker | `color_picker` | [Color Picker](features/color_picker.md) |
| Confirmation | `confirm.challenge`, `confirm.resolve` | [Confirmation](features/confirmation.md) |
| Connection | `connection.subscribe`, `connection.unsubscribe`, `connection.disconnect` | [Connection](features/connection.md) |
| Cron | `cron.schedule`, `cron.list`, `cron.remove` | [Cron](features/cron.md) |
| D-Bus | `dbus.call` | [D-Bus](features/d_bus.md) |
| Desktop Portal | `portal.screenshot`, `portal.screencast` | [Desktop Portal](features/desktop_portal.md) |
| Desktop Settings | `settings.get`, `settings.set`, `settings.schema` | [Desktop Settings](features/desktop_settings.md) |
| Files | `file.search`, `file.read`, `file.write`, `file.watch` | [Files](features/file.md) |
| Hotkeys | `hotkey.bind`, `hotkey.unbind` | [Hotkeys](features/hotkeys.md) |
| Keyboard Layouts | `layouts.list`, `layouts.switch`, `layouts.add`, `layouts.remove` | [Keyboard Layouts](features/keyboard_layouts.md) |
| Keyring | `keyring.get`, `keyring.set`, `keyring.delete` | [Keyring](features/keyring.md) |
| Location | `location.get` | [Location](features/location.md) |
| Lock/Mutex | `lock.acquire`, `lock.release`, `lock.list` | [Lock/Mutex](features/lock_mutex.md) |
| Macros | `macro.record`, `macro.replay` | [Macros](features/macro_recording_replay.md) |
| MPRIS Media | `mpris.list`, `mpris.control`, `mpris.nowplaying` | [MPRIS](features/mpris_media_control.md) |
| Mailbox | `mailbox.send`, `mailbox.read`, `mailbox.delete` | [Mailbox](features/mailbox.md) |
| Monitor | `monitor.list`, `monitor.set` | [Monitor](features/monitor.md) |
| Network | `network.wifi`, `network.status` | [Network](features/network.md) |
| Notifications | `notification.send`, `notification.close`, `notification.history` | [Notifications](features/notification.md) |
| Persistence | `state.get`, `state.set`, `state.delete` | [Persistence](features/persistence.md) |
| Print | `print.list`, `print.jobs`, `print.file` | [Print](features/print.md) |
| Process | `process.list`, `process.kill` | [Process](features/process.md) |
| Region Watch / Text Watch | `region.watch.create`, `region.watch.update`, `region.watch.remove`, `region.watch.list`, `text.watch.create`, `text.watch.remove`, `text.watch.list` | [Region / Text Watch](features/region_watch.md) |
| Rules Engine | `rules.list`, `rules.create`, `rules.trigger` | [Rules](features/rules.md) |
| Schedule | `schedule.list`, `schedule.add`, `schedule.remove` | [Schedule](features/schedule.md) |
| Screenshot | `screenshot`, `screenshot.ocr`, `screenshot.diff` | [Screenshot](features/screenshot.md) |
| Screencast | `screencast.start`, `screencast.stop` | [Screencast](features/screencast_video_recording.md) |
| Secrets | `secrets.set`, `secrets.get`, `secrets.delete` | [Secrets](features/secrets.md) |
| Self Update | `update.check`, `update.apply` | [Self Update](features/self-update.md) |
| Services | `service.list`, `service.start`, `service.stop`, `service.status` | [Services](features/systemd_units_journal_and_timers.md) |
| Sessions | `session.list`, `session.switch`, `session.lock`, `session.logout` | [Sessions](features/sessions.md) |
| System | `system.info`, `system.health`, `system.power`, `system.pressure`, `system.idle`, `system.confinement`, `system.sessions`, `system.lock_session`, `system.switch_user`, `system.inhibit`, `system.release_inhibit`, `system.thermal`, `system.cpu_frequency`, `system.cpu_governor`, `system.set_cpu_governor`, `system.elevate`, `system.check_auth`, `system.capabilities` | [System](features/system.md) |
| System Tray | `tray.menu`, `tray.action` | [System Tray](features/system-tray.md) |
| Terminal PTY | `terminal.create`, `terminal.write`, `terminal.read`, `terminal.kill` | [Terminal](features/terminal_pty.md) |
| Unified Search | `search.query` | [Unified Search](features/search.md) |
| Wait For | `wait.for` | [Wait For](features/wait_for.md) |
| Windows | `windows.list`, `windows.focus`, `windows.get`, `windows.close`, `windows.minimize`, `windows.maximize`, `windows.move_resize`, `windows.tile`, `windows.activate_or_launch` | [Windows](features/windows.md) |
| Workspaces | `workspaces.list`, `workspaces.switch` | [Workspaces](features/workspaces.md) |

## Protocol

- **[Overview](protocol/overview.md)** — JSON protocol fundamentals, dispatch rules, error codes
- **[Events](protocol/events.md)** — real-time event subscription and event types
- **[MCP Integration](protocol/mcp.md)** — Model Context Protocol server setup and tool map

## Integrations

- **[Python Client](integrations/python.md)** — Python library with sync and async APIs
- **[AI Agents](integrations/agents.md)** — Claude Desktop, Cursor, and MCP client configuration

## Development

| Document | Description |
|----------|-------------|
| [Architecture](../ARCHITECTURE.md) | System design, data flow, backend abstraction |

## API Reference

| Document | Description |
|----------|-------------|
| [API.md](../API.md) | All actions and parameters |
| [Protocol.md](../PROTOCOL.md) | Protocol implementation, action table, permissions |
| [Backends.md](../BACKENDS.md) | Backend provider details |

## Supported Desktops

| Desktop | Session | Status | Backend |
|---------|---------|--------|---------|
| GNOME 46–50 | Wayland/X11 | Supported | MPRIS/Shell RemoteDesktop + Shell Extension |
| Hyprland | Wayland | Supported | hyprctl + ydotool + grim + wlr-randr |
| KDE Plasma | Wayland | Supported | KWin D-Bus + ydotool + spectacle |
| Sway | Wayland | Supported | swaymsg + ydotool + grim + wlr-randr |
| Niri | Wayland | Partial | niri IPC + ydotool + grim |
| Wayfire | Wayland | Partial | wf-ipc + ydotool + grim + wlr-randr |
| Labwc | Wayland | Supported | wlrctl + wdotool + grim + wlr-randr |
| COSMIC | Wayland | Partial | cosmic-helper + cosmic-randr + ydotool + grim |
| Cinnamon / MATE / XFCE | X11 | Supported | xdotool + wmctrl + xclip + import |

## Auth / Permission Rules

- Elevated/system-mutating `system.*` actions require authorization through the Dashboard / confirmation UI by default.
- Safer path is via `confirm.challenge` / `confirm.resolve` rather than bypassing confirmation on the socket.
- Safe path = follow dashboard/confirm flow first.

## Quick Links

- Installation: `installation.md`
- Quick Start: `quick-start.md`
- Protocol Overview: `protocol/overview.md`
- Events: `protocol/events.md`
- MCP: `protocol/mcp.md`
- Python Client: `integrations/python.md`
- AI Agents: `integrations/agents.md`
- Architecture: `../ARCHITECTURE.md`
- API Reference: `../API.md`
- Backends: `../BACKENDS.md`
- v1.0.0 Release Notes: `../deskbrid-v1.0.0.md`

## Web Dashboard

Access the dashboard at `http://localhost:20129`. It provides live system information and interaction surface for the agentic loop.

## Repository

- Source: `src/`
- CI: `.github/workflows/`
- Install script: `install.sh`
- Distributions: Docker images for Debian 13 / Bookworm and Ubuntu 25.04.
