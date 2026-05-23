# Protocol Overview

Deskbrid uses a simple JSON-over-Unix-socket protocol for communicating with the daemon.

## Connection

The daemon listens on a Unix socket at `/run/user/$UID/deskbrid.sock`.

```bash
# Default socket path
/run/user/1000/deskbrid.sock
```

## Message Format

All messages are JSON objects terminated by a newline character.

### Request

```json
{"action": "windows.list"}
```

With parameters:

```json
{
  "action": "input.keyboard",
  "action": "type",
  "text": "Hello!"
}
```

### Response

Success:

```json
{
  "type": "response",
  "status": "ok",
  "data": [...]
}
```

Error:

```json
{
  "type": "response",
  "status": "error",
  "error": {
    "code": "not_found",
    "message": "Window not found"
  }
}
```

### Event

```json
{
  "type": "event",
  "event": "window.focused",
  "data": {
    "window_id": "12345678"
  }
}
```

## Actions

The protocol supports over 90 actions organized by domain:

### System
- `system.info` - Get system information
- `system.capabilities` - List supported features
- `system.health` - Check system health
- `system.power` - Power actions (suspend, reboot, shutdown)
- `system.battery` - Battery status
- `system.idle` - Idle detection

### Windows
- `windows.list` - List all windows
- `windows.focus` - Focus a window
- `windows.get` - Get window details
- `windows.close` - Close a window
- `windows.tile` - Tile window to preset position
- `windows.activate_or_launch` - Find or start an app

### Input
- `input.keyboard` - Type text or send key combinations
- `input.mouse` - Move, click, scroll

### Clipboard
- `clipboard.read` - Read clipboard
- `clipboard.write` - Write to clipboard
- `clipboard.history` - Get clipboard history

### Screenshot
- `screenshot` - Capture screen
- `screenshot.ocr` - Capture with OCR
- `screenshot.diff` - Compare screenshots

### Notifications
- `notification.send` - Send desktop notification
- `notification.close` - Close notification

### Services
- `service.list` - List systemd services
- `service.start` - Start a service
- `service.stop` - Stop a service

### Terminals
- `terminal.create` - Create PTY session
- `terminal.write` - Write to terminal
- `terminal.read` - Read from terminal

See [full action list](../protocol/actions.md) for all available actions.

## Event Subscription

Subscribe to real-time events:

```json
{"action": "events.subscribe", "events": ["window.*", "input.*"]}
```

Available event patterns:
- `window.*` - Window events (focus, close, open)
- `input.*` - Input events
- `clipboard.*` - Clipboard changes
- `monitor.*` - Display changes

Events are streamed continuously until you unsubscribe or close the connection.

## Python Example

```python
import socket
import json

sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect("/run/user/1000/deskbrid.sock")

# Send request
sock.send(b'{"action": "windows.list"}\n')

# Read response
response = sock.recv(4096)
data = json.loads(response)
print(data)
```

## Error Codes

| Code | Description |
|------|-------------|
| `invalid_params` | Invalid or missing parameters |
| `not_found` | Resource not found |
| `permission_denied` | Insufficient permissions |
| `not_supported` | Feature not supported on this system |
| `backend_error` | Backend-specific error |
| `internal_error` | Internal daemon error |

## Async Protocol

For long-running operations, the protocol returns immediately with a request ID:

```json
{"action": "terminal.create"}
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {"terminal_id": "term_123"},
  "request_id": "abc123"
}
```