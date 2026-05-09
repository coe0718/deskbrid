# Deskbrid v2 — Agent-Native Linux Desktop HAL Protocol

## Design Goals

- **Event-first**: agents subscribe to what they need, get push notifications
- **Script-first**: same protocol works from bash, Python, or any language via the CLI — no SDK required
- **Simple transport**: NDJSON over Unix socket. One line = one message. No framing, no binary, no magic.
- **Backend-agnostic schema**: the protocol doesn't care if you're on GNOME, KDE, or Sway
- **Composable**: connect multiple agents to one daemon, each with their own subscriptions
- **Self-describing**: every message has a `type`, `id` for tracing, and enough context to debug

---

## 1. Transport

**Socket**: `$XDG_RUNTIME_DIR/deskbrid.sock` (typically `/run/user/1000/deskbrid.sock`)

**Protocol**: Newline-delimited JSON (NDJSON). Each message is a single JSON object terminated by `\n`. No length prefixes, no chunking. Max message size: 1 MiB.

**Connection model**:
- Persistent TCP-style connection over Unix socket
- Daemon accepts multiple concurrent clients
- Daemon can push events at any time
- Client sends actions, gets responses
- Client sends subscribe/unsubscribe to control event flow
- Graceful close: client sends `{"type":"disconnect"}`, daemon cleans up subscriptions

**Security**:
- Socket permissions: `0700`, owned by the user
- No authentication on local socket (Unix permissions are the auth)
- Future: optional token auth for remote connections

---

## 2. Message Structure

Every message has a common envelope:

```json
{
  "type": "<message-type>",
  "id": "<uuid>",
  "ts": "<rfc3339-timestamp>",
  "seq": 42,
  ...
}
```

- `type` — determines the rest of the schema
- `id` — client-generated UUID for tracing. Server echoes it in responses
- `ts` — ISO 8601 / RFC 3339 timestamp
- `seq` — monotonically increasing per-connection sequence number. Clients start at 1 for each connection.

---

## 3. The `deskbrid` CLI

The same binary serves double duty — run as **daemon** (persistent server) or **client** (one-shot commands for scripts).

### Usage

```
deskbrid daemon                          # Start the daemon (systemd user service)
deskbrid daemon --verbose                # With debug logging

deskbrid windows list                    # List all windows
deskbrid windows focus 0x02400004        # Focus a window
deskbrid windows get 0x02400004          # Get window details

deskbrid workspaces list                 # List workspaces
deskbrid workspaces switch 2             # Switch to workspace 2
deskbrid workspaces move 0x02400004 3    # Move window to workspace 3

deskbrid input type "Hello world"        # Type text
deskbrid input key Return                # Press a key
deskbrid combo Control_L+c               # Keyboard combo

deskbrid mouse move 960 540              # Move mouse to position
deskbrid mouse click left                # Click
deskbrid mouse scroll 0 -3               # Scroll

deskbrid clipboard read                  # Read clipboard
deskbrid clipboard write "copied"        # Write clipboard

deskbrid screenshot                      # Full screenshot → /tmp/deskbrid/
deskbrid screenshot --monitor 0          # Specific monitor
deskbrid screenshot --region 100 100 500 400
deskbrid screenshot --window 0x02400004
deskbrid screenshot --output ~/screen.png

deskbrid notify send --title "Done" --body "Build finished"
deskbrid notify close 42

deskbrid system info                     # Desktop + monitor info
deskbrid system idle                     # Idle seconds
deskbrid system power suspend            # Suspend/hibernate/reboot/shutdown/lock
deskbrid system battery                  # Battery status

deskbrid network status                  # Online/offline
deskbrid network interfaces              # Interface list
deskbrid wifi scan                       # Available networks
deskbrid wifi connect MyNetwork          # Connect (prompts for password)

deskbrid bluetooth list                  # Known devices
deskbrid bluetooth scan                  # Discover new devices
deskbrid bluetooth connect AA:BB:CC:DD:EE:FF
deskbrid bluetooth disconnect AA:BB:CC:DD:EE:FF

deskbrid files search "*.rs"             # Find files
deskbrid files watch /home/user/projects # Start watching (blocking)
deskbrid files unwatch /home/user/projects

deskbrid audio sinks                     # List audio sinks
deskbrid audio volume 0 0.75             # Set sink volume

deskbrid wait window.focus_changed       # Block until event fires
deskbrid wait clipboard.changed          # (one-shot event listener)
```

