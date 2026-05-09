# deskbrid

<p align="center">
  <img src="logo.png" alt="deskbrid logo" width="400">
</p>

**The HAL your Linux desktop agents are missing.**

A single daemon that wraps GNOME Shell, DBus, NetworkManager, BlueZ, PipeWire, and Wayland utilities into a JSON-over-Unix-socket protocol. One binary. Your shell scripts and AI agents use the same socket.

```bash
# A human using the CLI
deskbrid windows list
deskbrid keyboard type "git push origin main"

# An AI agent using the same socket
{"action": "windows.list"}
→ {"type": "response", "data": [{"title": "PatchHive — VS Code", "app_id": "code", ...}]}

{"action": "windows.focus", "window_id": "0x..."}
{"action": "input.keyboard", "action": "type", "text": "Hey Codex, fix the build\n"}
```

## Why now

Every major AI lab is racing to ship desktop agents — and Linux is the gap nobody has solved cleanly. AppleScript gives macOS agents native control. Windows has UI Automation. Linux has... `xdotool` that breaks on Wayland.

Deskbrid is the missing abstraction layer. It doesn't bet on agents taking off — automation use cases validate it today, agents validate it tomorrow. Same daemon, same protocol, same socket.

## Real-world example: an AI agent opening VS Code

```
→ {"action": "windows.list"}
← {"type": "response", "data": [
    {"title": "PatchHive — Visual Studio Code", "app_id": "code", "id": "0x3a0000b", ...},
    {"title": "praxis/src/daemon.rs — Visual Studio Code", "app_id": "code", "id": "0x2c0000a", ...}
  ]}

→ {"action": "windows.focus", "window_id": "0x3a0000b"}   # Bring PatchHive to front
← {"type": "response", "status": "ok"}

→ {"action": "input.mouse", "action": "move", "x": 900, "y": 920}
→ {"action": "input.mouse", "action": "click", "button": "left"}   # Click Codex chat input
→ {"action": "input.keyboard", "action": "type", "text": "Fix the build errors\n"}
```

The agent picks the right window by title, focuses it, clicks into the chat, and types.

## One-minute demo

```bash
# Fire up the daemon
deskbrid daemon &

# Query your desktop
deskbrid system info
deskbrid windows list
deskbrid battery
deskbrid network status

# Interact
deskbrid clipboard read
deskbrid screenshot
deskbrid notification send "deskbrid" "Hello" "👋"

# Watch for file changes
deskbrid subscribe '["file.*"]'    # then touch /tmp/test — you'll see events
```

## Install

```bash
git clone https://github.com/coe0718/deskbrid
cd deskbrid
cargo build --release
sudo cp target/release/deskbrid /usr/local/bin/

# Systemd user service
mkdir -p ~/.config/systemd/user/
cp deploy/deskbrid.service ~/.config/systemd/user/
systemctl --user enable --now deskbrid
```

## Supported desktops

| Desktop | Session | Status |
|---|---|---|
| **GNOME 46+** | Wayland | ✅ Supported |
| GNOME 42-45 | Wayland | ❌ Dropped in v2 |
| KDE Plasma | Wayland | 🔄 Planned |
| X11 | X11 | ❌ Not planned |

## Prerequisites

```bash
sudo apt install wl-clipboard grim wtype   # clipboard + screenshots + keyboard
# For mouse input, also install ydotool:
sudo apt install ydotool
```

## What you can do

### 🖥️ Windows & Workspaces
| Action | What it does |
|---|---|
| `windows.list` | List all open windows (title, app_id, workspace, geometry) |
| `windows.focus` | Focus a window by ID |
| `windows.get` | Get details for a specific window |
| `workspaces.list` | List workspaces |
| `workspaces.switch` | Switch to a workspace |
| `workspaces.move_window` | Move a window to another workspace |

### ⌨️ Input
| Action | What it does |
|---|---|
| `input.keyboard type` | Type text into the focused window |
| `input.keyboard key` | Send a single keypress |
| `input.keyboard combo` | Send key combos (ctrl+shift+t) |
| `input.mouse move` | Move mouse to absolute position |
| `input.mouse click` | Click (left/middle/right) |
| `input.mouse scroll` | Scroll (dx/dy) |

### 📋 Clipboard
| Action | What it does |
|---|---|
| `clipboard.read` | Read current Wayland clipboard |
| `clipboard.write` | Write to Wayland clipboard |

### 📸 Screenshots
| Action | What it does |
|---|---|
| `screenshot` | Capture screen (full, region, or window) |

### 🔔 Notifications
| Action | What it does |
|---|---|
| `notification.send` | Send a desktop notification |
| `notification.close` | Close a notification by ID |

### ⚙️ System
| Action | What it does |
|---|---|
| `system.info` | Desktop info (GNOME version, monitors, workspaces) |
| `system.idle` | Seconds since last user input |
| `system.battery` | Battery percentage, state, time remaining |
| `system.power` | Suspend, hibernate, shutdown, reboot, lock, logout |

### 🌐 Network
| Action | What it does |
|---|---|
| `network.status` | Online/offline via NetworkManager |
| `network.interfaces` | List interfaces with IPs |
| `network.wifi.scan` | Scan for WiFi networks |
| `network.wifi.connect` | Connect to a WiFi network |

### 📡 Bluetooth
| Action | What it does |
|---|---|
| `bluetooth.list` | List known/available devices |
| `bluetooth.scan` | Start device discovery |
| `bluetooth.stop_scan` | Stop discovery |
| `bluetooth.connect` | Connect to a device |
| `bluetooth.disconnect` | Disconnect from a device |

### 🎵 Audio
| Action | What it does |
|---|---|
| `audio.list_sinks` | List audio output devices |
| `audio.set_sink_volume` | Set sink volume (0.0-1.0) |

### 📁 Files
| Action | What it does |
|---|---|
| `files.search` | Search files by name (fd/find) |
| `files.watch` | Watch a path for changes (creates, modifies, deletes) |
| `files.unwatch` | Stop watching a path |

### 📡 Events (subscribe)
| Pattern | What you get |
|---|---|
| `file.*` | file.created, file.modified, file.deleted |
| `file.created` | Just file creation events |
| `*` | Everything |

## Client libraries

| Language | Status | Install |
|---|---|---|
| **Python** | ✅ Done | `pip install ./clients/python/` |
| **Rust** (built-in CLI) | ✅ Done | `cargo install deskbrid` |
| TypeScript | 🔄 Planned | `npm install deskbrid` |

### Python

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

Deskbrid is a single Rust binary that binds a Unix socket at `$XDG_RUNTIME_DIR/deskbrid.sock`. Every interaction is one JSON line → one JSON line. Agents subscribe to events by sending `{"action": "subscribe", "events": ["file.*"]}` and get pushed events as they happen.

Under the hood it talks to:
- **GNOME Shell** via DBus (windows, workspaces)
- **Mutter** (IdleMonitor)
- **NetworkManager** (network status, WiFi)
- **BlueZ** (Bluetooth)
- **UPower** (battery)
- **org.freedesktop.Notifications** (notifications)
- **grim** (Wayland screenshots)
- **wtype** (keyboard injection)
- **ydotool** (mouse control)
- **wl-paste/wl-copy** (clipboard)
- **pactl** (audio)
- **notify crate** (file watching with inotify)

## License

MIT
