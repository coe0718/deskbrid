# deskbrid Python Client

Python client for the Deskbrid daemon — connects over Unix socket for typed desktop actions, event subscriptions, and automatic decoding of protocol responses.

## Install

```bash
pip install ./clients/python
```

Requires the [Deskbrid daemon](https://github.com/coe0718/deskbrid) to be running.

## Quick Start

```python
from deskbrid import Deskbrid

client = Deskbrid()

# List windows
windows = client.list_windows()
for w in windows:
    print(f"{w.app_id}: {w.title} (focused={w.is_focused})")

# Type into focused window
client.type_text("hello from Python\n")

# Read clipboard
clip = client.clipboard_read()
print(clip.text)

client.close()
```

## Sync vs Async

Use `Deskbrid` for blocking calls from normal Python code. Use `AsyncDeskbrid` for asyncio applications.

### Synchronous

```python
from deskbrid import Deskbrid

client = Deskbrid()
client.type_text("sync mode\n")
print(client.list_windows())
client.close()
```

### Asynchronous

```python
import asyncio
from deskbrid import AsyncDeskbrid

async def main():
    client = AsyncDeskbrid()
    await client.connect()
    await client.type_text("async mode\n")
    print(await client.list_windows())
    await client.close()

asyncio.run(main())
```

## Event Subscriptions

```python
from deskbrid import Deskbrid

client = Deskbrid()

@client.on("file.created")
def on_file_created(event):
    print(f"File created: {event['path']}")

@client.on("file.*")
def on_file_change(event):
    print(f"File event: {event['kind']} -> {event['path']}")

client.listen()  # blocks, streaming events
```

## Full API

### Windows

| Method | Description |
|---|---|
| `list_windows() -> list[WindowInfo]` | List all open windows |
| `focus_window(*, app_id=None, title=None, exact=False)` | Focus a window by app_id or title |
| `activate_or_launch(app_id, command=None, workdir=None, env=None) -> dict` | Focus an app if open, launch it if not |

### Layout Profiles

| Method | Description |
|---|---|
| `save_layout_profile(name, overwrite=False) -> dict` | Save current windows, monitors, workspaces, and active workspace |
| `list_layout_profiles() -> list[dict]` | List saved profile summaries |
| `get_layout_profile(name) -> dict` | Get a saved profile snapshot |
| `restore_layout_profile(name) -> dict` | Restore a saved profile |
| `delete_layout_profile(name) -> dict` | Delete a saved profile |

### Input

| Method | Description |
|---|---|
| `type_text(text: str)` | Type text into focused window |
| `send_keys(keys: list[str])` | Send key combo (e.g. `["ctrl", "t"]`) |
| `mouse_click(x, y, button="left")` | Click at position |
| `mouse_move(x, y)` | Move mouse to position |
| `mouse_scroll(dx=0.0, dy=0.0)` | Scroll |

### Clipboard, Screenshots, Notifications

| Method | Description |
|---|---|
| `clipboard_read() -> ClipboardContent` | Read clipboard |
| `clipboard_write(text: str)` | Write to clipboard |
| `screenshot(monitor=None) -> ScreenshotResult` | Capture screen |
| `screenshot_ocr(path=None, language=None, psm=None, bounding_boxes=False, monitor=None, region=None, window_id=None) -> dict` | Extract text from a screenshot path or fresh capture |
| `screenshot_diff(before_path, after_path=None, tolerance=None, diff_path=None, save_diff=False, monitor=None, region=None, window_id=None) -> dict` | Compare screenshots and optionally save a visual diff |
| `notify(title, body="", urgency="normal") -> int` | Send desktop notification |

### Terminal / PTY

| Method | Description |
|---|---|
| `terminal_create(shell=None, cwd=None, env=None, rows=None, cols=None) -> dict` | Create an interactive PTY session |
| `terminal_write(terminal_id, input) -> dict` | Write to a PTY session |
| `terminal_read(terminal_id, max_bytes=None, flush=True) -> dict` | Read buffered PTY output |
| `terminal_resize(terminal_id, rows, cols) -> dict` | Resize a PTY session |
| `terminal_list() -> list[dict]` | List active PTY sessions |
| `terminal_kill(terminal_id, signal=None) -> dict` | Signal and remove a PTY session |

### System

| Method | Description |
|---|---|
| `info() -> DaemonInfo` | Desktop info, monitors, capabilities |
| `list_displays() -> list[MonitorInfo]` | List connected displays |
| `set_primary_monitor(output) -> dict` | Set the primary output where supported |
| `set_monitor_resolution(output, width, height, refresh_rate=None) -> dict` | Set output resolution and optional refresh rate |
| `set_monitor_scale(output, scale) -> dict` | Set output scale |
| `set_monitor_rotation(output, rotation) -> dict` | Set output rotation: `normal`, `left`, `right`, `inverted` |
| `enable_monitor(output) -> dict` | Enable an output |
| `disable_monitor(output) -> dict` | Disable an output |
| `wait_for(condition, params=None, timeout_ms=30000, interval_ms=None) -> dict` | Wait for a daemon-polled condition |
| `audit_log(limit=None, action_type=None, status=None) -> list[dict]` | Read recent action audit entries |
| `audit_clear() -> dict` | Clear the in-memory audit log |
| `confinement() -> dict` | Detect sandbox/container/security confinement context |

## Data Models

| Type | Fields |
|---|---|
| `WindowInfo` | `id`, `title`, `app_id`, `pid`, `workspace_id`, `is_focused`, `is_minimized`, `geometry` |
| `ClipboardContent` | `text`, `mime_types`, `timestamp` |
| `MonitorInfo` | `id`, `name`, `width`, `height`, `scale`, `primary`, `enabled`, `x`, `y`, `refresh_rate`, `rotation` |
| `ScreenshotResult` | `path`, `width`, `height` |
| `DaemonInfo` | `desktop`, `desktop_version`, `compositor`, `session_type`, `monitors`, `workspace_count`, `current_workspace`, `idle_seconds` |

## Error Handling

All failures raise `DeskbridError`:

```python
from deskbrid import Deskbrid, DeskbridError

client = Deskbrid()
try:
    client.focus_window(title="Does Not Exist")
except DeskbridError as e:
    print(f"{e.code}: {e.message}")
finally:
    client.close()
```

## Notes

- Requires a running Deskbrid daemon on `$XDG_RUNTIME_DIR/deskbrid.sock`
- Desktop is auto-detected: GNOME (Mutter RemoteDesktop), Hyprland (hyprctl + ydotool + grim), or KDE (KWin D-Bus + ydotool + spectacle) — same client, same API
- Raw protocol requests may include `dry_run` and `timeout_ms` fields for permission-only validation and per-request action timeouts
- On Hyprland and KDE: `ydotoold` must be running and `/dev/uinput` must be writable by the `input` group
- On KDE: screenshots use `spectacle` + ImageMagick `convert` (not grim)
