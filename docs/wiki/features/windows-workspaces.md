# Windows & Workspaces

Manage windows and virtual desktops programmatically.

## Windows

### List Windows

```bash
deskbrid windows list
```

```json
{"action": "windows.list"}
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "id": "12345678",
      "title": "deskbrid – ~/projects/deskbrid/docs/wiki",
      "app_id": "org.gnome.Terminal",
      "workspace": 1,
      "x": 100,
      "y": 50,
      "width": 1280,
      "height": 720,
      "focused": false,
      "minimized": false
    }
  ]
}
```

### Focus Window

```bash
deskbrid windows focus --app code
deskbrid windows focus --window 12345678
deskbrid windows focus --title "VS Code" --exact
```

Protocol:
```json
{"action": "windows.focus", "window_id": "code"}
```

### Get Window Details

```bash
deskbrid windows get 12345678
```

Protocol:
```json
{"action": "windows.get", "window_id": "12345678"}
```

### Close Window

```bash
deskbrid windows close --app code
```

Protocol:
```json
{"action": "windows.close", "window_id": "code"}
```

### Minimize/Maximize

```bash
deskbrid windows minimize 12345678
deskbrid windows maximize 12345678
```

Protocol:
```json
{"action": "windows.minimize", "window_id": "12345678"}
{"action": "windows.maximize", "window_id": "12345678"}
```

### Move and Resize

```bash
deskbrid windows move-resize 12345678 --x 100 --y 100 --width 800 --height 600
```

Protocol:
```json
{
  "action": "windows.move_resize",
  "window_id": "12345678",
  "x": 100,
  "y": 100,
  "width": 800,
  "height": 600
}
```

### Tile Window

```bash
deskbrid windows tile 12345678 --preset left
deskbrid windows tile 12345678 --preset right --padding 10
```

Presets:
- `left` - Left half
- `right` - Right half
- `max` - Maximize
- `center` - Center on screen
- `top-left`, `top-right`, `bottom-left`, `bottom-right`

Protocol:
```json
{
  "action": "windows.tile",
  "window_id": "12345678",
  "preset": "left",
  "monitor": 0,
  "padding": 10
}
```

### Activate or Launch

```bash
deskbrid windows activate-or-launch code
deskbrid windows activate-or-launch firefox --command ["firefox", "--new-window"]
```

Protocol:
```json
{
  "action": "windows.activate_or_launch",
  "app_id": "code",
  "command": ["code", "--new-window"],
  "workdir": "/home/user/projects"
}
```

## Workspaces

### List Workspaces

```bash
deskbrid workspaces list
```

Protocol:
```json
{"action": "workspaces.list"}
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"id": 1, "name": "1", "focused": true},
    {"id": 2, "name": "2", "focused": false},
    {"id": 3, "name": "3", "focused": false}
  ]
}
```

### Switch Workspace

```bash
deskbrid workspaces switch 2
```

Protocol:
```json
{"action": "workspaces.switch", "workspace_id": 2}
```

### Move Window to Workspace

```bash
deskbrid workspaces move-window 12345678 --workspace 3
deskbrid workspaces move-window 12345678 --workspace 3 --follow
```

Protocol:
```json
{
  "action": "workspaces.move_window",
  "window_id": "12345678",
  "workspace_id": 3,
  "follow": true
}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# List and focus VS Code
windows = client.list_windows()
code_window = next((w for w in windows if w.app_id == 'code'), None)
if code_window:
    client.focus_window(app_id='code')
    client.type_text("Fixed the issue!\n")
```

## AI Agent Example

```json
→ {"action": "windows.list"}
← [{"id": "abc123", "title": "VS Code", "app_id": "code", ...}]

→ {"action": "windows.focus", "window_id": "abc123"}
← {"type": "response", "status": "ok"}

→ {"action": "input.keyboard", "action": "type", "text": "git status\n"}
```