### How It Works

- CLI connects to the running daemon's Unix socket
- Sends the appropriate NDJSON action message
- Reads the response
- Prints formatted output and exits

For one-shot commands (`screenshot`, `clipboard read`, `windows list`):
1. Connect → send action → read response → print → exit (fast)

For blocking commands (`files watch`, `wait`):
1. Connect → subscribe → print events as they arrive → Ctrl+C to stop

### Script Usage

```bash
# Shell one-liners
WINDOW_COUNT=$(deskbrid windows list | jq '.windows | length')

# Conditional logic
if deskbrid network status | jq -e '.online' > /dev/null; then
  echo "We're online"
fi

# Capture and use
deskbrid screenshot --output /tmp/latest.png
deskbrid clipboard write "screenshot at $(date)"

# Quick automation
deskbrid combo Super_L+d                 # Show desktop
sleep 0.5
deskbrid mouse move 960 540
deskbrid mouse click left
```

### daemon Status

```
deskbrid status                          # Is daemon running? PID? Uptime?
deskbrid clients                         # Connected clients + subscriptions
```

---

## 4. Message Types

### 4.1 Client → Server (Actions)

#### `ping`

Health check. Server responds with `pong`.

```json
{"type": "ping", "id": "abc-123", "seq": 1}
```

#### `subscribe`

Register for push events. Takes an array of event names or `"*"` for all.

```json
{"type": "subscribe", "id": "abc-124", "seq": 2, "events": ["window.*", "clipboard.changed", "notification.received"]}
```

Events field is a list of glob patterns:
- `"window.*"` — all window events
- `"*"` — everything (explicit opt-in, not the default)
- `"window.focus_changed"` — single event

#### `unsubscribe`

Remove subscriptions. Same pattern format.

```json
{"type": "unsubscribe", "id": "abc-125", "seq": 3, "events": ["window.*"]}
```

#### `windows.list`

Return all windows with their properties.

```json
{"type": "windows.list", "id": "abc-126", "seq": 4}
```

#### `windows.focus`

Focus (raise and activate) a specific window.

```json
{"type": "windows.focus", "id": "abc-127", "seq": 5, "window_id": "0x02400004"}
```

#### `windows.get`

Get details about a specific window.

```json
{"type": "windows.get", "id": "abc-128", "seq": 6, "window_id": "0x02400004"}
```

#### `workspaces.list`

List all workspaces/virtual desktops.

```json
{"type": "workspaces.list", "id": "abc-129", "seq": 7}
```

#### `workspaces.switch`

Switch to a workspace.

```json
{"type": "workspaces.switch", "id": "abc-130", "seq": 8, "workspace_id": 2}
```

#### `workspaces.move_window`

Move a window to a workspace (optionally follow it there).

```json
{"type": "workspaces.move_window", "id": "abc-131", "seq": 9, "window_id": "0x02400004", "workspace_id": 3, "follow": false}
```

#### `input.keyboard`

Send keyboard input. Supports three modes:
- `key` — press and release a single key (tap)
- `type` — type a string of text
- `combo` — press a chord (e.g. Ctrl+C)

```json
{"type": "input.keyboard", "id": "abc-132", "seq": 10, "action": "type", "text": "Hello, world!"}

{"type": "input.keyboard", "id": "abc-133", "seq": 11, "action": "key", "key": "Return"}

{"type": "input.keyboard", "id": "abc-134", "seq": 12, "action": "combo", "keys": ["Control_L", "c"]}
```

Keys follow XKB key names. Common ones: `Return`, `Tab`, `Escape`, `BackSpace`, `Control_L`, `Alt_L`, `Super_L`, `Shift_L`, `space`, `a`-`z`, `0`-`9`.

#### `input.mouse`

Mouse control.

```json
{"type": "input.mouse", "id": "abc-135", "seq": 13, "action": "move", "x": 960, "y": 540}

{"type": "input.mouse", "id": "abc-136", "seq": 14, "action": "click", "button": "left"}

{"type": "input.mouse", "id": "abc-137", "seq": 15, "action": "scroll", "dx": 0, "dy": -3}
```

Available buttons: `left`, `middle`, `right`, `scroll_up`, `scroll_down`.
Click actions: `press`, `release`, `click` (press+release).

#### `clipboard.read`

