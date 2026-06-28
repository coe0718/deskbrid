# Desktop Portal

Use the Freedesktop Desktop Portal API for sandbox-compatible screenshot
capture and screen recording. This is the recommended approach on Wayland
when running inside a sandbox (Flatpak, Snap, etc.).

## Actions

### desktop_portal.screenshot

Take a screenshot through the desktop portal (org.freedesktop.portal.Screenshot).

| Parameter     | Type    | Description                                          |
|---------------|---------|------------------------------------------------------|
| `interactive` | bool    | If true, shows a selection UI before capturing       |

```bash
deskbrid portal.screenshot
deskbrid portal.screenshot --interactive
```

```json
{"type": "portal.screenshot", "interactive": true}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "path": "/tmp/deskbrid_portal_screenshot_20240115_103000.png"
  }
}
```

### desktop_portal.screencast_start

Start screen recording through the desktop portal.

| Parameter     | Type   | Description                       |
|---------------|--------|-----------------------------------|
| `output_path` | string | File path for the recording output|

```bash
deskbrid portal.screencast.start '{"output_path": "/tmp/recording.mp4"}'
```

```json
{"type": "portal.screencast.start", "output_path": "/tmp/recording.mp4"}
```

### desktop_portal.screencast_stop

Stop the active portal screen recording.

```bash
deskbrid portal.screencast.stop
```

```json
{"type": "portal.screencast.stop"}
```

## Python Example

```python
from deskbrid import Deskbrid
client = Deskbrid()

# Interactive screenshot
result = client.portal_screenshot(interactive=True)
print(f"Screenshot saved to {result['path']}")

# Start recording
client.portal_screencast_start(output_path="/tmp/demo.mp4")

# Later, stop
client.portal_screencast_stop()
```

## When to Use the Portal

- **Wayland + Flatpak/Snap**: Always use the portal — direct PipeWire or
  X11 access may not be available inside sandboxes
- **Wayland (native)**: The non-portal `screenshot` and `screencast.*`
  actions work directly via PipeWire
- **X11**: The portal is bypassed in favor of direct X server access

## Requirements

- Requires `xdg-desktop-portal` and the appropriate backend
  (`xdg-desktop-portal-gnome`, `xdg-desktop-portal-kde`, etc.)
- May show a permission prompt on first use depending on desktop environment

## Current Status

**Stable** — portal screenshot and screencast supported.
