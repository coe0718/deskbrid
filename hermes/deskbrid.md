---
name: deskbrid
description: Desktop control via Deskbrid daemon — inject keystrokes, read clipboard, take screenshots, list windows, check system health. Supports GNOME, Hyprland, KDE, and X11.
---

# Deskbrid Hermes Skill

Use this skill when a Hermes agent needs to interact with the local Linux desktop through a running Deskbrid daemon.

## Compositor support

Deskbrid v0.6.0 auto-detects the running desktop environment at startup via `$XDG_CURRENT_DESKTOP` and process scanning.

| Compositor | Status | Backend |
|---|---|---|
| **GNOME (Mutter)** | ✅ Full support | RemoteDesktop DBus + Shell Extension |
| **Hyprland** | ✅ Full support | hyprctl + ydotool + grim |
| **KDE (KWin)** | ✅ Full support | KWin D-Bus + ydotool + spectacle |
| **X11** | ✅ Basic support | xdotool + xclip + ImageMagick import |

**All CLI commands work identically on all four backends.** Your agent doesn't need to know which compositor is running.

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

### Automated setup

```bash
deskbrid setup
```

Auto-detects the desktop, installs the GNOME Shell extension if needed, or prints setup instructions for Hyprland/KDE/X11.

## Dependencies by backend

| Backend | Required tools |
|---|---|
| **GNOME** | GNOME Shell extension (installed via `deskbrid setup`), grim, wl-copy/wl-paste |
| **Hyprland** | hyprctl, ydotoold + ydotool, grim, wl-copy/wl-paste |
| **KDE** | qdbus6, spectacle, imagemagick convert, ydotoold + ydotool, wl-copy/wl-paste |
| **X11** | xdotool, xclip, imagemagick import, notify-send |

## Actions (v0.6.0 schema)

Each action returns a capabilities descriptor with:
- `supported` (bool) — whether the backend implements this
- `degraded` (bool) — works with limitations
- `reason` — why it's degraded (if applicable)
- `requires` — list of dependency keys to check via `system.health`
- `session` — wayland or any (session-type constraint)
- `degraded_modes` — modes that don't work fully

Check capabilities at runtime:
```bash
deskbrid system capabilities
```

### Unsupported across all backends

These actions are registered but return unsupported on all current backends:
- `windows.close`, `windows.minimize`, `windows.maximize`, `windows.move_resize`
- `ui.tree.get`, `ui.element.click`, `ui.element.set_text`
- `bluetooth.pair`, `bluetooth.forget`

## Permissions (v0.5.0+)

v0.5.0 adds scoped, per-UID permission gating via TOML config. By default (no config file) all actions are allowed. When a permissions file exists, the daemon checks every action against the caller's UID via `SO_PEERCRED`.

### Config location

```
~/.config/deskbrid/permissions.toml
```

### Example — restrict an agent to specific actions

```toml
# Allow everything to your primary user
[permissions.1000]
allow = ["*"]

# Agent UID gets window listing, screenshot, and read-only system info
[permissions.1001]
allow = [
    "windows.list",
    "windows.get",
    "screenshot",
    "system.info",
    "system.idle",
]
```

### How it works

- **No file** → all actions allowed (backward compatible)
- **Empty file** → all actions denied for all UIDs
- **Missing UID entry** → all actions denied for that UID
- **Glob patterns** → `*`, `windows.*`, `input.keyboard`, etc.
- **Multiple patterns** → any match allows the action
- **Deny-overrides-allow** semantics

### Permission names

```
windows.list, windows.focus, windows.get
workspaces.list, workspaces.switch, workspaces.move_window
input.keyboard, input.mouse
clipboard.read, clipboard.write
screenshot
notifications.send, notifications.close
system.info, system.capabilities, system.health, system.remediation
system.normalize_coords, system.idle, system.power, system.battery
network.status, network.interfaces, network.wifi_scan, network.wifi_connect
bluetooth.list, bluetooth.scan, bluetooth.stop_scan, bluetooth.connect,
bluetooth.disconnect, bluetooth.pair, bluetooth.forget
files.watch, files.unwatch, files.search
process.list, process.start
hotkeys.register, hotkeys.unregister
audio.list_sinks, audio.set_sink_volume
monitor.list, location.get
```

### Error response

```json
{"type": "response", "status": "error", "error": {"code": "PERMISSION_DENIED", "message": "Caller UID 1001 not allowed: screenshot"}}
```

## System APIs (v0.6.0)

### system.capabilities

Returns the full action map with per-action metadata. Use this to discover what's available on the current backend before calling actions.