Read current clipboard contents.

```json
{"type": "clipboard.read", "id": "abc-138", "seq": 16}
```

#### `clipboard.write`

Write to clipboard.

```json
{"type": "clipboard.write", "id": "abc-139", "seq": 17, "text": "copied text"}
```

#### `screenshot`

Take a screenshot.

```json
{"type": "screenshot", "id": "abc-140", "seq": 18, "monitor": 0}
```

Optional fields:
- `monitor` — monitor index (0 = primary, omit = all)
- `region` — `{"x": 100, "y": 100, "width": 500, "height": 400}`
- `window_id` — capture a specific window

Response includes a path to the PNG file on disk (and optionally base64-encoded data for small captures).

#### `notification.send`

Send a desktop notification.

```json
{"type": "notification.send", "id": "abc-141", "seq": 19, "app_name": "deskbrid-agent", "title": "Build finished", "body": "All tests passed", "urgency": "normal"}
```

Urgency: `low`, `normal`, `critical`.

#### `notification.close`

Close a specific notification by its ID.

```json
{"type": "notification.close", "id": "abc-142", "seq": 20, "notification_id": 5}
```

#### `system.info`

Request system/environment info.

```json
{"type": "system.info", "id": "abc-143", "seq": 21}
```

Returns: desktop environment, compositor, display info, connected monitors, resolution, workspace count, seat info, idle state.

#### `system.idle`

Get current idle status.

```json
{"type": "system.idle", "id": "abc-144", "seq": 22}
```

#### `hotkeys.register`

Register a global hotkey. When pressed, daemon emits `input.hotkey` event to registered subscribers.

```json
{"type": "hotkeys.register", "id": "abc-145", "seq": 23, "hotkey_id": "my-hotkey-1", "keys": ["Control_L", "Alt_L", "t"]}
```

#### `hotkeys.unregister`

Remove a previously registered hotkey by its ID.

```json
{"type": "hotkeys.unregister", "id": "abc-146", "seq": 24, "hotkey_id": "my-hotkey-1"}
```

#### `monitor.list`

List connected monitors/display outputs.

```json
{"type": "monitor.list", "id": "abc-147", "seq": 25}
```

#### `audio.list_sinks`

List audio output sinks.

```json
{"type": "audio.list_sinks", "id": "abc-148", "seq": 26}
```

#### `audio.set_sink_volume`

Set volume on a sink.

```json
{"type": "audio.set_sink_volume", "id": "abc-149", "seq": 27, "sink_id": 0, "volume": 0.75}
```

#### `process.list`

List running processes (useful for detecting app state).

```json
{"type": "process.list", "id": "abc-150", "seq": 28}
```

#### `files.watch`

Watch a path (file or directory) for changes. Recursive by default.

```json
{"type": "files.watch", "id": "abc-151", "seq": 29, "path": "/home/user/projects", "recursive": true, "patterns": ["*.rs", "*.toml"]}
```

Optional `patterns` — glob filters to narrow what changes trigger events. Omit for all changes.

#### `files.unwatch`

Stop watching a previously watched path.

```json
{"type": "files.unwatch", "id": "abc-152", "seq": 30, "path": "/home/user/projects"}
```

#### `files.search`

Search for files by name pattern. Returns results immediately (not a subscription).

```json
{"type": "files.search", "id": "abc-153", "seq": 31, "pattern": "*.md", "root": "/home/user", "max_results": 20}
```

Optional: `root` defaults to home directory. `max_results` defaults to 50.

#### `process.start`

Launch a process. Optional: working directory, environment, capture output.

```json
{"type": "process.start", "id": "abc-154", "seq": 32, "command": ["code", "."], "workdir": "/home/user/projects", "env": {"TERM": "xterm-256color"}}
```

For long-lived processes, daemon emits `process.stdout`, `process.stderr`, and `process.exited` events. Client gets a `process_id` in the response.

#### `system.power`

Power management actions.

```json
{"type": "system.power", "id": "abc-155", "seq": 33, "action": "suspend"}
```

Available actions: `suspend`, `hibernate`, `reboot`, `shutdown`, `lock`, `logout`.

#### `system.battery`

Get current battery status (device batteries and UPS).

```json
{"type": "system.battery", "id": "abc-156", "seq": 34}
```

#### `network.status`

Get current network connectivity status.

```json
{"type": "network.status", "id": "abc-157", "seq": 35}
```

