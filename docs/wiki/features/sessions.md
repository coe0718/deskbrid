# Sessions (Named Sessions)

Create, destroy, list, and switch between named agent sessions. Each session
has its own isolated variable namespace, persisted across daemon restarts.

## Actions

### session.create

Create a new named session.

| Parameter | Type   | Description                      |
|-----------|--------|----------------------------------|
| `name`    | string | Unique name for the session      |

```bash
deskbrid session.create '{"name": "agent-1"}'
```

```json
{"type": "session.create", "name": "agent-1"}
```

### session.destroy

Destroy a session by ID.

| Parameter | Type   | Description       |
|-----------|--------|-------------------|
| `id`      | string | Session ID to destroy |

```bash
deskbrid session.destroy '{"id": "sess_abc123"}'
```

```json
{"type": "session.destroy", "id": "sess_abc123"}
```

### session.list

List all active sessions.

```bash
deskbrid session.list
```

```json
{"type": "session.list"}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "id": "sess_abc123",
      "name": "agent-1",
      "created_at": 1705312800,
      "variables": {"counter": "42"}
    }
  ]
}
```

### session.switch

Switch to a different session by ID or name.

| Parameter | Type     | Description                    |
|-----------|----------|--------------------------------|
| `id`      | string?  | Session ID to switch to        |
| `name`    | string?  | Session name to switch to      |

```bash
deskbrid session.switch '{"name": "agent-2"}'
```

```json
{"type": "session.switch", "name": "agent-2"}
```

### session.var.set

Set a variable in the current session's namespace.

| Parameter | Type   | Description          |
|-----------|--------|----------------------|
| `name`    | string | Variable name        |
| `value`   | string | Variable value       |

```bash
deskbrid session.var.set '{"name": "counter", "value": "42"}'
```

```json
{"type": "session.var.set", "name": "counter", "value": "42"}
```

### session.var.get

Get a variable from the current session's namespace.

| Parameter | Type   | Description    |
|-----------|--------|----------------|
| `name`    | string | Variable name  |

```bash
deskbrid session.var.get '{"name": "counter"}'
```

```json
{"type": "session.var.get", "name": "counter"}
```

### session.var.list

List all variables in the current session.

```bash
deskbrid session.var.list
```

```json
{"type": "session.var.list"}
```

## Python Example

```python
from deskbrid import Deskbrid
client = Deskbrid()

# Create sessions for two agents
client.session_create(name="coder")
client.session_create(name="tester")

# List sessions
sessions = client.session_list()
print(sessions)

# Switch to coder session and set a variable
client.session_switch(name="coder")
client.session_var_set(name="current_file", value="main.rs")

# Switch to tester and list its variables
client.session_switch(name="tester")
client.session_var_list()

# Destroy the coder session
client.session_destroy(name="coder")
```

## Storage

Sessions and their variables are stored in SQLite at
`~/.config/deskbrid/deskbrid.db` and survive daemon restarts.

## Related

- [Persistence](persistence.md) — SQLite storage overview
- [Blackboard](blackboard.md) — shared cross-session key/value storage
- [Rules](rules.md) — rule engine can trigger on session events

## Current Status

**Stable** — session lifecycle and variable management supported.
