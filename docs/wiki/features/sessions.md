# Sessions

Deskbrid v0.11.0 introduces named sessions that provide isolated variable namespaces for multi-agent coordination. Sessions survive daemon restarts via SQLite persistence.

## Overview

Each connection to the deskbrid daemon gets a session. Sessions have:
- A unique name (or auto-generated ID if unnamed)
- An isolated variable namespace (variables don't leak between sessions)
- Persistent storage of variables via SQLite
- Ability to clone variables from existing sessions

## Creating a Session

```bash
deskbrid session.create { name: "agent-1" }
```

### Parameters

- `name`: Human-readable identifier for the session (optional - generates UUID if omitted)
- `clone_from`: Name of existing session to clone variables from (optional)

## Listing Sessions

```bash
deskbrid session.list
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "sessions": [
      {
        "name": "agent-1",
        "variables": 5,
        "created": "2026-05-30T10:00:00Z",
        "last_used": "2026-05-30T10:30:00Z"
      },
      {
        "name": "agent-2",
        "variables": 12,
        "created": "2026-05-30T10:05:00Z",
        "last_used": "2026-05-30T10:25:00Z"
      }
    ]
  }
}
```

## Switching Sessions

```bash
deskbrid session.switch { name: "agent-2" }
```

This changes the current session context for subsequent commands.

## Destroying a Session

```bash
deskbrid session.destroy { name: "agent-1" }
```

## Session Variables

Each session has its own isolated key-value store for variables.

### Setting a Variable

```bash
deskbrid session.var.set { name: "counter", value: "42" }
```

### Getting a Variable

```bash
deskbrid session.var.get { name: "counter" }
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "name": "counter",
    "value": "42",
    "session": "agent-1"
  }
}
```

### Listing Variables

```bash
deskbrid session.var.list
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "variables": [
      {
        "name": "counter",
        "value": "42",
        "updated": "2026-05-30T10:30:00Z"
      },
      {
        "name": "username",
        "value": "alice",
        "updated": "2026-05-30T10:25:00Z"
      }
    ]
  }
}
```

## Examples

### Isolated Workspaces for Different Agents

```bash
# Create session for coding agent
deskbrid session.create { name: "coder" }
deskbrid session.var.set { name: "current_file", value: "main.rs" }
deskbrid session.var.set { name: "task", value: "implement feature" }

# Create session for testing agent
deskbrid session.create { name: "tester" }
deskbrid session.var.set { name: "test_suite", value: "integration" }
deskbrid session.var.set { name: "status", value: "pending" }

# Switch between agents
deskbrid session.switch { name: "coder" }
# ... do coding work ...
deskbrid session.switch { name: "tester" }
# ... do testing work ...
```

### Cloning Session State

```bash
# Create base session with common variables
deskbrid session.create { name: "base" }
deskbrid session.var.set { name: "project_dir", value: "/home/user/project" }
deskbrid session.var.set { name: "language", value: "Rust" }

# Create worker sessions that inherit base state
deskbrid session.create { name: "worker-1", clone_from: "base" }
deskbrid session.create { name: "worker-2", clone_from: "base" }

# Workers can override or add their own variables
deskbrid session.var.set { name: "worker_id", value: "1" }  # in worker-1 session
deskbrid session.var.set { name: "worker_id", value: "2" }  # in worker-2 session
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Create a session
session_name = client.session_create(name="agent-1")
print(f"Created session: {session_name}")

# List sessions
sessions = client.session_list()
for session in sessions['sessions']:
    print(f"Session: {session['name']} ({session['variables']} variables)")

# Switch to the session
client.session_switch(name="agent-1")

# Set variables
client.session_var_set(name="counter", value="0")
client.session_var_set(name="status", value="active")

# Get a variable
counter = client.session_var_get(name="counter")
print(f"Counter: {counter['value']}")

# List all variables
vars = client.session_var_list()
for var in vars['variables']:
    print(f"{var['name']} = {var['value']}")

# Create another session by cloning
client.session_create(name="agent-2", clone_from="agent-1")

# Switch back and destroy when done
client.session_switch(name="agent-1")
client.session_destroy(name="agent-2")
```