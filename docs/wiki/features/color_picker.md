# Color Picker

Pick the color value at specific screen coordinates. Returns the color
as RGB and hex values.

## Actions

### color_picker.pick

Get the color at a specific pixel coordinate on screen.

| Parameter | Type   | Description                                       |
|-----------|--------|---------------------------------------------------|
| `x`       | uint   | X coordinate (screen-space pixels)                |
| `y`       | uint   | Y coordinate (screen-space pixels)                |
| `path`    | string?| Path to image file to pick from instead of screen |

```bash
deskbrid color.pick --x 100 --y 200
```

```json
{"type": "color.pick", "x": 100, "y": 200}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "rgb": [123, 45, 67],
    "hex": "#7b2d43"
  }
}
```

Pick from a file:

```json
{"type": "color.pick", "x": 50, "y": 50, "path": "/tmp/screenshot.png"}
```

## Python Example

```python
from deskbrid import Deskbrid
client = Deskbrid()

# Pick color from screen
color = client.color_pick(x=100, y=200)
print(f"RGB: {color['rgb']}, Hex: {color['hex']}")

# Pick color from image file
color = client.color_pick(x=50, y=50, path="/tmp/screenshot.png")
```

## Requirements

- Requires the daemon to have screen capture access (Wayland: PipeWire portal, X11: direct X server access)

## Current Status

**Stable** — screen and file color picking supported.
