# Deskbrid API Reference

Every desktop function is exposed through the NDJSON protocol. Clients send action messages to the Unix socket and receive response messages. This reference covers every action, its parameters, response format, and a real request/response example.

## Protocol Basics

**Transport:** Unix domain socket at `$XDG_RUNTIME_DIR/deskbrid.sock`
**Encoding:** NDJSON — one JSON object per line, terminated by `\n`
**Sequence numbers:** Each client message increments a per-connection `seq` counter. The daemon echoes `seq` back in the response for correlation.

### Common Envelope

**Response (success):**
```json
{"type": "response", "id": "action", "seq": 1, "status": "ok", "data": { ... }}
```

**Response (error):**
```json
{"type": "response", "id": "action", "seq": 1, "status": "error", "error": {"code": "INTERNAL_ERROR", "message": "..."}}
```

**Error codes:**
| Code | Meaning |
|------|---------|
| `INVALID_PARAMS` | Malformed JSON or unknown action type |
| `INTERNAL_ERROR` | Backend operation failed |
| `NOT_SUPPORTED` | No desktop backend loaded |
| `PERMISSION_DENIED` | Caller UID not allowed for the requested action |

### Connection Handshake

On socket connect, the daemon immediately sends a `connected` message. Clients **must** wait for this before sending commands:

```json
{"type":"connected","id":"server","seq":0,"data":{"version":"0.4.1","protocol":"deskbrid-v2"}}
```

---

## Windows

### `windows.list`

List all open windows.

**Request:**
```json
{"type":"windows.list","id":"windows.list","seq":1}
```

