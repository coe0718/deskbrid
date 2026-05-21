# API Reference

Complete reference for the deskbrid NDJSON protocol. Every action's request format, response format, and example. The protocol runs over a Unix socket — see [ARCHITECTURE.md](ARCHITECTURE.md) for transport details and [PROTOCOL.md](../PROTOCOL.md) for the protocol specification.

## Convention

All requests carry:

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | Action name (dot notation, e.g. `windows.list`) |
| `id` | string | Client-chosen correlation token, echoed in response |

All responses carry:

| Field | Type | Description |
|-------|------|-------------|
| `type` | `"response"` | Response discriminator |
| `id` | string | Echo of request `id` |
| `seq` | integer | Monotonic connection counter |
| `status` | `"ok"` or `"error"` | Outcome |
| `data` | varies | Response payload (on success) |
| `error` | object | `{ "code": "...", "message": "..." }` (on failure) |

**Error codes**:
- `INVALID_PARAMS` — malformed JSON or unknown action type
- `NOT_FOUND` — resource not found (window, device, etc.)
- `NOT_SUPPORTED` — action not implemented by current backend
- `INTERNAL_ERROR` — backend operation failed
- `PERMISSION_DENIED` — caller UID not permitted

---

## Windows

### `windows.list`

List all open windows.

**Request:**
```json
{"type": "windows.list", "id": "req-1"}
```

**Response:**
```json
{
  "type": "response", "id": "req-1", "seq": 1, "status": "ok",
  "data": [
    {
      "id": "0x3a0000b",
      "title": "README.md — VS Code",
      "app_id": "code",
      "workspace_id": 0,
      "is_focused": true,
      "is_minimized": false,
      "geometry": { "x": 0, "y": 0, "width": 1920, "height": 1080 },
      "pid": 1234
    }
  ]
}
```

### `windows.focus`

Focus a window by its window ID.

**Request:**
```json
{"type": "windows.focus", "id": "req-2", "window_id": "0x3a0000b"}
```

**Response:**
```json
{
  "type": "response", "id": "req-2", "seq": 2, "status": "ok",
  "data": { "focused": "0x3a0000b", "id": "0x3a0000b" }
}
```

The daemon resolves the window's canonical ID from the backend after focusing, so subscribers receive the real window ID rather than the caller-provided selector.

### `windows.get`

Get details for a specific window.

**Request:**
```json
{"type": "windows.get", "id": "req-3", "window_id": "0x3a0000b"}
```

**Response:**
```json
{
  "type": "response", "id": "req-3", "seq": 3, "status": "ok",
  "data": {
    "id": "0x3a0000b",
    "title": "README.md — VS Code",
    "app_id": "code",
    "workspace_id": 0,
    "is_focused": true,
    "is_minimized": false,
    "geometry": { "x": 0, "y": 0, "width": 1920, "height": 1080 },
    "pid": 1234
  }
}
```

### `windows.close`

Request that a window close.

**Request:**
```json
{"type": "windows.close", "id": "req-4", "window_id": "0x3a0000b"}
```

**Response:**
```json
{
  "type": "response", "id": "req-4", "seq": 4, "status": "ok",
  "data": { "closed": "0x3a0000b" }
}
```

### `windows.minimize`

Minimize a window where the compositor supports it.

**Request:**
```json
{"type": "windows.minimize", "id": "req-5", "window_id": "0x3a0000b"}
```

**Response:**
```json
{
  "type": "response", "id": "req-5", "seq": 5, "status": "ok",
  "data": { "minimized": "0x3a0000b" }
}
```

Hyprland does not expose a native minimize dispatcher, so this action returns an error and is marked unsupported in `system.capabilities` on that backend.

### `windows.maximize`

Maximize a window.

**Request:**
```json
{"type": "windows.maximize", "id": "req-6", "window_id": "0x3a0000b"}
```

**Response:**
```json
{
  "type": "response", "id": "req-6", "seq": 6, "status": "ok",
  "data": { "maximized": "0x3a0000b" }
}
```

### `windows.move_resize`

Move and resize a window.

**Request:**
```json
{"type": "windows.move_resize", "id": "req-7", "window_id": "0x3a0000b",
 "x": 0, "y": 0, "width": 1920, "height": 1080}
```

**Response:**
```json
{
  "type": "response", "id": "req-7", "seq": 7, "status": "ok",
  "data": {
    "window_id": "0x3a0000b",
    "x": 0,
    "y": 0,
    "width": 1920,
    "height": 1080
  }
}
```

### `windows.activate_or_launch`

Focus an app if a matching window is already open; otherwise launch a command.

**Request:**
```json
{"type": "windows.activate_or_launch", "id": "req-8",
 "app_id": "code", "command": ["code", "."]}
```

`app_id` is matched against open window `app_id` first, then title. `command` is optional; when omitted, Deskbrid tries to launch `app_id` as the executable name.

When permissions are configured, callers need both `windows.activate_or_launch` and `process.start` because the action may launch a process.

**Response when activated:**
```json
{
  "type": "response", "id": "req-8", "seq": 8, "status": "ok",
  "data": {
    "app_id": "code",
    "activated": true,
    "launched": false,
    "window_id": "0x3a0000b"
  }
}
```

**Response when launched:**
```json
{
  "type": "response", "id": "req-8", "seq": 8, "status": "ok",
  "data": {
    "app_id": "code",
    "activated": false,
    "launched": true,
    "pid": 12345,
    "command": ["code", "."]
  }
}
```

---

## Layout Profiles

Layout profiles are named snapshots of open windows, monitors, workspaces, and the active workspace. Profile names may contain only letters, numbers, `.`, `-`, and `_`. Profiles are stored under `~/.config/deskbrid/layout_profiles/`.

Restore reapplies window workspace placement, geometry, minimized state, and the active workspace. Monitor topology is saved and compared during restore, but monitor mode changes are not applied yet.

