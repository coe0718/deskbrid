# Events

Subscribe to real-time desktop events using `event.subscribe` and
`event.unsubscribe`.

## Subscribe

```json
{"type":"event.subscribe","events":["window.*","clipboard.*"]}
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
{"type":"event.unsubscribe","events":["window.*"]}
```

## Event format

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

## Available events

| Event | Data |
|---|---|
| `window.focused` | `window_id`, `app_id` |
| `window.unfocused` | `window_id`, `app_id` |
| `window.opened` | `window_id`, `title`, `app_id` |
| `window.closed` | `window_id`, `app_id` |
| `window.moved` | `window_id`, `x`, `y` |
| `window.resized` | `window_id`, `width`, `height` |
| `clipboard.changed` | `content_type`, preview |
| `clipboard.history.added` | `entry_id`, `text` |
| `input.keyboard` | `key`, `combo` |
| `input.mouse.click` | `x`, `y`, `button` |
| `input.mouse.scroll` | `dx`, `dy` |
| `monitor.connected` | `output`, `width`, `height` |
| `monitor.disconnected` | `output` |
| `monitor.changed` | `output`, `scale`, `rotation` |

## Pattern matching

```json
{"type":"event.subscribe","events":["window.*","monitor.*"]}
{"type":"event.subscribe","events":["*"]}
```

## Python example

```python
import socket, json

sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect("/run/user/1000/deskbrid.sock")

sock.send(b'{"type":"event.subscribe","events":["window.*"]}\n')

while True:
    data = b""
    while b"\n" not in data:
        data += sock.recv(4096)
    line = data.split(b"\n")[0]
    event = json.loads(line)
    if event.get("type") == "event":
        print(event["event"], event["data"])
```

## Asyncio example

```python
async def watch(client, patterns):
    await client.send({"type": "event.subscribe", "events": patterns})
    while True:
        event = await client.recv()
        if event.get("type") == "event":
            yield event["event"], event["data"]
```
