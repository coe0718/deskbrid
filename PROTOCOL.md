# Deskbrid Protocol v0.1

The agent-native Linux desktop protocol. JSON-over-Unix-socket, newline-delimited, bidirectional. Any agent can connect and get full desktop control.

## Transport

- **Socket**: `$XDG_RUNTIME_DIR/deskbrid.sock` (typically `/run/user/1000/deskbrid.sock`)
- **Protocol**: NDJSON (`\n`-delimited), bidirectional
- **Encoding**: UTF-8
- **Max message size**: 1 MiB

## Message Format

### Client → Server (requests)

Every message has a `type` field (the action name) and an `id` field:

```json
{"type": "windows.list", "id": "req-1"}
{"type": "windows.focus", "id": "req-2", "window_id": "0x3a0000b"}
{"type": "input.keyboard", "id": "req-3", "action": "type", "text": "hello\n"}
{"type": "subscribe", "id": "req-4", "events": ["file.*", "clipboard"]}
```

### Server → Client (responses)

Responses echo the `id` and include a `seq` number:

```json
{"type": "response", "id": "req-1", "seq": 1, "status": "ok", "data": [...]}
{"type": "response", "id": "req-2", "seq": 2, "status": "error", "error": {"code": "NOT_FOUND", "message": "window not found"}}
```

### Server → Client (events)

Events are pushed asynchronously to subscribed clients:

```json
{"type": "event", "id": "file.created", "data": {"path": "/tmp/test.txt"}}
```

## Actions

All action names use dot notation: `domain.action`.

### Windows and Workspaces

| Action | Type | Params | Description |
|---|---|---|---|
| `windows.list` | `"windows.list"` | — | List all open windows |
| `windows.focus` | `"windows.focus"` | `window_id` | Focus a window by ID |
| `windows.get` | `"windows.get"` | `window_id` | Get window details |
| `workspaces.list` | `"workspaces.list"` | — | List workspaces |
| `workspaces.switch` | `"workspaces.switch"` | `workspace_id` | Switch to workspace |
| `workspaces.move_window` | `"workspaces.move_window"` | `window_id`, `workspace_id`, `follow` | Move window to workspace |

```json
→ {"type": "windows.list", "id": "1"}
← {"type": "response", "id": "1", "seq": 1, "status": "ok", "data": [
    {"id": "3", "title": "README.md — VS Code", "app_id": "code", "workspace_id": 0, "is_focused": true}
  ]}
```

### Input

| Action | Type | Params | Description |
|---|---|---|---|
| `input.keyboard` | `"input.keyboard"` | `action`: "type", "key", or "combo" | Keyboard input |
| `input.mouse` | `"input.mouse"` | `action`: "move", "click", "scroll" | Mouse input |

```json
→ {"type": "input.keyboard", "id": "2", "action": "type", "text": "git push\n"}
→ {"type": "input.keyboard", "id": "3", "action": "combo", "keys": ["ctrl", "shift", "t"]}
→ {"type": "input.mouse", "id": "4", "action": "click", "x": 500, "y": 300, "button": "left"}
→ {"type": "input.mouse", "id": "5", "action": "scroll", "dx": 0, "dy": -3}
```

### Clipboard, Screenshot, Notifications

| Action | Type | Params | Description |
|---|---|---|---|
| `clipboard.read` | `"clipboard.read"` | — | Read clipboard |
| `clipboard.write` | `"clipboard.write"` | `text` | Write to clipboard |
| `screenshot` | `"screenshot"` | `monitor`, `region`, `window_id` (optional) | Capture screen |
| `notification.send` | `"notification.send"` | `app_name`, `title`, `body`, `urgency` | Send notification |
| `notification.close` | `"notification.close"` | `notification_id` | Close notification |

```json
→ {"type": "screenshot", "id": "6", "monitor": 0}
← {"type": "response", "id": "6", "status": "ok", "data": {"path": "/tmp/screenshot.png", "width": 1920, "height": 1080}}
```

### System

| Action | Type | Params | Description |
|---|---|---|---|
| `system.info` | `"system.info"` | — | Desktop info, monitors, capabilities |
| `system.idle` | `"system.idle"` | — | Seconds since last input |
| `system.power` | `"system.power"` | `action`: "suspend", "hibernate", "shutdown", "reboot", "lock", "logout" | Power actions |
| `system.battery` | `"system.battery"` | — | Battery status |

### Network

| Action | Type | Params | Description |
|---|---|---|---|
| `network.status` | `"network.status"` | — | Online/offline status |
| `network.interfaces` | `"network.interfaces"` | — | List interfaces with IPs |
| `network.wifi.scan` | `"network.wifi.scan"` | — | Scan WiFi networks |
| `network.wifi.connect` | `"network.wifi.connect"` | `ssid`, `password` (optional) | Connect to WiFi |

### Bluetooth

| Action | Type | Params | Description |
|---|---|---|---|
| `bluetooth.list` | `"bluetooth.list"` | — | List known devices |
| `bluetooth.scan` | `"bluetooth.scan"` | `duration` (optional) | Start discovery |
| `bluetooth.stop_scan` | `"bluetooth.scan_stop"` | — | Stop discovery |
| `bluetooth.connect` | `"bluetooth.connect"` | `address` | Connect to device |
| `bluetooth.disconnect` | `"bluetooth.disconnect"` | `address` | Disconnect device |

### Audio, Files, Monitor, Process, Location

| Action | Type | Params | Description |
|---|---|---|---|
| `audio.list_sinks` | `"audio.list_sinks"` | — | List audio sinks |
| `audio.set_sink_volume` | `"audio.set_sink_volume"` | `sink_id`, `volume` | Set volume (0.0-1.0) |
| `files.search` | `"files.search"` | `pattern`, `root`, `max_results` | Search files |
| `files.watch` | `"files.watch"` | `path`, `recursive`, `patterns` | Watch for file changes |
| `files.unwatch` | `"files.unwatch"` | `path` | Stop watching |
| `monitor.list` | `"monitor.list"` | — | List displays |
| `process.list` | `"process.list"` | — | List running processes |
| `process.start` | `"process.start"` | `command`, `workdir`, `env` | Start a process |
| `location.get` | `"location.get"` | — | Get geolocation |

## Events

Subscribe with `{"type": "subscribe", "id": "...", "events": ["file.*", "clipboard"]}`.

| Pattern | Description |
|---|---|
| `file.*` | file.created, file.modified, file.deleted |
| `file.created` | File creation events |
| `file.modified` | File modification events |
| `file.deleted` | File deletion events |
| `*` | All events |

Event format:
```json
{"type": "event", "id": "file.created", "data": {"path": "/tmp/test.txt", "kind": "created"}}
```

## Error Handling

Errors return `status: "error"` with a code and message:

```json
{"type": "response", "id": "req-1", "seq": 1, "status": "error", "error": {"code": "NOT_FOUND", "message": "window not found: firefox"}}
```

Error codes: `INVALID_PARAMS`, `NOT_FOUND`, `NOT_SUPPORTED`, `INTERNAL_ERROR`.

## Connection

- **Connect** to `$XDG_RUNTIME_DIR/deskbrid.sock`
- **Subscribe** to events you want pushed
- **Send** action messages and read responses
- **Ping** with `{"type": "ping", "id": "..."}` to check liveness
- **Disconnect** with `{"type": "disconnect", "id": "..."}` or close socket

---

*Protocol version: 0.1. Evolving with the desktop.*