#### `network.interfaces`

List network interfaces and their state.

```json
{"type": "network.interfaces", "id": "abc-158", "seq": 36}
```

#### `network.wifi.scan`

Scan for available Wi-Fi networks.

```json
{"type": "network.wifi.scan", "id": "abc-159", "seq": 37}
```

#### `network.wifi.connect`

Connect to a Wi-Fi network.

```json
{"type": "network.wifi.connect", "id": "abc-160", "seq": 38, "ssid": "MyNetwork", "password": "hunter2"}
```

#### `bluetooth.list`

List known Bluetooth devices and their connection status.

```json
{"type": "bluetooth.list", "id": "abc-161", "seq": 39}
```

#### `bluetooth.scan`

Start scanning for discoverable Bluetooth devices. Emits `bluetooth.device_found` events.

```json
{"type": "bluetooth.scan", "id": "abc-162", "seq": 40, "duration": 10}
```

`duration` in seconds. After `duration`, scan stops automatically. Omitting `duration` keeps scanning until `bluetooth.scan_stop` is sent.

#### `bluetooth.scan_stop`

Stop an active Bluetooth scan.

```json
{"type": "bluetooth.scan_stop", "id": "abc-163", "seq": 41}
```

#### `bluetooth.connect`

Connect to a paired Bluetooth device.

```json
{"type": "bluetooth.connect", "id": "abc-164", "seq": 42, "address": "AA:BB:CC:DD:EE:FF"}
```

#### `bluetooth.disconnect`

Disconnect a Bluetooth device.

```json
{"type": "bluetooth.disconnect", "id": "abc-165", "seq": 43, "address": "AA:BB:CC:DD:EE:FF"}
```

#### `bluetooth.pair`

Initiate pairing with a discoverable device.

```json
{"type": "bluetooth.pair", "id": "abc-166", "seq": 44, "address": "AA:BB:CC:DD:EE:FF"}
```

#### `bluetooth.forget`

Remove a paired device.

```json
{"type": "bluetooth.forget", "id": "abc-167", "seq": 45, "address": "AA:BB:CC:DD:EE:FF"}
```

#### `location.get`

Get current location (reverse geocode from network).

```json
{"type": "location.get", "id": "abc-168", "seq": 46}
```

Returns: latitude, longitude, accuracy, approximate address.

#### `disconnect`

Gracefully close the connection. Daemon cleans up subscriptions and sends final `disconnected` event.

```json
{"type": "disconnect", "id": "abc-169", "seq": 47}
```

---

### 4.2 Server → Client (Responses)

Every action gets a response. Success:

```json
{"type": "response", "id": "abc-126", "seq": 4, "status": "ok", "data": { ... }}
```

Error:

```json
{"type": "response", "id": "abc-127", "seq": 5, "status": "error", "error": {"code": "WINDOW_NOT_FOUND", "message": "No window with id 0x02400004"}}
```

Error codes:
| Code | Meaning |
|------|---------|
| `UNKNOWN_ACTION` | Action type not recognized |
| `INVALID_PARAMS` | Missing or invalid parameters |
| `WINDOW_NOT_FOUND` | Window ID doesn't exist |
| `WORKSPACE_NOT_FOUND` | Workspace doesn't exist |
| `NOT_SUPPORTED` | Feature not available on this desktop |
| `INTERNAL_ERROR` | Something broke |
| `RATE_LIMITED` | Too many requests |
| `NOT_SUBSCRIBED` | Can't unsubscribe from something you didn't subscribe to |
| `DUPLICATE_HOTKEY` | Hotkey ID already registered |
| `PATH_NOT_FOUND` | File or directory doesn't exist |
| `PATH_NOT_WATCHED` | Can't unwatch something not being watched |
| `PERMISSION_DENIED` | Access denied for this operation |
| `BLUETOOTH_UNAVAILABLE` | No Bluetooth adapter or service |
| `DEVICE_NOT_FOUND` | Bluetooth device address not found |
| `DEVICE_NOT_CONNECTED` | Cannot disconnect something not connected |
| `SCAN_ALREADY_ACTIVE` | Bluetooth scan already in progress |
| `WIFI_UNAVAILABLE` | No Wi-Fi hardware or service |
| `NETWORK_UNREACHABLE` | Cannot reach the network |
| `BATTERY_UNAVAILABLE` | No battery detected (desktop may not have one) |
| `POWER_ACTION_DENIED` | Power action blocked by policy or polkit |
| `PROCESS_ALREADY_RUNNING` | Process ID already tracked |
| `PROCESS_NOT_FOUND` | Process ID not tracked |
| `LOCATION_UNAVAILABLE` | Cannot determine location |

