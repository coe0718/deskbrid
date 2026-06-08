# Windows & Workspaces

Deskbrid v1.0.0 manages windows and virtual desktops through dot-notation
actions routed over the daemon socket.

## Windows

### List windows

```bash
deskbrid windows.list
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "id": "12345678",
      "title": "VS Code",
      "app_id": "code",
      "workspace": 1,
      "x": 0,
      "y": 0,
      "width": 1280,
      "height": 720,
      "focused": true,
      "minimized": false
    }
  ]
}
```

### Focus a window

```bash
deskbrid windows.focus { window_id: "12345678" }
deskbrid windows.focus { app_id: "code" }
deskbrid windows.focus { title: "VS Code", exact: true }
```

### Get window details

```bash
deskbrid windows.get { window_id: "12345678" }
```

### Close window

```bash
deskbrid windows.close { window_id: "12345678" }
```

### Minimize / maximize

```bash
deskbrid windows.minimize { window_id: "12345678" }
deskbrid windows.maximize { window_id: "12345678" }
```

### Move and resize

```bash
deskbrid windows.move_resize {
  window_id: "12345678",
  x: 100,
  y: 100,
  width: 800,
  height: 600
}
```

### Tile window

```bash
deskbrid windows.tile {
  window_id: "12345678",
  preset: "left",
  monitor: 0,
  padding: 10
}
```

Presets: `left`, `right`, `top`, `bottom`, `center`, `max`.

### Activate or launch

```bash
deskbrid windows.activate_or_launch {
  app_id: "code",
  command: ["code", "--new-window"],
  workdir: "/home/user/projects"
}
```

## Workspaces

### List workspaces

```bash
deskbrid workspaces.list
```

### Switch workspace

```bash
deskbrid workspaces.switch { workspace_id: 2 }
```

### Move window between workspaces

```bash
deskbrid workspaces.move_window {
  window_id: "12345678",
  workspace_id: 3,
  follow: true
}
```

## Python example

```python
from deskbrid import Deskbrid

client = Deskbrid()
windows = client.windows_list()
for w in windows:
    print(w["id"], w["title"], w["app_id"])

client.windows_focus(window_id=windows[0]["id"])
```
