---
name: deskbrid
description: Desktop control via Deskbrid daemon — inject keystrokes, read clipboard, take screenshots, list windows. Supports GNOME and Hyprland.
---

# Deskbrid Hermes Skill

Use this skill when a Hermes agent needs to interact with the local Linux desktop through a running Deskbrid daemon.

## Compositor support

Deskbrid v0.3.0 auto-detects the running desktop environment at startup. Detection order: `$XDG_CURRENT_DESKTOP` → process scan (`pgrep Hyprland`, `pgrep kwin_wayland`) → GNOME fallback.

| Compositor | Status | Backend |
|---|---|---|
| **GNOME (Mutter)** | ✅ Full support | RemoteDesktop DBus + Shell Extension |
| **Hyprland** | ✅ Full support (v0.3.0) | hyprctl + ydotool + grim |
| **KDE (KWin)** | 🔜 Planned | — |

**All CLI commands work identically on both backends.** Your agent doesn't need to know which compositor is running — `deskbrid windows list`, `deskbrid input type`, `deskbrid screenshot` all work the same way.

## Requirement

Deskbrid must already be running and listening on `$XDG_RUNTIME_DIR/deskbrid.sock`.

### Starting the daemon

```bash
# Manual
./target/release/deskbrid daemon &

# From Hermes terminal (background)
terminal("~/deskbrid daemon", background=True)
```

The daemon auto-detects your compositor and loads the right backend.

## CLI usage

```bash
# Windows
deskbrid windows list
deskbrid windows focus firefox
deskbrid windows focus "code"        # by app_id substring
deskbrid windows focus "0x55f..."    # by hex address

# Workspaces
deskbrid workspaces list
deskbrid workspaces switch 2

# Input
deskbrid input type "git push\n"
deskbrid key "Enter"
deskbrid combo "ctrl+l"

# Screenshot
deskbrid screenshot

# Clipboard
deskbrid clipboard read
deskbrid clipboard write "text"

# System
deskbrid system info
```

## Connect from Hermes (Python)

Inside `execute_code`, import the Python client:

```python
from deskbrid import Deskbrid

client = Deskbrid()
```

Close when done:

```python
client.close()
```

## Common Examples

### Check what window is focused

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    windows = client.list_windows()
    focused = [w for w in windows if w.is_focused]
    if focused:
        print(f"Focused: {focused[0].app_id} — {focused[0].title}")
finally:
    client.close()
```

### Type into the focused window

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    client.type_text("command\n")
finally:
    client.close()
```

### Take a screenshot

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    result = client.screenshot()
    print(result.path)
finally:
    client.close()
```

### Focus a specific window, then type

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    client.focus_window(app_id="code")
    client.type_text("Fix the build errors\n")
finally:
    client.close()
```

## Troubleshooting

### GNOME: Windows/workspaces return INTERNAL_ERROR

The GNOME Shell extension is not active:

```bash
gnome-extensions info deskbrid@deskbrid | grep State
```

If INACTIVE, bump the extension version to force a reload:

```bash
cd ~/.local/share/gnome-shell/extensions/deskbrid@deskbrid
python3 -c "
import json
with open('metadata.json') as f: m = json.load(f)
m['version'] = m.get('version', 1) + 1
with open('metadata.json', 'w') as f: json.dump(m, f, indent=2)
"
busctl --user call org.gnome.Shell /org/gnome/Shell org.gnome.Shell.Extensions DisableExtension s "deskbrid@deskbrid"
sleep 1
busctl --user call org.gnome.Shell /org/gnome/Shell org.gnome.Shell.Extensions EnableExtension s "deskbrid@deskbrid"
```

### Hyprland: ydotool returns empty error

Two causes:

1. **ydotoold not running** — start it via `hyprctl dispatch exec ydotoold` or add `exec-once = ydotoold` to `hyprland.conf`.

2. **/dev/uinput permissions** — ydotool needs write access. On Arch/EndeavourOS, `/dev/uinput` is root-only by default:
   ```bash
   echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee /etc/udev/rules.d/99-input.rules
   sudo chmod 0660 /dev/uinput && sudo chgrp input /dev/uinput
   ```
   The user must be in the `input` group.

### Hyprland: all hyprctl commands fail from daemon

The daemon auto-detects the Hyprland instance at startup by scanning `/run/user/1000/hypr/` for the newest instance directory. Verify detection works:

```bash
# Check if daemon found the instance
cat /proc/$(pgrep -f "deskbrid daemon" | head -1)/environ | tr '\0' '\n' | grep HYPRLAND
```

If `HYPRLAND_INSTANCE_SIGNATURE` is empty or unset, the detection failed. Most common cause: the daemon started before the Hyprland session created its socket directory. Restart the daemon.

### Daemon not running

```bash
systemctl --user start deskbrid
# or manually:
./target/release/deskbrid daemon
```

### Socket not found

Socket is at `$XDG_RUNTIME_DIR/deskbrid.sock` (typically `/run/user/1000/deskbrid.sock`).