The special `pong` response:

```json
{"type": "pong", "id": "abc-123", "seq": 1}
```

#### Response Data Shapes

**`windows.list`** response:
```json
{
  "type": "response",
  "id": "abc-126",
  "status": "ok",
  "data": {
    "windows": [
      {
        "id": "0x02400004",
        "title": "Firefox",
        "app_id": "firefox",
        "workspace_id": 1,
        "is_focused": true,
        "is_minimized": false,
        "geometry": {"x": 50, "y": 50, "width": 1920, "height": 1040},
        "pid": 1234
      }
    ]
  }
}
```

**`screenshot`** response:
```json
{
  "type": "response",
  "id": "abc-140",
  "status": "ok",
  "data": {
    "path": "/tmp/deskbrid/screenshot_1712345678.png",
    "width": 1920,
    "height": 1080,
    "format": "png"
  }
}
```

**`system.info`** response:
```json
{
  "type": "response",
  "id": "abc-143",
  "status": "ok",
  "data": {
    "desktop": "GNOME",
    "desktop_version": "46",
    "compositor": "mutter",
    "session_type": "wayland",
    "monitors": [{"id": 0, "name": "eDP-1", "width": 1920, "height": 1080, "scale": 1.0, "primary": true}],
    "workspace_count": 4,
    "current_workspace": 1,
    "idle_seconds": 120
  }
}
```

**`clipboard.read`** response:
```json
{
  "type": "response",
  "id": "abc-138",
  "status": "ok",
  "data": {
    "text": "copied content",
    "mime_types": ["text/plain;charset=utf-8"]
  }
}
```

---

### 4.3 Server → Client (Events)

Events are pushed to subscribed clients. Same envelope, different type.

#### `window.focus_changed`

```json
{
  "type": "window.focus_changed",
  "id": "<server-generated-uuid>",
  "seq": 1001,
  "data": {
    "window_id": "0x02400004",
    "title": "Firefox",
    "app_id": "firefox"
  }
}
```

#### `window.opened`

```json
{
  "type": "window.opened",
  "id": "...",
  "seq": 1002,
  "data": {
    "window_id": "0x02400005",
    "title": "Terminal",
    "app_id": "gnome-terminal",
    "workspace_id": 1
  }
}
```

#### `window.closed`

```json
{
  "type": "window.closed",
  "id": "...",
  "seq": 1003,
  "data": {
    "window_id": "0x02400005",
    "app_id": "gnome-terminal"
  }
}
```

#### `window.minimized` / `window.unminimized`

```json
{
  "type": "window.minimized",
  "id": "...",
  "seq": 1004,
  "data": {
    "window_id": "0x02400004",
    "title": "Firefox",
    "app_id": "firefox"
  }
}
```

#### `window.workspace_changed`

Window moved to another workspace.

```json
{
  "type": "window.workspace_changed",
  "id": "...",
  "seq": 1005,
  "data": {
    "window_id": "0x02400004",
    "app_id": "firefox",
    "from_workspace": 1,
    "to_workspace": 3
  }
}
```

#### `window.title_changed`

```json
{
  "type": "window.title_changed",
  "id": "...",
  "seq": 1006,
  "data": {
    "window_id": "0x02400004",
    "app_id": "firefox",
    "title": "Deskbrid Protocol Spec — Mozilla Firefox"
  }
}
```

#### `clipboard.changed`

```json
{
  "type": "clipboard.changed",
  "id": "...",
  "seq": 1010,
  "data": {
    "text": "new clipboard content",
    "mime_types": ["text/plain;charset=utf-8"]
  }
}
```

#### `notification.received`

```json
{
  "type": "notification.received",
  "id": "...",
  "seq": 1015,
  "data": {
    "notification_id": 42,
    "app_name": "Slack",
    "app_icon": "slack",
    "title": "New message from @alice",
    "body": "Hey, can you review the PR?",
    "urgency": "normal",
    "timestamp": "2026-05-08T23:30:00Z",
    "actions": [{"id": "default", "label": "View in Slack"}]
  }
}
```

