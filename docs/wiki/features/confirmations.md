# Action Confirmation

Confirm or deny pending actions before they execute. This is the core of
Deskbrid's safety model — privileged or destructive actions require
user approval before they proceed.

## Actions

### confirm.action

Approve a pending action by its confirmation ID.

| Parameter | Type   | Description        |
|-----------|--------|--------------------|
| `id`      | string | Confirmation ID    |

```bash
deskbrid confirm.action '{"id": "confirm-abc123"}'
```

```json
{
  "type": "confirm.action",
  "id": "confirm-abc123"
}
```

### confirm.deny

Reject/deny a pending action.

| Parameter | Type   | Description        |
|-----------|--------|--------------------|
| `id`      | string | Confirmation ID    |

```bash
deskbrid confirm.deny '{"id": "confirm-abc123"}'
```

```json
{
  "type": "confirm.deny",
  "id": "confirm-abc123"
}
```

### confirm.list

List all pending action confirmations.

```bash
deskbrid confirm.list
```

No parameters.

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "id": "confirm-abc123",
      "action": "system.power",
      "params": {"action": "shutdown"},
      "requested_by": "automation",
      "requested_at": "2024-01-15T10:00:00Z",
      "expires_at": "2024-01-15T10:00:30Z"
    }
  ]
}
```

## Confirmation Flow

1. An action marked as requiring confirmation is dispatched.
2. Deskbrid pauses execution and creates a confirmation entry.
3. The client receives a pending confirmation event.
4. The user (or automated responder) sends `confirm.action` or `confirm.deny`.
5. If confirmed, the original action executes. If denied or expired, it's
   discarded.

### Timeout

Pending confirmations expire after 30 seconds by default. Expired confirmations
are treated as denied.

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Check pending confirmations
pending = client.confirm_list()
if pending:
    # Auto-confirm any pending actions
    for confirm in pending:
        print(f"Confirming: {confirm['action']} (id: {confirm['id']})")
        client.confirm_action(id=confirm["id"])
```

## Safety

- The confirmation system is the primary safety boundary for destructive operations.
- Rules engine actions that require confirmation will also generate
  confirmation entries.
- The `permissions.toml` config controls which actions require confirmation.

## Current Status

**Stable** — v1.0.0 core safety feature.
