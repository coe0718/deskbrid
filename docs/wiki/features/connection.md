# Connection

Manage the daemon connection lifecycle — subscribe to events, unsubscribe,
and disconnect cleanly.

## Actions

### connection.subscribe

Subscribe to one or more event patterns. The daemon pushes events matching
the subscribed patterns to the client in real-time.

| Parameter | Type     | Description                                       |
|-----------|----------|---------------------------------------------------|
| `events`  | string[] | Event patterns to subscribe to (supports wildcards) |

Event pattern examples:
- `screencast.*` — all screencast events
- `update.*` — update-related events
- `notification.*` — notification events
- `rule.*` — rule engine events
- `agent.*` — agent messaging events

```bash
deskbrid subscribe '["screencast.*", "notification.*"]'
```

```json
{"type": "connection.subscribe", "events": ["screencast.*", "notification.*"]}
```

The daemon will push events as they occur:

```json
{"type": "event", "event": "screencast.started", "data": {"output_path": "/tmp/rec.mp4"}}
```

### connection.unsubscribe

Unsubscribe from one or more event patterns.

| Parameter | Type     | Description                            |
|-----------|----------|----------------------------------------|
| `events`  | string[] | Event patterns to unsubscribe from     |

```json
{"type": "connection.unsubscribe", "events": ["screencast.*"]}
```

### connection.disconnect

Disconnect from the daemon cleanly.

```bash
deskbrid disconnect
```

```json
{"type": "connection.disconnect"}
```

No response is sent — the connection is closed immediately.

## Python Example

```python
from deskbrid import Deskbrid
client = Deskbrid()

# Subscribe to events
client.connection_subscribe(events=["screencast.*", "update.*"])

# Later, unsubscribe
client.connection_unsubscribe(events=["screencast.*"])

# Disconnect
client.connection_disconnect()
```

## Current Status

**Stable** — subscribe, unsubscribe, and disconnect supported.
