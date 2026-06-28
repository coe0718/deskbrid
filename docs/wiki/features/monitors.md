# Monitors

Configure display settings.

> **Note:** The canonical documentation for display configuration is in
> [monitor.md](monitor.md) (using `monitor.*` actions). This page is a
> convenience alias with the `monitors.*` CLI prefix.

## List Displays

```bash
deskbrid monitors list
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "output": "HDMI-A-1",
      "width": 1920,
      "height": 1080,
      "refresh": 60.0,
      "active": true,
      "primary": true,
      "scale": 1.0,
      "transform": "normal"
    },
    {
      "output": "DP-1",
      "width": 2560,
      "height": 1440,
      "refresh": 144.0,
      "active": true,
      "primary": false,
      "scale": 1.0,
      "transform": "normal"
    }
  ]
}
```

Protocol:
```json
{"type": "monitor.list"}
```

## Set Primary Monitor

```bash
deskbrid monitors set-primary DP-1
```

Protocol:
```json
{"type": "monitor.set_primary", "output": "DP-1"}
```

## Set Resolution

```bash
deskbrid monitors set-resolution DP-1 --width 2560 --height 1440
deskbrid monitors set-resolution DP-1 --width 2560 --height 1440 --rate 144
```

Protocol:
```json
{
  "type": "monitor.set_resolution",
  "output": "DP-1",
  "width": 2560,
  "height": 1440,
  "refresh_rate": 144.0
}
```

## Set Scale

```bash
deskbrid monitors set-scale DP-1 --scale 1.5
deskbrid monitors set-scale DP-1 --scale 2.0  # HiDPI
```

Protocol:
```json
{"type": "monitor.set_scale", "output": "DP-1", "scale": 2.0}
```

## Set Rotation

```bash
deskbrid monitors set-rotation DP-1 --rotation normal
deskbrid monitors set-rotation DP-1 --rotation left
deskbrid monitors set-rotation DP-1 --rotation right
deskbrid monitors set-rotation DP-1 --rotation upside-down
```

Protocol:
```json
{"type": "monitor.set_rotation", "output": "DP-1", "rotation": "left"}
```

## Enable/Disable Monitor

```bash
deskbrid monitors enable DP-1
deskbrid monitors disable DP-1
```

Protocol:
```json
{"type": "monitor.enable", "output": "DP-1"}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# List monitors
monitors = client.list_displays()
for m in monitors:
    print(f"{m.output}: {m.width}x{m.height}@{m.refresh}Hz")

# Set primary
client.set_primary_monitor("DP-1")

# Enable HiDPI
client.set_monitor_scale("DP-1", 2.0)
```