Permission-gated deployments should grant `windows.list`, `workspaces.list`, and `system.info` for saving. Restoring requires `layout_profiles.restore` plus window and workspace control permissions because restore can move/resize windows and switch workspaces.

### `layout_profiles.save`

Save the current layout.

**Request:**
```json
{"type": "layout_profiles.save", "id": "req-9", "name": "coding", "overwrite": true}
```

**Response:**
```json
{
  "type": "response", "id": "req-9", "seq": 9, "status": "ok",
  "data": {
    "profile": {
      "schema_version": 1,
      "name": "coding",
      "saved_at": 1778976000,
      "desktop": "gnome",
      "session_type": "wayland",
      "current_workspace": 1,
      "monitors": [],
      "workspaces": [],
      "windows": []
    },
    "path": "/home/alice/.config/deskbrid/layout_profiles/coding.json"
  }
}
```

### `layout_profiles.list`

List saved profile summaries.

**Request:**
```json
{"type": "layout_profiles.list", "id": "req-10"}
```

**Response:**
```json
{
  "type": "response", "id": "req-10", "seq": 10, "status": "ok",
  "data": [
    {
      "name": "coding",
      "saved_at": 1778976000,
      "desktop": "gnome",
      "session_type": "wayland",
      "current_workspace": 1,
      "monitor_count": 2,
      "workspace_count": 4,
      "window_count": 6
    }
  ]
}
```

### `layout_profiles.get`

Get a saved profile snapshot.

**Request:**
```json
{"type": "layout_profiles.get", "id": "req-11", "name": "coding"}
```

**Response:** same profile object returned by `layout_profiles.save` under `data.profile`, but directly as `data`.

### `layout_profiles.restore`

Restore a saved profile.

**Request:**
```json
{"type": "layout_profiles.restore", "id": "req-12", "name": "coding"}
```

**Response:**
```json
{
  "type": "response", "id": "req-12", "seq": 12, "status": "ok",
  "data": {
    "profile": "coding",
    "restored": [],
    "missing": [],
    "errors": [],
    "workspace_switched": true,
    "current_workspace": 1,
    "monitor_topology_matches": true,
    "saved_monitor_count": 2,
    "current_monitor_count": 2
  }
}
```

### `layout_profiles.delete`

Delete a saved profile.

**Request:**
```json
{"type": "layout_profiles.delete", "id": "req-13", "name": "coding"}
```

**Response:**
```json
{
  "type": "response", "id": "req-13", "seq": 13, "status": "ok",
  "data": { "deleted": "coding" }
}
```

---

## Workspaces

### `workspaces.list`

List all workspaces.

**Request:**
```json
{"type": "workspaces.list", "id": "req-6"}
```

**Response:**
```json
{
  "type": "response", "id": "req-6", "seq": 6, "status": "ok",
  "data": [
    { "id": 1, "name": "Workspace 1", "is_active": true, "num_windows": 3 },
    { "id": 2, "name": "Workspace 2", "is_active": false, "num_windows": 1 }
  ]
}
```

### `workspaces.switch`

Switch to a workspace.

**Request:**
```json
{"type": "workspaces.switch", "id": "req-7", "workspace_id": 2}
```

**Response:**
```json
{
  "type": "response", "id": "req-7", "seq": 7, "status": "ok",
  "data": { "workspace": 2 }
}
```

Emits a `workspace.changed` event on the broadcast channel.

### `workspaces.move_window`

Move a window to a different workspace.

**Request:**
```json
{"type": "workspaces.move_window", "id": "req-8",
 "window_id": "0x3a0000b", "workspace_id": 2, "follow": true}
```

`follow` (optional, default `false`): switch the active workspace to the destination.

**Response:**
```json
{
  "type": "response", "id": "req-8", "seq": 8, "status": "ok",
  "data": { "moved": true }
}
```

Emits a `workspace.window_moved` event.

---

## Input

### `input.keyboard`

Three sub-modes controlled by the `action` parameter.

**Mode: type** — type text as if from a keyboard:
```json
{"type": "input.keyboard", "id": "req-9", "action": "type", "text": "git push\n"}
```
```json
{
  "type": "response", "id": "req-9", "seq": 9, "status": "ok",
  "data": { "typed": 8 }
}
```

**Mode: key** — press a single named key:
```json
{"type": "input.keyboard", "id": "req-10", "action": "key", "key": "Return"}
```
```json
{
  "type": "response", "id": "req-10", "seq": 10, "status": "ok",
  "data": { "key": "Return" }
}
```

**Mode: combo** — press a key combination simultaneously:
```json
{"type": "input.keyboard", "id": "req-11", "action": "combo", "keys": ["ctrl", "shift", "t"]}
```
```json
{
  "type": "response", "id": "req-11", "seq": 11, "status": "ok",
  "data": { "combo": ["ctrl", "shift", "t"] }
}
```

### `input.mouse`

Four sub-modes controlled by the `action` parameter.

**Mode: move** — move mouse to absolute screen coordinates:
```json
{"type": "input.mouse", "id": "req-12", "action": "move", "x": 500, "y": 300}
```
```json
{
  "type": "response", "id": "req-12", "seq": 12, "status": "ok",
  "data": { "mouse": "move" }
}
```

**Mode: click** — click a mouse button:
```json
{"type": "input.mouse", "id": "req-13", "action": "click", "button": "right"}
```
```json
{
  "type": "response", "id": "req-13", "seq": 13, "status": "ok",
  "data": { "mouse": "click" }
}
```

`button` values: `"left"` (default), `"right"`, `"middle"`.

**Mode: scroll** — scroll the mouse wheel:
```json
{"type": "input.mouse", "id": "req-14", "action": "scroll", "dx": 0, "dy": -3}
```
```json
{
  "type": "response", "id": "req-14", "seq": 14, "status": "ok",
  "data": { "mouse": "scroll" }
}
```

Positive `dy`: scroll down. Negative: scroll up. `dx`: horizontal scroll (positive = right).

---

## Clipboard

### `clipboard.read`

