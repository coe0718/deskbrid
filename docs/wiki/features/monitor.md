# Monitor Feature

The monitor feature queries and controls connected displays — listing their properties, setting the primary display, changing resolution, scale, rotation, and enabling or disabling outputs. This is the primary way for agents to manage multi-monitor desktop layouts.

## Actions

### monitor.list

List all connected monitors and their current properties: resolution, position, scale, rotation, refresh rate, and whether each is enabled or primary.

No parameters required.

```bash
deskbrid monitor list
```

```json
{"type": "monitor.list"}
```

Response (array of monitor objects):
```json
[
  {
    "id": 0,
    "name": "DP-1",
    "width": 3840,
    "height": 2160,
    "scale": 1.5,
    "primary": true,
    "enabled": true,
    "x": 0,
    "y": 0,
    "refresh_rate": 144.0,
    "rotation": "normal"
  },
  {
    "id": 1,
    "name": "HDMI-1",
    "width": 1920,
    "height": 1080,
    "scale": 1.0,
    "primary": false,
    "enabled": true,
    "x": 2560,
    "y": 0,
    "refresh_rate": 60.0,
    "rotation": "normal"
  }
]
```

### monitor.set_primary

Set a display as the primary monitor. The primary monitor typically receives the top-left origin, the login screen, and new window placements.

| Parameter | Type | Description |
|-----------|------|-------------|
| `output` | string (required) | Monitor name (e.g. `"DP-1"`, `"HDMI-1"`). Matches the `name` field from `monitor.list`. |

```bash
deskbrid monitor primary "DP-1"
```

```json
{"type": "monitor.set_primary", "output": "DP-1"}
```

Response:
```json
{"output": "DP-1", "primary": true}
```

### monitor.set_resolution

Change the resolution (and optionally refresh rate) of a connected monitor.

| Parameter | Type | Description |
|-----------|------|-------------|
| `output` | string (required) | Monitor name (e.g. `"DP-1"`). |
| `width` | uint (required) | Horizontal resolution in pixels. |
| `height` | uint (required) | Vertical resolution in pixels. |
| `refresh_rate` | float (optional) | Refresh rate in Hz. Also accepted as `refresh`. |

```bash
deskbrid monitor resolution "DP-1" 1920 1080 --refresh 60
```

```json
{"type": "monitor.set_resolution", "output": "DP-1", "width": 1920, "height": 1080, "refresh_rate": 60.0}
```

Response:
```json
{"output": "DP-1", "width": 1920, "height": 1080, "refresh_rate": 60.0}
```

### monitor.set_scale

Set the fractional scaling factor for a monitor. Common values: `1.0` (no scaling), `1.5`, `2.0` (HiDPI).

| Parameter | Type | Description |
|-----------|------|-------------|
| `output` | string (required) | Monitor name (e.g. `"DP-1"`). |
| `scale` | float (required) | Scaling factor (e.g. `1.0`, `1.5`, `2.0`). |

```bash
deskbrid monitor scale "DP-1" 1.5
```

```json
{"type": "monitor.set_scale", "output": "DP-1", "scale": 1.5}
```

Response:
```json
{"output": "DP-1", "scale": 1.5}
```

### monitor.set_rotation

Rotate the display output. Useful for portrait monitors or physically rotated displays.

| Parameter | Type | Description |
|-----------|------|-------------|
| `output` | string (required) | Monitor name (e.g. `"DP-1"`). |
| `rotation` | string (required) | One of: `"normal"`, `"left"` (90° CCW), `"right"` (90° CW), `"inverted"` (180°). |

```bash
deskbrid monitor rotate "DP-1" right
```

```json
{"type": "monitor.set_rotation", "output": "DP-1", "rotation": "right"}
```

Response:
```json
{"output": "DP-1", "rotation": "right"}
```

### monitor.enable

Enable a connected but disabled monitor so it becomes active.

| Parameter | Type | Description |
|-----------|------|-------------|
| `output` | string (required) | Monitor name (e.g. `"DP-1"`). |

```bash
deskbrid monitor enable "HDMI-1"
```

```json
{"type": "monitor.enable", "output": "HDMI-1"}
```

Response:
```json
{"output": "HDMI-1", "enabled": true}
```

### monitor.disable

Disable (turn off) an active monitor. The monitor remains connected but will not display anything.

| Parameter | Type | Description |
|-----------|------|-------------|
| `output` | string (required) | Monitor name (e.g. `"HDMI-1"`). |

```bash
deskbrid monitor disable "HDMI-1"
```

```json
{"type": "monitor.disable", "output": "HDMI-1"}
```

Response:
```json
{"output": "HDMI-1", "enabled": false}
```

## Safety Boundary

- Disabling the only active monitor will leave the desktop without a display — the system may fall back to a mirrored configuration or show a blank screen. Use `monitor.list` first to verify you have at least one other enabled monitor.
- Setting an unsupported resolution or refresh rate may result in a blank screen or fallback to the closest supported mode. Some desktop environments automatically revert changes after a confirmation timeout.
- Scaling changes affect the perceived DPI of applications. Setting very high scale factors on small monitors may make UI elements impractically large.
- Monitor names are compositor-specific — `"DP-1"`, `"HDMI-1"`, `"eDP-1"`, etc. Use `monitor.list` to discover available names before making changes.

## Local Development

```bash
# List monitors
deskbrid monitor list

# Test resolution change on a secondary monitor
deskbrid monitor resolution "HDMI-1" 1920 1080

# Cycle rotation to verify it works
deskbrid monitor rotate "HDMI-1" left
deskbrid monitor rotate "HDMI-1" normal

# Enable/disable a secondary monitor
deskbrid monitor enable "HDMI-1"
deskbrid monitor disable "HDMI-1"
```

To test without affecting real displays, use a nested compositor (e.g. `gnome-shell --nested` or `weston`) or the mock backend in integration tests.

## Configuration

No monitor-specific configuration options. Monitor detection and control is handled by the active desktop backend (GNOME, KDE, Hyprland, Sway, X11, etc.) using each compositor's native APIs or tools (`xrandr`, `wlr-randr`, `gnome-monitor-config`, etc.). The appropriate backend is auto-detected at daemon startup.