#### `notification.closed`

```json
{
  "type": "notification.closed",
  "id": "...",
  "seq": 1016,
  "data": {
    "notification_id": 42,
    "reason": "dismissed"
  }
}
```

Reasons: `dismissed`, `timed_out`, `action_invoked`, `closed_by_agent`.

#### `input.hotkey`

Fired when a registered global hotkey is pressed.

```json
{
  "type": "input.hotkey",
  "id": "...",
  "seq": 1020,
  "data": {
    "hotkey_id": "my-hotkey-1",
    "keys": ["Control_L", "Alt_L", "t"]
  }
}
```

#### `workspace.changed`

Current active workspace changed.

```json
{
  "type": "workspace.changed",
  "id": "...",
  "seq": 1025,
  "data": {
    "workspace_id": 2,
    "name": "Workspace 2"
  }
}
```

#### `monitor.added` / `monitor.removed`

```json
{
  "type": "monitor.added",
  "id": "...",
  "seq": 1030,
  "data": {
    "monitor_id": 1,
    "name": "HDMI-1",
    "width": 1920,
    "height": 1080
  }
}
```

#### `system.idle_changed`

```json
{
  "type": "system.idle_changed",
  "id": "...",
  "seq": 1035,
  "data": {
    "idle": true,
    "idle_seconds": 305
  }
}
```

#### `system.screensaver_changed`

Lock screen / screensaver state changes.

```json
{
  "type": "system.screensaver_changed",
  "id": "...",
  "seq": 1036,
  "data": {
    "active": true
  }
}
```

#### `audio.sink_changed`

Audio sink volume/mute changes.

