# Location & UI Tree

Get the user's geographic location (via GeoClue) and interact with the
desktop UI accessibility tree — inspect elements, click, and set text.

## Actions

### location.get

Get the current geographic location. Requires GeoClue D-Bus service.

```bash
deskbrid location.get
```

```json
{"type": "location.get"}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "latitude": 37.7749,
    "longitude": -122.4194,
    "accuracy": 100.0,
    "timestamp": 1705312800
  }
}
```

### location.ui_tree_get

Get the accessibility tree from the currently focused window or application.

```bash
deskbrid ui.tree.get
```

```json
{"type": "ui.tree.get"}
```

Response includes a tree structure with node roles, names, states, and positions.

### location.ui_element_click

Click a UI element identified by a CSS-like selector in the accessibility tree.

| Parameter    | Type    | Description                                         |
|--------------|---------|-----------------------------------------------------|
| `selector`   | string  | Element selector (e.g., `button#submit`, `textfield`)|
| `tab_index`  | uint?   | Tab index within the window (for multi-tab apps)    |

```bash
deskbrid ui.element.click '{"selector": "button#submit"}'
```

```json
{"type": "ui.element.click", "selector": "button#submit"}
```

### location.ui_element_set_text

Set the text content of a UI element (e.g., a text field).

| Parameter    | Type    | Description                                         |
|--------------|---------|-----------------------------------------------------|
| `selector`   | string  | Element selector (e.g., `textfield#username`)       |
| `text`       | string  | Text to set                                         |
| `tab_index`  | uint?   | Tab index within the window                         |

```bash
deskbrid ui.element.set_text '{"selector": "textfield#username", "text": "admin"}'
```

```json
{
  "type": "ui.element.set_text",
  "selector": "textfield#username",
  "text": "admin"
}
```

## Python Example

```python
from deskbrid import Deskbrid
client = Deskbrid()

# Get location
loc = client.location_get()
print(f"Location: {loc['latitude']}, {loc['longitude']}")

# Get UI tree
tree = client.ui_tree_get()
print(tree)

# Click an element
client.ui_element_click(selector="button#submit")

# Fill a text field
client.ui_element_set_text(selector="textfield#username", text="admin")
```

## Requirements

- **Location**: Requires GeoClue D-Bus service (`org.freedesktop.GeoClue2`)
- **UI Tree**: Requires AT-SPI2 accessibility bus

## Current Status

**Stable** — location and UI tree interaction supported.
