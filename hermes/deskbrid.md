---
name: deskbrid
description: Desktop control via Deskbrid daemon — inject keystrokes, read clipboard, take screenshots, list windows
---

# Deskbrid Hermes Skill

Use this skill when a Hermes agent needs to interact with the local Linux desktop through a running Deskbrid daemon.

## Requirement

Deskbrid must already be running and listening on `$XDG_RUNTIME_DIR/deskbrid.sock`.

## Connect from Hermes

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

### Read or write the clipboard

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    print(client.clipboard_read().text)
    client.clipboard_write("new clipboard contents")
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

### Send a desktop notification

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    client.notify("Hermes", "Task finished")
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

## Event Subscription

Watch for file system events:

```python
from deskbrid import Deskbrid

client = Deskbrid()

@client.on("file.created")
def on_create(event):
    print(f"Created: {event['path']}")

@client.on("file.*")
def on_change(event):
    print(f"{event['kind']}: {event['path']}")

client.listen()
```

## Practical Guidance

- Use `client.info()` first to inspect daemon capabilities
- Use `client.focus_window(app_id="code")` to target a specific application before typing
- Expect input injection to require a GNOME Wayland session
- Prefer short, explicit operations over long unverified chains
- The daemon binds at `$XDG_RUNTIME_DIR/deskbrid.sock` (typically `/run/user/1000/deskbrid.sock`)

## Troubleshooting

### Windows/workspaces actions return INTERNAL_ERROR

This means the GNOME Shell extension is not active. Check its state:

```bash
gnome-extensions info deskbrid@deskbrid | grep State
```

**State: INACTIVE** — the gsettings flag is set but GNOME Shell hasn't loaded the extension. On GNOME 46 Wayland, `ReloadExtension` is deprecated, `gnome-extensions enable` doesn't trigger a reload, and `Alt+F2` → `r` doesn't work (Wayland). The ONLY way to force a reload without logout:

```bash
busctl --user call org.gnome.Shell /org/gnome/Shell org.gnome.Shell.Extensions DisableExtension s "deskbrid@deskbrid"
sleep 1
busctl --user call org.gnome.Shell /org/gnome/Shell org.gnome.Shell.Extensions EnableExtension s "deskbrid@deskbrid"
sleep 2
gnome-extensions info deskbrid@deskbrid | grep State  # Should show ACTIVE
```

### Extension works then silently dies (~10 minutes)

Known GNOME 46 GJS GC bug. The extension needs a GC root in `enable()`: set `_extensionInstance = this`. Without it, GJS garbage-collects the Extension instance and GNOME Shell calls `disable()`. Fixed in extension.js as of commit `1e75b06`.

If you encounter this and can't update the extension code, use the DBus reload trick above — it buys you another ~10 minutes.

### Daemon not running

```bash
systemctl --user start deskbrid
# or manually:
~/projects/deskbrid/target/release/deskbrid daemon
```

### Socket not found

Check `echo $XDG_RUNTIME_DIR` — socket is at `$XDG_RUNTIME_DIR/deskbrid.sock` (typically `/run/user/1000/deskbrid.sock`).

### Extension stuck INACTIVE after Disable/Enable

The DBus reload trick sometimes stops working. GNOME Shell caches the disabled state and refuses to reload the extension even after `EnableExtension` returns success. **Fix:** bump the version number in metadata.json to force GNOME Shell to treat it as a new load:

```bash
cd ~/.local/share/gnome-shell/extensions/deskbrid@deskbrid
python3 -c "
import json
with open('metadata.json') as f: m = json.load(f)
m['version'] = m.get('version', 1) + 1
with open('metadata.json', 'w') as f: json.dump(m, f, indent=2)
"
```

Then retry the DBus Disable/Enable cycle.

### Launching GUI apps from Hermes terminal (VS Code, browser, etc.)

When using Hermes's `terminal()` tool to launch GUI apps on the same Wayland desktop, you MUST export all display environment variables. **Without XAUTHORITY, Electron apps crash with SIGSEGV.**

```bash
export DISPLAY=:0 DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/1000/bus" \
       WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 \
       XAUTHORITY=$(ls /run/user/1000/.mutter-Xwaylandauth.* 2>/dev/null | head -1)
```

The XAUTHORITY path is machine-specific. Find it with `ls /run/user/1000/.mutter-Xwaylandauth.*`.

**Redaction pitfall:** Hermes's `redact_secrets` feature may mask the XAUTHORITY path (the random suffix looks like a token). Use `ls` directly to get the real path — don't rely on reading `/proc/*/environ`.
