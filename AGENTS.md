# Deskbrid

Deskbrid bridges AI agents to any Linux desktop over a Unix socket. Control windows, inject keystrokes, take screenshots, manage clipboards — on GNOME, KDE, Hyprland, COSMIC, Sway, Niri, Wayfire, Labwc, or X11.

## Install

```bash
bash <(curl -fsSL https://deskbrid.patchhive.dev/install.sh)
```

Auto-detects distro + DE, installs deps, sets up uinput, downloads binary.

## Quick Start

```bash
deskbrid daemon                               # start daemon
deskbrid windows list                         # CLI: list windows
deskbrid clipboard read                       # CLI: read clipboard
deskbrid screenshot                           # CLI: screenshot
```

```bash
# Socket (agent-native)
echo '{"type":"system.info","id":"1"}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 2
echo '{"type":"windows.list","id":"2"}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 2
```

## Features

- **Windows & Workspaces** — list, focus, close, minimize, maximize, tile, move/resize
- **Input** — keyboard typing, key combos, mouse control
- **Clipboard** — read/write with history
- **Screenshots** — capture, OCR, diffing
- **System** — info, battery, idle, power management
- **Audio** — volume, mute, sink management
- **Network & Bluetooth** — WiFi, device pairing
- **MPRIS** — media player control
- **Terminal** — PTY sessions
- **Files** — search, read, write, watch
- **Notifications** — send, dismiss, history
- **Keyboard Layouts** — list, switch, add, remove
- **Desktop Settings** — gsettings read/write, schema discovery (GNOME, KDE, X11, Hyprland, Sway, COSMIC, Labwc, Niri, Wayfire)
- **Backlight** — list, get, set brightness via sysfs (all backends)
- **Print** — list printers, set default, list/cancel/pause/resume jobs, send files to printer (CUPS)
- **Self-update** — `deskbrid update` pulls latest from GitHub releases
- **Confirmation Mode** — require explicit approval for destructive actions before execution
- **Agent Messaging** — inter-agent mailbox for message passing between sessions
- **Unified Search** — cross-surface search (windows, apps, files, clipboard, audit log) with relevance scoring

## Dashboard

Built-in web dashboard at `localhost:20129` — system info, monitors, windows, network, audio, clipboard, audit log, confirmations, agent mailbox, search index — all live via SSE.

**[🔴 Live Demo →](https://deskbrid.patchhive.dev/live)**

## MCP Integration

```bash
deskbrid mcp   # MCP stdio server for AI coding tools
```

100+ tools across 22 categories: window management, accessibility tree, keyboard, mouse, clipboard, screenshots, system info, print, confirmation, agent messaging, unified search, and more. Claude Desktop, Codex, Cursor — any MCP client.

## Python Client

```python
from deskbrid import Deskbrid
client = Deskbrid()
client.focus_window(app_id='code')
client.type_text("Hello from Deskbrid!\n")
```

## Supported Desktops

GNOME 46–50, Hyprland, KDE Plasma, COSMIC, Sway, Niri, Wayfire, Labwc, Cinnamon, MATE, X11. Auto-detected at startup — no config.

## Docs

https://deskbrid.patchhive.dev
