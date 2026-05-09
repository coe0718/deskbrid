# deskbrid

<p align="center">
  <img src="logo.png" alt="deskbrid logo" width="280">
</p>

<p align="center">
  <a href="https://github.com/coe0718/deskbrid/actions"><img src="https://github.com/coe0718/deskbrid/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/rust-2021%20edition-orange.svg" alt="Rust 2021"></a>
</p>

**The HAL your Linux desktop agents are missing.**

Deskbrid is a single Rust binary that wraps GNOME Shell, DBus, NetworkManager, BlueZ, PipeWire, and Wayland utilities into one JSON-over-Unix-socket protocol. Your shell scripts and AI agents use the same socket.

```bash
# Human
deskbrid windows list
deskbrid keyboard type "git push origin main"

# Agent (same socket)
{"action": "windows.list"}  →  [{"title": "VS Code", "app_id": "code", ...}]
```

## Why

Every major AI lab is racing to ship desktop agents. AppleScript gives macOS agents native control. Windows has UI Automation. Linux has `xdotool` — which breaks on Wayland, the default display protocol for every major distro.

Deskbrid fills that gap. It doesn't bet on agents taking off — automation use cases validate it today, agents validate it tomorrow. Same daemon, same protocol, same socket.

## Quick start

```bash
# 1. Clone and install system dependencies
git clone https://github.com/coe0718/deskbrid
cd deskbrid
sudo apt install -y grim wl-clipboard wtype

# 2. Install the GNOME Shell extension
cp -r extensions/deskbrid@deskbrid ~/.local/share/gnome-shell/extensions/
gnome-extensions enable deskbrid@deskbrid

# 3. Log out and back in (GNOME must reload extensions)

# 4. Build and run
cargo build --release
./target/release/deskbrid daemon &

# 5. Test it
./target/release/deskbrid windows list
./target/release/deskbrid system info
```

## Supported desktops

| Desktop | Session | Status |
|---|---|---|
| **GNOME 46+** | Wayland | ✅ Supported |
| GNOME 45 | Wayland | ✅ Supported |
| KDE Plasma | Wayland | 🔄 Planned |
| X11 | X11 | ❌ Not planned |

## What it can do

### 🖥️ Windows & Workspaces
| Action | Description |
|---|---|
| `windows.list` | List all open windows (title, app_id, workspace, geometry) |
| `windows.focus` | Focus a window by ID |
| `windows.get` | Get details for a specific window |
| `workspaces.list` | List workspaces |
| `workspaces.switch` | Switch to a workspace |
| `workspaces.move_window` | Move a window to another workspace |

### ⌨️ Input
| Action | Description |
|---|---|
| `input.keyboard type` | Type text into the focused window |
| `input.keyboard key` | Send a single keypress |
| `input.keyboard combo` | Send key combos (ctrl+shift+t) |
| `input.mouse move` | Move mouse to absolute position |
| `input.mouse click` | Click (left/middle/right) |
| `input.mouse scroll` | Scroll (dx/dy) |

### 📋 Clipboard · 📸 Screenshots · 🔔 Notifications
| Action | Description |
|---|---|
| `clipboard.read` | Read Wayland clipboard |
| `clipboard.write` | Write to Wayland clipboard |
| `screenshot` | Capture screen (full, region, or window) |
| `notification.send` | Send a desktop notification |
| `notification.close` | Close a notification by ID |

### ⚙️ System · 🌐 Network · 📡 Bluetooth · 🎵 Audio · 📁 Files
| Action | Description |
|---|---|
| `system.info` | Desktop info (GNOME version, monitors, workspaces) |
| `system.idle` | Seconds since last user input |
| `system.battery` | Battery percentage, state, time remaining |
| `system.power` | Suspend, hibernate, shutdown, reboot, lock, logout |
| `network.status` | Online/offline via NetworkManager |
| `network.interfaces` | List interfaces with IPs |
| `network.wifi.scan` | Scan for WiFi networks |
| `network.wifi.connect` | Connect to a WiFi network |
| `bluetooth.list` | List known/available devices |
| `bluetooth.scan` | Start device discovery |
| `bluetooth.connect` | Connect to a device |
| `audio.list_sinks` | List audio output devices |
| `audio.set_sink_volume` | Set sink volume (0.0-1.0) |
| `files.search` | Search files by name |
| `files.watch` | Watch a path for changes (creates, modifies, deletes) |
| `files.unwatch` | Stop watching a path |

### 📡 Events (subscribe)
```json
{"action": "subscribe", "events": ["file.*"]}
```
| Pattern | What you get |
|---|---|
| `file.*` | file.created, file.modified, file.deleted |
| `file.created` | Just file creation events |
| `*` | Everything |

## Real-world example: an AI agent controlling VS Code

```
→ {"action": "windows.list"}
← [{"title": "PatchHive — VS Code", "app_id": "code", ...},
   {"title": "praxis — VS Code", "app_id": "code", ...}]

→ {"action": "windows.focus", "window_id": "0x3a0000b"}
← {"type": "response", "status": "ok"}

→ {"action": "input.mouse", "action": "move", "x": 900, "y": 920}
→ {"action": "input.mouse", "action": "click", "button": "left"}
→ {"action": "input.keyboard", "action": "type", "text": "Fix the build errors\n"}
```

The agent picks the right window by title, brings it to front, clicks into the chat input, and types.

## Client libraries

| Language | Status | Install |
|---|---|---|
| **Python** | ✅ Done | `pip install ./clients/python/` |
| **Rust** (built-in CLI) | ✅ Done | CLI included in binary |
| TypeScript | 🔄 Planned | `npm install deskbrid` |

### Python example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Subscribe to events
@client.on("file.*")
def on_file_change(event):
    print(f"File changed: {event['path']}")

# Actions
client.windows_list()
client.keyboard_type("deploy production\n")
text = client.clipboard_read()
path = client.screenshot()

client.listen()  # blocks, streaming events
```

## How it works

Deskbrid binds a Unix socket at `$XDG_RUNTIME_DIR/deskbrid.sock`. Every interaction is one JSON line in → one JSON line out. Agents subscribe to events and get pushed real-time updates.

Under the hood it talks to:
- **GNOME Shell** via DBus (windows, workspaces)
- **Mutter** (IdleMonitor)
- **NetworkManager** (network, WiFi)
- **BlueZ** (Bluetooth)
- **UPower** (battery)
- **org.freedesktop.Notifications** (notifications)
- **grim** (Wayland screenshots)
- **wtype** (keyboard injection)
- **ydotool** (mouse control)
- **wl-paste/wl-copy** (clipboard)
- **pactl** (audio)
- **notify crate** (inotify file watching)

## Compared to alternatives

| Tool | Wayland | Agent-native | JSON protocol | Typed input | Screen capture |
|---|---|---|---|---|---|
| **deskbrid** | ✅ | ✅ | ✅ | ✅ | ✅ |
| xdotool | ❌ | ❌ | ❌ | ✅ | ❌ |
| ydotool | ✅ | ❌ | ❌ | ❌ | ❌ |
| atspi | limited | ❌ | ❌ | ❌ | ❌ |
| wtype | ✅ | ❌ | ❌ | ✅ | ❌ |
| grim | ✅ | ❌ | ❌ | ❌ | ✅ |

Deskbrid is the only tool that combines all of these into a single daemon with a structured protocol designed for programmatic use.

## Full protocol

See **[PROTOCOL.md](PROTOCOL.md)** for the complete JSON-over-socket specification.

## License

MIT