```json
{
  "type": "audio.sink_changed",
  "id": "...",
  "seq": 1040,
  "data": {
    "sink_id": 0,
    "name": "Built-in Audio",
    "volume": 0.5,
    "muted": false
  }
}

#### `files.changed`

Fired when a watched file or directory changes.

```json
{
  "type": "files.changed",
  "id": "...",
  "seq": 1050,
  "data": {
    "path": "/home/user/projects/src/main.rs",
    "kind": "modified",
    "is_directory": false
  }
}
```

`kind`: `created`, `modified`, `deleted`, `renamed`. When `renamed`, additional fields `from_path` and `to_path` are included.

#### `files.renamed`

```json
{
  "type": "files.renamed",
  "id": "...",
  "seq": 1051,
  "data": {
    "from_path": "/home/user/projects/src/old.rs",
    "to_path": "/home/user/projects/src/new.rs"
  }
}
```

#### `process.stdout` / `process.stderr`

Emitted for processes started with `process.start` that produce output.

```json
{
  "type": "process.stdout",
  "id": "...",
  "seq": 1060,
  "data": {
    "process_id": "proc-abc-123",
    "data": "[INFO] Build started...",
    "pid": 12345
  }
}
```

#### `process.exited`

Fired when a tracked process exits.

```json
{
  "type": "process.exited",
  "id": "...",
  "seq": 1061,
  "data": {
    "process_id": "proc-abc-123",
    "pid": 12345,
    "exit_code": 0,
    "signal": null
  }
}
```

#### `network.interface_changed`

Network interface state change (up/down, IP change).

```json
{
  "type": "network.interface_changed",
  "id": "...",
  "seq": 1070,
  "data": {
    "interface": "wlp2s0",
    "state": "up",
    "ipv4": "192.168.1.108",
    "ipv6": "fe80::..."
  }
}
```

#### `network.connectivity_changed`

Overall internet connectivity changed.

```json
{
  "type": "network.connectivity_changed",
  "id": "...",
  "seq": 1071,
  "data": {
    "online": true,
    "type": "wifi"
  }
}
```

`type`: `wifi`, `ethernet`, `cellular`, `none`.

#### `network.wifi.network_found`

Fired during a Wi-Fi scan when networks are discovered.

```json
{
  "type": "network.wifi.network_found",
  "id": "...",
  "seq": 1072,
  "data": {
    "ssid": "HomeNetwork",
    "strength": 85,
    "secured": true,
    "frequency": 5180
  }
}
```

#### `bluetooth.device_found`

Fired during Bluetooth discovery.

```json
{
  "type": "bluetooth.device_found",
  "id": "...",
  "seq": 1080,
  "data": {
    "address": "AA:BB:CC:DD:EE:FF",
    "name": "Sony WH-1000XM5",
    "rssi": -65,
    "paired": false,
    "connected": false
  }
}
```

#### `bluetooth.device_connected` / `bluetooth.device_disconnected`

```json
{
  "type": "bluetooth.device_connected",
  "id": "...",
  "seq": 1081,
  "data": {
    "address": "AA:BB:CC:DD:EE:FF",
    "name": "Sony WH-1000XM5"
  }
}
```

#### `bluetooth.device_paired` / `bluetooth.device_unpaired`

```json
{
  "type": "bluetooth.device_paired",
  "id": "...",
  "seq": 1082,
  "data": {
    "address": "AA:BB:CC:DD:EE:FF",
    "name": "Sony WH-1000XM5",
    "bonded": true
  }
}
```

#### `bluetooth.scan_complete`

Fired when a timed Bluetooth scan finishes or `bluetooth.scan_stop` is called.

```json
{
  "type": "bluetooth.scan_complete",
  "id": "...",
  "seq": 1083,
  "data": {
    "devices_found": 3
  }
}
```

#### `power.battery_changed`

Battery level or charging status changed.

```json
{
  "type": "power.battery_changed",
  "id": "...",
  "seq": 1090,
  "data": {
    "source": "BAT0",
    "percentage": 73.5,
    "state": "discharging",
    "time_remaining_minutes": 187
  }
}
```

`state`: `charging`, `discharging`, `full`, `not_charging`, `unknown`.

#### `power.cable_changed`

Power cable plugged/unplugged.

```json
{
  "type": "power.cable_changed",
  "id": "...",
  "seq": 1091,
  "data": {
    "plugged_in": true
  }
}
```

#### `power.suspend` / `power.wake`

System suspend/resume lifecycle events.

```json
{"type": "power.suspend", "id": "...", "seq": 1092, "data": {}}
```

#### `power.screensaver_changed` (alias)

Alias for `system.screensaver_changed`. Both fire.

---

#### `connected`

Sent immediately after a client connects and before any other messages.

```json
{"type": "connected", "id": "<server-uuid>", "seq": 0, "data": {"version": "2.0.0", "protocol": "deskbrid-v2"}}
```

#### `disconnected`

Sent after the server processes a `disconnect` action, before closing the connection.

```json
{"type": "disconnected", "id": "abc-151", "seq": 29}
```

---

## 5. Error Semantics

- **Unknown action**: server responds with `{"status": "error", "error": {"code": "UNKNOWN_ACTION", ...}}`
- **Invalid params**: server responds with `{"status": "error", "error": {"code": "INVALID_PARAMS", ...}}`
- **Malformed JSON**: server responds with a parse error message, but the connection stays open
- **Backend unavailable**: e.g. `{"code": "NOT_SUPPORTED", "message": "screenshots not available on this desktop"}`
- **Rate limiting**: if > 100 actions/second, server may respond with `RATE_LIMITED`
- **Unparseable line**: server logs a warning and skips the line. One bad message doesn't kill the connection

---

## 6. Connection Lifecycle

```
  Client                          Server
    |                               |
    |--- connect (unix socket) ---->|
    |<--- {"type":"connected"} -----|
    |--- {"type":"subscribe"...} -->|  subscriptions registered
    |--- {"type":"windows.list"} -->|
    |<-- {"type":"response",...} ---|
    |<-- {"type":"window.focus..."} |  push event (async)
    |<-- {"type":"clipboard....."}  |  push event (async)
    |--- {"type":"disconnect"} ---->|
    |<-- {"type":"disconnected"} ---|
    |       connection closed       |
```

---

## 7. Implementation Notes

- **Socket path**: `$XDG_RUNTIME_DIR/deskbrid.sock` or `/tmp/deskbrid.sock` fallback
- **Line buffer**: daemon reads lines with a 1 MiB max to prevent OOM from garbage input
- **Write buffer**: daemon buffers events per-client. If a client is slow to read, events older than 30s are dropped. Daemon logs a warning.
- **Keepalive**: daemon sends `ping` every 30s if no traffic. Client can ignore it.
- **Reconnection**: clients should reconnect with exponential backoff (1s, 2s, 4s, max 30s). Subscriptions don't persist across disconnects.
- **Concurrency**: daemon uses tokio. Each client gets a dedicated task for reads and a channel for writes.
