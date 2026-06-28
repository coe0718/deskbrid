# Agent-to-Agent Messaging

Send and receive messages between Deskbrid agents running on different
sessions, devices, or within the same instance. Supports direct messages,
broadcasts, a persistent mailbox, agent registration, and heartbeat monitoring.

## Actions

### agent.message

Send a direct message to a specific agent session.

| Parameter   | Type          | Description                           |
|-------------|---------------|---------------------------------------|
| `to_session`| string        | Target agent session ID               |
| `subject`   | string        | Message subject / type                |
| `body`      | JSON value    | Message payload                       |
| `ttl_ms`    | uint?         | Time-to-live in ms before expiry      |
| `reply_to`  | string?       | Message ID this is a reply to         |

```bash
deskbrid agent.message '{"to_session": "session-abc", "subject": "status.request", "body": {"query": "health"}}'
```

```json
{
  "type": "agent.message",
  "to_session": "session-abc",
  "subject": "status.request",
  "body": {"query": "health"},
  "ttl_ms": 30000
}
```

### agent.broadcast

Broadcast a message to all connected agents.

| Parameter      | Type          | Description                           |
|----------------|---------------|---------------------------------------|
| `subject`      | string        | Message subject / type                |
| `body`         | JSON value    | Message payload                       |
| `exclude_self` | bool?         | If true, sender doesn't receive its own broadcast |

```bash
deskbrid agent.broadcast '{"subject": "status.request", "body": {"from": "session-xyz"}, "exclude_self": true}'
```

```json
{
  "type": "agent.broadcast",
  "subject": "announce",
  "body": {"text": "I'm going offline for maintenance"},
  "exclude_self": true
}
```

### agent.mailbox

Retrieve all undelivered messages from the agent's mailbox. Messages sent
while the agent was offline are stored until collected.

```bash
deskbrid agent.mailbox
```

No parameters.

### agent.register

Register this agent with a name and capabilities so other agents can discover
and message it.

| Parameter                | Type          | Description                             |
|--------------------------|---------------|-----------------------------------------|
| `name`                   | string        | Agent display name                      |
| `agent_type`             | string?       | Type/role (e.g. `worker`, `monitor`)   |
| `capabilities`           | string[]      | List of action capabilities            |
| `metadata`               | JSON value?   | Arbitrary agent metadata               |
| `heartbeat_interval_ms`  | uint?         | Heartbeat interval for liveness checks |

```bash
deskbrid agent.register '{"name": "worker-1", "capabilities": ["files", "system"], "agent_type": "worker", "heartbeat_interval_ms": 15000}'
```

```json
{
  "type": "agent.register",
  "name": "worker-1",
  "capabilities": ["files", "system"],
  "agent_type": "worker",
  "metadata": {"version": "1.0.0"},
  "heartbeat_interval_ms": 15000
}
```

### agent.list

List all registered agents on the network.

```bash
deskbrid agent.list
```

No parameters.

### agent.get

Get details about a specific registered agent.

| Parameter | Type   | Description   |
|-----------|--------|---------------|
| `name`    | string | Agent name    |

```bash
deskbrid agent.get '{"name": "worker-1"}'
```

### agent.heartbeat

Send a liveness signal for a registered agent. Used to indicate the agent is
still alive and operational.

| Parameter | Type   | Description   |
|-----------|--------|---------------|
| `name`    | string | Agent name    |

```bash
deskbrid agent.heartbeat '{"name": "worker-1"}'
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Register this agent
client.agent_register(
    name="my-agent",
    capabilities=["input", "system", "notifications"],
    heartbeat_interval_ms=30000
)

# List other agents
agents = client.agent_list()
print("Connected agents:", [a["name"] for a in agents])

# Send a message to another agent
client.agent_message(
    to_session="other-agent-session-id",
    subject="ping",
    body={"timestamp": "2024-01-15T10:00:00Z"}
)

# Check mailbox for offline messages
inbox = client.agent_mailbox()
for msg in inbox:
    print(f"From: {msg['from_session']}, Subject: {msg['subject']}")
```

## Requirements

- Agent messaging requires the Deskbrid daemon to be connected to the
  messaging backend (configured in `config.toml`).
- Mailbox storage uses the daemon's state directory.
- Heartbeat intervals are advisory; agents that miss 3 consecutive heartbeats
  may be considered disconnected.

## Current Status

**Experimental** — v1.0.0 feature.
