# Deskbrid Documentation

**The HAL your Linux desktop agents are missing.**

Deskbrid is a single Rust binary that auto-detects your desktop environment and provides a unified JSON-over-Unix-socket protocol for desktop automation on Linux. One daemon, one protocol, one binary — works across GNOME, Hyprland, KDE, wlroots compositors, and X11.

## Quick Links

- **[Installation](docs/wiki/installation.md)** — install Deskbrid and configure desktop dependencies
- **[Quick Start](docs/wiki/quick-start.md)** — get running in a few minutes
- **[Protocol Overview](docs/wiki/protocol/overview.md)** — JSON protocol fundamentals
- **[v1.0.0 Release Notes](docs/deskbrid-v1.0.0.md)** — stable release notes, breaking changes, migration
- **[Product Profile](docs/products/deskbrid.md)** — role, workflow, integration points

## Features

### Core Features

- Windows & workspaces
- Clipboard
- Input control
- Screenshots
- Screen recording

### System Features

- System information, health, power
- Notifications
- Monitors
- Layout profiles
- Services and timers

### Advanced Features

- Rules engine
- Blackboard
- Sessions
- Macros
- Cron scheduling
- Secrets

## Protocol

- **[Overview](docs/wiki/protocol/overview.md)** — JSON protocol fundamentals
- **[Events](docs/wiki/protocol/events.md)** — real-time event subscription
- **[MCP Integration](docs/wiki/protocol/mcp.md)** — Model Context Protocol server

## Integrations

- **[Python Client](docs/wiki/integrations/python.md)** — Python library usage
- **[AI Agents](docs/wiki/integrations/agents.md)** — Claude Code, Cursor, etc.

## Development

| Document | Description |
|----------|-------------|
| [Architecture](docs/ARCHITECTURE.md) | System design deep dive |

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

## API Reference

| Document | Description |
|----------|-------------|
| [API.md](docs/API.md) | All actions and parameters |
| [Protocol.md](docs/PROTOCOL.md) | Protocol implementation |
| [Backends.md](docs/BACKENDS.md) | Backend provider details |

## Auth / Permission Rules

- Elevated/system-mutating `system.*` actions require authorization through the Dashboard / confirmation UI by default.
- Safer path is via `confirm.challenge` / `confirm.resolve` rather than bypassing confirmation on the socket.
- Safe path = follow dashboard/confirm flow first.

## Quick Links

- Installation: `docs/wiki/installation.md`
- Quick Start: `docs/wiki/quick-start.md`
- Protocol Overview: `docs/wiki/protocol/overview.md`
- Events: `docs/wiki/protocol/events.md`
- MCP: `docs/wiki/protocol/mcp.md`
- Python Client: `docs/wiki/integrations/python.md`
- AI Agents: `docs/wiki/integrations/agents.md`
- Architecture: `docs/ARCHITECTURE.md`
- API Reference: `docs/API.md`
- Backends: `docs/BACKENDS.md`
- v1.0.0 Release Notes: `docs/deskbrid-v1.0.0.md`
- Product Profile: `docs/products/deskbrid.md`

## Web Dashboard

Access the dashboard at `http://localhost:20129`. It provides live system information and interaction surface for the agentic loop.

## Repository

- Source: `src/`
- CI: `.github/workflows/`
- Install script: `install.sh`
- Distributions: Docker images for Debian 13 / Bookworm and Ubuntu 25.04.
