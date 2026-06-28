# Audit

Query the action audit log and clear history. Every action dispatched through
the daemon is logged with a timestamp, action type, status, and originating
agent session.

## Actions

### audit.log

Retrieve audit log entries.

| Parameter     | Type     | Description                                          |
|---------------|----------|------------------------------------------------------|
| `limit`       | uint?    | Max entries to return (default: 50)                  |
| `action_type` | string?  | Filter by action type (e.g., `windows.list`, `system.info`) |
| `status`      | string?  | Filter by status (`ok`, `error`, `denied`)           |

```bash
deskbrid audit.log '{"limit": 10}'
deskbrid audit.log '{"action_type": "windows.list", "status": "ok"}'
```

```json
{"type": "audit.log", "limit": 10}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "id": 1423,
      "timestamp": 1705312800,
      "action_type": "windows.list",
      "status": "ok",
      "agent": "agent-1",
      "duration_ms": 12
    },
    {
      "id": 1422,
      "timestamp": 1705312799,
      "action_type": "system.info",
      "status": "error",
      "agent": "agent-1",
      "duration_ms": 5,
      "error": "Permission denied"
    }
  ]
}
```

### audit.clear

Clear all audit log entries.

```bash
deskbrid audit.clear
```

```json
{"type": "audit.clear"}
```

## Python Example

```python
from deskbrid import Deskbrid
client = Deskbrid()

# Get recent errors
errors = client.audit_log(status="error", limit=20)
for entry in errors:
    print(f"[{entry['timestamp']}] {entry['action_type']}: {entry.get('error', '')}")

# Clear log
client.audit_clear()
```

## Storage

The audit log is stored in SQLite at `~/.config/deskbrid/deskbrid.db` in the
`audit_log` table. Logs are automatically trimmed to prevent unbounded growth
(configurable via `permissions.toml`).

## Current Status

**Stable** — log querying and clearing supported.
