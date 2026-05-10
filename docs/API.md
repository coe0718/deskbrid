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

### Connection Handshake

On socket connect, the daemon immediately sends a `connected` message. Clients **must** wait for this before sending commands:

```json
{"type":"connected","id":"server","seq":0,"data":{"version":"2.0.0","protocol":"deskbrid-v2"}}
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
| `window_id` | string{?} | Window ID to capture (via `slurp -o`) |

**Response fields:**

| Field | Type | Description |
|-------|------|-------------|
| `path` | string | Absolute path to the saved PNG file |
| `width` | number | Image width in pixels |
| `height` | number | Image height in pixels |
| `format` | string | Always `"png"` |

**Backend:** `grim` with optional `slurp` for region/window selection. Screenshots are saved to `/tmp/deskbrid-screenshot-<unix_timestamp>.png`.

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
| `notification_id` | number | ID returned by `notification.send` |

**Backend:** DBus `org.freedesktop.Notifications.CloseNotification`.

---

## System

### `system.info`

Get comprehensive desktop environment information.

**Request:**
```json
{"type":"system.info","id":"system.info","seq":21}
```

**Response:**
```json
{
  "type": "response",
  "id": "action",
  "seq": 21,
  "status": "ok",
  "data": {
    "desktop": "GNOME",
    "desktop_version": "46",
    "compositor": "Mutter",
    "session_type": "wayland",
    "monitors": [
      {"id": 0, "name": "eDP-1", "width": 1920, "height": 1080, "scale": 1.0, "primary": true}
    ],
    "workspace_count": 4,
    "current_workspace": 0,
    "idle_seconds": 142
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `desktop` | string | Desktop environment name |
| `desktop_version` | string | Version string |
| `compositor` | string | Compositor name |
| `session_type` | string | `"wayland"` or `"x11"` |
| `monitors` | array | List of connected monitors (see `MonitorList`) |
| `workspace_count` | number | Number of workspaces |
| `current_workspace` | number | Active workspace index |
| `idle_seconds` | number | Seconds since last user input |

**Backend:** `$XDG_SESSION_DESKTOP` / `$XDG_SESSION_TYPE`, Mutter idle monitor (`GetIdletime`), `wlr-randr`, GNOME Shell extension workspace queries.

---

### `system.idle`

Get idle time only (lightweight alternative to `system.info`).

**Request:**
```json
{"type":"system.idle","id":"system.idle","seq":22}
```

**Response:**
```json
{"type":"response","id":"action","seq":22,"status":"ok","data":{"idle_seconds":142}}
```

**Backend:** Same as `system.info` idle — Mutter `GetIdletime`, or `loginctl`/`xssstate` fallback.

---

### `system.power`

Perform a power action.

**Request:**
```json
{"type":"system.power","action":"suspend","id":"system.power","seq":23}
```

| Param | Type | Description |
|-------|------|-------------|
| `action` | string | One of: `"suspend"`, `"hibernate"`, `"reboot"`, `"shutdown"` |

**Response:**
```json
{"type":"response","id":"action","seq":23,"status":"ok","data":{"power":"suspend"}}
```

**Backend:** Executes `systemctl <action>` for the corresponding systemd target.

---

### `system.battery`

Get battery status from UPower.

**Request:**
```json
{"type":"system.battery","id":"system.battery","seq":24}
```

**Response:**
```json
{
  "type": "response",
  "id": "action",
  "seq": 24,
  "status": "ok",
  "data": [
    {
      "source": "BAT0",
      "percentage": 85.5,
      "state": "charging",
      "time_remaining_minutes": 45
    }
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `source` | string | Power source name (e.g. `"BAT0"`) |
| `percentage` | number | Charge percentage (0.0–100.0) |
| `state` | string | `"charging"`, `"discharging"`, `"full"`, `"empty"` |
| `time_remaining_minutes` | number{?} | Estimated minutes remaining or to full |

**Backend:** UPower DBus → `org.freedesktop.UPower` → iterates display devices.

---

## Network

### `network.status`

Get overall network connectivity status.

**Request:**
```json
{"type":"network.status","id":"network.status","seq":25}
```

**Response:**
```json
{
  "type": "response",
  "id": "action",
  "seq": 25,
  "status": "ok",
  "data": {
    "online": true,
    "type": "wifi"
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `online` | boolean | Whether the system has internet connectivity |
| `type` | string | Connection type: `"wifi"`, `"ethernet"`, `"unknown"` |

**Backend:** NetworkManager DBus → `org.freedesktop.NetworkManager` → `state` property and primary connection type.

---

### `network.interfaces`

List network interfaces with IP addresses.

**Request:**
```json
{"type":"network.interfaces","id":"network.interfaces","seq":26}
```

**Response:**
```json
{
  "type": "response",
  "id": "action",
  "seq": 26,
  "status": "ok",
  "data": [
    {"name": "wlp2s0", "state": "activated", "ipv4": "192.168.1.42", "ipv6": "fe80::..."},
    {"name": "enp3s0", "state": "disconnected", "ipv4": null, "ipv6": null}
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Interface name |
| `state` | string | NM device state string |
| `ipv4` | string{?} | IPv4 address, or `null` if not assigned |
| `ipv6` | string{?} | IPv6 address, or `null` if not assigned |

**Backend:** NetworkManager DBus → `GetAllDevices`, device properties, and `IP4Config` address data.

---

### `network.wifi.scan`

Scan for visible Wi-Fi access points.

**Request:**
```json
{"type":"network.wifi.scan","id":"network.wifi.scan","seq":27}
```

**Response:**
```json
{
  "type": "response",
  "id": "action",
  "seq": 27,
  "status": "ok",
  "data": [
    {"ssid": "CoffeeShop", "strength": 85, "secured": true, "frequency": 5180},
    {"ssid": "GuestNet", "strength": 30, "secured": false, "frequency": 2437}
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `ssid` | string | Network name |
| `strength` | number | Signal strength (0–100) |
| `secured` | boolean | Whether the network has encryption |
| `frequency` | number{?} | Frequency in MHz, or `null` if unknown |

**Backend:** NetworkManager DBus → wireless device → `GetAllAccessPoints`, then `org.freedesktop.NetworkManager.AccessPoint` properties.

---

### `network.wifi.connect`

Connect to a Wi-Fi network.

**Request:**
```json
{"type":"network.wifi.connect","ssid":"CoffeeShop","password":"my_password","id":"network.wifi.connect","seq":28}
```

**Response:**
```json
{"type":"response","id":"action","seq":28,"status":"ok","data":{"connected":"CoffeeShop"}}
```

| Param | Type | Description |
|-------|------|-------------|
| `ssid` | string | Network SSID to connect to |
| `password` | string{?} | Network password (omit for open networks) |

**Backend:** `nmcli device wifi connect <ssid> password <password>`.

---

## Bluetooth

### `bluetooth.list`

List known Bluetooth devices.

**Request:**
```json
{"type":"bluetooth.list","id":"bluetooth.list","seq":29}
```

**Response:**
```json
{
  "type": "response",
  "id": "action",
  "seq": 29,
  "status": "ok",
  "data": [
    {
      "address": "AA:BB:CC:DD:EE:FF",
      "name": "Wireless Headphones",
      "paired": true,
      "connected": false,
      "rssi": -45
    }
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `address` | string | MAC address in `XX:XX:XX:XX:XX:XX` format |
| `name` | string | Human-readable device name |
| `paired` | boolean | Whether the device is paired |
| `connected` | boolean | Whether the device is currently connected |
| `rssi` | number{?} | Signal strength in dBm, or `null` |

**Backend:** BlueZ DBus → `org.bluez` → `ObjectManager.GetManagedObjects` — iterates all objects with `org.bluez.Device1` interface.

---

### `bluetooth.scan`

Start Bluetooth device discovery.

**Request:**
```json
{"type":"bluetooth.scan","duration":10,"id":"bluetooth.scan","seq":30}
```

**Response:**
```json
{"type":"response","id":"action","seq":30,"status":"ok","data":{"scanning":true}}
```

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `duration` | number{?} | continuous | Scan duration in seconds (optional, implementation may vary) |

**Backend:** BlueZ DBus → finds first adapter → `org.bluez.Adapter1.StartDiscovery`. The `duration` parameter is accepted but the daemon does not auto-stop the scan after it — call `bluetooth.scan_stop` explicitly.

---

### `bluetooth.scan_stop`

Stop Bluetooth discovery.

**Request:**
```json
{"type":"bluetooth.scan_stop","id":"bluetooth.scan_stop","seq":31}
```

**Response:**
```json
{"type":"response","id":"action","seq":31,"status":"ok","data":{"scanning":false}}
```

**Backend:** BlueZ DBus → `org.bluez.Adapter1.StopDiscovery`.

---

### `bluetooth.connect`

Connect to a paired Bluetooth device.

**Request:**
```json
{"type":"bluetooth.connect","address":"AA:BB:CC:DD:EE:FF","id":"bluetooth.connect","seq":32}
```

**Response:**
```json
{"type":"response","id":"action","seq":32,"status":"ok","data":{"connected":"AA:BB:CC:DD:EE:FF"}}
```

| Param | Type | Description |
|-------|------|-------------|
| `address` | string | Bluetooth MAC address |

**Backend:** BlueZ DBus → looks up device path by normalised address → `org.bluez.Device1.Connect`.

---

### `bluetooth.disconnect`

Disconnect a Bluetooth device.

**Request:**
```json
{"type":"bluetooth.disconnect","address":"AA:BB:CC:DD:EE:FF","id":"bluetooth.disconnect","seq":33}
```

**Response:**
```json
{"type":"response","id":"action","seq":33,"status":"ok","data":{"disconnected":"AA:BB:CC:DD:EE:FF"}}
```

**Backend:** BlueZ DBus → `org.bluez.Device1.Disconnect`. Does not fail if device was already disconnected.

---

### `bluetooth.pair`

Pair with a Bluetooth device.

**Request:**
```json
{"type":"bluetooth.pair","address":"AA:BB:CC:DD:EE:FF","id":"bluetooth.pair","seq":34}
```

**Response:**
```json
{"type":"response","id":"action","seq":34,"status":"ok","data":{"paired":"AA:BB:CC:DD:EE:FF","note":"not yet supported"}}
```

**Status:** Not yet implemented in the GNOME backend. The request is accepted but returns a stub response.

---

### `bluetooth.forget`

Forget/unpair a Bluetooth device.

**Request:**
```json
{"type":"bluetooth.forget","address":"AA:BB:CC:DD:EE:FF","id":"bluetooth.forget","seq":35}
```

**Response:**
```json
{"type":"response","id":"action","seq":35,"status":"ok","data":{"forgotten":"AA:BB:CC:DD:EE:FF","note":"not yet supported"}}
```

**Status:** Not yet implemented in the GNOME backend. The request is accepted but returns a stub response.

---

## Audio

### `audio.list_sinks`

List audio output sinks (speakers, headphones, etc.).

**Request:**
```json
{"type":"audio.list_sinks","id":"audio.list_sinks","seq":36}
```

**Response:**
```json
{
  "type": "response",
  "id": "action",
  "seq": 36,
  "status": "ok",
  "data": [
    {"id": 0, "name": "alsa_output.pci-0000_00_1f.3.analog-stereo", "description": "Built-in Audio Analog Stereo", "volume": 0.75, "muted": false},
    {"id": 1, "name": "bluez_sink.AA_BB_CC_DD_EE_FF.a2dp_sink", "description": "Wireless Headphones", "volume": 0.50, "muted": false}
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | number | Sink ID (used for `audio.set_sink_volume`) |
| `name` | string | PulseAudio/PipeWire sink name |
| `description` | string | Human-readable description |
| `volume` | number | Volume level 0.0–1.0 (reported as 0.0–100% internally) |
| `muted` | boolean | Whether the sink is muted |

**Backend:** `pactl list sinks` parsed by `parse_pactl_sinks()`.

---

### `audio.set_sink_volume`

Set a sink's volume level.

**Request:**
```json
{"type":"audio.set_sink_volume","sink_id":0,"volume":0.5,"id":"audio.set_sink_volume","seq":37}
```

**Response:**
```json
{"type":"response","id":"action","seq":37,"status":"ok","data":{"sink":0,"volume":0.5}}
```

| Param | Type | Description |
|-------|------|-------------|
| `sink_id` | number | Sink ID from `audio.list_sinks` |
| `volume` | number | Volume level 0.0–1.0 (internally multiplied by 100 for `pactl`) |

**Backend:** `pactl set-sink-volume <id> <vol%>`.

---

## Files

### `files.watch`

Watch a file or directory for changes. Emits events on file create, modify, delete, and rename.

**Request:**
```json
{"type":"files.watch","path":"/home/user/projects","recursive":true,"id":"files.watch","seq":38}
```

**Response:**
```json
{"type":"response","id":"action","seq":38,"status":"ok","data":{"watching":"/home/user/projects"}}
```

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | string | — | Absolute path to file or directory |
| `recursive` | boolean | `true` | Watch subdirectories recursively |
| `patterns` | array{?} | `null` | Glob patterns (currently accepted but not filtered) |

Events are delivered to clients with matching subscriptions. See [Event Subscriptions](#event-subscriptions).

**Backend:** `notify::recommended_watcher` (inotify on Linux, FSEvents on macOS).

---

### `files.unwatch`

Stop watching a file or directory.

**Request:**
```json
{"type":"files.unwatch","path":"/home/user/projects","id":"files.unwatch","seq":39}
```

**Response:**
```json
{"type":"response","id":"action","seq":39,"status":"ok","data":{"unwatched":"/home/user/projects"}}
```

The watcher is removed from the backend's watcher map on the Rust side.

---

### `files.search`

Search for files by name pattern.

**Request (with fd):**
```json
{"type":"files.search","pattern":"*.rs","root":"/home/user/projects","max_results":20,"id":"files.search","seq":40}
```

**Response:**
```json
{
  "type": "response",
  "id": "action",
  "seq": 40,
  "status": "ok",
  "data": {
    "matches": [
      "/home/user/projects/deskbrid/src/main.rs",
      "/home/user/projects/deskbrid/src/daemon.rs"
    ]
  }
}
```

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `pattern` | string | — | File name pattern/glob |
| `root` | string{?} | `"."` | Root directory to search from |
| `max_results` | number | `50` | Maximum number of results to return |

**Backend:** `fd --max-results <N> --search-path <root> <pattern>`, or `find <root> -name <pattern> -maxdepth 10` if `fd` is not available.

---

## Processes

### `process.list`

List running processes via `ps aux`.

**Request:**
```json
{"type":"process.list","id":"process.list","seq":41}
```

**Response:**
```json
{
  "type": "response",
  "id": "action",
  "seq": 41,
  "status": "ok",
  "data": {
    "processes": [
      {"user": "user", "pid": 1234, "cpu": "0.0", "mem": "0.1", "command": "/usr/bin/gnome-shell"}
    ]
  }
}
```

Results are capped at 200 processes.

**Backend:** `ps aux --no-headers`.

---

### `process.start`

Start a background process.

**Request:**
```json
{"type":"process.start","command":["notify-send","Done"],"workdir":"/tmp","env":{"FOO":"bar"},"id":"process.start","seq":42}
```

**Response:**
```json
{"type":"response","id":"action","seq":42,"status":"ok","data":{"pid":5678,"command":["notify-send","Done"]}}
```

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `command` | array of strings | — | Command and arguments (e.g. `["cargo", "build"]`) |
| `workdir` | string{?} | daemon's cwd | Working directory for the process |
| `env` | object{?} | inherited | Environment variable overrides `{"KEY": "value"}` |

The process is spawned with stdin/stdout/stderr directed to `/dev/null` (fire-and-forget).

---

## Hotkeys

### `hotkeys.register`

Register a global hotkey combination.

**Request:**
```json
{"type":"hotkeys.register","hotkey_id":"my-hotkey","keys":["ctrl","alt","t"],"id":"hotkeys.register","seq":43}
```

**Response:**
```json
{"type":"response","id":"action","seq":43,"status":"ok","data":{"registered":"my-hotkey","keys":["ctrl","alt","t"]}}
```

**Status:** Registration is accepted and stored in per-connection state, but hotkey listening is not yet implemented in the GNOME backend.

---

### `hotkeys.unregister`

Unregister a previously registered hotkey.

**Request:**
```json
{"type":"hotkeys.unregister","hotkey_id":"my-hotkey","id":"hotkeys.unregister","seq":44}
```

**Response:**
```json
{"type":"response","id":"action","seq":44,"status":"ok","data":{"unregistered":"my-hotkey"}}
```

---

## Display / Monitor

### `monitor.list`

List connected display monitors.

**Request:**
```json
{"type":"monitor.list","id":"monitor.list","seq":45}
```

**Response:**
```json
{
  "type": "response",
  "id": "action",
  "seq": 45,
  "status": "ok",
  "data": [
    {"id": 0, "name": "eDP-1", "width": 1920, "height": 1080, "scale": 1.0, "primary": true}
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | number | Monotonically increasing monitor ID |
| `name` | string | Monitor name (e.g. `"eDP-1"`, `"DP-2"`) |
| `width` | number | Horizontal resolution in pixels |
| `height` | number | Vertical resolution in pixels |
| `scale` | number | Display scale factor (1.0, 1.25, 1.5, 2.0, etc.) |
| `primary` | boolean | Whether this is the primary display |

**Backend:** `wlr-randr` output parsing. Falls back to a single 1920×1080 monitor entry if `wlr-randr` is unavailable.

---

## Location

### `location.get`

Get system location.

**Request:**
```json
{"type":"location.get","id":"location.get","seq":46}
```

**Response:**
```json
{"type":"response","id":"action","seq":46,"status":"ok","data":{"location":"not yet implemented"}}
```

**Status:** Not yet implemented. Returns a stub response.

---

## Connection Management

### `ping`

Health check — no backend required.

**Request:**
```json
{"type":"ping","id":"ping","seq":47}
```

**Response:**
```json
{"type":"pong","id":"ping","seq":47}
```

---

### `disconnect`

Gracefully close the daemon connection.

**Request:**
```json
{"type":"disconnect","id":"disconnect","seq":48}
```

**Response:**
```json
{"type":"disconnected","id":"dc","seq":48}
```

After receiving `disconnected`, the client should close the socket.

---

## Event Subscriptions

The daemon supports a publish-subscribe model for desktop events. Clients register interest in event types, and matching events are pushed as they occur.

### `subscribe`

Subscribe to one or more event patterns.

**Request:**
```json
{"type":"subscribe","events":["file.created","file.*"],"id":"subscribe","seq":49}
```

**Response:**
```json
{"type":"response","id":"subscribe","seq":49,"status":"ok","data":{}}
```

| Param | Type | Description |
|-------|------|-------------|
| `events` | array of strings | Event patterns to subscribe to |

### Glob matching rules

| Pattern | Matches | Does not match |
|---------|---------|----------------|
| `file.created` | `file.created` only | `file.modified` |
| `file.*` | `file.created`, `file.modified`, `file.deleted`, `file.renamed` | `clipboard.changed` |
| `*` | Everything | — |

The matching function (`event_matches_any`) supports:
- **Exact match:** `"file.created"` matches only `"file.created"`
- **Prefix glob:** `"file.*"` matches anything starting with `"file."` followed by at least one character after the dot
- **Wildcard:** `"*"` matches everything

### `unsubscribe`

Remove one or more event subscriptions.

**Request:**
```json
{"type":"unsubscribe","events":["file.*"],"id":"unsubscribe","seq":50}
```

**Response:**
```json
{"type":"response","id":"unsubscribe","seq":50,"status":"ok","data":{}}
```

### Event Types

| Event | Payload |
|-------|---------|
| `file.created` | `{"event":"file.created","path":"/abs/path/to/file","timestamp":1715000000}` |
| `file.modified` | `{"event":"file.modified","path":"/abs/path/to/file","timestamp":1715000000}` |
| `file.deleted` | `{"event":"file.deleted","path":"/abs/path/to/file","timestamp":1715000000}` |
| `file.renamed` | `{"event":"file.renamed","old_path":"/old/path","new_path":"/new/path","timestamp":1715000000}` |

**Event envelope:**
```json
{"type":"event","id":"file.created","data":{"event":"file.created","path":"/tmp/new.txt","timestamp":1715000000}}
```

Events are only delivered to clients whose subscription set matches the event type. You must have initiated a `files.watch` on the relevant path before events will be produced.

---

## Data Types Reference

### `WindowInfo`
```rust
pub struct WindowInfo {
    pub id: String,            // Window XID as hex string
    pub title: String,         // Window title
    pub app_id: String,        // Application ID (WM class)
    pub workspace_id: u32,     // 0-based workspace index
    pub is_focused: bool,      // Currently focused?
    pub is_minimized: bool,    // Minimized?
    pub geometry: Option<Geometry>,  // Position and size
    pub pid: Option<u32>,       // Process ID
}
```

### `MonitorInfo`
```rust
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub primary: bool,
}
```

### `Geometry`
```rust
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
```

### `SystemInfo`
```rust
pub struct SystemInfo {
    pub desktop: String,           // e.g. "GNOME"
    pub desktop_version: String,   // e.g. "46"
    pub compositor: String,        // e.g. "Mutter"
    pub session_type: String,      // "wayland" or "x11"
    pub monitors: Vec<MonitorInfo>,
    pub workspace_count: u32,
    pub current_workspace: u32,
    pub idle_seconds: u64,
}
```

### `BatteryInfo`
```rust
pub struct BatteryInfo {
    pub source: String,
    pub percentage: f64,
    pub state: String,
    pub time_remaining_minutes: Option<u32>,
}
```

### `NetworkStatusInfo`
```rust
pub struct NetworkStatusInfo {
    pub online: bool,
    #[serde(rename = "type")]
    pub net_type: String,          // "wifi", "ethernet", "unknown"
}
```

### `NetworkInterfaceInfo`
```rust
pub struct NetworkInterfaceInfo {
    pub name: String,
    pub state: String,             // NM device state
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
}
```

### `WifiNetworkInfo`
```rust
pub struct WifiNetworkInfo {
    pub ssid: String,
    pub strength: u32,             // 0–100
    pub secured: bool,
    pub frequency: Option<u32>,    // MHz
}
```

### `BluetoothDeviceInfo`
```rust
pub struct BluetoothDeviceInfo {
    pub address: String,           // "XX:XX:XX:XX:XX:XX"
    pub name: String,
    pub paired: bool,
    pub connected: bool,
    pub rssi: Option<i32>,         // dBm
}
```

### `AudioSinkInfo`
```rust
pub struct AudioSinkInfo {
    pub id: u32,
    pub name: String,              // PulseAudio sink name
    pub description: String,       // Human-readable
    pub volume: f64,               // 0.0–1.0
    pub muted: bool,
}
```

### `ScreenshotResult`
```rust
pub struct ScreenshotResult {
    pub path: String,              // Absolute path to PNG
    pub width: u32,
    pub height: u32,
    pub format: String,            // Always "png"
}
```

---

## Implementation Status

| Domain | Action | Status |
|--------|--------|--------|
| Windows | list, focus, get | ✅ Implemented |
| Workspaces | list, switch, move_window | ✅ Implemented |
| Input | keyboard (type/key/combo), mouse (move/click/scroll) | ✅ Implemented |
| Clipboard | read, write | ✅ Implemented |
| Screenshot | capture (full/monitor/region/window) | ✅ Implemented |
| Notifications | send, close | ✅ Implemented |
| System | info, idle, power, battery | ✅ Implemented |
| Network | status, interfaces, wifi.scan, wifi.connect | ✅ Implemented |
| Bluetooth | list, scan, scan_stop, connect, disconnect | ✅ Implemented |
| Bluetooth | pair, forget | 🚧 Stub (returns placeholder) |
| Audio | list_sinks, set_sink_volume | ✅ Implemented |
| Files | watch, unwatch, search | ✅ Implemented |
| Processes | list, start | ✅ Implemented |
| Hotkeys | register, unregister | 🚧 Stub (accepted, not wired) |
| Monitor | list | ✅ Implemented |
| Location | get | 🚧 Stub |
| Events | subscribe, unsubscribe | ✅ Implemented |
| Connection | ping, disconnect | ✅ Implemented |