Read the current clipboard content.

**Request:**
```json
{"type": "clipboard.read", "id": "req-15"}
```

**Response:**
```json
{
  "type": "response", "id": "req-15", "seq": 15, "status": "ok",
  "data": { "text": "copied content" }
}
```

### `clipboard.write`

Write text to the clipboard.

**Request:**
```json
{"type": "clipboard.write", "id": "req-16", "text": "hello world"}
```

**Response:**
```json
{
  "type": "response", "id": "req-16", "seq": 16, "status": "ok",
  "data": { "written": true }
}
```

---

## Screenshot

### `screenshot`

Capture a screenshot. All parameters are optional.

**Request:**
```json
{"type": "screenshot", "id": "req-17", "monitor": 0}
```

Optional params:

| Field | Type | Description |
|-------|------|-------------|
| `monitor` | number | Monitor index to capture |
| `region` | object | `{ "x": 0, "y": 0, "width": 800, "height": 600 }` |
| `window_id` | string | Window ID to capture |

**Response:**
```json
{
  "type": "response", "id": "req-17", "seq": 17, "status": "ok",
  "data": {
    "path": "/tmp/deskbrid/screenshot_1715000000.png",
    "width": 1920,
    "height": 1080,
    "format": "png"
  }
}
```

The screenshot is saved to `/tmp/deskbrid/screenshot_<unix_timestamp>.png`. The daemon tries `gnome-screenshot` first, then falls back to the XDG Desktop Portal Python script.

---

## Notifications

### `notification.send`

Send a desktop notification.

**Request:**
```json
{"type": "notification.send", "id": "req-18",
 "title": "Build complete", "body": "Exit code 0", "urgency": "normal"}
```

`urgency` values: `"low"`, `"normal"` (default), `"critical"`.

**Response:**
```json
{
  "type": "response", "id": "req-18", "seq": 18, "status": "ok",
  "data": { "notification_id": 42 }
}
```

### `notification.close`

Close a notification by ID.

**Request:**
```json
{"type": "notification.close", "id": "req-19", "notification_id": 42}
```

**Response:**
```json
{
  "type": "response", "id": "req-19", "seq": 19, "status": "ok",
  "data": { "closed": 42 }
}
```

---

## System

### `system.info`

Get desktop environment information, including connected monitors.

**Request:**
```json
{"type": "system.info", "id": "req-20"}
```

**Response:**
```json
{
  "type": "response", "id": "req-20", "seq": 20, "status": "ok",
  "data": {
    "desktop": "GNOME",
    "hostname": "thinkpad-x1",
    "session_type": "wayland",
    "kernel": "6.8.0-111-generic",
    "monitors": [
      { "id": 0, "name": "eDP-1", "primary": true,
        "width": 1920, "height": 1080, "scale": 1.0,
        "enabled": true, "x": 0, "y": 0,
        "refresh_rate": 60.0, "rotation": "normal" }
    ]
  }
}
```

### `system.idle`

Get the number of seconds since the last user input.

**Request:**
```json
{"type": "system.idle", "id": "req-21"}
```

**Response:**
```json
{
  "type": "response", "id": "req-21", "seq": 21, "status": "ok",
  "data": { "idle_seconds": 342 }
}
```

### `system.battery`

Get battery status for all batteries.

**Request:**
```json
{"type": "system.battery", "id": "req-22"}
```

**Response:**
```json
{
  "type": "response", "id": "req-22", "seq": 22, "status": "ok",
  "data": [
    { "id": "BAT0", "percentage": 85.0, "status": "Discharging",
      "time_remaining": 7200, "energy_rate": 12.5 }
  ]
}
```

### `system.power`

Perform a power action.

**Request:**
```json
{"type": "system.power", "id": "req-23", "action": "lock"}
```

`action` values: `"suspend"`, `"hibernate"`, `"shutdown"`, `"reboot"`, `"lock"`, `"logout"`.

**Response:**
```json
{
  "type": "response", "id": "req-23", "seq": 23, "status": "ok",
  "data": { "power": "lock" }
}
```

### `system.inhibit` / `system.release_inhibit`

Hold a systemd inhibitor while an agent task is running, then release it by ID.

**Requests:**
```json
{"type": "system.inhibit", "id": "req-24", "what": "sleep", "who": "deskbrid", "why": "deploy running", "mode": "block"}
{"type": "system.release_inhibit", "id": "req-25", "inhibitor_id": 1}
```

### `system.sessions`, `system.lock_session`, `system.switch_user`

List logind sessions, lock the current or specified session, or ask the display manager to switch users.

**Requests:**
```json
{"type": "system.sessions", "id": "req-26"}
{"type": "system.lock_session", "id": "req-27", "session_id": "2"}
{"type": "system.switch_user", "id": "req-28", "username": "alice"}
```

### `system.check_auth` / `system.elevate`

Check a polkit action, or request authorization with user interaction via `pkcheck`.

**Requests:**
```json
{"type": "system.check_auth", "id": "req-29", "action_id": "org.deskbrid.system.service-control"}
{"type": "system.elevate", "id": "req-30", "action_id": "org.deskbrid.system.service-control", "reason": "restart a failed unit"}
```

### `service.*`, `journal.query`, `timer.*`

## Systemd Services

### `service.status`
Show one unit's status.

**Request:**
```json
{"type": "service.status", "id": "req-31", "name": "ssh.service"}
```

**Response:**
```json
{
  "type": "response", "id": "req-31", "seq": 31, "status": "ok",
  "data": { "name": "ssh.service", "active": "active", "sub": "running", "loaded": "loaded" }
}
```

---

### `service.start`
Start a unit.

**Request:**
```json
{"type": "service.start", "id": "req-32", "name": "ssh.service"}
```

**Response:**
```json
{
  "type": "response", "id": "req-32", "seq": 32, "status": "ok",
  "data": { "name": "ssh.service", "started": true }
}
```

---

### `service.stop`
Stop a unit.