**Response:**
```json
{
 "type": "response",
 "id": "action",
 "seq": 1,
 "status": "ok",
 "data": [
   {
     "id": "0x1a00003",
     "title": "Terminal",
     "app_id": "org.gnome.Terminal",
     "workspace_id": 0,
     "is_focused": true,
     "is_minimized": false,
     "geometry": {"x": 0, "y": 0, "width": 1920, "height": 1080},
     "pid": 1234
   }
 ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Window XID (hex string) |
| `title` | string | Window title |
| `app_id` | string | Application ID (WM class) |
| `workspace_id` | number | Workspace index (0-based) |
| `is_focused` | boolean | Whether this window currently has focus |
| `is_minimized` | boolean | Whether the window is minimized |
| `geometry` | object{?} | `{x, y, width, height}` — present when available |
| `pid` | number{?} | Process ID — `null` when unavailable |

**Backend:** GNOME Shell extension → `ListWindows()`

---

### `windows.focus`

Focus a window by one or more matching criteria.

**Request:**
```json
{"type":"windows.focus","window_id":"0x1a00003","id":"windows.focus","seq":2}
```

**Response:**
```json
{"type":"response","id":"action","seq":2,"status":"ok","data":{"focused":"0x1a00003"}}
```

| Param | Type | Description |
|-------|------|-------------|
| `window_id` | string | Window ID to focus |

The GNOME extension's `FocusWindow` method supports matching by `app_id` or `title` with optional case-insensitive substring or exact match. The daemon currently dispatches by raw window ID.

**Backend:** GNOME Shell extension → `FocusWindow(app_id, title, exact)`

---

### `windows.get`

Get information about a single window by ID.

**Request:**
```json
{"type":"windows.get","window_id":"0x1a00003","id":"windows.get","seq":3}
```

**Response:** Same per-window format as `windows.list` data items.

**Backend:** GNOME Shell extension → filters `ListWindows` result by ID.

---

## Workspaces

### `workspaces.list`

List all workspaces (virtual desktops).

**Request:**
```json
{"type":"workspaces.list","id":"workspaces.list","seq":4}
```

**Response:**
```json
{
 "type": "response",
 "id": "action",
 "seq": 4,
 "status": "ok",
 "data": [
   {"id": 0, "name": "Workspace 1", "is_active": true},
   {"id": 1, "name": "Workspace 2", "is_active": false}
 ]
}
```

**Backend:** GNOME Shell extension → `WorkspacesList` / `ActiveWorkspace` via `ext_call_parsed`.

---

### `workspaces.switch`

Switch to a specific workspace by index.

**Request:**
```json
{"type":"workspaces.switch","workspace_id":2,"id":"workspaces.switch","seq":5}
```

**Response:**
```json
{"type":"response","id":"action","seq":5,"status":"ok","data":{"workspace":2}}
```

| Param | Type | Description |
|-------|------|-------------|
| `workspace_id` | number | Workspace index to activate |

**Backend:** GNOME Shell extension → `ext_call_parsed("SwitchWorkspace", workspace_id)`. Uses the extension's DBus method — no Eval, no blocking calls.

---

### `workspaces.move_window`

Move a window to a workspace, optionally following it.

**Request:**
```json
{"type":"workspaces.move_window","window_id":"0x1a00003","workspace_id":2,"follow":true,"id":"workspaces.move_window","seq":6}
```

**Response:**
```json
{"type":"response","id":"action","seq":6,"status":"ok","data":{"moved":true}}
```

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `window_id` | string | — | Window ID to move |
| `workspace_id` | number | — | Target workspace index |
| `follow` | boolean | `false` | Whether to also switch to the target workspace |

**Backend:** GNOME Shell extension → `ext_call_parsed("MoveWindowToWorkspace", window_id, workspace_id)`. Uses the extension's DBus method — no Eval, no blocking calls.

---

## Input

### `input.keyboard` (type text)

Type text into the currently focused window.

**Request:**
```json
{"type":"input.keyboard","action":"type","text":"Hello, world!\n","id":"input.keyboard","seq":7}
```

**Response:**
```json
{"type":"response","id":"action","seq":7,"status":"ok","data":{"typed":14}}
```

| Sub-action | Param | Type | Description |
|-----------|-------|------|-------------|
| `type` | `text` | string | Text to type. Supports `\n`, `\t` escape sequences |

`data.typed` reports the number of characters typed.

**Backend:** Mutter RemoteDesktop API → `NotifyKeyboardKeysym`.

---

### `input.keyboard` (send key press)

Press and release a single named key.

**Request:**
```json
{"type":"input.keyboard","action":"key","key":"Return","id":"input.keyboard","seq":8}
```

**Response:**
```json
{"type":"response","id":"action","seq":8,"status":"ok","data":{"key":"Return"}}
```

| Sub-action | Param | Type | Description |
|-----------|-------|------|-------------|
| `key` | `key` | string | Named key (e.g. `Return`, `Escape`, `Tab`, `BackSpace`) |

**Backend:** Mutter RemoteDesktop API → `NotifyKeyboardKeysym`.

---

### `input.keyboard` (key combo)

Press multiple keys simultaneously (like `Ctrl+C`).

**Request:**
```json
{"type":"input.keyboard","action":"combo","keys":["ctrl","c"],"id":"input.keyboard","seq":9}
```

**Response:**
```json
{"type":"response","id":"action","seq":9,"status":"ok","data":{"combo":["ctrl","c"]}}
```

| Sub-action | Param | Type | Description |
|-----------|-------|------|-------------|
| `combo` | `keys` | array of strings | Ordered list of keys to press simultaneously |

**Backend:** Mutter RemoteDesktop API → `NotifyKeyboardKeysym` with modifier mask.

---

### `input.mouse` (move)

Move the mouse cursor to absolute coordinates.

**Request:**
```json
{"type":"input.mouse","action":"move","x":500.0,"y":300.0,"id":"input.mouse","seq":10}
```

**Response:**
```json
{"type":"response","id":"action","seq":10,"status":"ok","data":{"mouse":"move"}}
```

| Param | Type | Description |
|-------|------|-------------|
| `action` | string | Must be `"move"` |
| `x` | number | Absolute X coordinate |
| `y` | number | Absolute Y coordinate |

**Backend:** Mutter RemoteDesktop API → `NotifyPointerMotion` (relative) or `NotifyPointerMotionAbsolute` (requires ScreenCast).

---

### `input.mouse` (click)

Click a mouse button at the current cursor position.

**Request:**
```json
{"type":"input.mouse","action":"click","button":"left","id":"input.mouse","seq":11}
```

**Response:**
```json
{"type":"response","id":"action","seq":11,"status":"ok","data":{"mouse":"click"}}
```

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `action` | string | — | Must be `"click"` |
| `button` | string | `"left"` | `"left"`, `"right"`, `"middle"` |

**Backend:** Mutter RemoteDesktop API → `NotifyPointerButton`.

---

### `input.mouse` (scroll)

Scroll the mouse wheel.

**Request:**
```json
{"type":"input.mouse","action":"scroll","dx":0.0,"dy":-5.0,"id":"input.mouse","seq":12}
```

**Response:**
```json
{"type":"response","id":"action","seq":12,"status":"ok","data":{"mouse":"scroll"}}
```

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `action` | string | — | Must be `"scroll"` |
| `dx` | number | `0.0` | Horizontal scroll amount (positive = right) |
| `dy` | number | `0.0` | Vertical scroll amount (positive = down, negative = up) |

**Backend:** Mutter RemoteDesktop API → `NotifyPointerAxis`.

---

## Clipboard

### `clipboard.read`

Read the current clipboard contents.

**Request:**
```json
{"type":"clipboard.read","id":"clipboard.read","seq":13}
```

**Response:**
```json
{"type":"response","id":"action","seq":13,"status":"ok","data":{"text":"current clipboard content"}}
```

| Response field | Type | Description |
|---------------|------|-------------|
| `text` | string | Plain text clipboard content |

**Backend:** `wl-paste`.

---

### `clipboard.write`

Write text to the clipboard.

**Request:**
```json
{"type":"clipboard.write","text":"new content","id":"clipboard.write","seq":14}
```

**Response:**
```json
{"type":"response","id":"action","seq":14,"status":"ok","data":{"written":true}}
```

| Param | Type | Description |
|-------|------|-------------|
| `text` | string | Text to set as clipboard content |

**Backend:** `wl-copy`.

---

## Screenshot

### `screenshot`

Capture a screenshot.

**Request (full screen):**
```json
{"type":"screenshot","id":"screenshot","seq":15}
```

**Request (specific monitor):**
```json
{"type":"screenshot","monitor":0,"id":"screenshot","seq":16}
```

**Request (region selection via slurp):**
```json
{"type":"screenshot","region":{"x":100,"y":100,"width":800,"height":600},"id":"screenshot","seq":17}
```

**Request (focused window):**
```json
{"type":"screenshot","window_id":"0x1a00003","id":"screenshot","seq":18}
```

**Response:**
```json
{
 "type": "response",
 "id": "action",
 "seq": 15,
 "status": "ok",
 "data": {
   "path": "/tmp/deskbrid-screenshot-1715000000.png",
   "width": 1920,
   "height": 1080,
   "format": "png"
 }
}
```

| Param | Type | Description |
|-------|------|-------------|
| `monitor` | number{?} | Monitor index to capture (omit for all monitors) |
| `region` | object{?} | `{x, y, width, height}` in pixels |
| `window_id` | string{?} | Window ID to capture (via backend-specific tooling — grim on GNOME/Hyprland, spectacle + ImageMagick on KDE) |

**Response fields:**

| Field | Type | Description |
|-------|------|-------------|
| `path` | string | Absolute path to the saved PNG file |
| `width` | number | Image width in pixels |
| `height` | number | Image height in pixels |
| `format` | string | Always `"png"` |

**Backend:** `grim` (GNOME/Hyprland) or `spectacle` + ImageMagick `convert -crop` (KDE). Screenshots are saved to `/tmp/deskbrid-screenshot-<unix_timestamp>.png`.

---

## Notifications

### `notification.send`

Send a desktop notification.

**Request:**
```json
{
 "type": "notification.send",
 "app_name": "deskbrid",
 "title": "Download Complete",
 "body": "Your file has finished downloading.",
 "urgency": "normal",
 "id": "notification.send",
 "seq": 19
}
```

**Response:**
```json
{"type":"response","id":"action","seq":19,"status":"ok","data":{"notification_id":42}}
```

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `app_name` | string | `"deskbrid"` | Application name shown in notification |
| `title` | string | — | Notification title |
| `body` | string | `""` | Notification body text |
| `urgency` | string | `"normal"` | `"low"`, `"normal"`, or `"critical"` |

**Backend:** `notify-send`.

---

### `notification.close`

Close a notification by ID.

**Request:**
```json
{"type":"notification.close","notification_id":42,"id":"notification.close","seq":20}
```

**Response:**
```json
{"type":"response","id":"action","seq":20,"status":"ok","data":{"closed":42}}
```

| Param | Type | Description |
|-------|------|-------------|
