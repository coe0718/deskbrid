# Accessibility (AT-SPI)

Inspect accessibility trees via AT-SPI.

## Get Accessibility Tree

```bash
deskbrid a11y tree
deskbrid a11y tree --app code
deskbrid a11y tree --window 12345678
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "role": "application",
    "name": "Visual Studio Code",
    "description": "",
    "state": ["enabled", "focusable"],
    "children": [
      {
        "role": "window",
        "name": "main.rs - project",
        "children": [
          {
            "role": "button",
            "name": "Close",
            "state": ["enabled", "focusable"]
          }
        ]
      }
    ]
  }
}
```

Protocol:
```json
{"action": "a11y.tree", "app": "code"}
```

## Find Accessibility Node

```bash
deskbrid a11y find --role button --name "Close" --app code
```

Protocol:
```json
{
  "action": "a11y.find",
  "role": "button",
  "name": "Close",
  "app": "code"
}
```

## Perform Action

```bash
deskbrid a11y action --app code --path "/window[0]/button[0]" --action click
deskbrid a11y action --app code --path "/window[0]/button[0]" --action focus
```

Protocol:
```json
{
  "action": "a11y.action",
  "path": "/window[0]/button[0]",
  "action_name": "click",
  "app": "code"
}
```

## Get Node Value

```bash
deskbrid a11y value --app code --path "/window[0]/text[0]"
```

Protocol:
```json
{
  "action": "a11y.get_value",
  "path": "/window[0]/text[0]",
  "app": "code"
}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Get accessibility tree for VS Code
tree = client.a11y_tree(app="code")

# Find close button
close_button = client.a11y_find(
    role="push button",
    name="Close",
    app="code"
)

# Click it
client.a11y_action(path=close_button["path"], action_name="click")
```

## AT-SPI Roles

Common roles:
- `application` - Application root
- `window` - Window
- `dialog` - Dialog box
- `button` - Push button
- `checkbox` - Checkbox
- `text` - Text field
- `menu` - Menu
- `menuitem` - Menu item
- `table` - Table
- `cell` - Table cell

## Accessibility States

Common states:
- `enabled` - Element is enabled
- `disabled` - Element is disabled
- `focused` - Element has focus
- `focusable` - Element can receive focus
- `visible` - Element is visible
- `hidden` - Element is hidden
- `checked` - Checkbox/radio is checked
- `selected` - Item is selected