**Request:**
```json
{"type": "service.stop", "id": "req-33", "name": "ssh.service"}
```

**Response:**
```json
{
  "type": "response", "id": "req-33", "seq": 33, "status": "ok",
  "data": { "name": "ssh.service", "stopped": true }
}
```

---

### `service.restart`
Restart a unit.

**Request:**
```json
{"type": "service.restart", "id": "req-34", "name": "ssh.service"}
```

**Response:**
```json
{
  "type": "response", "id": "req-34", "seq": 34, "status": "ok",
  "data": { "name": "ssh.service", "restarted": true }
}
```

---

### `service.enable`
Enable a unit.

**Request:**
```json
{"type": "service.enable", "id": "req-35", "name": "ssh.service", "runtime": false}
```

**Parameters:**
- `name` (required)
- `runtime` (optional, defaults to false - if true, enables only until next reboot)

**Response:**
```json
{
  "type": "response", "id": "req-35", "seq": 35, "status": "ok",
  "data": { "name": "ssh.service", "enabled": true }
}
```

---

### `service.disable`
Disable a unit.

**Request:**
```json
{"type": "service.disable", "id": "req-36", "name": "ssh.service", "runtime": false}
```

**Parameters:**
- `name` (required)
- `runtime` (optional, defaults to false - if true, disables only until next reboot)

**Response:**
```json
{
  "type": "response", "id": "req-36", "seq": 36, "status": "ok",
  "data": { "name": "ssh.service", "enabled": false }
}
```

---

### `service.list`
List units by type.

**Request:**
```json
{"type": "service.list", "id": "req-37", "unit_type": "service"}
```

**Parameters:**
- `unit_type` (optional, defaults to service - can be service, socket, target, etc.)

**Response:**
```json
{
  "type": "response", "id": "req-37", "seq": 37, "status": "ok",
  "data": { "units": [ { "name": "ssh.service", "load": "loaded", "active": "active", "description": "OpenSSH server daemon" } ] }
}
```

## Systemd Services

### `service.status`
Show one unit's status.

**Request:**
```json
{"type": "service.status", "id": "req-31", "name": "ssh.service"}
```

**Response:**
```json
{
  "type": "response", "id": "req-31", "seq": 31, "status": "ok",
  "data": { "name": "ssh.service", "active": "active", "sub": "running", "loaded": "loaded" }
}
```

### `service.start`
Start a unit.

**Request:**
```json
{"type": "service.start", "id": "req-32", "name": "ssh.service"}
```

**Response:**
```json
{
  "type": "response", "id": "req-32", "seq": 32, "status": "ok",
  "data": { "name": "ssh.service", "started": true }
}
```

### `service.stop`
Stop a unit.

**Request:**
```json
{"type": "service.stop", "id": "req-33", "name": "ssh.service"}
```

**Response:**
```json
{
  "type": "response", "id": "req-33", "seq": 33, "status": "ok",
  "data": { "name": "ssh.service", "stopped": true }
}
```

### `service.restart`
Restart a unit.

**Request:**
```json
{"type": "service.restart", "id": "req-34", "name": "ssh.service"}
```

**Response:**
```json
{
  "type": "response", "id": "req-34", "seq": 34, "status": "ok",
  "data": { "name": "ssh.service", "restarted": true }
}
```

### `service.enable`
Enable a unit.

**Request:**
```json
{"type": "service.enable", "id": "req-35", "name": "ssh.service", "runtime": false}
```

**Parameters:** `name` (required), `runtime` (optional, defaults to false - if true, enables only until next reboot).

**Response:**
```json
{
  "type": "response", "id": "req-35", "seq": 35, "status": "ok",
  "data": { "name": "ssh.service", "enabled": true }
}
```

### `service.disable`
Disable a unit.

**Request:**
```json
{"type": "service.disable", "id": "req-36", "name": "ssh.service", "runtime": false}
```

**Parameters:** `name` (required), `runtime` (optional, defaults to false - if true, disables only until next reboot).

**Response:**
```json
{
  "type": "response", "id": "req-36", "seq": 36, "status": "ok",
  "data": { "name": "ssh.service", "enabled": false }
}
```

### `service.list`
List units by type.

**Request:**
```json
{"type": "service.list", "id": "req-37", "unit_type": "service"}
```

**Parameters:** `unit_type` (optional, defaults to service - can be service, socket, target, etc.).

**Response:**
```json
{
  "type": "response", "id": "req-37", "seq": 37, "status": "ok",
  "data": { "units": [ { "name": "ssh.service", "load": "loaded", "active": "active", "description": "OpenSSH server daemon" } ] }
}
```

## Timers

### `timer.list`
List systemd timers.

**Request:**
```json
{"type": "timer.list", "id": "req-38"}
```

**Response:**
```json
{
  "type": "response", "id": "req-38", "seq": 38, "status": "ok",
  "data": { "timers": [ { "name": "apt-daily.timer", "next": "Left 6h 12min 30s", "last": "Right 12h 30min ago", "unit": "apt-daily.service" } ] }
}
```

### `timer.start`
Start a timer.

**Request:**
```json
{"type": "timer.start", "id": "req-39", "name": "apt-daily.timer"}
```

**Response:**
```json
{
  "type": "response", "id": "req-39", "seq": 39, "status": "ok",
  "data": { "name": "apt-daily.timer", "started": true }
}
```

### `timer.stop`
Stop a timer.

**Request:**
```json
{"type": "timer.stop", "id": "req-40", "name": "apt-daily.timer"}
```

**Response:**
```json
{
  "type": "response", "id": "req-40", "seq": 40, "status": "ok",
  "data": { "name": "apt-daily.timer", "stopped": true }
}
```


## Network



## Journald

### `journal.query`
Query journald logs.

**Request:**
```json
{"type": "journal.query", "id": "req-35", "unit": "ssh.service", "priority": 4, "tail": 100}
```

**Parameters:**
- `unit` (optional)
- `priority` (optional, 0-7)
- `tail` (optional, lines to show)
- `since` (optional, microseconds since epoch)
- `until` (optional, microseconds since epoch)

