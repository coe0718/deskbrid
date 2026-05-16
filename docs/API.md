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

Close a window. **Not yet implemented** — returns a stub response.

**Request:**
```json
{"type": "windows.close", "id": "req-4", "window_id": "0x3a0000b"}
```

**Response:**
```json
{
  "type": "response", "id": "req-4", "seq": 4, "status": "ok",
  "data": { "window_id": "0x3a0000b", "supported": false,
            "reason": "backend has no close API yet" }
}
```

### `windows.minimize`

Minimize a window. **Not yet implemented.**

### `windows.maximize`

Maximize a window. **Not yet implemented.**

### `windows.move_resize`

Move and resize a window. **Not yet implemented.**

**Request:**
```json
{"type": "windows.move_resize", "id": "req-5", "window_id": "0x3a0000b",
 "x": 0, "y": 0, "width": 1920, "height": 1080}
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
        "x": 0, "y": 0, "refresh_rate": 60.0 }
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

### `system.capabilities`

Get a detailed capability matrix for the current backend. Returns every action with support status, degradation notes, dependency requirements, and session requirements.

**Request:**
```json
{"type": "system.capabilities", "id": "req-24"}
```

**Response:**
```json
{
  "type": "response", "id": "req-24", "seq": 24, "status": "ok",
  "data": {
    "schema_version": 1,
    "backend": "gnome",
    "actions": {
      "windows.list": { "supported": true, "degraded": false, "reason": null,
                        "requires": ["gnome-extension"], "session": "any", "degraded_modes": [] },
      "input.mouse": { "supported": true, "degraded": true,
                       "reason": "absolute_move_may_be_unavailable_without_screencast",
                       "requires": [], "session": "wayland", "degraded_modes": ["..."] },
      "windows.close": { "supported": false, "degraded": false,
                         "reason": "not_implemented", "requires": [],
                         "session": "any", "degraded_modes": [] }
    },
    "backend_notes": {
      "gnome": "window control via Shell extension + Mutter DBus",
      "kde": "window control via KWin scripting/DBus",
      "hyprland": "window control via hyprctl dispatch"
    }
  }
}
```

Each action entry has:

| Field | Type | Meaning |
|-------|------|---------|
| `supported` | bool | Whether the action works |
| `degraded` | bool | Works but with known limitations |
| `reason` | string or null | Human-readable explanation |
| `requires` | string[] | Prerequisite components |
| `session` | `"wayland"` / `"x11"` / `"any"` | Session-type requirement |
| `degraded_modes` | string[] | Specific degraded modes |

### `system.health`

Check backend dependency health. Reports each dependency as present or missing, with remediation suggestions.

**Request:**
```json
{"type": "system.health", "id": "req-25"}
```

**Response (GNOME):**
```json
{
  "type": "response", "id": "req-25", "seq": 25, "status": "ok",
  "data": {
    "schema_version": 1,
    "backend": "gnome",
    "deps": {
      "gnome-extension": { "ok": true, "details": "reachable" },
      "grim": { "ok": true, "details": "present" },
      "wl_clipboard": { "ok": true, "details": "wl-copy and wl-paste present" }
    },
    "remediation": {
      "ydotoold": "Start ydotoold in your user session (e.g. autostart entry).",
      "uinput": "Configure udev: KERNEL==\"uinput\", GROUP=\"input\", MODE=\"0660\"...",
      "gnome-extension": "Install/enable deskbrid GNOME extension, then restart shell/session.",
      "grim": "Install grim package for screenshots.",
      "spectacle": "Install spectacle package for KDE screenshots."
    }
  }
}
```

### `system.remediate`

Auto-fix missing dependencies. Currently supports `"ydotoold"` and `"kde_ydotoold_autostart"`.

**Request (check only):**
```json
{"type": "system.remediate", "id": "req-26", "check": "ydotoold", "apply": false}
```

**Response:**
```json
{
  "type": "response", "id": "req-26", "seq": 26, "status": "ok",
  "data": {
    "check": "ydotoold", "applied": false,
    "command": "ydotoold &",
    "note": "Set apply=true to start ydotoold in current user session"
  }
}
```

**Request (apply):**
```json
{"type": "system.remediate", "id": "req-27", "check": "ydotoold", "apply": true}
```

**Response:**
```json
{
  "type": "response", "id": "req-27", "seq": 27, "status": "ok",
  "data": {
    "check": "ydotoold", "applied": true,
    "details": "started_or_already_running"
  }
}
```

### `system.normalize_coords`

Convert monitor-relative coordinates (e.g., from an LLM's spatial model) to absolute backend coordinates, factoring in monitor scale.

**Request:**
```json
{"type": "system.normalize_coords", "id": "req-28",
 "x": 960, "y": 540, "monitor": 0}
```

**Response:**
```json
{
  "type": "response", "id": "req-28", "seq": 28, "status": "ok",
  "data": {
    "input": { "x": 960, "y": 540, "monitor": 0 },
    "monitor": { "id": 0, "name": "eDP-1", "scale": 1.0,
                 "width": 1920, "height": 1080 },
    "backend_coords": { "x": 960, "y": 540 }
  }
}
```

---

## Network

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

### `files.search`

Search for files by name pattern.

**Request:**
```json
{"type": "files.search", "id": "req-41",
 "pattern": "*.rs", "root": "/home/user/projects", "max_results": 20}
```

Parameters: `pattern` (required glob), `root` (optional, defaults to `/`), `max_results` (optional, defaults to 50).

**Response:**
```json
{
  "type": "response", "id": "req-41", "seq": 41, "status": "ok",
  "data": { "matches": ["/home/user/projects/src/main.rs",
                          "/home/user/projects/src/daemon.rs"] }
}
```

### `files.watch`

Watch a path for file changes.

**Request:**
```json
{"type": "files.watch", "id": "req-42",
 "path": "/home/user/projects", "recursive": true,
 "patterns": ["*.rs", "*.toml"]}
```

`patterns` is optional — if omitted, watches all file changes.

**Response:**
```json
{
  "type": "response", "id": "req-42", "seq": 42, "status": "ok",
  "data": { "watching": "/home/user/projects" }
}
```

Events are pushed to subscribed clients as `file.created`, `file.modified`, `file.deleted`, and `file.renamed` events.

### `files.unwatch`

Stop watching a path.

**Request:**
```json
{"type": "files.unwatch", "id": "req-43", "path": "/home/user/projects"}
```

**Response:**
```json
{
  "type": "response", "id": "req-43", "seq": 43, "status": "ok",
  "data": { "unwatched": "/home/user/projects" }
}
```

---

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
      "x": 0, "y": 0, "refresh_rate": 60.0 }
  ]
}
```

### `location.get`

Get geolocation. **Not yet implemented** — returns a placeholder.

**Request:**
```json
{"type": "location.get", "id": "req-52"}
```

**Response:**
```json
{
  "type": "response", "id": "req-52", "seq": 52, "status": "ok",
  "data": { "location": "not yet implemented" }
}
```

---

## Capabilities

### `capabilities.list`

List all known actions with per-backend support status. Returns both the full action list and two sub-lists: `supported` (actions that work on this backend) and `unsupported` (actions that are stubbed or require future work).

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
      { "action": "windows.close", "reason": "not implemented in backend trait" },
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
