# Event Subscription

Subscribe to real-time events from the desktop.

## Subscribe to Events

```json
{"action": "events.subscribe", "events": ["window.*", "clipboard.*"]}
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "subscribed": ["window.*", "clipboard.*"]
  }
}
```

## Unsubscribe

```json
{"action": "events.unsubscribe", "events": ["window.*"]}
```

## Event Format

```json
{
  "type": "event",
  "event": "window.focused",
  "data": {
    "window_id": "12345678",
    "app_id": "org.gnome.Terminal"
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

## Available Events

### Window Events

| Event | Description | Data |
|-------|-------------|------|
| `window.focused` | Window gained focus | `window_id`, `app_id` |
| `window.unfocused` | Window lost focus | `window_id`, `app_id` |
| `window.opened` | New window opened | `window_id`, `title`, `app_id` |
| `window.closed` | Window closed | `window_id`, `app_id` |
| `window.moved` | Window moved | `window_id`, `x`, `y` |
| `window.resized` | Window resized | `window_id`, `width`, `height` |

### Clipboard Events

| Event | Description | Data |
|-------|-------------|------|
| `clipboard.changed` | Clipboard content changed | `content_type`, preview |
| `clipboard.history.added` | History entry added | `entry_id`, `text` |

### Input Events

| Event | Description | Data |
|-------|-------------|------|
| `input.keyboard` | Key pressed | `key`, `combo` |
| `input.mouse.click` | Mouse clicked | `x`, `y`, `button` |
| `input.mouse.scroll` | Mouse scrolled | `dx`, `dy` |

### Monitor Events

| Event | Description | Data |
|-------|-------------|------|
| `monitor.connected` | Display connected | `output`, `width`, `height` |
| `monitor.disconnected` | Display disconnected | `output` |
| `monitor.changed` | Display settings changed | `output`, `scale`, `rotation` |

## Pattern Matching

Use wildcards to subscribe to multiple events:

```json
{"action": "events.subscribe", "events": ["window.*", "monitor.*"]}
```

Subscribe to all events:

```json
{"action": "events.subscribe", "events": ["*"]}
```

## Python Example

```python
import socket
import json

sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect("/run/user/1000/deskbrid.sock")

# Subscribe to events
sock.send(b'{"action": "events.subscribe", "events": ["window.*"]}\n')

# Read events
while True:
    data = sock.recv(4096)
    for line in data.decode().strip().split('\n'):
        event = json.loads(line)
        if event.get("type") == "event":
            print(f"{event['event']}: {event['data']}")
```

## Integration with Async Python

```python
import asyncio
import json

async def watch_events(client, patterns):
    """Watch for events matching patterns."""
    await client.send({"action": "events.subscribe", "events": patterns})
    
    while True:
        event = await client.recv()
        if event.get("type") == "event":
            yield event["event"], event["data"]

# Usage
async for event_type, data in watch_events(client, ["window.*"]):
    print(f"Event: {event_type}")
```