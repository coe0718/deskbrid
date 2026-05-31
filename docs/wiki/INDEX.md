# Deskbrid Documentation

**The HAL your Linux desktop agents are missing.**

Deskbrid is a single Rust binary that auto-detects your desktop environment and provides a unified JSON-over-Unix-socket protocol for desktop automation on Linux. One daemon, one protocol, one binary — works across GNOME, Hyprland, KDE, wlroots compositors, and X11.

## Quick Links

- **[Installation](Installation)** - Install Deskbrid on your system
- **[Quick Start](Quick-Start)** - Get running in 5 minutes
- **[Protocol Overview](Protocol-Overview)** - JSON protocol specification
- **[Python Client](Integrations-Python)** - Python integration guide

## Features

### Core Features

| Feature | Description | Documentation |
|---------|-------------|---------------|
| **Windows & Workspaces** | List, focus, move, resize, and tile windows; manage workspaces | [docs](Features-Windows-Workspaces) |
| **Input Control** | Keyboard typing, key combos, mouse movement, clicks, scroll | [docs](Features-Input) |
| **Clipboard** | Read/write, history, monitoring | [docs](Features-Clipboard) |
| **Screenshots** | Capture, OCR, diff comparison | [docs](Features-Screenshots) |

### System Features

| Feature | Description | Documentation |
|---------|-------------|---------------|
| **System Info** | Desktop info, battery, idle, power actions | [docs](Features-System) |
| **Media Control** | MPRIS player integration | [docs](Features-Media) |
| **Audio** | Sink listing and volume control | [docs](Features-Audio) |
| **Network** | WiFi status and connections | [docs](Features-Network) |
| **Bluetooth** | Device discovery and pairing | [docs](Features-Bluetooth) |
| **Services** | systemd unit management | [docs](Features-Services) |
| **Terminals** | Interactive PTY sessions | [docs](Features-Terminals) |
| **Monitors** | Display configuration | [docs](Features-Monitors) |
| **Screen Recording** | PipeWire-based capture, web dashboard | [docs](Features-Screen-Recording) |
| **Self-Update** | Automatic binary updates from GitHub | [docs](Features-Self-Update) |
| **System Tray** | Tray icon with update notifications and quick actions | [docs](Features-System-Tray) |

### Advanced Features

| Feature | Description | Documentation |
|---------|-------------|---------------|
| **Notifications** | Desktop notification API | [docs](Features-Notifications) |
| **Files** | File search and watching | [docs](Features-Files) |
| **Layout Profiles** | Save and restore workspace layouts | [docs](Features-Layout-Profiles) |
| **Accessibility** | AT-SPI tree inspection | [docs](Features-Accessibility) |
| **Persistence** | SQLite-backed clipboard history, audit log, blackboard, rules, sessions | [docs](Features-Persistence) |
| **Rules Engine** | Event-driven automation with cooldown and max_fires | [docs](Features-Rules) |
| **Sessions** | Named sessions with isolated variable namespaces | [docs](Features-Sessions) |
| **Blackboard** | Namespace-scoped KV store for multi-agent coordination | [docs](Features-Blackboard) |
| **Macros** | Record and replay action sequences | [docs](Features-Macros) |
| **Cron** | Schedule actions at intervals | [docs](Features-Cron) |

## Protocol

| Document | Description |
|----------|-------------|
| [Overview](Protocol-Overview) | JSON protocol fundamentals |
| [Events](Protocol-Events) | Real-time event subscription |
| [MCP Integration](Protocol-Mcp) | Model Context Protocol server |

## Integrations

| Integration | Description |
|-------------|-------------|
| [Python Client](Integrations-Python) | Python library usage |
| [AI Agents](Integrations-Agents) | Claude Code, Cursor, etc. |

## Development

| Document | Description |
|----------|-------------|
| [Architecture](Development-Architecture) | System design deep dive |

## Supported Desktops

| Desktop | Status | Notes |
|---------|--------|-------|
| GNOME 46-50 | ✅ Full | Requires Shell extension |
| Hyprland | ✅ Full | Requires ydotool |
| KDE Plasma | ✅ Full | Requires ydotoold |
| Sway | ✅ Full | Requires ydotool |
| Niri | ✅ Partial | Geometry degraded |
| Wayfire | ✅ Partial | No move/resize |
| Labwc | ✅ Partial | No move/resize |
| COSMIC | ⚠️ Partial | Some limitations |
| Cinnamon / MATE | ✅ Full | X11 shared backend |

## Example Usage

### CLI

```bash
# List windows
deskbrid windows list

# Focus a window
deskbrid windows focus --app code

# Type text
deskbrid input keyboard type "Hello, world!\n"

# Take screenshot
deskbrid screenshot --output ./screenshot.png
```

### Python

```python
from deskbrid import Deskbrid

client = Deskbrid()
windows = client.windows_list()
client.focus_window(app_id='code')
client.type_text("Fixed the bug!\n")
```

### MCP (AI Agents)

```json
// In your AI coding tool's MCP config
{
  "mcpServers": {
    "deskbrid": {
      "command": "deskbrid",
      "args": ["mcp"]
    }
  }
}
```