# Lock / Mutex Primitives

Acquire and release distributed mutexes for coordinating access to shared
resources across multiple agents. Locks are tracked server-side with TTL
(time-to-live) to handle agent crashes gracefully.

## Actions

### `lock.acquire`

Acquire a named mutex lock for a shared resource.

| Parameter | Type    | Required | Description                                                  |
|-----------|---------|----------|--------------------------------------------------------------|
| `resource`| string  | yes      | Name of the resource to lock (e.g., `clipboard`, `screen`)   |
| `holder`  | string  | no       | Identifier for the lock holder (default: auto-assigned)      |
| `ttl_ms`  | uint    | no       | Time-to-live in ms before lock auto-release (default: 30000) |
| `wait_ms` | uint    | no       | Max time in ms to wait if the lock is held (default: 0)      |
| `force`   | bool    | no       | Force-acquire, releasing any existing holder (default: false)|

```json
{
  "type": "lock.acquire",
  "resource": "clipboard",
  "holder": "agent-alpha",
  "ttl_ms": 30000,
  "wait_ms": 5000
}
```

**Response (acquired):**

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "acquired": true,
    "lock": {
      "resource": "clipboard",
      "holder": "agent-alpha",
      "token": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "acquired_at": 1705312800000,
      "expires_at": 1705312830000,
      "ttl_ms": 30000
    },
    "owner": null,
    "timed_out": false,
    "already_held": false
  }
}
```

**Response (already held — wait timed out):**

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "acquired": false,
    "lock": null,
    "owner": {
      "resource": "clipboard",
      "holder": "agent-beta",
      "token": "f6e5d4c3-b2a1-0987-fedc-ba6543210987",
      "acquired_at": 1705312790000,
      "expires_at": 1705312820000,
      "ttl_ms": 30000
    },
    "timed_out": true,
    "already_held": false
  }
}
```

**Response (force-acquire — existing holder preempted):**

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "acquired": true,
    "lock": { "...": "new lock entry ..." },
    "owner": null,
    "timed_out": false,
    "already_held": false
  }
}
```

### `lock.release`

Release a previously acquired lock by resource and token.

| Parameter  | Type   | Required | Description                    |
|------------|--------|----------|--------------------------------|
| `resource` | string | yes      | Name of the resource           |
| `token`    | string | yes      | Token returned from acquire    |

```json
{"type": "lock.release", "resource": "clipboard", "token": "a1b2c3d4-e5f6-7890-abcd-ef1234567890"}
```

**Response:**

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "released": true,
    "lock": null,
    "reason": null
  }
}
```

**Response (invalid token):**

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "released": false,
    "lock": {
      "resource": "clipboard",
      "holder": "agent-beta",
      "token": "f6e5d4c3-b2a1-0987-fedc-ba6543210987",
      "acquired_at": 1705312790000,
      "expires_at": 1705312820000,
      "ttl_ms": 30000
    },
    "reason": "token mismatch"
  }
}
```

### `lock.list`

List all currently held locks with their holders and expiry times.

```json
{"type": "lock.list"}
```

**Response:**

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "locks": [
      {
        "resource": "clipboard",
        "holder": "agent-alpha",
        "token": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
        "acquired_at": 1705312800000,
        "expires_at": 1705312830000,
        "ttl_ms": 30000
      },
      {
        "resource": "window:editor",
        "holder": "agent-beta",
        "token": "f6e5d4c3-b2a1-0987-fedc-ba6543210987",
        "acquired_at": 1705312780000,
        "expires_at": 1705312810000,
        "ttl_ms": 30000
      }
    ],
    "count": 2
  }
}
```

## Events

### `lock.timeout`

Emitted when a lock expires and is automatically released by the server-side sweeper.

```json
{
  "type": "lock.timeout",
  "resource": "clipboard",
  "holder": "agent-alpha",
  "token": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "timestamp": 1705312830
}
```

## Python Example

```python
from deskbrid import Deskbrid
import time

client = Deskbrid()

# Acquire clipboard lock with 30s TTL, wait up to 5s
result = client.lock_acquire(
    resource="clipboard",
    ttl_ms=30000,
    wait_ms=5000,
)

if result["acquired"]:
    token = result["lock"]["token"]
    try:
        # Do work with clipboard
        client.clipboard_write("Shared data")
    finally:
        # Always release
        client.lock_release(resource="clipboard", token=token)
else:
    print(f"Lock held by {result['owner']['holder']} — try again later")
```

## Safety

- Locks are server-side with automatic TTL expiry — a crashed agent won't hold a lock forever
- A background sweeper runs every second to prune expired locks and emits `lock.timeout` events
- The `force` flag should be used sparingly; it preempts the current holder without notification
- The token is a UUIDv4 — keep it secret to prevent third-party release
- Use locks whenever multiple agents may access the same resource (clipboard, screen, filesystem paths)

## Related

- [Architecture — FailGuard](../architecture/failguard.md) — clock recovery if lock sweeper stalls
