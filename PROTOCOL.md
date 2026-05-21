# Deskbrid Protocol v2

The agent-native Linux desktop protocol. JSON-over-Unix-socket, newline-delimited, bidirectional. Any agent can connect and get full desktop control.

## Transport

- **Socket**: `$XDG_RUNTIME_DIR/deskbrid.sock` (typically `/run/user/1000/deskbrid.sock`)
- **Protocol**: NDJSON (`\n`-delimited), bidirectional
- **Encoding**: UTF-8
- **Max message size**: 1 MiB

## Message Format

### Client → Server (requests)

Every message has a `type` field (the action name) and an `id` field (correlation token — echoed back in the response):

```json
{"type": "windows.list", "id": "req-1"}
{"type": "windows.focus", "id": "req-2", "window_id": "0x3a0000b"}
{"type": "input.keyboard", "id": "req-3", "action": "type", "text": "hello\n"}
{"type": "subscribe", "id": "req-4", "events": ["file.*", "clipboard"]}
```

### Server → Client (responses)

Responses echo the `id` and include a `seq` number (per-connection monotonic counter):

```json
{"type": "response", "id": "req-1", "seq": 1, "status": "ok", "data": [...]}
{"type": "response", "id": "req-2", "seq": 2, "status": "error", "error": {"code": "NOT_FOUND", "message": "window not found"}}
```

### Server → Client (events)

Events are pushed asynchronously to subscribed clients:

```json
{"type": "event", "id": "file.created", "data": {"path": "/tmp/test.txt"}}
```

## Connection

- **Connect**: Open socket to `$XDG_RUNTIME_DIR/deskbrid.sock`
- **Handshake**: Daemon immediately sends a `connected` message; clients **must** wait for this before sending commands
- **Subscribe**: Request events you want pushed
- **Send**: Action messages receive response messages (correlated by `id`)
- **Ping**: `{"type": "ping", "id": "..."}` — responds with `pong`
- **Disconnect**: `{"type": "disconnect", "id": "..."}` or close the socket

### Handshake Message

```json
{"type": "connected", "id": "server", "seq": 0,
 "data": {"version": "0.6.0", "protocol": "deskbrid-v2", "uid": 1000}}
```

`uid` is the peer credential (`SO_PEERCRED`) of the connecting process, used for permission evaluation.

## Actions

All action names use dot notation: `domain.action`. Every action is sent with `"type"` set to the action name and `"id"` set to a client-chosen correlation token.

### Windows & Workspaces

| Action | Params | Description |
|--------|--------|-------------|
| `windows.list` | — | List all open windows |
| `windows.focus` | `window_id` (string) | Focus a window by ID |
| `windows.get` | `window_id` (string) | Get window details |
| `workspaces.list` | — | List workspaces |
| `workspaces.switch` | `workspace_id` (number) | Switch to workspace |
| `workspaces.move_window` | `window_id` (string), `workspace_id` (number), `follow` (bool, optional) | Move window to workspace |

```json
→ {"type": "windows.list", "id": "1"}
← {"type": "response", "id": "1", "seq": 1, "status": "ok", "data": [
    {"id": "3", "title": "README.md — VS Code", "app_id": "code",
     "workspace_id": 0, "is_focused": true, "is_minimized": false,
     "geometry": {"x": 0, "y": 0, "width": 1920, "height": 1080}, "pid": 1234}
  ]}
```

### Input

| Action | Params | Description |
|--------|--------|-------------|
| `input.keyboard` | `action`: `"type"` / `"key"` / `"combo"` | Keyboard input |
| `input.mouse` | `action`: `"move"` / `"click"` / `"scroll"` | Mouse input |

```json
→ {"type": "input.keyboard", "id": "2", "action": "type", "text": "git push\n"}
→ {"type": "input.keyboard", "id": "3", "action": "combo", "keys": ["ctrl", "shift", "t"]}
→ {"type": "input.keyboard", "id": "4", "action": "key", "key": "Return"}
→ {"type": "input.mouse", "id": "5", "action": "move", "x": 500, "y": 300}
→ {"type": "input.mouse", "id": "6", "action": "click", "button": "right"}
→ {"type": "input.mouse", "id": "7", "action": "scroll", "dx": 0, "dy": -3}
```

### Clipboard

| Action | Params | Description |
|--------|--------|-------------|
| `clipboard.read` | — | Read clipboard |
| `clipboard.write` | `text` (string) | Write to clipboard |

```json
→ {"type": "clipboard.read", "id": "8"}
← {"type": "response", "id": "8", "seq": 8, "status": "ok", "data": {"text": "copied content"}}
```

### Screenshot

