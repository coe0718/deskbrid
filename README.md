# deskbrid

**The HAL your Linux desktop agents are missing.**

A standalone daemon that wraps Wayland protocols, DBus APIs, and PipeWire into a single JSON-over-Unix-socket protocol. One binary. Zero config. Every agent platform can use it.

```json
→ {"action": "inject:type", "params": {"text": "git push origin main\n"}}
← {"type": "event", "event": "clipboard", "data": {"text": "git push origin main"}}
```

## One-minute demo

```bash
cargo run daemon &
deskbrid info                         # see what's available
deskbrid subscribe window:focus       # watch what you click
deskbrid action screenshot            # take the screenshot
deskbrid action clipboard:read        # grab what's copied
```

Or run `bash demo.sh` for a full walkthrough.

## Why

macOS agents walk on water — AppleScript, Accessibility APIs, a platform that doesn't fight back. Linux agents have `xdotool` that breaks on Wayland and a pile of different DBus APIs with different conventions.

Every agent platform on Linux has the same problem. Deskbrid is the **one thing to install** that solves it for all of them.

## Install

```bash
# From source
cargo install deskbrid
deskbrid daemon

# Or from the repo
git clone https://github.com/coe0718/deskbrid
cd deskbrid
cargo run daemon

# Python client (optional — agents use this)
pip install ./clients/python/
```

Systemd user service:
```bash
cp deploy/deskbrid.service ~/.config/systemd/user/
systemctl --user enable --now deskbrid
```

## Prerequisites

```bash
sudo apt install wl-clipboard grim    # clipboard + screenshots
```

## What you can do

### 🖥️ Window control
| Action | What it does |
|---|---|
| `window:list` | List all open windows (title, app_id, pid, workspace, geometry) |
| `window:focus` | Focus a window by app_id or title |
| subscribe `window:focus` | Stream focus changes in real-time |

### ⌨️ Input injection
| Action | What it does |
|---|---|
| `inject:type` | Type text into the focused window |
| `inject:key` | Send key combos (ctrl+shift+t, alt+f4, super+d) |
| `inject:mouse` | Click, move, scroll the mouse |

### 📋 Clipboard
| Action | What it does |
|---|---|
| `clipboard:read` | Read current clipboard content |
| `clipboard:write` | Write to clipboard |
| subscribe `clipboard` | Watch for clipboard changes |

### 📸 Screen capture
| Action | What it does |
|---|---|
| `screenshot` | Capture the screen (gnome-screenshot or grim) |
| `screencast:start/stop` | Stream screen via PipeWire *(Phase 2)* |

### 🔔 Notifications
| Action | What it does |
|---|---|
| `notification:send` | Send a desktop notification |
| subscribe `notifications` | Watch incoming notifications |

### 📺 Display
| Action | What it does |
|---|---|
| `display:list` | List monitors (resolution, scale, refresh rate) |

### 🎵 Audio *(planned)*
| subscribe `audio:node` | Watch audio device state changes |

## Client libraries

| Language | Status | Install |
|---|---|---|
| **Python** | ✅ **Done** | `pip install ./clients/python/` |
| **Rust** (built-in CLI) | ✅ **Done** | `cargo install deskbrid` |
| **Hermes / Praxis** | ✅ **Done** | See [`hermes/`](hermes/) directory |
| TypeScript | 🔄 Planned | `npm install deskbrid` |

### Python

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Subscribe to events
@client.on("window:focus")
def on_focus(w):
    print(f"Focused: {w.app_id} — {w.title}")

# Actions
client.type_text("deploy production\n")
keys = client.send_keys(["ctrl", "shift", "t"])
text = client.clipboard_read().text
path = client.screenshot()

client.listen()  # blocks, streaming events
```

## Architecture

```
┌──────────────┐     JSON over Unix socket
│   Agent       │◄──────────────────────────┐
│ (any platform)│                           │
└──────────────┘                            │
                                     ┌──────┴──────────┐
                                     │  deskbrid daemon  │
                                     │  ┌────┐ ┌──────┐ │
                                     │  │DBus│ │Input │ │
                                     │  │Hub │ │Muttr │ │
                                     │  └─┬──┘ └──┬───┘ │
                                     │  ┌─▼──────▼───┐  │
                                     │  │  Clipboard │  │
                                     │  │  Screenshot│  │
                                     │  └────────────┘  │
                                     └──────────────────┘
```

## Supported desktops

| Desktop | Session | Status |
|---|---|---|
| GNOME 42+ | Wayland | ✅ Tested |
| GNOME 40+ | Wayland | ⚠️ Should work |
| KDE Plasma | Wayland | 🔄 Planned |
| Sway / wlroots | Wayland | 🔄 Planned |
| X11 | X11 | ❌ Not planned |

## Why standalone

Deskbrid is **not** tied to any agent platform. It's a Unix daemon with a documented protocol, like `pipewire` or `systemd`. Any agent — Hermes, Praxis, Claude Code, OpenAI Operator — implements the 15-line client and gets full desktop control.

> "But I can just call DBus from my agent."
>
> Cool. You'll need to learn GNOME Shell's Eval API, Mutter's RemoteDesktop session lifecycle, PipeWire's stream negotiation, the portal API for screenshots, and the notification spec. You'll need to figure out GVariant parsing. You'll need to handle permission grants and session teardown.
>
> Or you install one binary and send JSON.

```python
# The "agent platform integration" — this is it
from deskbrid import Deskbrid
client = Deskbrid()
client.type_text("git push\n")
```

## License

MIT