**Response:**
```json
{
  "type": "response", "id": "req-35", "seq": 35, "status": "ok",
  "data": { "entries": [ { "__REALTIME_TIMESTAMP": "1640995200000000", "MESSAGE": "sshd started", "_SYSTEMD_UNIT": "ssh.service" } ] }
}
```

## Journald

### `journal.query`
Query journald logs.

**Request:**
```json
{"type": "journal.query", "id": "req-35", "unit": "ssh.service", "priority": 4, "tail": 100}
```

**Parameters:** `unit` (optional), `priority` (optional, 0-7), `tail` (optional, lines to show), `since` (optional, microseconds since epoch), `until` (optional, microseconds since epoch).

**Response:**
```json
{
  "type": "response", "id": "req-35", "seq": 35, "status": "ok",
  "data": { "entries": [ { "__REALTIME_TIMESTAMP": "1640995200000000", "MESSAGE": "sshd started", "_SYSTEMD_UNIT": "ssh.service" } ] }
}
```

### `network.status`

Get network connectivity status.

**Request:**
```json
{"type": "network.status", "id": "req-29"}
```

**Response:**
```json
{
  "type": "response", "id": "req-29", "seq": 29, "status": "ok",
  "data": { "online": true, "connectivity": "full" }
}
```

### `network.interfaces`

List network interfaces with IP addresses.

**Request:**
```json
{"type": "network.interfaces", "id": "req-30"}
```

**Response:**
```json
{
  "type": "response", "id": "req-30", "seq": 30, "status": "ok",
  "data": [
    { "name": "wlp2s0", "type": "wifi", "ip": "192.168.1.42",
      "mac": "aa:bb:cc:dd:ee:ff", "connected": true }
  ]
}
```

### `network.wifi.scan`

Scan for nearby WiFi networks.

**Request:**
```json
{"type": "network.wifi.scan", "id": "req-31"}
```

**Response:**
```json
{
  "type": "response", "id": "req-31", "seq": 31, "status": "ok",
  "data": [
    { "ssid": "HomeNet", "strength": 80, "secured": true,
      "frequency": 5180, "bssid": "aa:bb:cc:11:22:33" }
  ]
}
```

### `network.wifi.connect`

Connect to a WiFi network.

**Request:**
```json
{"type": "network.wifi.connect", "id": "req-32",
 "ssid": "HomeNet", "password": "secret123"}
```

**Response:**
```json
{
  "type": "response", "id": "req-32", "seq": 32, "status": "ok",
  "data": { "connected": "HomeNet" }
}
```

---

## Bluetooth

### `bluetooth.list`

List known Bluetooth devices.

**Request:**
```json
{"type": "bluetooth.list", "id": "req-33"}
```

**Response:**
```json
{
  "type": "response", "id": "req-33", "seq": 33, "status": "ok",
  "data": [
    { "address": "AA:BB:CC:11:22:33", "name": "MX Master 3",
      "connected": true, "paired": true, "type": "mouse" }
  ]
}
```

### `bluetooth.scan`

Start Bluetooth device discovery. `duration` (optional, in seconds) controls how long scanning runs; the implementation uses a background timeout.

**Request:**
```json
{"type": "bluetooth.scan", "id": "req-34", "duration": 10}
```

**Response:**
```json
{
  "type": "response", "id": "req-34", "seq": 34, "status": "ok",
  "data": { "scanning": true }
}
```

### `bluetooth.scan_stop`

Stop active discovery.

**Request:**
```json
{"type": "bluetooth.scan_stop", "id": "req-35"}
```

**Response:**
```json
{
  "type": "response", "id": "req-35", "seq": 35, "status": "ok",
  "data": { "scanning": false }
}
```

### `bluetooth.connect`

Connect to a known Bluetooth device.

**Request:**
```json
{"type": "bluetooth.connect", "id": "req-36", "address": "AA:BB:CC:11:22:33"}
```

**Response:**
```json
{
  "type": "response", "id": "req-36", "seq": 36, "status": "ok",
  "data": { "connected": "AA:BB:CC:11:22:33" }
}
```

### `bluetooth.disconnect`

Disconnect a connected device.

**Request:**
```json
{"type": "bluetooth.disconnect", "id": "req-37", "address": "AA:BB:CC:11:22:33"}
```

**Response:**
```json
{
  "type": "response", "id": "req-37", "seq": 37, "status": "ok",
  "data": { "disconnected": "AA:BB:CC:11:22:33" }
}
```

### `bluetooth.pair`

Pair with a new device. **Not yet supported in the backend trait** — returns a stub.

**Request:**
```json
{"type": "bluetooth.pair", "id": "req-38", "address": "AA:BB:CC:11:22:33"}
```

**Response:**
```json
{
  "type": "response", "id": "req-38", "seq": 38, "status": "ok",
  "data": { "paired": "AA:BB:CC:11:22:33", "note": "not yet supported" }
}
```

### `bluetooth.forget`

Remove a paired device. **Not yet supported in the backend trait.**

---

## Audio



## UI Accessibility

### `a11y.tree`
Get the accessibility tree. **Not yet implemented** — requires AT-SPI integration.

**Request:**
```json
{"type": "a11y.tree", "id": "req-1"}
```

**Response:**
```json
{
  "type": "response", "id": "req-1", "seq": 1, "status": "ok",
  "data": { "supported": false, "reason": "AT-SPI not integrated yet", "nodes": [] }
}
```

---

### `a11y.get_element`
Get an accessibility element by role, name, and/or index.

**Request:**
```json
{"type": "a11y.get_element", "id": "req-2", "role": "button", "name": "Submit", "index": 0}
```

**Response:**
```json
{
  "type": "response", "id": "req-2", "seq": 2, "status": "ok",
  "data": {
    "role": "button",
    "name": "Submit",
    "index": 0,
    "screen_rectangle": { "x": 100, "y": 200, "width": 80, "height": 25 },
    "children": []
  }
}
```

