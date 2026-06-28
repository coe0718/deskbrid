# Clients

List connected agent clients. Each active connection to the daemon is tracked
with metadata including the client type, protocol version, and connection age.

## Actions

### clients.list

List all currently connected clients.

```bash
deskbrid clients.list
```

```json
{"type": "clients.list"}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "client_id": "sock_abc123",
      "agent_name": "agent-1",
      "protocol": "json",
      "connected_since": 1705312800,
      "address": "/run/user/1000/deskbrid.sock"
    }
  ]
}
```

## Python Example

```python
from deskbrid import Deskbrid
client = Deskbrid()

clients = client.clients_list()
for c in clients:
    print(f"{c['agent_name']} connected since {c['connected_since']}")
```

## Current Status

**Stable** — listing connected clients.
