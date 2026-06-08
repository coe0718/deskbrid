# Blackboard

Deskbrid v1.0.0 exposes a namespace-scoped key/value store via
`blackboard.get`, `blackboard.set`, `blackboard.delete`, and
`blackboard.list`. Data is persisted in SQLite and survives daemon restarts.

## Overview

- Keys are scoped to a `namespace` string. Omit it to use `"default"`.
- Values are stored as opaque strings; the daemon does not interpret or parse
  them.
- There is no TTL. Data remains until explicitly deleted.

## Read

```bash
deskbrid blackboard.get { key: "counter", namespace: "agent-1" }
```

Success:

```json
{"type":"response","status":"ok","data":{"key":"counter","value":"42","namespace":"agent-1","updated":"..."}}
```

Not found:

```json
{"type":"response","status":"error","error":{"code":"not_found","message":"Key not found: counter"}}
```

## Write

```bash
deskbrid blackboard.set {
  key: "counter",
  value: "42",
  namespace: "agent-1"
}
```

## Delete

```bash
deskbrid blackboard.delete { key: "counter", namespace: "agent-1" }
```

## List

```bash
deskbrid blackboard.list { namespace: "agent-1" }
```

Response:

```json
{"type":"response","status":"ok","data":{"keys":[{"key":"counter","value":"42","updated":"..."}]}}
```

Omit `namespace` to list across all namespaces.

## Examples

### Multi-agent task coordination

```bash
# producer
deskbrid blackboard.set {
  key: "current_task",
  value: "implement-feature",
  namespace: "project-x"
}

# consumer
deskbrid blackboard.get { key: "current_task", namespace: "project-x" }
```

### Shared config

```bash
deskbrid blackboard.set {
  key: "api_endpoint",
  value: "https://api.example.com",
  namespace: "shared"
}
```

### Work queue

```bash
deskbrid blackboard.set { key: "work_001", value: "process-file-a.txt", namespace: "queue" }
deskbrid blackboard.list { namespace: "queue" }
deskbrid blackboard.delete { key: "work_001", namespace: "queue" }
```

## Python example

```python
from deskbrid import Deskbrid

client = Deskbrid()
client.blackboard_set(key="counter", value="0", namespace="agent-1")

result = client.blackboard_get(key="counter", namespace="agent-1")
print(result["value"])

for key in client.blackboard_list(namespace="agent-1")["keys"]:
    print(key["key"], "=", key["value"])

client.blackboard_delete(key="counter", namespace="agent-1")
```
