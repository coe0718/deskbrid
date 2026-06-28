# Lock / Mutex Primitives

Acquire and release distributed mutexes for coordinating access to shared
resources across multiple agents. Locks are tracked server-side with TTL
(time-to-live) to handle agent crashes gracefully.

## Actions

### lock.acquire

Acquire a named mutex lock for a shared resource.

| Parameter  | Type    | Description                                              |
|------------|---------|----------------------------------------------------------|
| `resource` | string  | Name of the resource to lock (e.g., `clipboard`, `screen`)|
| `holder`   | string? | Identifier for the lock holder (default: auto-assigned)  |
| `ttl_ms`   | uint?   | Time-to-live in milliseconds before lock auto-release    |
| `wait_ms`  | uint?   | Max time to wait if the lock is held by another agent    |
| `force`    | bool    | Force-acquire the lock, releasing any existing holder    |

```bash
deskbrid lock.acquire '{"resource": "clipboard", "ttl_ms": 30000, "wait_ms": 5000}'
```

```json
{
  "type": "lock.acquire",
  "resource": "clipboard",
  "ttl_ms": 30000,
  "wait_ms": 5000
}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "token": "lock_clipboard_abc123",
    "acquired": true,
    "holder": "agent-1"
  }
}
```

If the lock is already held:

```json
{
  "type": "response",
  "status": "error",
  "error": "Resource 'clipboard' is locked by 'agent-2' (expires in 15s)"
}
```

### lock.release

Release a previously acquired lock by token.

| Parameter  | Type   | Description                    |
|------------|--------|--------------------------------|
| `resource` | string | Name of the resource           |
| `token`    | string | Token returned from acquire    |

```bash
deskbrid lock.release '{"resource": "clipboard", "token": "lock_clipboard_abc123"}'
```

```json
{"type": "lock.release", "resource": "clipboard", "token": "lock_clipboard_abc123"}
```

### lock.list

List all currently held locks and their holders.

```bash
deskbrid lock.list
```

```json
{"type": "lock.list"}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "resource": "clipboard",
      "holder": "agent-1",
      "acquired_at": 1705312800,
      "expires_at": 1705312830,
      "token": "lock_clipboard_abc123"
    }
  ]
}
```

## Python Example

```python
from deskbrid import Deskbrid
import time

client = Deskbrid()

# Acquire clipboard lock with 30s TTL
result = client.lock_acquire(
    resource="clipboard",
    ttl_ms=30000,
    wait_ms=5000,
)
if result["acquired"]:
    token = result["token"]
    try:
        # Do work with clipboard
        client.clipboard_write("Shared data")
    finally:
        # Always release
        client.lock_release(resource="clipboard", token=token)
```

## Safety

- Locks are server-side with automatic TTL expiry — a crashed agent won't
  hold a lock forever
- The `force` flag should be used sparingly; it can preempt other agents
- Use locks whenever multiple agents may access the same resource
  (clipboard, screen, filesystem paths)

## Current Status

**Stable** — acquire, release, and list locks supported.