---

### `a11y.click_element`
Click an accessibility element by role, name, and/or index.

**Request:**
```json
{"type": "a11y.click_element", "id": "req-3", "role": "button", "name": "Submit", "index": 0}
```

**Response:**
```json
{
  "type": "response", "id": "req-3", "seq": 3, "status": "ok",
  "data": { "clicked": true }
}
```

---

### `a11y.get_text`
Get text from an accessibility element by role, name, and/or index.

**Request:**
```json
{"type": "a11y.get_text", "id": "req-4", "role": "label", "name": "Status", "index": 0}
```

**Response:**
```json
{
  "type": "response", "id": "req-4", "seq": 4, "status": "ok",
  "data": { "text": "Ready" }
}
```

---

## Browser (Chrome DevTools Protocol)

### `browser.list_tabs`
List all browser tabs.

**Request:**
```json
{"type": "browser.list_tabs", "id": "req-1"}
```

**Response:**
```json
{
  "type": "response", "id": "req-1", "seq": 1, "status": "ok",
  "data": {
    "tabs": [
      { "index": 0, "title": "GitHub", "url": "https://github.com", "favicon": "https://github.com/favicon.ico" },
      { "index": 1, "title": "Stack Overflow", "url": "https://stackoverflow.com", "favicon": "https://stackoverflow.com/favicon.ico" }
    ]
  }
}
```

---

### `browser.navigate`
Navigate a browser tab to a URL.

**Request:**
```json
{"type": "browser.navigate", "id": "req-2", "tab_index": 0, "url": "https://example.com"}
```

**Response:**
```json
{
  "type": "response", "id": "req-2", "seq": 2, "status": "ok",
  "data": { "tab_index": 0, "url": "https://example.com", "success": true }
}
```

---

### `browser.evaluate`
Evaluate JavaScript in a browser tab.

**Request:**
```json
{"type": "browser.evaluate", "id": "req-3", "tab_index": 0, "expression": "document.title", "await_promise": false}
```

**Response:**
```json
{
  "type": "response", "id": "req-3", "seq": 3, "status": "ok",
  "data": { "tab_index": 0, "result": "Example Domain", "type": "string" }
}
```

---

### `browser.screenshot_tab`
Take a screenshot of a browser tab.

**Request:**
```json
{"type": "browser.screenshot_tab", "id": "req-4", "tab_index": 0}
```

**Response:**
```json
{
  "type": "response", "id": "req-4", "seq": 4, "status": "ok",
  "data": { "tab_index": 0, "screenshot": "/tmp/deskbrid/screenshot_12345.png", "width": 1920, "height": 1080 }
}
```

---

### `browser.click`
Click an element in a browser tab using a CSS selector.

**Request:**
```json
{"type": "browser.click", "id": "req-5", "tab_index": 0, "selector": "button.submit"}
```

**Response:**
```json
{
  "type": "response", "id": "req-5", "seq": 5, "status": "ok",
  "data": { "tab_index": 0, "selector": "button.submit", "success": true }
}
```

## `a11y.tree`
Get the accessibility tree. **Not yet implemented** — requires AT-SPI integration.

**Request:**
```json
{"type": "a11y.tree", "id": "req-1"}
```

**Response:**
```json
{
  "type": "response", "id": "req-1", "seq": 1, "status": "ok",
  "data": { "supported": false, "reason": "AT-SPI not integrated yet", "nodes": [] }
}
```

### `a11y.get_element`
Get an accessibility element by role, name, and/or index.

**Request:**
```json
{"type": "a11y.get_element", "id": "req-2", "role": "button", "name": "Submit", "index": 0}
```

**Response:**
```json
{
  "type": "response", "id": "req-2", "seq": 2, "status": "ok",
  "data": {
    "role": "button",
    "name": "Submit",
    "index": 0,
    "screen_rectangle": { "x": 100, "y": 200, "width": 80, "height": 25 },
    "children": []
  }
}
```

### `a11y.click_element`
Click an accessibility element by role, name, and/or index.

**Request:**
```json
{"type": "a11y.click_element", "id": "req-3", "role": "button", "name": "Submit", "index": 0}
```

**Response:**
```json
{
  "type": "response", "id": "req-3", "seq": 3, "status": "ok",
  "data": { "clicked": true }
}
```

### `a11y.get_text`
Get text from an accessibility element by role, name, and/or index.

**Request:**
```json
{"type": "a11y.get_text", "id": "req-4", "role": "label", "name": "Status", "index": 0}
```

**Response:**
```json
{
  "type": "response", "id": "req-4", "seq": 4, "status": "ok",
  "data": { "text": "Ready" }
}
```

## Browser (Chrome DevTools Protocol)

### `browser.list_tabs`
List all browser tabs.

**Request:**
```json
{"type": "browser.list_tabs", "id": "req-1"}
```