| Action | Params | Description |
|--------|--------|-------------|
| `screenshot` | `monitor` (number, optional), `region` (object, optional), `window_id` (string, optional) | Capture screen |
| `screenshot.ocr` | `path` (string, optional), `language` (string, optional), `psm` (number, optional), `bounding_boxes` (bool, optional) | OCR an existing screenshot or capture a fresh one |

```json
→ {"type": "screenshot", "id": "9", "monitor": 0}
← {"type": "response", "id": "9", "seq": 9, "status": "ok",
   "data": {"path": "/tmp/deskbrid-screenshot-1715000000.png", "width": 1920, "height": 1080, "format": "png"}}
```

### Notifications

| Action | Params | Description |
|--------|--------|-------------|
| `notification.send` | `app_name` (string), `title` (string), `body` (string), `urgency` (`"low"`/`"normal"`/`"critical"`) | Send notification |
| `notification.close` | `notification_id` (number) | Close notification |

```json
→ {"type": "notification.send", "id": "10", "title": "Build complete", "body": "Exit code 0", "urgency": "normal"}
← {"type": "response", "id": "10", "seq": 10, "status": "ok", "data": {"notification_id": 42}}
```

### System

| Action | Params | Description |
|--------|--------|-------------|
| `system.info` | — | Desktop info, monitors, capabilities |
| `system.idle` | — | Seconds since last user input |
| `system.battery` | — | Battery status |
| `system.power` | `action`: `"suspend"` / `"hibernate"` / `"shutdown"` / `"reboot"` / `"lock"` / `"logout"` | Power actions |
| `system.capabilities` | — | Detailed capability matrix per backend |
| `system.health` | — | Dependency health check with remediation suggestions |
| `system.remediate` | — | Auto-fix missing dependencies |
| `system.normalize_coords` | `x` (number), `y` (number), `from` (object) | Convert monitor-relative coords to absolute |
| `wait.for` | `condition` (string), `params` (object), `timeout_ms` (number), `interval_ms` (number, optional) | Wait for windows, clipboard, process, file, idle, or screenshot-stable conditions |

### Network

| Action | Params | Description |
|--------|--------|-------------|
| `network.status` | — | Online/offline status |
| `network.interfaces` | — | List interfaces with IPs |
| `network.wifi.scan` | — | Scan WiFi networks |
| `network.wifi.connect` | `ssid` (string), `password` (string, optional) | Connect to WiFi |

### Bluetooth

| Action | Params | Description |
|--------|--------|-------------|
| `bluetooth.list` | — | List known devices |
| `bluetooth.scan` | `duration` (number, optional) | Start device discovery |
| `bluetooth.scan_stop` | — | Stop discovery |
| `bluetooth.connect` | `address` (string) | Connect to device |
| `bluetooth.disconnect` | `address` (string) | Disconnect device |
| `bluetooth.pair` | `address` (string) | Pair with a device |
| `bluetooth.forget` | `address` (string) | Remove a paired device |

### Audio

| Action | Params | Description |
|--------|--------|-------------|
| `audio.list_sinks` | — | List audio output sinks |
| `audio.set_sink_volume` | `sink_id` (number), `volume` (number, 0.0–1.0) | Set sink volume |

### Files

| Action | Params | Description |
|--------|--------|-------------|
| `files.search` | `pattern` (string), `root` (string, optional), `max_results` (number, optional) | Search files by name |
| `files.watch` | `path` (string), `recursive` (bool, optional), `patterns` (string[], optional) | Watch for file changes |
| `files.unwatch` | `path` (string) | Stop watching a path |

### Process

| Action | Params | Description |
|--------|--------|-------------|
| `process.list` | — | List running processes |
| `process.start` | `command` (string), `args` (string[]), `workdir` (string, optional), `env` (object, optional) | Start a process |
| `process.stop` | `pid` (number), `signal` (string, optional: `"SIGTERM"`/`"SIGKILL"`) | Stop a process |
| `process.signal` | `pid` (number), `signal` (string) | Send arbitrary signal |
| `process.exists` | `pid` (number) | Check if PID exists |
| `process.wait` | `pid` (number), `timeout` (number, optional, seconds) | Wait for process exit |

### Terminal / PTY

| Action | Params | Description |
|--------|--------|-------------|
| `terminal.create` | `shell` (string, optional), `cwd` (string, optional), `env` (object, optional), `rows`/`cols` (number, optional) | Create an interactive pseudo-terminal session |
| `terminal.write` | `terminal_id` (string), `input` (string) | Write text/control bytes to the PTY |
| `terminal.read` | `terminal_id` (string), `max_bytes` (number, optional), `flush` (bool, default true) | Read buffered PTY output |
| `terminal.resize` | `terminal_id` (string), `rows` (number), `cols` (number) | Resize the PTY and signal the shell |
| `terminal.list` | — | List active PTY sessions |
| `terminal.kill` | `terminal_id` (string), `signal` (string, optional) | Signal and remove a PTY session |