```bash
deskbrid system capabilities
```

Example response:
```json
{
  "schema_version": 1,
  "backend": "gnome",
  "actions": {
    "input.keyboard": {
      "supported": true,
      "degraded": false,
      "reason": null,
      "requires": [],
      "session": "any",
      "degraded_modes": []
    },
    "windows.focus": {
      "supported": true,
      "degraded": false,
      "reason": null,
      "requires": ["gnome-extension"],
      "session": "any",
      "degraded_modes": []
    }
  }
}
```

### system.health

Returns per-dependency health checks plus remediation instructions.

```bash
deskbrid system health
```

Example:
```json
{
  "schema_version": 1,
  "backend": "hyprland",
  "deps": {
    "ydotoold": {"ok": false, "details": "not running"},
    "uinput": {"ok": true, "details": "write access"}
  },
  "remediation": {
    "ydotoold": "Start ydotoold in your user session (e.g. autostart entry).",
    "uinput": "udev rule: KERNEL==\"uinput\", GROUP=\"input\", MODE=\"0660\""
  }
}
```

### system.remediation

Auto-apply a remediation. Pass the dep key and `apply=true`.

```bash
# Check what would be done (dry-run)
deskbrid system remediate ydotoold

# Apply the fix
deskbrid system remediate ydotoold --apply
```

Supported remediations:
- `ydotoold` — starts ydotoold via nohup (verified by pgrep after launch)
- `kde_ydotoold_autostart` — creates XDG autostart desktop entry

### system.normalize_coords

Transform fractional coordinates to backend pixel coordinates given monitor scale. If no monitor specified, uses primary or first monitor.

```bash
deskbrid system normalize 0.5 0.5          # primary monitor center
deskbrid system normalize 0.5 0.5 --monitor 0
```

## CLI usage

```bash
# Windows
deskbrid windows list
deskbrid windows focus firefox
deskbrid windows get 0x55f...        # v0.6.0 — get window metadata by ID

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
deskbrid system capabilities          # v0.6.0
deskbrid system health                # v0.6.0
deskbrid system remediate <dep>       # v0.6.0
deskbrid system normalize <x> <y>     # v0.6.0
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

### Get window metadata by ID

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    # Get a specific window's title, app_id, workspace, geometry
    win = client.get_window(window_id="0x55f...")
    print(f"{win.title} — geometry: {win.geometry}")
finally:
    client.close()
```

### Check system health before acting

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    health = client.system_health()
    if not health["deps"]["ydotoold"]["ok"]:
        client.system_remediate("ydotoold", apply=True)
        print("Started ydotoold")
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

2. **/dev/uinput permissions** — ydotool needs write access:
   ```bash
   echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee /etc/udev/rules.d/99-input.rules
   sudo chmod 0660 /dev/uinput && sudo chgrp input /dev/uinput
   ```
   The user must be in the `input` group.

### Hyprland: all hyprctl commands fail from daemon

The daemon auto-detects the Hyprland instance at startup by scanning `/run/user/1000/hypr/` for the newest instance directory.

```bash
cat /proc/$(pgrep -f "deskbrid daemon" | head -1)/environ | tr '\0' '\n' | grep HYPRLAND
```

If `HYPRLAND_INSTANCE_SIGNATURE` is empty, the detection failed. Most common cause: the daemon started before the Hyprland session created its socket directory. Restart the daemon.

### KDE: ydotool returns empty error

Same root causes as Hyprland:

1. **ydotoold not running** — on KDE you need an XDG autostart entry:
   ```bash
   mkdir -p ~/.config/autostart
   cat > ~/.config/autostart/ydotoold.desktop << 'EOF'
   [Desktop Entry]
   Type=Application
   Name=ydotoold
   Exec=ydotoold
   Terminal=false
   X-KDE-autostart-phase=2
   EOF
   ```
   Then log out and back in, or start manually: `ydotoold &`

2. **/dev/uinput permissions** — Same fix as Hyprland (udev rule + `input` group).

### X11: commands return "not found"

X11 backend depends on:
```bash
# Required
xdotool          # window/input/workspace control
xclip            # clipboard read/write
imagemagick      # screenshots (import command)
notify-send      # desktop notifications
```

Install:
```bash
sudo apt install xdotool xclip imagemagick libnotify-bin
```

### Daemon not running

```bash
systemctl --user start deskbrid
# or manually:
./target/release/deskbrid daemon
```

### Socket not found

Socket is at `$XDG_RUNTIME_DIR/deskbrid.sock` (typically `/run/user/1000/deskbrid.sock`).