**Response:**
```json
{
  "type": "response", "id": "req-1", "seq": 1, "status": "ok",
  "data": {
    "tabs": [
      { "index": 0, "title": "GitHub", "url": "https://github.com", "favicon": "https://github.com/favicon.ico" },
      { "index": 1, "title": "Stack Overflow", "url": "https://stackoverflow.com", "favicon": "https://stackoverflow.com/favicon.ico" }
    ]
  }
}

### `browser.navigate`
Navigate a browser tab to a URL.

**Request:**
```json
{"type": "browser.navigate", "id": "req-2", "tab_index": 0, "url": "https://example.com"}
```

**Response:**
```json
{
  "type": "response", "id": "req-2", "seq": 2, "status": "ok",
  "data": { "tab_index": 0, "url": "https://example.com", "success": true }
}
```

### `browser.evaluate`
Evaluate JavaScript in a browser tab.

**Request:**
```json
{"type": "browser.evaluate", "id": "req-3", "tab_index": 0, "expression": "document.title", "await_promise": false}
```

**Response:**
```json
{
  "type": "response", "id": "req-3", "seq": 3, "status": "ok",
  "data": { "tab_index": 0, "result": "Example Domain", "type": "string" }
}
```

### `browser.screenshot_tab`
Take a screenshot of a browser tab.

**Request:**
```json
{"type": "browser.screenshot_tab", "id": "req-4", "tab_index": 0}
```

**Response:**
```json
{
  "type": "response", "id": "req-4", "seq": 4, "status": "ok",
  "data": { "tab_index": 0, "screenshot": "/tmp/deskbrid/screenshot_12345.png", "width": 1920, "height": 1080 }
}
```

### `browser.click`
Click an element in a browser tab using a CSS selector.

**Request:**
```json
{"type": "browser.click", "id": "req-5", "tab_index": 0, "selector": "button.submit"}
```

**Response:**
```json
{
  "type": "response", "id": "req-5", "seq": 5, "status": "ok",
  "data": { "tab_index": 0, "selector": "button.submit", "success": true }
}
```

### `audio.list_sinks`

List PulseAudio/PipeWire audio output sinks.

**Request:**
```json
{"type": "audio.list_sinks", "id": "req-39"}
```

**Response:**
```json
{
  "type": "response", "id": "req-39", "seq": 39, "status": "ok",
  "data": [
    { "id": 0, "name": "alsa_output.pci-0000_00_1f.3.analog-stereo",
      "description": "Built-in Audio Analog Stereo",
      "volume": 0.75, "muted": false, "active_port": "analog-output-speaker" }
  ]
}
```

### `audio.set_sink_volume`

Set a sink's volume (0.0–1.0).

**Request:**
```json
{"type": "audio.set_sink_volume", "id": "req-40", "sink_id": 0, "volume": 0.5}
```

**Response:**
```json
{
  "type": "response", "id": "req-40", "seq": 40, "status": "ok",
  "data": { "sink": 0, "volume": 0.5 }
}
```

---

## Files
## Process

### `process.list`

List running processes (from `ps aux --no-headers`).

**Request:**
```json
{"type": "process.list", "id": "req-44"}
```

**Response:**
```json
{
  "type": "response", "id": "req-44", "seq": 44, "status": "ok",
  "data": {
    "processes": [
      { "user": "user", "pid": 1234, "cpu": "0.1", "mem": "1.2", "command": "/usr/bin/code" },
      { "user": "user", "pid": 5678, "cpu": "0.0", "mem": "0.3", "command": "bash" }
    ]
  }
}
```

Limited to the first 200 processes from the `ps` output.

### `process.start`

Start a new process.

**Request:**
```json
{"type": "process.start", "id": "req-45",
 "command": ["notify-send", "Hello", "World"], "workdir": "/home/user",
 "env": {"DISPLAY": ":0"}}
```

`command` is required (first element is the binary, rest are args). `workdir` and `env` are optional.

**Response:**
```json
{
  "type": "response", "id": "req-45", "seq": 45, "status": "ok",
  "data": { "pid": 9999, "command": ["notify-send", "Hello", "World"] }
}
```

### `process.stop`

Stop a process by PID. Uses `libc::kill()` with signal.

**Request:**
```json
{"type": "process.stop", "id": "req-46", "pid": 9999, "signal": "SIGTERM"}
```

`signal` defaults to `"TERM"` if omitted. The `SIG` prefix is optional — `"TERM"` and `"SIGTERM"` are equivalent.

Supported signals: `HUP`, `INT`, `QUIT`, `KILL`, `TERM`, `USR1`, `USR2`, `CONT`, `STOP`.

Safety: refuses to target PID 0, 1, or the daemon's own PID.

**Response:**
```json
{
  "type": "response", "id": "req-46", "seq": 46, "status": "ok",
  "data": { "stopped": 9999, "signal": 15 }
}
```

### `process.signal`

Send an arbitrary signal to a process.

**Request:**
```json
{"type": "process.signal", "id": "req-47", "pid": 9999, "signal": "SIGUSR1"}
```

**Response:**
```json
{
  "type": "response", "id": "req-47", "seq": 47, "status": "ok",
  "data": { "signaled": 9999, "signal": 10 }
}
```

### `process.exists`

Check if a process is running (sends signal 0).

**Request:**
```json
{"type": "process.exists", "id": "req-48", "pid": 1234}
```

**Response (running):**
```json
{
  "type": "response", "id": "req-48", "seq": 48, "status": "ok",
  "data": { "pid": 1234, "exists": true }
}
```

**Response (not found):**
```json
{
  "type": "response", "id": "req-48", "seq": 48, "status": "ok",
  "data": { "pid": 9999, "exists": false }
}
```

### `process.wait`

Wait for a process to exit, with timeout.

**Request:**
```json
{"type": "process.wait", "id": "req-49", "pid": 9999, "timeout_ms": 30000}
```

`timeout_ms` defaults to 30000 (30s).

**Response (exited):**
```json
{
  "type": "response", "id": "req-49", "seq": 49, "status": "ok",
  "data": { "pid": 9999, "exited": true, "elapsed_ms": 2345 }
}
```

**Response (timeout):**
```json
{
  "type": "response", "id": "req-49", "seq": 49, "status": "ok",
  "data": { "pid": 9999, "exited": false, "timeout_ms": 30000 }
}
```

---

## Hotkeys

### `hotkeys.register`

Register a global hotkey combination. Not yet wired to the backend — returns immediately.

**Request:**
```json
{"type": "hotkeys.register", "id": "req-50",
 "hotkey_id": "open-terminal", "keys": ["ctrl", "alt", "t"]}
