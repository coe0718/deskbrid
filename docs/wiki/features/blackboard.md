# Blackboard

Deskbrid's blackboard provides a namespace-scoped key-value store for multi-agent coordination. Data persists in SQLite and survives daemon restarts.

## Overview

The blackboard is a shared space where agents can store and retrieve data. Key features:
- Namespace-scoped keys (prevents collisions between agents)
- Persistent storage via SQLite
- Simple set/get/delete/list operations
- No TTL or expiration (data persists until explicitly deleted)

## Setting a Value

```bash
deskbrid blackboard set { key: "counter", value: "42", namespace: "agent-1" }
```

### Parameters

- `key`: The key to store the value under
- `value`: The value to store (string)
- `namespace`: Optional namespace to scope the key (defaults to "default")

## Getting a Value

```bash
deskbrid blackboard get { key: "counter", namespace: "agent-1" }
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "key": "counter",
    "value": "42",
    "namespace": "agent-1",
    "updated": "2026-05-30T10:30:00Z"
  }
}
```

If the key doesn't exist:
```json
{
  "type": "response",
  "status": "error",
  "error": {
    "code": "not_found",
    "message": "Key not found: counter"
  }
}
```

## Listing Keys

```bash
deskbrid blackboard list { namespace: "agent-1" }
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "keys": [
      {
        "key": "counter",
        "value": "42",
        "updated": "2026-05-30T10:30:00Z"
      },
      {
        "key": "username",
        "value": "alice",
        "updated": "2026-05-30T10:25:00Z"
      }
    ]
  }
}
```

To list all namespaces, omit the namespace parameter:
```bash
deskbrid blackboard list
```

## Deleting a Key

```bash
deskbrid blackboard delete { key: "counter", namespace: "agent-1" }
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "message": "Key deleted"
  }
}
```

## Examples

### Multi-Agent Task Coordination

```bash
# Agent 1 sets a task
deskbrid blackboard set { key: "current_task", value: "implement-feature", namespace: "project-x" }

# Agent 2 reads the task
deskbrid blackboard get { key: "current_task", namespace: "project-x" }

# Agent 1 updates progress
deskbrid blackboard set { key: "progress", value: "50%", namespace: "project-x" }

# Agent 2 checks progress
deskbrid blackboard get { key: "progress", namespace: "project-x" }
```

### Shared Configuration

```bash
# Store shared configuration
deskbrid blackboard set { key: "api_endpoint", value: "https://api.example.com", namespace: "shared" }
deskbrid blackboard set { key: "timeout", value: "30", namespace: "shared" }

# All agents can read the configuration
deskbrid blackboard get { key: "api_endpoint", namespace: "shared" }
deskbrid blackboard get { key: "timeout", namespace: "shared" }
```

### Work Queue Pattern

```bash
# Producer adds work items
deskbrid blackboard set { key: "work_001", value: "process-file-a.txt", namespace: "queue" }
deskbrid blackboard set { key: "work_002", value: "process-file-b.txt", namespace: "queue" }

# Consumer lists and processes work
deskbrid blackboard list { namespace: "queue" }
# ... process items ...
deskbrid blackboard delete { key: "work_001", namespace: "queue" }
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Set a value in a namespace
client.blackboard_set(
    key="counter",
    value="0",
    namespace="agent-1"
)

# Get a value
result = client.blackboard_get(key="counter", namespace="agent-1")
print(f"Counter: {result['value']}")

# List all keys in a namespace
keys = client.blackboard_list(namespace="agent-1")
for key in keys['keys']:
    print(f"{key['key']} = {key['value']}")

# Set multiple related values
client.blackboard_set(key="username", value="alice", namespace="agent-1")
client.blackboard_set(key="role", value="developer", namespace="agent-1")

# Delete a key
client.blackboard_delete(key="counter", namespace="agent-1")

# List all namespaces (requires iterating through all keys or direct DB access)
# For now, you'd need to query the SQLite database directly to see all namespaces
```