### Hotkeys

| Action | Params | Description |
|--------|--------|-------------|
| `hotkeys.register` | `hotkey_id` (string), `keys` (string[]) | Register a hotkey combo |
| `hotkeys.unregister` | `hotkey_id` (string) | Unregister a hotkey |

### Monitor & Location

| Action | Params | Description |
|--------|--------|-------------|
| `monitor.list` | — | List connected displays |
| `location.get` | — | Get geolocation |

### Capabilities

| Action | Params | Description |
|--------|--------|-------------|
| `capabilities.list` | — | List all supported/unsupported actions for current backend |

## Events

Subscribe with `{"type": "subscribe", "id": "...", "events": ["file.*", "window.focused"]}`.
Unsubscribe with `{"type": "unsubscribe", "id": "...", "events": ["file.*"]}`.

### Event Types

| Pattern | Description |
|---------|-------------|
| `file.created` | File created at watched path |
| `file.modified` | File modified at watched path |
| `file.deleted` | File deleted from watched path |
| `file.renamed` | File renamed at watched path |
| `window.focused` | Window focus changed (future — requires extension support) |
| `workspace.changed` | Active workspace changed (future) |
| `workspace.window_moved` | Window moved between workspaces (future) |
| `*` | All events |

Glob matching is supported: `file.*` matches `file.created`, `file.modified`, etc.

### Event Envelope

```json
{"type": "event", "id": "file.created", "data": {"event": "file.created", "path": "/tmp/test.txt", "timestamp": 1715000000}}
```

## Permissions

The daemon can restrict actions by caller UID using a TOML config file.

### Permission file location

```
~/.config/deskbrid/permissions.toml
```

### Permission file format

```toml
# Allow everything to UID 1000
[permissions.1000]
allow = ["*"]

# Restrict a secondary user to read-only operations
[permissions.1001]
allow = ["windows.*", "workspaces.list", "system.*"]
```

### Behavior

| Scenario | Result |
|----------|--------|
| No file | All actions allowed (backward compatible) |
| Empty file | All actions denied for all UIDs |
| Missing UID | All actions denied for that UID |
| Glob patterns | `*`, `windows.*`, `input.keyboard`, etc. |
| Deny override | Deny always takes precedence over allow |

### Permission Denied Response

```json
{"type": "response", "id": "req-1", "seq": 1, "status": "error",
 "error": {"code": "PERMISSION_DENIED", "message": "Caller UID 1001 not allowed: input.keyboard"}}
```

### Permission Names

```
windows.list, windows.focus, windows.get
workspaces.list, workspaces.switch, workspaces.move_window
input.keyboard, input.mouse
clipboard.read, clipboard.write
screenshot
notifications.send, notifications.close
system.info, system.idle, system.power, system.battery, system.capabilities, system.health
network.status, network.interfaces, network.wifi_scan, network.wifi_connect
bluetooth.list, bluetooth.scan, bluetooth.scan_stop, bluetooth.connect, bluetooth.disconnect, bluetooth.pair, bluetooth.forget
files.watch, files.unwatch, files.search
process.list, process.start, process.stop, process.signal, process.exists, process.wait
hotkeys.register, hotkeys.unregister
audio.list_sinks, audio.set_sink_volume
monitor.list, location.get
capabilities.list
```

## Error Handling

Errors return `status: "error"` with a code and message:

```json
{"type": "response", "id": "req-1", "seq": 1, "status": "error",
 "error": {"code": "NOT_FOUND", "message": "window not found: 0xabc"}}
```

### Error Codes

| Code | Meaning |
|------|---------|
| `INVALID_PARAMS` | Malformed JSON or unknown action type |
| `NOT_FOUND` | Requested resource not found (window, device, etc.) |
| `NOT_SUPPORTED` | Action not supported by current backend |
| `INTERNAL_ERROR` | Backend operation failed |
| `PERMISSION_DENIED` | Caller UID not allowed for the requested action |

## System Control Messages

These are not actions but metaprotocol commands:

| Type | Direction | Description |
|------|-----------|-------------|
| `ping` | Client → Daemon | Liveness check; responds with `pong` |
| `disconnect` | Client → Daemon | Graceful close; responds with `disconnected` |
| `subscribe` | Client → Daemon | Register event pattern subscriptions |
| `unsubscribe` | Client → Daemon | Remove event pattern subscriptions |

---

*Protocol version: 2. Evolving with the desktop.*