```

**Response:**
```json
{
  "type": "response", "id": "req-50", "seq": 50, "status": "ok",
  "data": { "registered": "open-terminal", "keys": ["ctrl", "alt", "t"] }
}
```

### `hotkeys.unregister`

Unregister a previously registered hotkey.

---

## Monitor & Location

### `monitor.list`

List connected monitors/displays.

**Request:**
```json
{"type": "monitor.list", "id": "req-51"}
```

**Response:**
```json
{
  "type": "response", "id": "req-51", "seq": 51, "status": "ok",
  "data": [
    { "id": 0, "name": "eDP-1", "primary": true,
      "width": 1920, "height": 1080, "scale": 1.0,
      "enabled": true, "x": 0, "y": 0,
      "refresh_rate": 60.0, "rotation": "normal" }
  ]
}
```

### `monitor.set_primary`

Set the primary output where the compositor supports a primary monitor concept.

**Request:**
```json
{"type": "monitor.set_primary", "id": "req-52", "output": "DP-1"}
```

**Response:**
```json
{
  "type": "response", "id": "req-52", "seq": 52, "status": "ok",
  "data": { "output": "DP-1", "primary": true }
}
```

Hyprland does not expose a native primary-monitor setting, so this action returns an error there.

### `monitor.set_resolution`

Set an output resolution, with optional refresh rate.

**Request:**
```json
{"type": "monitor.set_resolution", "id": "req-53",
 "output": "DP-1", "width": 2560, "height": 1440, "refresh_rate": 144}
```

**Response:**
```json
{
  "type": "response", "id": "req-53", "seq": 53, "status": "ok",
  "data": { "output": "DP-1", "width": 2560, "height": 1440, "refresh_rate": 144 }
}
```

### `monitor.set_scale`

Set output scale.

**Request:**
```json
{"type": "monitor.set_scale", "id": "req-54", "output": "eDP-1", "scale": 1.25}
```

### `monitor.set_rotation`

Set output rotation. Valid values: `normal`, `left`, `right`, `inverted`.

**Request:**
```json
{"type": "monitor.set_rotation", "id": "req-55", "output": "DP-1", "rotation": "left"}
```

### `monitor.enable` / `monitor.disable`

Enable or disable an output.

**Requests:**
```json
{"type": "monitor.enable", "id": "req-56", "output": "HDMI-A-1"}
{"type": "monitor.disable", "id": "req-57", "output": "HDMI-A-1"}
```

Monitor control uses compositor tooling: KDE uses `kscreen-doctor`, Hyprland uses `hyprctl`, X11 uses `xrandr`, and GNOME uses `xrandr` on X11 or `wlr-randr` where available. Permission-gated deployments should grant only the specific `monitor.*` write actions they intend to allow.

### `location.get`

Get geolocation. **Not yet implemented** — returns a placeholder.

**Request:**
```json
{"type": "location.get", "id": "req-58"}
```

**Response:**
```json
{
  "type": "response", "id": "req-58", "seq": 58, "status": "ok",
  "data": { "location": "not yet implemented" }
}
```

---

## Capabilities

### `capabilities.list`

List all known actions with per-backend support status. Returns both the full action list and two sub-lists: `supported` (actions that work on this backend) and `unsupported` (actions that are stubbed, compositor-limited, or require future work).

**Request:**
```json
{"type": "capabilities.list", "id": "req-53"}
```

**Response:**
```json
{
  "type": "response", "id": "req-53", "seq": 53, "status": "ok",
  "data": {
    "desktop": "gnome",
    "actions": ["windows.list", "windows.focus", "windows.get", ...],
    "supported": ["windows.list", "windows.focus", "workspaces.list", ...],
    "unsupported": [
      { "action": "ui.tree.get", "reason": "AT-SPI not integrated yet" }
    ]
  }
}
```

---

## UI Accessibility

### `ui.tree.get`

Get the accessibility tree. **Not yet implemented** — requires AT-SPI integration.

**Request:**
```json
{"type": "ui.tree.get", "id": "req-54"}
```

**Response:**
```json
{
  "type": "response", "id": "req-54", "seq": 54, "status": "ok",
  "data": { "supported": false, "reason": "AT-SPI not integrated yet", "nodes": [] }
}
```

### `ui.element.click`

Click a UI element by accessibility selector. **Not yet implemented.**

### `ui.element.set_text`

Set text on a UI element by accessibility selector. **Not yet implemented.**

---

## Event Subscription

### Subscribe

Register event subscriptions using glob patterns. Multiple events can be subscribed in one message.

**Request:**
```json
{"type": "subscribe", "id": "req-55", "events": ["file.*", "window.focused"]}
```

**Response:**
```json
{
  "type": "response", "id": "req-55", "seq": 55, "status": "ok",
  "data": {}
}
```

### Unsubscribe

Remove event subscriptions.

**Request:**
```json
{"type": "unsubscribe", "id": "req-56", "events": ["file.*"]}
```

**Response:**
```json
{
  "type": "response", "id": "req-56", "seq": 56, "status": "ok",
  "data": {}
}
```

### Event Envelope

All events are pushed asynchronously to subscribed clients in this format:

```json
{
  "type": "event",
  "id": "file.created",
  "data": {
    "event": "file.created",
    "path": "/home/user/projects/src/main.rs",
    "timestamp": 1715000000
  }
}
```

The `data` field structure depends on the event type:

| Event Type | `data` Fields |
|------------|--------------|
| `file.created` | `{ event, path, timestamp }` |
| `file.modified` | `{ event, path, timestamp }` |
| `file.deleted` | `{ event, path, timestamp }` |
| `file.renamed` | `{ event, path, new_path, timestamp }` |
| `window.focused` | `{ event, window_id, timestamp }` |
| `workspace.changed` | `{ event, workspace_id, timestamp }` |
| `workspace.window_moved` | `{ event, window_id, workspace_id, timestamp }` |

---

## System Control

### Ping

Liveness check.

**Request:**
```json
{"type": "ping", "id": "ping-1"}
```

**Response:**
```json
{
  "type": "pong", "id": "ping", "seq": 0
}
```

### Disconnect

Gracefully close the connection.

**Request:**
```json
{"type": "disconnect", "id": "dc-1"}
```

**Response:**
```json
{
  "type": "disconnected", "id": "dc", "seq": 0
}
```
