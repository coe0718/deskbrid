# Input

Inject keyboard and mouse input into the desktop, and manage keyboard layout
profiles.

## Actions

### input.keyboard_type

Type a string of text at the current cursor position. Uses uinput or the
desktop's virtual keyboard interface.

| Parameter | Type   | Description                      |
|-----------|--------|----------------------------------|
| `text`    | string | Text to type                     |

```bash
deskbrid input.keyboard_type "Hello, world!"
```

```json
{
  "type": "input.keyboard_type",
  "text": "Hello, world!"
}
```

### input.keyboard_key

Press and release a single key.

| Parameter | Type   | Description                                              |
|-----------|--------|----------------------------------------------------------|
| `key`     | string | Key name (e.g. `a`, `Return`, `Escape`, `Tab`, `space`) |

```bash
deskbrid input.keyboard_key Return
deskbrid input.keyboard_key Escape
```

```json
{
  "type": "input.keyboard_key",
  "key": "Return"
}
```

### input.keyboard_combo

Press a chord of keys simultaneously (modifier + key combos).

| Parameter | Type     | Description                                   |
|-----------|----------|-----------------------------------------------|
| `keys`    | string[] | List of key names, e.g. `["Ctrl", "c"]`        |

```bash
deskbrid input.keyboard_combo '["Ctrl", "c"]'
deskbrid input.keyboard_combo '["Ctrl", "Shift", "Escape"]'
deskbrid input.keyboard_combo '["Alt", "Tab"]'
```

```json
{
  "type": "input.keyboard_combo",
  "keys": ["Ctrl", "c"]
}
```

### input.mouse

Perform a mouse action at a specific position, with optional button and
scroll delta.

| Parameter | Type    | Description                                       |
|-----------|---------|---------------------------------------------------|
| `action`  | string  | `click`, `double_click`, `right_click`, `move`, `scroll`, `press`, `release` |
| `x`       | float?  | Absolute X coordinate (optional)                   |
| `y`       | float?  | Absolute Y coordinate (optional)                   |
| `button`  | string? | `left`, `middle`, `right` (optional, default `left`) |
| `dx`      | float?  | Scroll delta X (for `scroll` action)               |
| `dy`      | float?  | Scroll delta Y (for `scroll` action)               |

```bash
deskbrid input.mouse '{"action": "move", "x": 500, "y": 300}'
deskbrid input.mouse '{"action": "click", "x": 500, "y": 300}'
deskbrid input.mouse '{"action": "right_click", "x": 500, "y": 300}'
deskbrid input.mouse '{"action": "scroll", "dy": -3}'
deskbrid input.mouse '{"action": "double_click", "x": 500, "y": 300}'
```

```json
{
  "type": "input.mouse",
  "action": "click",
  "x": 500.0,
  "y": 300.0,
  "button": "left"
}
```

### input.mouse_drag

Drag the mouse from one coordinate to another over a specified duration.

| Parameter     | Type    | Description                              |
|---------------|---------|------------------------------------------|
| `from_x`      | float   | Starting X                               |
| `from_y`      | float   | Starting Y                               |
| `to_x`        | float   | Ending X                                 |
| `to_y`        | float   | Ending Y                                 |
| `button`      | string? | `left` (default), `middle`, `right`      |
| `duration_ms` | uint?   | Drag duration in ms (default: 200)       |

```bash
deskbrid input.mouse_drag '{"from_x": 100, "from_y": 100, "to_x": 500, "to_y": 300}'
```

```json
{
  "type": "input.mouse_drag",
  "from_x": 100.0,
  "from_y": 100.0,
  "to_x": 500.0,
  "to_y": 300.0,
  "button": "left",
  "duration_ms": 200
}
```

### input.list_layouts

List all configured keyboard layouts.

```bash
deskbrid input.list_layouts
```

No parameters.

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"index": 0, "name": "us", "variant": null},
    {"index": 1, "name": "de", "variant": "neo"},
    {"index": 2, "name": "us", "variant": "dvorak"}
  ]
}
```

### input.get_layout

Get the currently active keyboard layout.

```bash
deskbrid input.get_layout
```

No parameters.

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {"index": 0, "name": "us", "variant": null}
}
```

### input.set_layout

Switch to a different keyboard layout by index, name, or name+variant.

| Parameter | Type    | Description                        |
|-----------|---------|------------------------------------|
| `index`   | uint?   | Layout index from list             |
| `name`    | string? | Layout name (e.g. `us`, `de`)     |
| `variant` | string? | Layout variant (e.g. `dvorak`, `neo`) |

At least one of `index` or `name` is required. If both `name` and `variant` are
provided, switches to the matching layout.

```bash
deskbrid input.set_layout '{"index": 1}'
deskbrid input.set_layout '{"name": "de", "variant": "neo"}'
```

```json
{
  "type": "input.set_layout",
  "name": "de",
  "variant": "neo"
}
```

### input.add_layout

Add a new keyboard layout to the configuration.

| Parameter | Type    | Description              |
|-----------|---------|--------------------------|
| `name`    | string  | Layout name (e.g. `fr`)  |
| `variant` | string? | Layout variant (optional) |

```bash
deskbrid input.add_layout fr
deskbrid input.add_layout '{"name": "us", "variant": "colemak"}'
```

```json
{
  "type": "input.add_layout",
  "name": "us",
  "variant": "colemak"
}
```

### input.remove_layout

Remove a keyboard layout by its index.

| Parameter | Type | Description     |
|-----------|------|-----------------|
| `index`   | uint | Layout index to remove |

```bash
deskbrid input.remove_layout 2
```

```json
{
  "type": "input.remove_layout",
  "index": 2
}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Type text
client.input_keyboard_type("Hello from Deskbrid!\n")

# Send keyboard shortcut
client.input_keyboard_combo(["Ctrl", "c"])

# Click at a position
client.input_mouse(action="click", x=500, y=300)

# Drag across the screen
client.input_mouse_drag(from_x=100, from_y=100, to_x=500, to_y=300)

# Switch keyboard layout
layouts = client.input_list_layouts()
client.input_set_layout(index=(layouts[1]["index"] if len(layouts) > 1 else 0))
```

## Requirements

- **X11**: Uses Xtest / XSendEvent. Requires `xdotool` or `libxdo`.
- **Wayland**: Requires the `uinput` kernel module + write access to
  `/dev/uinput`. The install script sets up the `uinput` udev rule.
- **GNOME**: Also supports the D-Bus virtual keyboard interface.
- Keyboard layout management requires `setxkbmap` (X11) or `gsettings`/`swaymsg`
  (Wayland compositors).

## Safety Boundary

- Input injection is a privileged operation. On most systems, the agent must
  have the appropriate group membership (`input` group for uinput).
- Confirmation mode may require explicit approval for keyboard input actions.
- Mouse coordinates use absolute screen coordinates — use
  `system.normalize_coords` if working with logical coordinates on mixed-DPI
  setups.

## Current Status

**Stable** — keyboard typing, key combos, mouse click/move/scroll.
**Experimental** — mouse drag, keyboard layout management.
